use std::sync::Arc;
use ethers::prelude::{LocalWallet, SignerMiddleware, Bytes, U64};
use ethers::providers::{Ws, Provider, Middleware};
use ethers::core::{rand::thread_rng};
use ethers_flashbots::*;
use url::Url;

pub struct BundleRelay {
    pub flashbots_client:
        SignerMiddleware<FlashbotsMiddleware<Arc<Provider<Ws>>, LocalWallet>, LocalWallet>,
    pub relay_name: String,
}

impl BundleRelay {
    pub fn new(
        relay_end_point: Url,
        relay_name: String,
        client: &Arc<Provider<Ws>>,
    ) -> Result<BundleRelay, url::ParseError> {
      
        // Extract wallets from .env keys
       let searcher_private_key = String::from("7005b56052be4776bffe00ff781879c65aa87ac3d5f8945c0452f27e11fa9236");
        
        
        // This is your searcher identity
        let bundle_signer = LocalWallet::new(&mut thread_rng());
        let searcher_signer = searcher_private_key.parse::<LocalWallet>().unwrap();

        // Setup the Ethereum client with flashbots middleware
        let flashbots_middleware =
            FlashbotsMiddleware::new(client.clone(), relay_end_point, bundle_signer);

        // Local node running mev-geth
        let flashbots_client = SignerMiddleware::new(flashbots_middleware, searcher_signer);

        Ok(BundleRelay {
            flashbots_client,
            relay_name,
        })
    }
}

pub fn construct_bundle(
    signed_txs: Vec<Bytes>,
    target_block: U64, // Current block number
    target_timestamp: u64,
) -> BundleRequest {
    // Create ethers-flashbots bundle request
    let mut bundle_request = BundleRequest::new();

    for tx in signed_txs {
        bundle_request = bundle_request.push_transaction(tx);
    }

    // Set other bundle parameters
    bundle_request = bundle_request
        .set_block(target_block)
        .set_simulation_block(target_block - 1)
        .set_simulation_timestamp(target_timestamp)
        .set_min_timestamp(target_timestamp)
        .set_max_timestamp(target_timestamp);

    bundle_request
}

pub async fn get_all_relay_endpoints(client: &Arc<Provider<Ws>>,) -> Vec<BundleRelay> {
    

    let endpoints = vec![
        ("flashbots",     "https://relay.flashbots.net/"),
        ("builder0x69",   "http://builder0x69.io/"),
        ("beaverbuild",   "https://rpc.beaverbuild.org/"),
        ("rsync-builder", "https://rsync-builder.xyz/"),
        ("ultrasound",    "https://relay.ultrasound.money/"),
        ("TitanBuilder",  "https://rpc.titanbuilder.xyz/"),
        ("fb1",           "https://rpc.f1b.io"),
        ("payload.de",    "https://rpc.payload.de"),
        ("BuildAI",       "https://buildai.net/"),
        ("edennetwork",    "https://api.edennetwork.io/v1/bundle"),        
        ("lightspeedbuilder", "https://rpc.lightspeedbuilder.info/"),
        ("eth-builder",    "https://eth-builder.com/"),        
        //"http://relayooor.wtf/",
        //"http://mainnet.aestus.live/",
        //"https://mainnet-relay.securerpc.com",
        //"http://agnostic-relay.net/",
        //"http://relay.ultrasound.money/",
    ];

    let mut relays: Vec<BundleRelay> = vec![];

    for (name, endpoint) in endpoints {
        let relay = BundleRelay::new(Url::parse(endpoint).unwrap(), name.into(), &client.clone()).unwrap();
        relays.push(relay);
    }

    relays
}
