use alloy_sol_types::{sol, SolType};
use anyhow::Result;
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    types::{Address, Filter, TransactionReceipt, U256, U64},
    utils::hex,
};
use ethers_aws::aws_signer::AWSSigner;
use log::{debug, error, info};
use std::{env, str::FromStr, sync::Arc};

// Generates the contract bindings for the EigenlayerBeaconOracle contract.
abigen!(
    EigenlayerBeaconOracle,
    "./abi/EigenlayerBeaconOracle.abi.json"
);

// Maximum number of blocks to search backwards for (1 day of blocks).
const MAX_DISTANCE_TO_FILL: u64 = 8191;

type EigenLayerBeaconOracleUpdate = sol! { tuple(uint256, uint256, bytes32) };

/// Asynchronously gets the latest block in the contract.
async fn get_latest_block_in_contract(
    rpc_url: String,
    oracle_address_bytes: Address,
) -> Result<u64> {
    let provider =
        Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");
    let latest_block = provider.get_block_number().await?;

    let mut curr_block = latest_block;
    while curr_block > curr_block - MAX_DISTANCE_TO_FILL {
        let range_start_block = std::cmp::max(curr_block - MAX_DISTANCE_TO_FILL, curr_block - 500);
        // Filter for the events in chunks.
        let filter = Filter::new()
            .from_block(range_start_block)
            .to_block(curr_block)
            .address(vec![oracle_address_bytes])
            .event("EigenLayerBeaconOracleUpdate(uint256,uint256,bytes32)");

        let logs = provider.get_logs(&filter).await?;

        // Get the most recent log from the logs (if any).
        let most_recent_log = logs.iter().max_by_key(|log| log.block_number);
        if let Some(most_recent_log) = most_recent_log {
            let log_bytes = &most_recent_log.data.0;
            let decoded = EigenLayerBeaconOracleUpdate::abi_decode(&log_bytes, true).unwrap();

            let slot: U256 = U256::from_little_endian(&decoded.0.as_le_bytes());

            let consensus_rpc_url = env::var("CONSENSUS_RPC")?;
            let response = reqwest::get(format!(
                "{}/eth/v1/beacon/blocks/{}",
                consensus_rpc_url,
                slot.as_u64()
            ))
            .await?;

            // Get the execution block number for a specific slot.
            let json: serde_json::Value = serde_json::from_str(&response.text().await?).unwrap();
            let block_number = json["data"]["message"]["body"]["execution_payload"]["block_number"]
                .as_str()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap();

            return Ok(block_number);
        }

        curr_block -= U64::from(500u64);
    }

    Err(anyhow::Error::msg(
        "Could not find the latest block in the contract",
    ))
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
    env::set_var("RUST_LOG", "debug");
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

        let contract_curr_block =
            get_latest_block_in_contract(rpc_url.clone(), Address::from(oracle_address_bytes))
                .await?;

        // Check if latest_block + block_interval is less than the current block number.
        let latest_block = client.get_block_number().await?;

        debug!(
            "The contract's current latest update is from block: {} and Goerli's latest block is: {}. Difference: {}",
            contract_curr_block, latest_block, latest_block - contract_curr_block
        );

        // Get contract_curr_block + block_interval - (contract_curr_block % block_interval)
        let block_to_request =
            contract_curr_block + block_interval - (contract_curr_block % block_interval);

        // To avoid RPC stability issues, we use a block number 5 blocks behind the current block.
        if block_to_request < latest_block.as_u64() - 5 {
            debug!(
                "Attempting to add timestamp of block {} to contract",
                block_to_request
            );

            // Check if interval_block_nb is stored in the contract.
            let interval_block = client.get_block(block_to_request).await?;
            let interval_block_timestamp = interval_block.unwrap().timestamp;
            let interval_beacon_block_root = contract
                .timestamp_to_block_root(interval_block_timestamp)
                .call()
                .await?;

            // If the interval block is not in the contract, store it.
            if interval_beacon_block_root == [0; 32] {
                let tx: Option<TransactionReceipt> = contract
                    .add_timestamp(interval_block_timestamp)
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Failed to add timestamp: {}", e);
                        e
                    })?
                    .await
                    .map_err(|e| {
                        error!("Failed to send tx: {}", e);
                        e
                    })?;

                if let Some(tx) = tx {
                    info!(
                        "Added block {:?} to the contract! Transaction: {:?}",
                        block_to_request, tx.transaction_hash
                    );
                }
            }
        }
        debug!("Sleeping for 1 minute");
        // Sleep for 1 minute.
        let _ = tokio::time::sleep(tokio::time::Duration::from_secs((60) as u64)).await;
    }
}
