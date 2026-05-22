use super::types::RelayErrors;
use crate::routes::relayer::types::{PoktChains, Relayer};
use axum::{body::Bytes, extract::Path, http::StatusCode, response::IntoResponse};
use thiserror::Error;

pub async fn route_call(
    Path(route_info): Path<[String; 2]>,
    body: Bytes,
) -> Result<impl IntoResponse, RouterErrors> {
    let raw_destination = route_info
        .first()
        .ok_or_else(|| RouterErrors::DestinationError)?;
    let dest = raw_destination.parse::<PoktChains>()?;
    let result = dest.relay_transaction(body).await?;
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
    use http_body_util::BodyExt;
    use serde_json::json;

    #[tokio::test]
    async fn relay_test() {
        let body = json!({
            "jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id": 1
        })
        .to_string()
        .leak()
        .as_bytes();

        let bytes = axum::body::Bytes::from_static(body);
        let chain = "anvil";
        let dest = chain.parse::<PoktChains>().unwrap();
        let res = dest.relay_transaction(bytes).await;
        assert!(res.is_ok());
        let text = res.unwrap().collect().await.unwrap().to_bytes().to_vec();
        let text = String::from_utf8(text).unwrap();
        println!("{text:?}");
    }
}
