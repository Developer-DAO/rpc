use super::{types::{EthCall, GetTransactionByHash}, errors::EthCallError};
use crate::json_rpc::types::JsonRpcResponse;

impl GetTransactionByHash {

    pub fn new(hash: [u8; 32]) -> GetTransactionByHash {
        Self {
            data: hash,
        }
    }
}

impl EthCall for GetTransactionByHash 
{
   async fn call(&self) -> Result<JsonRpcResponse<GetTransactionByHash>, EthCallError> 
   {
        let res = reqwest::Client::new()
            .post("")
            .json(&self)
            .send()
            .await?
            .json::<JsonRpcResponse<GetTransactionByHash>>()
            .await?;

        Ok(res)
   } 
}
