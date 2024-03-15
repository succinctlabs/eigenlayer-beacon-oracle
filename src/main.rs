use anyhow::Result;
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    types::TransactionReceipt,
    utils::hex,
};
use ethers_aws::aws_signer::AWSSigner;
use std::{env, str::FromStr, sync::Arc};

// Generates the contract bindings for the EigenlayerBeaconOracle contract.
abigen!(
    EigenlayerBeaconOracle,
    "./abi/EigenlayerBeaconOracle.abi.json"
);

// Maximum number of blocks to search backwards for (1 day of blocks).
const MAX_DISTANCE_TO_FILL: u64 = 7200;

/// Asynchronously gets the latest block in the contract.
async fn get_latest_block_in_contract(
    latest_block: u64,
    block_interval: u64,
    rpc_url: String,
    oracle_address_bytes: [u8; 20],
) -> Result<u64> {
    let provider =
        Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");
    let contract = EigenlayerBeaconOracle::new(oracle_address_bytes, provider.clone().into());

    let mut curr_block_number = latest_block - (latest_block % block_interval);
    loop {
        if curr_block_number < latest_block - MAX_DISTANCE_TO_FILL {
            return Ok(curr_block_number);
        }
        let curr_block = provider.get_block(curr_block_number).await?;
        let curr_block_timestamp = curr_block.unwrap().timestamp;
        let curr_block_root = contract
            .timestamp_to_block_root(curr_block_timestamp)
            .call()
            .await?;

        if curr_block_root != [0u8; 32] {
            return Ok(curr_block_number);
        }
        curr_block_number -= block_interval;
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

/// The main function that runs the application.
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

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

        // Check if latest_block + block_interval is less than the current block number.
        let latest_block = client.get_block_number().await?;

        let contract_curr_block = get_latest_block_in_contract(
            latest_block.as_u64(),
            block_interval,
            rpc_url.clone(),
            oracle_address_bytes,
        )
        .await
        .unwrap();

        println!(
            "The contract's current latest update is from block: {} and Goerli's latest block is: {}. Difference: {}",
            contract_curr_block, latest_block, latest_block - contract_curr_block
        );

        // To avoid RPC stability issues, we use a block number 5 blocks behind the current block.
        if contract_curr_block + block_interval < latest_block.as_u64() - 5 {
            println!(
                "Attempting to add timestamp of block {} to contract",
                contract_curr_block + block_interval
            );
            let interval_block_nb = contract_curr_block + block_interval;

            // Check if interval_block_nb is stored in the contract.
            let interval_block = client.get_block(interval_block_nb).await?;
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
                    .await?
                    .await?;

                if let Some(tx) = tx {
                    println!(
                        "Added block {:?} to the contract! Transaction: {:?}",
                        interval_block_nb, tx.transaction_hash
                    );
                }
            }
        }
        // Sleep for 1 minute.
        let _ = tokio::time::sleep(tokio::time::Duration::from_secs((60) as u64)).await;
    }
}
