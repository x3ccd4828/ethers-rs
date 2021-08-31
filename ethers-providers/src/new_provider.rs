use super::JsonRpcClient as Transport;
use async_trait::async_trait;
use eyre::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

use ethers_core::types::{BlockNumber, TransactionReceipt, U64};

#[derive(Clone, Debug)]
pub struct MyProvider<T> {
    transport: T,
}

#[async_trait]
impl<TS: Transport> Transport for MyProvider<TS> {
    async fn request<T, R>(&self, method: &str, params: T) -> Result<R>
    where
        T: Debug + Serialize + Send + Sync,
        R: Serialize + DeserializeOwned,
    {
        self.transport.request(method, params).await
    }
}

impl<T: Transport> Provider for MyProvider<T> {}

#[async_trait]
trait Provider: Transport {
    // async fn get_block_receipts<T: Into<BlockNumber> + Send + Sync>(
    //     &self,
    //     block: T,
    // ) -> Result<Vec<TransactionReceipt>>
    // where Self: Sized
    // {
    //     self.request("eth_getBlockReceipts", [block.into()]).await
    // }

    async fn get_block_number(&self) -> Result<U64>
    where
        Self: Sized,
    {
        self.request("eth_blockNumber", ()).await
    }
}

#[derive(Clone, Debug)]
struct Layer1<P> {
    inner: P,
}

#[async_trait]
impl<P: Provider> Transport for Layer1<P> {
    async fn request<T, R>(&self, method: &str, params: T) -> Result<R>
    where
        T: Debug + Serialize + Send + Sync,
        R: Serialize + DeserializeOwned,
        Self: Sized,
    {
        self.inner.request(method, params).await
    }
}

// defaults to the lower level one instead of the inner ones
impl<P: Provider> Provider for Layer1<P> {}

#[derive(Clone, Debug)]
struct Layer2<P> {
    inner: P,
}

#[async_trait]
impl<P: Provider> Transport for Layer2<P> {
    async fn request<T, R>(&self, method: &str, params: T) -> Result<R>
    where
        T: Debug + Serialize + Send + Sync,
        R: Serialize + DeserializeOwned,
        Self: Sized,
    {
        self.inner.request(method, params).await
    }
}

impl<P: Provider> Provider for Layer2<P> {}

#[cfg(test)]
mod tests {
    use crate::transports;

    use super::*;
    use ethers_core::utils::Ganache;
    use std::str::FromStr;

    #[tokio::test]
    async fn works() -> Result<()> {
        let ganache = Ganache::new().spawn();

        let transport = transports::Http::from_str(&ganache.endpoint()).unwrap();
        let provider = MyProvider { transport };
        let provider = Layer1 { inner: provider };
        let provider: Box<dyn Provider> = if true {
            Box::new(Layer2 { inner: provider })
        } else {
            Box::new(provider)
        };

        // This won't compile:
        // error: the `get_block_number` method cannot be invoked on a trait object
        //    --> ethers-providers/src/new_provider.rs:103:28
        //     |
        // 38  |     async fn get_block_number(&self) -> Result<U64> where Self: Sized {
        //     |                                                                 ----- this has a `Sized` requirement
        // ...
        // 103 |         let blk = provider.get_block_number().await?;
        //
        let blk = provider.get_block_number().await?;
        dbg!(&blk);
        Ok(())
    }
}
