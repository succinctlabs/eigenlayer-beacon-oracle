use crate::types::{RelayRequest, RelayResponse};
use ethers::{
    types::{Address, H256},
    utils::hex,
};
use reqwest::Client;
use std::{env, str::FromStr};

/// Send a request to the Secure Production Relayer to relay a proof using the KMS wallet.
pub async fn send_secure_kms_relay_request(
    calldata: Vec<u8>,
    chain_id: u64,
    address: Address,
) -> Result<H256, Box<dyn std::error::Error>> {
    // Create the relay request.
    let relay_request = RelayRequest {
        chain_id: chain_id as u64, // Assuming req.ChainID is of type uint32 and needs conversion
        address: address.to_string(),
        calldata: hex::encode(calldata),
        platform_request: false,
    };

    // Read relayer endpoint from env
    let relayer_endpoint = env::var("SECURE_RELAYER_ENDPOINT").unwrap();

    // Send request to the Secure Production Relayer.
    let url = format!("{}/relay", relayer_endpoint);
    println!("url: {}", url);
    let client = Client::new();
    let res = client.post(&url).json(&relay_request).send().await?;

    // If the status is not 200, return an error.
    if res.status() != reqwest::StatusCode::OK {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("server responded with status code: {}", res.status()),
        )));
    }

    // Read and parse the response body.
    let response_bytes = res.bytes().await?;
    let relay_resp: RelayResponse = serde_json::from_slice(&response_bytes)?;

    // If the status is 1, the relay request was successful.
    if relay_resp.status == 1 {
        // Parse the transaction hash from the response
        let tx_hash = H256::from_str(
            relay_resp
                .transaction_hash
                .as_ref()
                .ok_or("Missing transaction hash")?,
        )?;
        Ok(tx_hash)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "relay failed with status {}: {}",
                relay_resp.status,
                relay_resp.message.as_ref().ok_or("Missing error message")?
            ),
        )))
    }
}
