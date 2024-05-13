use alloy_sol_types::private::U256;
use alloy_sol_types::{sol, SolCall};
use contract::ContractClient;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, U64},
};
use log::debug;

pub mod contract;
pub mod request;
mod types;

// Generates the contract bindings for the EigenlayerBeaconOracle contract.
sol! {
    function addTimestamp(uint256 _targetTimestamp);

    function timestampToBlockRoot(uint256 _targetTimestamp) returns (bytes32);
}

// Maximum number of blocks to search backwards for (1 day of blocks).
const MAX_DISTANCE_TO_FILL: u64 = 8191;

/// Asynchronously gets the block of the latest update to the contract.
/// Find the most recent log, get the slot, and then get the block number from the slot.
pub async fn get_latest_block_in_contract(
    chain_id: u64,
    rpc_url: String,
    oracle_address_bytes: Address,
    block_interval: u64,
) -> Option<u64> {
    let contract_client = ContractClient::new(chain_id, &rpc_url, oracle_address_bytes)
        .await
        .unwrap();

    let provider =
        Provider::<Http>::try_from(rpc_url.clone()).expect("could not connect to client");
    let latest_block = provider.get_block_number().await.unwrap();

    // Query backwards over MAX_DISTANCE_TO_FILL blocks to find the most recent update. Find the most recent update which is
    // a multiple of block_interval.
    let mut curr_block_nb = latest_block - (latest_block % block_interval);
    while curr_block_nb > latest_block - MAX_DISTANCE_TO_FILL {
        // Get timestamp of the block.
        let interval_block = provider.get_block(curr_block_nb).await.unwrap();

        let timestamp = U256::from(interval_block.clone().unwrap().timestamp.as_u128());
        let timestamp_to_block_root_call = timestampToBlockRootCall {
            _targetTimestamp: timestamp,
        };

        let timestamp_to_block_root_calldata = timestamp_to_block_root_call.abi_encode();

        let interval_beacon_block_root = contract_client
            .read(timestamp_to_block_root_calldata)
            .await
            .unwrap();

        // If the interval block is in the contract, return it.
        if interval_beacon_block_root != [0u8; 32] {
            return Some(curr_block_nb.as_u64());
        }
        curr_block_nb -= U64::from(block_interval);
    }

    None
}

/// If contract_curr_block is None, set a default start_block. Otherwise, return contract_curr_block + block_interval.
pub fn get_block_to_request(
    contract_curr_block: Option<u64>,
    block_interval: u64,
    latest_block: u64,
) -> u64 {
    // If the contract's current block is None, we need to set a default start_block.
    if contract_curr_block.is_none() {
        let default_start_block = latest_block - (latest_block % block_interval);
        debug!(
            "Contract has not been updated in {} blocks. Requesting timestamp for block: {}",
            MAX_DISTANCE_TO_FILL, default_start_block
        );
        return default_start_block;
    } else {
        let block_to_request = contract_curr_block.unwrap() + block_interval;
        debug!(
            "Contract's current block is {}. Requesting timestamp for block: {}",
            contract_curr_block.unwrap(),
            block_to_request
        );
        return block_to_request;
    }
}
