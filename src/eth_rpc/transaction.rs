use super::{
    errors::EthCallError,
    types::{EthCall, GetTransactionByHash, GetTransactionByHashResult},
};
use crate::json_rpc::types::JsonRpcResponse;

impl GetTransactionByHash {
    pub fn new(hash: [u8; 32]) -> GetTransactionByHash {
        Self { data: hash }
    }
}

impl EthCall for GetTransactionByHash {
   
   type Inner = GetTransactionByHashResult;

    async fn call(&self) -> Result<JsonRpcResponse<Self::Inner>, EthCallError> {
        let res = reqwest::Client::new()
            .post("")
            .json(&self)
            .send()
            .await?
            .json::<JsonRpcResponse<GetTransactionByHashResult>>()
            .await?;

        Ok(res)
    }
}
