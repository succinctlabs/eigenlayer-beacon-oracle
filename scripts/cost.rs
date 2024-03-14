use dotenv::dotenv;
use ethers::prelude::*;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let rpc_url = env::var("RPC_URL").expect("RPC_URL must be set");
    let relayer_address: Address = env::var("RELAYER_ADDRESS")
        .expect("RELAYER_ADDRESS must be set")
        .parse()?;
    let contract_address: Address = env::var("CONTRACT_ADDRESS")
        .expect("CONTRACT_ADDRESS must be set")
        .parse()?;
    let client = Provider::<Http>::try_from(rpc_url)?.with_sender(relayer_address);

    // Read the block number using Clap parser in Rust
    let args: Vec<String> = env::args().collect();

    // Parse the second argument into i64.
    let start_block_nb = args[1].parse::<u64>()?;
    let end_block_nb = args[2].parse::<u64>()?;

    let start_block: U64 = U64::from(start_block_nb);
    let end_block: U64 = U64::from(end_block_nb);

    let mut total_cost = U256::zero();
    let chunk_size = U64::from(1000);
    let mut from_block = start_block;
    while from_block < end_block {
        let to_block = std::cmp::min(from_block + chunk_size, end_block);
        // Filter for the events in chunks.
        let filter = Filter::new()
            .from_block(from_block)
            .to_block(to_block)
            .address(vec![contract_address])
            .event("EigenLayerBeaconOracleUpdate(uint256,uint256,bytes32)");

        let logs = client.get_logs(&filter).await?;

        for log in logs {
            let tx_origin = client
                .get_transaction(log.transaction_hash.unwrap())
                .await?
                .expect("Transaction not found")
                .from;
            if tx_origin == relayer_address {
                // Assuming the log is from the relayer, compute the cost.
                let tx_receipt = client
                    .get_transaction_receipt(log.transaction_hash.unwrap())
                    .await?
                    .expect("Transaction receipt not found");
                let tx_cost = tx_receipt.gas_used.expect("Gas used not available")
                    * tx_receipt
                        .effective_gas_price
                        .expect("Effective gas price not available");
                total_cost += tx_cost;
            }
        }
        // Move to the next chunk.
        from_block += chunk_size + U64::one();
    }

    // Convert the total cost to ETH.
    let eth_total_cost = ethers::utils::format_units(total_cost, 18)?;
    println!(
        "Total cost in ETH for transactions from RELAYER_ADDRESS over the past month: {}",
        eth_total_cost
    );

    Ok(())
}
