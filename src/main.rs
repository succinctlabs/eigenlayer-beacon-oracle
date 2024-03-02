use anyhow::Result;
use ethers::{
    contract::abigen,
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::TransactionReceipt,
    utils::hex,
};
use std::{env, str::FromStr, sync::Arc};

abigen!(
    EigenlayerBeaconOracle,
    "./abi/EigenlayerBeaconOracle.abi.json"
);

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let block_interval = env::var("BLOCK_INTERVAL")?;
    let block_interval = u64::from_str(&block_interval)?;

    let rpc_url = env::var("RPC_URL")?;

    let private_key = Some(env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set"));
    let wallet = LocalWallet::from_str(private_key.as_ref().unwrap()).expect("invalid private key");

    let chain_id = env::var("CHAIN_ID")?;
    let chain_id = u64::from_str(&chain_id)?;

    let contract_address = env::var("CONTRACT_ADDRESS")?;
    let oracle_address_bytes: [u8; 20] = hex::decode(contract_address).unwrap().try_into().unwrap();

    loop {
        // Replace with your Ethereum node's HTTP endpoint
        let provider =
            Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");

        let wallet = wallet.clone().with_chain_id(chain_id);

        let client = Arc::new(SignerMiddleware::new(provider, wallet.clone()));

        let contract = EigenlayerBeaconOracle::new(oracle_address_bytes, client.clone());

        let block_number = client.get_block_number().await?;

        // To avoid RPC stability issues, we use a block number 5 blocks behind the current block.
        let safe_block_number = block_number - 5;

        let interval_block_nb = safe_block_number - (safe_block_number % block_interval);

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
                println!("Transaction sent: {:?}", tx.transaction_hash);
            }
        }
        // Sleep for 1 minute.
        let _ = tokio::time::sleep(tokio::time::Duration::from_secs((60) as u64)).await;
    }
}
