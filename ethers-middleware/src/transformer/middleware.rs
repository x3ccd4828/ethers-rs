use super::{Transformer, TransformerError};
use async_trait::async_trait;
use ethers_core::types::{transaction::eip2718::TypedTransaction, *};
use ethers_providers::{FromErr, Middleware, PendingTransaction};
use eyre::Result;
use thiserror::Error;

#[derive(Debug)]
/// Middleware used for intercepting transaction requests and transforming them to be executed by
/// the underneath `Transformer` instance.
pub struct TransformerMiddleware<M, T> {
    inner: M,
    transformer: T,
}

impl<M, T> TransformerMiddleware<M, T>
where
    M: Middleware,
    T: Transformer,
{
    /// Creates a new TransformerMiddleware that intercepts transactions, modifying them to be sent
    /// through the Transformer.
    pub fn new(inner: M, transformer: T) -> Self {
        Self { inner, transformer }
    }
}

#[derive(Error, Debug)]
pub enum TransformerMiddlewareError {
    #[error(transparent)]
    TransformerError(#[from] TransformerError),
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<M, T> Middleware for TransformerMiddleware<M, T>
where
    M: Middleware,
    T: Transformer,
{
    type Provider = M::Provider;
    type Inner = M;

    fn inner(&self) -> &M {
        &self.inner
    }

    async fn send_transaction<Tx: Into<TypedTransaction> + Send + Sync>(
        &self,
        tx: Tx,
        block: Option<BlockId>,
    ) -> Result<PendingTransaction<'_, Self::Provider>> {
        let mut tx = tx.into();

        // construct the appropriate proxy tx.
        self.transformer.transform(&mut tx)?;

        self.fill_transaction(&mut tx, block).await?;
        // send the proxy tx.
        self.inner.send_transaction(tx, block).await
    }
}
