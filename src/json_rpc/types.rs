use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u16,
    pub result: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<sonic_rs::Value>,
    pub id: u16,
}

impl JsonRpcRequest {
    pub fn new(method: String, params: Option<sonic_rs::Value>, id: u16) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            method,
            params,
            id,
        }
    }
}
