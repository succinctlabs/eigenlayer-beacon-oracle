use clap::Parser;
use dotenv::dotenv;
use ethers::prelude::*;

#[derive(Parser, Debug, Clone)]
#[command(
    about = "Get the last block of the block range to fill and whether to post the data on-chain."
)]
pub struct FillBlockRangeArgs {
    #[arg(long, required = true)]
    pub rpc_url: String,
    #[arg(long, required = true)]
    pub relayer_address: String,
    #[arg(long, required = true)]
    pub contract_address: String,
    #[arg(long, required = true)]
    pub start_timestamp: String,
    #[arg(long, required = true)]
    pub end_timestamp: String,
}

// Searches for the nearest block number to the given timestamp.
async fn get_block_from_timestamp_ethereum(timestamp: u64) -> anyhow::Result<u64> {
    // Query https://coins.llama.fi/block/{chain}/{timestamp} to get the block number.
    let response = reqwest::get(format!(
        "https://coins.llama.fi/block/ethereum/{}",
        timestamp
    ))
    .await?
    .json::<serde_json::Value>()
    .await?;

    let block_number = response
        .get("height")
        .ok_or_else(|| anyhow::anyhow!("'height' field is missing in the response"))?
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse 'height' as u64"))?;
    Ok(block_number)
}

// Compute the total cost (ETH) of relaying all EigenlayerBeaconOracle to CONTRACT_ADDRESS over the period [start_timestamp, end_timestamp].
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let args = FillBlockRangeArgs::parse();

    let rpc_url = args.rpc_url;
    let relayer_address: Address = args.relayer_address.parse()?;
    let contract_address: Address = args.contract_address.parse()?;
    let client = Provider::<Http>::try_from(rpc_url)?.with_sender(relayer_address);

    // Parse the start and end timestamps from datestring format into u64.
    let start_timestamp_str = args.start_timestamp;
    let end_timestamp_str = args.end_timestamp;
    let start_timestamp =
        chrono::DateTime::parse_from_rfc3339(&start_timestamp_str)?.timestamp() as u64;
    let end_timestamp =
        chrono::DateTime::parse_from_rfc3339(&end_timestamp_str)?.timestamp() as u64;

    let start_block_number = get_block_from_timestamp_ethereum(start_timestamp).await?;
    let end_block_number = get_block_from_timestamp_ethereum(end_timestamp).await?;

    // Convert timestamps to block numbers (This is a placeholder conversion. The actual conversion depends on the blockchain's block time and would likely involve querying the blockchain)
    let start_block: U64 = U64::from(start_block_number); // Assuming 15 seconds per block
    let end_block: U64 = U64::from(end_block_number);

    use std::fs::File;
    use std::io::Write;

    let mut transactions_data = Vec::new();
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
            let tx_hash = log.transaction_hash.unwrap();
            let tx_origin = client
                .get_transaction(tx_hash)
                .await?
                .expect("Transaction not found")
                .from;
            if tx_origin == relayer_address {
                // Assuming the log is from the relayer, compute the cost.
                let tx_receipt = client
                    .get_transaction_receipt(tx_hash)
                    .await?
                    .expect("Transaction receipt not found");
                let tx_cost = tx_receipt.gas_used.expect("Gas used not available")
                    * tx_receipt
                        .effective_gas_price
                        .expect("Effective gas price not available");
                transactions_data.push((tx_cost, tx_hash, log.block_number.unwrap()));
            }
        }
        // Move to the next chunk.
        from_block += chunk_size + U64::one();
    }

    // Write transactions data to a CSV file
    let file_name = format!("cost_{}_{}.csv", start_block, end_block);
    let mut wtr = File::create(&file_name)?;
    writeln!(wtr, "cost_eth,tx_hash,block_no")?;
    let mut eth_total_cost = U256::zero();
    for (cost, tx_hash, block_no) in &transactions_data {
        eth_total_cost += *cost;
        let cost_in_eth = ethers::utils::format_units(*cost, 18)?;
        writeln!(wtr, "{},{},{}", cost_in_eth, tx_hash, block_no)?;
    }

    // Convert the total cost to ETH
    let eth_total_cost = ethers::utils::format_units(eth_total_cost, 18)?;

    // Convert the total cost to ETH and print it along with the CSV file name.
    println!(
        "Total cost in ETH for transactions from RELAYER_ADDRESS over the period: {}, Data written to {}",
        eth_total_cost, file_name
    );

    Ok(())
}
