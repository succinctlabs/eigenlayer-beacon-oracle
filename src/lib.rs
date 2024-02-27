use anyhow::Result;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    const SLOT_INTERVAL: usize = 10;

    // Every 10 slots, call add timestamp with the slot's timestamp.
    let url = Url::parse("http://localhost:5052").unwrap();
    let client = Client::new(url);

    match client.get_latest_slot().await {
        Ok(slot_data) => println!("Latest slot: {}", slot_data.slot),
        Err(e) => eprintln!("Error fetching latest slot: {}", e),
    }

    println!("Hello, world!");

    Ok(())
}
