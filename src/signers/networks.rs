//! Networks are used inside wallets to ensure type-safety across networks. That way
//! a transaction that is designed to work with testnet does not accidentally work
//! with mainnet because the URL was changed.

use crate::types::U64;

pub trait Network {
    const CHAIN_ID: Option<U64>;

    // TODO: Default providers? e.g. `mainnet.infura.io/XXX`?
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Mainnet;

impl Network for Mainnet {
    const CHAIN_ID: Option<U64> = Some(U64([1]));
}

/// No EIP155
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EIP155Disabled;

// EIP155 being disabled means no chainId will be used
impl Network for EIP155Disabled {
    const CHAIN_ID: Option<U64> = None;
}

pub mod instantiated {
    use super::*;
    use crate::signers::Wallet;

    pub type MainnetWallet = Wallet<Mainnet>;
    pub type AnyWallet = Wallet<EIP155Disabled>;
}