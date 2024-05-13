use anyhow::Result;
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{transaction::eip2718::TypedTransaction, Address},
};
/// Wrapper of a `SignerMiddleware` client to send transactions to the given
/// contract's `Address`.
pub struct ContractClient {
    chain_id: u64,
    provider: Provider<Http>,
    contract: Address,
}

impl ContractClient {
    /// Creates a new `ContractClient`.
    pub async fn new(chain_id: u64, rpc_url: &str, contract: Address) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;

        Ok(ContractClient {
            chain_id,
            provider,
            contract,
        })
    }

    /// Read data from the contract using calldata.
    pub async fn read(&self, calldata: Vec<u8>) -> Result<Vec<u8>> {
        let mut tx = TypedTransaction::default();
        tx.set_chain_id(self.chain_id);
        tx.set_to(self.contract);
        tx.set_data(calldata.into());
        let data = self.provider.call(&tx, None).await?;

        Ok(data.to_vec())
    }
}
