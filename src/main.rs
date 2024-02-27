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

        let block = client.get_block(block_number - 10).await?;

        match block {
            Some(block) => {
                let timestamp = block.timestamp;

                let tx: Option<TransactionReceipt> =
                    contract.add_timestamp(timestamp).send().await?.await?;

                if let Some(tx) = tx {
                    println!("Transaction sent: {:?}", tx.transaction_hash);
                }
            }
            None => println!("Could not fetch the latest block."),
        }

        // Sleep for BLOCK_INTERVAL blocks.
        let _ = tokio::time::sleep(tokio::time::Duration::from_secs(
            (block_interval * 12) as u64,
        ))
        .await;
    }
}
