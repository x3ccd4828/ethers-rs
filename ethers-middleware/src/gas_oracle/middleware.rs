use super::{GasOracle, GasOracleError};
use async_trait::async_trait;
use ethers_core::types::{transaction::eip2718::TypedTransaction, *};
use ethers_providers::{FromErr, Middleware, PendingTransaction};
use eyre::Result;
use thiserror::Error;

#[derive(Debug)]
/// Middleware used for fetching gas prices over an API instead of `eth_gasPrice`
pub struct GasOracleMiddleware<M, G> {
    inner: M,
    gas_oracle: G,
}

impl<M, G> GasOracleMiddleware<M, G>
where
    M: Middleware,
    G: GasOracle,
{
    pub fn new(inner: M, gas_oracle: G) -> Self {
        Self { inner, gas_oracle }
    }
}

#[derive(Error, Debug)]
pub enum MiddlewareError {
    #[error(transparent)]
    GasOracleError(#[from] GasOracleError),

    #[error("This gas price oracle only works with Legacy and EIP2930 transactions.")]
    UnsupportedTxType,
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<M, G> Middleware for GasOracleMiddleware<M, G>
where
    M: Middleware,
    G: GasOracle,
{
    type Provider = M::Provider;
    type Inner = M;

    // OVERRIDEN METHODS

    fn inner(&self) -> &M {
        &self.inner
    }

    async fn get_gas_price(&self) -> Result<U256> {
        Ok(self.gas_oracle.fetch().await?)
    }

    async fn estimate_eip1559_fees(
        &self,
        _: Option<fn(U256, Vec<Vec<U256>>) -> (U256, U256)>,
    ) -> Result<(U256, U256)> {
        Ok(self.gas_oracle.estimate_eip1559_fees().await?)
    }

    async fn send_transaction<T: Into<TypedTransaction> + Send + Sync>(
        &self,
        tx: T,
        block: Option<BlockId>,
    ) -> Result<PendingTransaction<'_, Self::Provider>> {
        let mut tx = tx.into();

        match tx {
            TypedTransaction::Legacy(ref mut tx) => {
                if tx.gas_price.is_none() {
                    tx.gas_price = Some(self.get_gas_price().await?);
                }
            }
            TypedTransaction::Eip2930(ref mut inner) => {
                if inner.tx.gas_price.is_none() {
                    inner.tx.gas_price = Some(self.get_gas_price().await?);
                }
            }
            TypedTransaction::Eip1559(ref mut inner) => {
                if inner.max_priority_fee_per_gas.is_none() || inner.max_fee_per_gas.is_none() {
                    let (max_fee_per_gas, max_priority_fee_per_gas) =
                        self.estimate_eip1559_fees(None).await?;
                    if inner.max_priority_fee_per_gas.is_none() {
                        inner.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
                    }
                    if inner.max_fee_per_gas.is_none() {
                        inner.max_fee_per_gas = Some(max_fee_per_gas);
                    }
                }
            }
        };
        Ok(self.inner.send_transaction(tx, block).await?)
    }
}
