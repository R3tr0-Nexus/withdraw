pub mod relay;

use ethers::core::{rand::thread_rng, types::transaction::eip2718::TypedTransaction};
use ethers::prelude::{types::{Address, TransactionRequest, Bytes, U256}, Provider, ProviderError, abigen, LocalWallet, Signer, SignerMiddleware, Ws, Middleware, 
abi::Token};
use ethers::types::{BlockNumber, Block, TxHash, U64, BlockId};
use ethers_flashbots::{BundleRequest, BroadcasterMiddleware, PendingBundleError};
//use eyre::Result;
use anyhow::Result;
use crate::relay::get_all_relay_endpoints;
use std::str::FromStr;
use url::Url;
use std::sync::Arc;

// See https://www.mev.to/builders for up to date builder URLs
static BUILDER_URLS: &[&str] = &[
    "https://builder0x69.io",
    "https://rpc.beaverbuild.org",
    "https://relay.flashbots.net",
    "https://rsync-builder.xyz",
    "https://rpc.titanbuilder.xyz",
    "https://api.blocknative.com/v1/auction",
    "https://mev.api.blxrbdn.com",
    "https://eth-builder.com",
    "https://builder.gmbit.co/rpc",
    "https://buildai.net",
    "https://rpc.payload.de",
    "https://rpc.lightspeedbuilder.info",
    "https://rpc.nfactorial.xyz",
];

abigen!(BundleSwap, "src/bundleswap.json");



#[tokio::main]
async fn main() -> Result<(), ProviderError> {
    // Connect to the network
    let provider = Arc::new(Provider::<Ws>::connect("wss://ws-nd-952-680-493.p2pify.com/9c5e0ea6fd64857fe4cff78ca16962f3").await.unwrap());
   
    let my_priv = String::from("9d2c26d9f4c5eeb0794ed9e991ee98d5f7b0f791619f03c26b22f86635c10438");
    let searcher_private_key = String::from("7005b56052be4776bffe00ff781879c65aa87ac3d5f8945c0452f27e11fa9236");
    let searcher_private_key_2 = String::from("7005b56052be4776bffe00ff781879c65aa87ac3d5f8945c0452f27e11fa9236");
    
    let my_signer =  my_priv.parse::<LocalWallet>().unwrap();
    let searcher_signer = searcher_private_key.parse::<LocalWallet>().unwrap();
    let searcher_signer2 = searcher_private_key_2.parse::<LocalWallet>().unwrap();

    // This is your searcher identity
    let bundle_signer = LocalWallet::new(&mut thread_rng());

   

    let addy = Address::from_str("0x5C1201e06F2EB55dDf656F0a82e57cF92F634273").unwrap();
    let contract_addy = Address::from_str("0x1093CB124bbc616D9AA3D4564eEbD901c40714E4").unwrap();
    let my_addy = Address::from_str("0x3A10d7dcB863DcF6865F86846Bd0371Ea8187471").unwrap();

    /* Build to send eth first bundle that pays 0x0000000000000000000000000000000000000000
    let transfer_tx = {
        let mut inner: TypedTransaction = ethers::types::transaction::eip2718::TypedTransaction::Legacy(TransactionRequest {
            to: Some(ethers::types::NameOrAddress::Address(addy)),
            value: Some(U256::from("1300000000000000")), // 1 ether in wei
            ..Default::default()
        });
        inner
    };

    let signature = my_signer.sign_transaction(&transfer_tx).await.unwrap();    
    let tranfer_raw = transfer_tx.rlp_signed(&signature);*/

   
    // get last block number
    
    let latest_block = match provider.get_block(BlockNumber::Latest).await {
        Ok(b) => b.unwrap(),
        Err(e) => return Err(e),
    };

    let block_hash = latest_block.hash.unwrap();
    let block_number = Some(BlockId::from(block_hash));

    let block_number = latest_block.number.unwrap();
    let timestamp = latest_block.timestamp;
    let timestamp = timestamp + U256::from(12);

    let base_fee = calculate_next_block_base_fee(latest_block);
    let max_fee = base_fee + base_fee.clone();

    // Convert the bytes to a hexadecimal representation
    let bundle_swap = BundleSwap::new(contract_addy, provider.clone());
    let data = bundle_swap.withdraw_eth().calldata();


    let withdraw_tx = {
        let mut inner: TypedTransaction = TypedTransaction::Legacy(TransactionRequest {
            from: Some(addy),
            to: Some(ethers::types::NameOrAddress::Address(contract_addy)),   
            gas: Some(U256::from(30000)),
            gas_price: Some(max_fee),   
            value: Some(U256::from(0)),      
            data,
            nonce: Some(58.into()),
            chain_id: Some(U64::from(1)),  
        });
        inner
    };
    
    let signature = searcher_signer2.sign_transaction(&withdraw_tx).await.unwrap(); 
    let withdraw_raw = withdraw_tx.rlp_signed(&signature);

    let final_tx = {        
        let mut inner: TypedTransaction = TypedTransaction::Legacy(TransactionRequest {
            from: Some(addy),
            to: Some(ethers::types::NameOrAddress::Address(my_addy)),
            gas: Some(U256::from(30000)),
            gas_price: Some(max_fee),    
            value: Some(U256::from("15000000000000000")),       
            data: None,
            nonce: Some(59.into()),
            chain_id: Some(U64::from(1)),       
        });
        
        inner
    };

    
    let signature = searcher_signer2.sign_transaction(&final_tx).await.unwrap(); 
    let final_raw = final_tx.rlp_signed(&signature);

      
    let bundle = BundleRequest::new()
        .push_transaction(tranfer_raw)
        .push_transaction(withdraw_raw)
        .push_transaction(final_raw)
        .set_block(block_number + 1)
        .set_simulation_block(block_number)
        .set_simulation_timestamp(timestamp.as_u64())
        .set_min_timestamp(timestamp.as_u64())
        .set_max_timestamp(timestamp.as_u64());


     // Add signer and Flashbots middleware
     let client = SignerMiddleware::new(
        BroadcasterMiddleware::new(
            provider,
            BUILDER_URLS
                .iter()
                .map(|url| Url::parse(url).unwrap())
                .collect(),
            Url::parse("https://relay.flashbots.net").unwrap(),
            bundle_signer,
        ),
        searcher_signer,
    );


    // Send it
    let results = client.inner().send_bundle(&bundle).await.unwrap();

    // You can also optionally wait to see if the bundle was included
    for result in results {
        match result {
            Ok(pending_bundle) => match pending_bundle.await {
                Ok(bundle_hash) => println!(
                    "Bundle with hash {:?} was included in target block",
                    bundle_hash
                ),
                Err(PendingBundleError::BundleNotIncluded) => {
                    println!("Bundle was not included in target block.")
                }
                Err(e) => println!("An error occured: {}", e),
            },
            Err(e) => println!("An error occured: {}", e),
        }
    }

    /* 
    for relay in relay::get_all_relay_endpoints(&provider).await{
        let bundle = bundle.clone();
        tokio::spawn(async move{
            let pending_bundle = match relay.flashbots_client.inner().send_bundle(&bundle).await {
                Ok(pb) => pb,
                Err(e) => {
                    println!("Failed to send bundle: {:?}", e);
                    return;
                }
            };

            let bundle_hash = pending_bundle.bundle_hash;

            match pending_bundle.await {
                Ok(_) => println!(
                    "Bundle with hash {:?} was included in target block",
                    bundle_hash
                ),
                Err(ethers_flashbots::PendingBundleError::BundleNotIncluded) =>  println!("Bundle was not included in target block."),
                Err(e) => println!("An error occured: {}", e),
            };
        });
    }*/



    
     Ok(()) 
    
}


pub fn calculate_next_block_base_fee(block: Block<TxHash>) -> U256 {
    // Get the block base fee per gas
    let current_base_fee_per_gas = block.base_fee_per_gas.unwrap_or_default();

    // Get the mount of gas used in the block
    let current_gas_used = block.gas_used;

    let current_gas_target = block.gas_limit / 2;

    if current_gas_used == current_gas_target {
        current_base_fee_per_gas
    } else if current_gas_used > current_gas_target {
        let gas_used_delta = current_gas_used - current_gas_target;
        let base_fee_per_gas_delta =
            current_base_fee_per_gas * gas_used_delta / current_gas_target / 8;

        return current_base_fee_per_gas + base_fee_per_gas_delta;
    } else {
        let gas_used_delta = current_gas_target - current_gas_used;
        let base_fee_per_gas_delta =
            current_base_fee_per_gas * gas_used_delta / current_gas_target / 8;

        return current_base_fee_per_gas - base_fee_per_gas_delta;
    }
}