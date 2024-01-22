use serde_json::json;

use crate::json_rpc::types::{JsonRpcRequest, JsonRpcResponse};

use super::{
    errors::EthCallError,
    types::{EthCall, GetTransactionByHash, ResultWrapper, Transaction, ETHEREUM_ENDPOINT},
};

impl GetTransactionByHash {
    pub fn new(hash: String) -> GetTransactionByHash {
        Self { data: hash }
    }
}

impl EthCall for GetTransactionByHash {
    type Inner = JsonRpcResponse<ResultWrapper<Transaction>>;

    async fn call(&self) -> Result<Self::Inner, EthCallError> {
        let endpoint = ETHEREUM_ENDPOINT.get().unwrap();
        let res = reqwest::Client::new()
            .post(endpoint.url.to_string())
            .json(&JsonRpcRequest {
                jsonrpc: "2.0".to_owned(),
                method: "eth_getTransactionByHash".to_owned(),
                params: Some(json!([self.data])),
                id: 1,
            })
            .send()
            .await?
            .json::<Self::Inner>()
            .await?;

        Ok(res)
    }
}

#[cfg(test)]
pub mod tests {

    use crate::eth_rpc::types::{Endpoints, GetTransactionByHash, ETHEREUM_ENDPOINT};

    #[tokio::test]
    async fn get_tx_by_hash() -> Result<(), Box<dyn std::error::Error>> {
        Endpoints::init()?;
        let hash = "0x10d26a9726e85f6bd33b5a1455219d8d56dd53d105e69e1be062119e8c7808a2";
        let provider = ETHEREUM_ENDPOINT.get().unwrap();
        let args = GetTransactionByHash::new(hash.to_owned());
        provider.get_transaction_by_hash(args).await?;
        Ok(())
    }
}
