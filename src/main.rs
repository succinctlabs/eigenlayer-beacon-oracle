use anyhow::Result;
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{AwsSigner, LocalWallet, Signer},
    types::{Address, Filter, TransactionReceipt, U64},
    utils::hex,
};
use rusoto_core::Client;
use rusoto_kms::{Kms, KmsClient};
use std::{env, str::FromStr, sync::Arc};

// Generates the contract bindings for the EigenlayerBeaconOracle contract.
abigen!(
    EigenlayerBeaconOracle,
    "./abi/EigenlayerBeaconOracle.abi.json"
);

// Maximum number of blocks to search backwards for (1 day of blocks).
const MAX_DISTANCE_TO_FILL: u64 = 8191;

/// Asynchronously gets the latest block in the contract.
async fn get_latest_block_in_contract(
    block_interval: u64,
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
        // Return the most recent block number from the logs (if any).
        if logs.len() > 0 {
            return Ok(logs[0].block_number.unwrap().as_u64());
        }

        curr_block -= U64::from(500u64);
    }

    Err(anyhow::Error::msg(
        "Could not find the latest block in the contract",
    ))
}

/// The main function that runs the application.
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    let block_interval = env::var("BLOCK_INTERVAL")?;
    let block_interval = u64::from_str(&block_interval)?;

    let rpc_url = env::var("RPC_URL")?;

    let chain_id = env::var("CHAIN_ID")?;
    let chain_id = u64::from_str(&chain_id)?;

    let contract_address = env::var("CONTRACT_ADDRESS")?;
    let oracle_address_bytes: [u8; 20] = hex::decode(contract_address).unwrap().try_into().unwrap();

    loop {
        // Replace with your Ethereum node's HTTP endpoint
        let client = Client::new_with(EnvironmentProvider::default(), HttpClient::new().unwrap());
        let kms_client = KmsClient::new_with_client(client, Region::UsWest1);
        let key_id = "...";
        let chain_id = 1;

        let signer = AwsSigner::new(kms_client, key_id, chain_id).await?;

        let provider =
            Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");

        let wallet = wallet.clone().with_chain_id(chain_id);

        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));

        let contract = EigenlayerBeaconOracle::new(oracle_address_bytes, client.clone());

        // Check if latest_block + block_interval is less than the current block number.
        let latest_block = client.get_block_number().await?;

        let contract_curr_block = get_latest_block_in_contract(
            block_interval,
            rpc_url.clone(),
            Address::from(oracle_address_bytes),
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
