use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    jsonrpc: f32,
    id: u32,
    result: T,
}
