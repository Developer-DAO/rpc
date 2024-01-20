use crate::{eth_rpc::errors::EthCallError, json_rpc::types::JsonRpcResponse};
use serde::{Deserialize, Serialize};
use std::future::Future;

pub trait EthCall {

    type Inner;

    fn call(
        &self,
    ) -> impl Future<Output = Result<JsonRpcResponse<Self::Inner>, EthCallError>> + Send; 
}

pub enum Methods {
    GetTxByHash,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionByHash {
    pub data: [u8; 32],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionByHashResult {
    blockhash: [u8; 32],
    blocknumber: String,
    from: [u8; 20],
    gas: String,
    gas_price: String,
    hash: [u8; 32],
    input: [u8; 32],
    nonce: String,
    to: [u8; 20],
    transaction_index: String,
    value: String,
    v: String,
    r: String,
    s: String,
}
