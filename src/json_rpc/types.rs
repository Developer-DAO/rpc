use serde::{Deserialize, Serialize};

use crate::eth_rpc::types::ResultWrapper;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u16,
    #[serde(flatten)]
    pub result: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: u16,
}
