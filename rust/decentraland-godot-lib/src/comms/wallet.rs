use std::sync::Arc;

use async_trait::async_trait;
use ethers::{
    signers::{LocalWallet, Signer, WalletError},
    types::{transaction::eip2718::TypedTransaction, Address, Signature, H160},
    utils::hex,
};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
#[derive(Clone)]
pub struct Wallet {
    inner: Arc<Box<dyn ObjSafeWalletSigner + 'static + Send + Sync>>,
}

impl Wallet {
    pub async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, WalletError> {
        self.inner.sign_message(message.as_ref()).await
    }

    pub fn address(&self) -> Address {
        self.inner.address()
    }

    pub fn new_local_wallet() -> Self {
        Wallet {
            inner: Arc::new(Box::new(LocalWallet::new(&mut thread_rng()))),
        }
    }
}

#[async_trait]
trait ObjSafeWalletSigner {
    async fn sign_message(&self, message: &[u8]) -> Result<Signature, WalletError>;

    /// Signs the transaction
    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, WalletError>;

    /// Returns the signer's Ethereum Address
    fn address(&self) -> Address;

    /// Returns the signer's chain id
    fn chain_id(&self) -> u64;
}

#[async_trait]
impl ObjSafeWalletSigner for LocalWallet {
    async fn sign_message(&self, message: &[u8]) -> Result<Signature, WalletError> {
        Signer::sign_message(self, message).await
    }

    async fn sign_transaction(&self, message: &TypedTransaction) -> Result<Signature, WalletError> {
        Signer::sign_transaction(self, message).await
    }

    fn address(&self) -> Address {
        Signer::address(self)
    }

    fn chain_id(&self) -> u64 {
        Signer::chain_id(self)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SimpleAuthChain(Vec<ChainLink>);

impl SimpleAuthChain {
    pub fn new(signer_address: Address, payload: String, signature: Signature) -> Self {
        Self(vec![
            ChainLink {
                ty: "SIGNER".to_owned(),
                payload: format!("{signer_address:#x}"),
                signature: String::default(),
            },
            ChainLink {
                ty: "ECDSA_SIGNED_ENTITY".to_owned(),
                payload,
                signature: format!("0x{signature}"),
            },
        ])
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChainLink {
    #[serde(rename = "type")]
    ty: String,
    payload: String,
    signature: String,
}

// convert string -> Address
pub trait AsH160 {
    fn as_h160(&self) -> Option<H160>;
}

impl AsH160 for &str {
    fn as_h160(&self) -> Option<H160> {
        if self.starts_with("0x") {
            return (&self[2..]).as_h160();
        }

        let Ok(hex_bytes) = hex::decode(self.as_bytes()) else { return None };
        if hex_bytes.len() != H160::len_bytes() {
            return None;
        }

        Some(H160::from_slice(hex_bytes.as_slice()))
    }
}

impl AsH160 for String {
    fn as_h160(&self) -> Option<H160> {
        self.as_str().as_h160()
    }
}
