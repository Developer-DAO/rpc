use super::types::RelayErrors;
use crate::routes::relayer::types::{PoktChains, Relayer};
use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};
use serde_json::Value;
use thiserror::Error;

pub async fn route_call(
    Path(route_info): Path<[String; 2]>,
    Json(body): Json<Value>,
) -> Result<impl IntoResponse, RouterErrors> {
    let raw_destination = route_info
        .first()
        .ok_or_else(|| RouterErrors::DestinationError)?;
    let dest = raw_destination.parse::<PoktChains>()?;
    let result = dest.relay_transaction(&body).await?;
    Ok((StatusCode::OK, result))
}

#[derive(Debug, Error)]
pub enum RouterErrors {
    #[error("Could not parse destination from the first Path parameter")]
    DestinationError,
    #[error(transparent)]
    Relay(#[from] RelayErrors),
    #[error("malformed payload")]
    NotJsonRpc,
}

impl IntoResponse for RouterErrors {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[cfg(test)]
pub mod test {
    use crate::routes::relayer::types::{PoktChains, Relayer};
    use serde_json::json;

    #[tokio::test]
    async fn relay_test() {
        let body = json!({
            "jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id": 1
        });
        let chain = "anvil";
        let dest = chain.parse::<PoktChains>().unwrap();
        let res = dest.relay_transaction(&body).await;
        assert!(res.is_ok());
        let text = res.unwrap();
        println!("{text:?}");
    }
}
