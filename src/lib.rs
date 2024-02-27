use anyhow::Result;
use beacon_api_client::BlockId::Slot;
use beacon_api_client::Client;
use url::Url;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    const SLOT_INTERVAL: usize = 10;

    // Every 10 blocks, call add timestamp with the slot's timestamp.
    let rpc_url = env::var("RPC").unwrap();

    let url = Url::parse("http://localhost:5052").unwrap();
    let client = Client::new(url);

    let head = client.get_beacon_header_at_head().await?;
    head.header.message.slot;

    let block = client
        .get_beacon_block(Slot(head.header.message.slot))
        .await?;

    let timestamp = block.

    match client.get_latest_slot().await {
        Ok(slot_data) => println!("Latest slot: {}", slot_data.slot),
        Err(e) => eprintln!("Error fetching latest slot: {}", e),
    }

    println!("Hello, world!");

    Ok(())
}
