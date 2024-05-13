use anyhow::Result;
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    types::{
        transaction::eip2718::TypedTransaction, Address, TransactionReceipt, TransactionRequest,
    },
};
use ethers_aws::aws_signer::AWSSigner;

use crate::signer::create_aws_signer;

/// Wrapper of a `SignerMiddleware` client to send transactions to the given
/// contract's `Address`.
pub struct ContractClient {
    chain_id: u64,
    client: SignerMiddleware<Provider<Http>, AWSSigner>,
    contract: Address,
}

impl ContractClient {
    /// Creates a new `ContractClient`.
    pub async fn new(chain_id: u64, rpc_url: &str, contract: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;

        let signer = create_aws_signer(chain_id).await;
        let client = SignerMiddleware::new(provider.clone(), signer);
        let contract = contract.parse::<Address>()?;

        Ok(ContractClient {
            chain_id,
            client,
            contract,
        })
    }

    /// Read data from the contract using calldata.
    pub async fn read(&self, calldata: Vec<u8>) -> Result<Vec<u8>> {
        let mut tx = TypedTransaction::default();
        tx.set_chain_id(self.chain_id);
        tx.set_to(self.contract);
        tx.set_data(calldata.into());
        let data = self.client.call(&tx, None).await?;

        Ok(data.to_vec())
    }
}
