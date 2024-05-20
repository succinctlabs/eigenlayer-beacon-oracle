use alloy_sol_types::private::U256;
use alloy_sol_types::SolCall;
use anyhow::Result;
use eigenlayer_beacon_oracle::{
    addTimestampCall, contract::ContractClient, get_block_to_request, get_latest_block_in_contract,
    request::send_secure_kms_relay_request, timestampToBlockRootCall,
};
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::LocalWallet,
    types::{Address, NameOrAddress, TransactionRequest, H160, H256},
    utils::hex,
};
use log::{debug, error, info};
use std::{env, str::FromStr, sync::Arc};

/// The operator for the EigenlayerBeaconOracle contract.
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();
    env_logger::init();

    // If RELAYER_PRIVATE_KEY is set, the operator will relay the request to the contract directly.
    let self_relay = env::var("RELAYER_PRIVATE_KEY").is_ok();

    let block_interval = env::var("BLOCK_INTERVAL")?;
    let block_interval = u64::from_str(&block_interval)?;

    let rpc_url = env::var("RPC_URL")?;
    let chain_id = env::var("CHAIN_ID")?;
    let chain_id = u64::from_str(&chain_id)?;

    let contract_address = env::var("CONTRACT_ADDRESS")?;
    let oracle_address_bytes: [u8; 20] = hex::decode(contract_address).unwrap().try_into().unwrap();

    loop {
        let contract_client =
            ContractClient::new(chain_id, &rpc_url, Address::from(oracle_address_bytes)).await?;

        // Replace with your Ethereum node's HTTP endpoint
        let provider =
            Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");

        let latest_block = provider.get_block_number().await?;

        // Get the block of the most recent update to the contract. This will always be a multiple of block_interval.
        let contract_curr_block = get_latest_block_in_contract(
            chain_id,
            rpc_url.clone(),
            Address::from(oracle_address_bytes),
            block_interval,
        )
        .await;

        let block_nb_to_request =
            get_block_to_request(contract_curr_block, block_interval, latest_block.as_u64());

        // To avoid RPC stability issues, we use a block number 1 block behind the current block.
        if block_nb_to_request < latest_block.as_u64() - 1 {
            debug!(
                "Attempting to add timestamp of block {} to contract",
                block_nb_to_request
            );

            // Check if interval_block_nb is stored in the contract.
            let interval_block = provider.get_block(block_nb_to_request).await?;
            let interval_block_timestamp = interval_block.clone().unwrap().timestamp;
            let timestamp = U256::from(interval_block_timestamp.as_u128());
            let timestamp_to_block_root_call = timestampToBlockRootCall {
                _targetTimestamp: timestamp,
            };

            let timestamp_to_block_root_calldata = timestamp_to_block_root_call.abi_encode();

            let interval_beacon_block_root = contract_client
                .read(timestamp_to_block_root_calldata)
                .await
                .unwrap();

            // If the interval block is not in the contract, store it.
            if interval_beacon_block_root == [0; 32] {
                let add_timestamp_call = addTimestampCall {
                    _targetTimestamp: timestamp,
                };

                let add_timestamp_calldata = add_timestamp_call.abi_encode();

                let tx_hash = if self_relay {
                    self_relay_data(
                        provider,
                        chain_id,
                        add_timestamp_calldata,
                        oracle_address_bytes,
                    )
                    .await
                } else {
                    send_secure_kms_relay_request(
                        add_timestamp_calldata,
                        chain_id,
                        Address::from(oracle_address_bytes),
                    )
                    .await
                };

                if let Err(e) = tx_hash {
                    error!("Transaction failed! {:?}", e);
                } else if let Ok(tx_hash) = tx_hash {
                    info!(
                        "Relayed transaction: {:?} to {:?} on chain {:?}",
                        tx_hash,
                        Address::from(oracle_address_bytes),
                        chain_id
                    );
                }
            }
        }
        debug!("Sleeping for 1 minute");
        // Sleep for 5 minutes.
        let _ = tokio::time::sleep(tokio::time::Duration::from_secs((300) as u64)).await;
    }
}

/// Relay the transaction to the contract using the account defined by RELAYER_PRIVATE_KEY.
async fn self_relay_data(
    provider: Provider<Http>,
    chain_id: u64,
    add_timestamp_calldata: Vec<u8>,
    oracle_address_bytes: [u8; 20],
) -> Result<H256> {
    let private_key: String = env::var("RELAYER_PRIVATE_KEY").unwrap();
    let wallet = LocalWallet::from_str(&private_key).expect("invalid private key");

    println!("Client chain id: {}", provider.get_chainid().await?);

    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    println!("Contract chain id: {}", chain_id);
    let tx = TransactionRequest {
        chain_id: Some(chain_id.into()),
        to: Some(NameOrAddress::Address(H160::from_slice(
            &oracle_address_bytes,
        ))),
        data: Some(add_timestamp_calldata.into()),
        ..Default::default()
    };
    let tx = client.send_transaction(tx.clone(), None).await?.await?;
    if let Some(tx) = tx {
        let tx_hash = tx.transaction_hash;
        Ok(tx_hash)
    } else {
        Err(anyhow::Error::msg("Transaction failed"))
    }
}
