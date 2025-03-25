use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};

use super::types::RelayErrors;
use crate::{
    json_rpc::types::JsonRpcRequest,
    routes::relayer::types::{PoktChains, Relayer},
};

use thiserror::Error;

pub async fn route_call(
    Path(route_info): Path<[String; 2]>,
    Json(body): Json<JsonRpcRequest>,
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
}

impl IntoResponse for RouterErrors {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

// #[cfg(test)]
// pub mod test {
//     use reqwest::Url;
//     use serde_json::json;
//     #[tokio::test]
//     async fn relay_basic() {
//         let request = json!({
//             "jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id": 1
//         });
//
//         let res = reqwest::Client::new()
//             .post("http://localhost:8080".parse::<Url>().unwrap())
//             .json(&request)
//             .send()
//             .await
//             .unwrap();
//
//         assert_eq!(res.status(), reqwest::StatusCode::OK);
//
//         let text = res.text().await.unwrap();
//         println!("{text:?}");
//
//
//     }
// }
