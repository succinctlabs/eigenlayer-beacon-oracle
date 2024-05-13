use alloy_sol_types::sol;
use anyhow::Result;
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    types::{Address, TransactionReceipt, U64},
    utils::hex,
};
use ethers_aws::aws_signer::AWSSigner;
use log::{debug, error, info};
use std::{env, str::FromStr, sync::Arc};

// Generates the contract bindings for the EigenlayerBeaconOracle contract.
sol! {
    function addTimestamp(uint256 _targetTimestamp);

    function timestampToBlockRoot(uint256 _targetTimestamp) returns (bytes32);
}

// Maximum number of blocks to search backwards for (1 day of blocks).
const MAX_DISTANCE_TO_FILL: u64 = 8191;

/// Asynchronously gets the block of the latest update to the contract.
/// Find the most recent log, get the slot, and then get the block number from the slot.
async fn get_latest_block_in_contract(
    rpc_url: String,
    oracle_address_bytes: Address,
    block_interval: u64,
) -> Option<u64> {
    let provider =
        Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");
    let latest_block = provider.get_block_number().await.unwrap();

    let contract = EigenlayerBeaconOracle::new(oracle_address_bytes, provider.clone().into());

    // Query backwards over MAX_DISTANCE_TO_FILL blocks to find the most recent update. Find the most recent update which is
    // a multiple of block_interval.
    let mut curr_block_nb = latest_block - (latest_block % block_interval);
    while curr_block_nb > latest_block - MAX_DISTANCE_TO_FILL {
        // Get timestamp of the block.
        let interval_block = provider.get_block(curr_block_nb).await.unwrap();
        let interval_block_timestamp = interval_block.unwrap().timestamp;
        let interval_beacon_block_root = contract
            .timestamp_to_block_root(interval_block_timestamp)
            .call()
            .await
            .unwrap();

        // If the interval block is in the contract, return it.
        if interval_beacon_block_root != [0; 32] {
            return Some(curr_block_nb.as_u64());
        }
        curr_block_nb -= U64::from(block_interval);
    }

    None
}

/// If contract_curr_block is None, set a default start_block. Otherwise, return contract_curr_block + block_interval.
fn get_block_to_request(
    contract_curr_block: Option<u64>,
    block_interval: u64,
    latest_block: u64,
) -> u64 {
    // If the contract's current block is None, we need to set a default start_block.
    if contract_curr_block.is_none() {
        let default_start_block = latest_block - (latest_block % block_interval);
        debug!(
            "Contract has not been updated in {} blocks. Requesting timestamp for block: {}",
            MAX_DISTANCE_TO_FILL, default_start_block
        );
        return default_start_block;
    } else {
        let block_to_request = contract_curr_block.unwrap() + block_interval;
        debug!(
            "Contract's current block is {}. Requesting timestamp for block: {}",
            contract_curr_block.unwrap(),
            block_to_request
        );
        return block_to_request;
    }
}

async fn create_aws_signer() -> AWSSigner {
    let access_key = std::env::var("ACCESS_KEY").expect("ACCESS_KEY must be in environment");
    let secret_access_key =
        std::env::var("SECRET_ACCESS_KEY").expect("SECRET_ACCESS_KEY must be in environment");
    let key_id: String = std::env::var("KEY_ID").expect("KEY_ID must be in environment");
    let region = std::env::var("REGION").expect("REGION must be in environment");
    let chain_id = std::env::var("CHAIN_ID").expect("CHAIN_ID must be in environment");
    let chain_id = u64::from_str(&chain_id).expect("CHAIN_ID must be a number");
    let aws_signer = AWSSigner::new(chain_id, access_key, secret_access_key, key_id, region)
        .await
        .expect("Cannot create AWS signer");
    aws_signer
}

/// The operator for the EigenlayerBeaconOracle contract.
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    env_logger::init();

    let block_interval = env::var("BLOCK_INTERVAL")?;
    let block_interval = u64::from_str(&block_interval)?;

    let rpc_url = env::var("RPC_URL")?;

    let contract_address = env::var("CONTRACT_ADDRESS")?;
    let oracle_address_bytes: [u8; 20] = hex::decode(contract_address).unwrap().try_into().unwrap();

    loop {
        // Replace with your Ethereum node's HTTP endpoint
        let provider =
            Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");

        let signer = create_aws_signer().await;

        let client = Arc::new(SignerMiddleware::new(provider, signer));

        let contract = EigenlayerBeaconOracle::new(oracle_address_bytes, client.clone());

        let latest_block = client.get_block_number().await?;

        // Get the block of the most recent update to the contract. This will always be a multiple of block_interval.
        let contract_curr_block = get_latest_block_in_contract(
            rpc_url.clone(),
            Address::from(oracle_address_bytes),
            block_interval,
        )
        .await;

        let block_nb_to_request =
            get_block_to_request(contract_curr_block, block_interval, latest_block.as_u64());

        // To avoid RPC stability issues, we use a block number 1 block behind the current block.
        if block_nb_to_request < latest_block.as_u64() - 1 {
            debug!(
                "Attempting to add timestamp of block {} to contract",
                block_nb_to_request
            );

            // Check if interval_block_nb is stored in the contract.
            let interval_block = client.get_block(block_nb_to_request).await?;
            let interval_block_timestamp = interval_block.unwrap().timestamp;
            let interval_beacon_block_root = contract
                .timestamp_to_block_root(interval_block_timestamp)
                .call()
                .await?;

            // If the interval block is not in the contract, store it.
            if interval_beacon_block_root == [0; 32] {
                let data = contract
                    .add_timestamp(interval_block_timestamp)
                    .call()
                    .await
                    .unwrap();

                println!("Calldata for adding timestamp: {:?}", data);

                // if let Some(tx) = tx {
                //     info!(
                //         "Added block {:?} to the contract! Transaction: {:?}",
                //         block_nb_to_request, tx.transaction_hash
                //     );
                // }
            }
        }
        debug!("Sleeping for 1 minute");
        // Sleep for 5 minutes.
        let _ = tokio::time::sleep(tokio::time::Duration::from_secs((300) as u64)).await;
    }
}
