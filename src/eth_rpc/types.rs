use crate::eth_rpc::errors::EthCallError;
use axum::http::Uri;
use serde::{Deserialize, Serialize};
use std::{future::Future, str::FromStr, sync::OnceLock};

pub static ETHEREUM_ENDPOINT: OnceLock<Provider> = OnceLock::new();

pub struct Endpoints;

impl Endpoints {
    pub fn init() -> Result<(), Box<dyn std::error::Error>> {
        ETHEREUM_ENDPOINT.get_or_init(|| Provider {
            url: Uri::from_str(&dotenvy::var("ETHEREUM_ENDPOINT").unwrap()).unwrap(),
        });

        Ok(())
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Chains {
    Ethereum,
}

pub trait EthCall {
    type Inner;

    fn call(&self) -> impl Future<Output = Result<Self::Inner, EthCallError>> + Send;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Methods<T>
where
    T: EthCall,
{
    GetTxByHash(T),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionByHash {
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResultWrapper<T> {
    result: T,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    #[serde(rename = "accessList")]
    access_list: serde_json::Value,
    block_hash: String,
    block_number: String,
    chain_id: String,
    from: String,
    gas: String,
    gas_price: String,
    hash: String,
    input: String,
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
    nonce: String,
    to: String,
    transaction_index: String,
    #[serde(rename = "type")]
    tx_type: String,
    value: String,
    v: String,
    r: String,
    s: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub url: axum::http::Uri,
}

impl Provider {
    pub fn new(url: Uri) -> Provider {
        Self { url }
    }

    pub async fn get_transaction_by_hash(
        &self,
        args: GetTransactionByHash,
    ) -> Result<Transaction, EthCallError> {
        let res = args.call().await?.result.result;
        Ok(res)
    }
}
