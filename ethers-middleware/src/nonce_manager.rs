use async_trait::async_trait;
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::types::*;
use ethers_providers::{FromErr, Middleware, PendingTransaction};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use eyre::Result;

#[derive(Debug)]
/// Middleware used for calculating nonces locally, useful for signing multiple
/// consecutive transactions without waiting for them to hit the mempool
pub struct NonceManagerMiddleware<M> {
    inner: M,
    initialized: AtomicBool,
    nonce: AtomicU64,
    address: Address,
}

impl<M> NonceManagerMiddleware<M>
where
    M: Middleware,
{
    /// Instantiates the nonce manager with a 0 nonce. The `address` should be the
    /// address which you'll be sending transactions from
    pub fn new(inner: M, address: Address) -> Self {
        Self {
            initialized: false.into(),
            nonce: 0.into(),
            inner,
            address,
        }
    }

    /// Returns the next nonce to be used
    pub fn next(&self) -> U256 {
        let nonce = self.nonce.fetch_add(1, Ordering::SeqCst);
        nonce.into()
    }

    async fn get_transaction_count_with_manager(
        &self,
        block: Option<BlockId>,
    ) -> Result<U256> {
        // initialize the nonce the first time the manager is called
        if !self.initialized.load(Ordering::SeqCst) {
            let nonce = self
                .inner
                .get_transaction_count(self.address, block)
                .await?;
            self.nonce.store(nonce.as_u64(), Ordering::SeqCst);
            self.initialized.store(true, Ordering::SeqCst);
        }

        Ok(self.next())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<M> Middleware for NonceManagerMiddleware<M>
where
    M: Middleware,
{
    type Provider = M::Provider;
    type Inner = M;

    fn inner(&self) -> &M {
        &self.inner
    }

    /// Signs and broadcasts the transaction. The optional parameter `block` can be passed so that
    /// gas cost and nonce calculations take it into account. For simple transactions this can be
    /// left to `None`.
    async fn send_transaction<T: Into<TypedTransaction> + Send + Sync>(
        &self,
        tx: T,
        block: Option<BlockId>,
    ) -> Result<PendingTransaction<'_, Self::Provider>> {
        let mut tx = tx.into();

        if tx.nonce().is_none() {
            tx.set_nonce(self.get_transaction_count_with_manager(block).await?);
        }

        match self.inner.send_transaction(tx.clone(), block).await {
            Ok(tx_hash) => Ok(tx_hash),
            Err(err) => {
                let nonce = self.get_transaction_count(self.address, block).await?;
                if nonce != self.nonce.load(Ordering::SeqCst).into() {
                    // try re-submitting the transaction with the correct nonce if there
                    // was a nonce mismatch
                    self.nonce.store(nonce.as_u64(), Ordering::SeqCst);
                    tx.set_nonce(nonce);
                    Ok(self.inner
                        .send_transaction(tx, block)
                        .await?)
                } else {
                    // propagate the error otherwise
                    Err(err.into())
                }
            }
        }
    }
}
