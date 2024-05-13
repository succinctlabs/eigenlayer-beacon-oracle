/// The status of a relay request. Parsed as an int from the request.
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum RelayStatus {
    Unknown = 0,
    Relayed = 1,
    PreflightError = 2,
    SimulationFailure = 3,
    PreflightFailure,
}

/// The input for a relay request.
#[derive(Deserialize, Serialize, Debug)]
pub struct RelayRequest {
    pub chain_id: u64,
    pub address: String,
    pub calldata: String,
    pub platform_request: bool,
}

/// The output for a relay request.
#[derive(Serialize, Deserialize)]
pub struct RelayResponse {
    pub transaction_hash: Option<String>,
    pub message: Option<String>,
    pub status: u32,
}
