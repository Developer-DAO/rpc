use super::types::RelayErrors;
use crate::{proxy::client::ProxyClient, routes::relayer::types::PoktChains};
use axum::{
    body::{Body, Bytes},
    extract::{Path, Request},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use hyper::{Method, Request as HyperRequest, body::Incoming, header::HOST};
use thiserror::Error;

pub async fn route_call(
    Path(route_info): Path<[String; 2]>,
    payload: Request<Body>,
) -> Result<impl IntoResponse, RouterErrors> {
    let raw_destination = route_info
        .first()
        .ok_or_else(|| RouterErrors::DestinationError)?;
    let dest = raw_destination.parse::<PoktChains>()?;
    let result = dest.relay_transaction(payload).await?;
    Ok(result)
}

impl PoktChains {
    async fn relay_transaction(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Incoming>, RelayErrors> {
        let (parts, body) = req.into_parts();
        let body = body.collect().await.unwrap().to_bytes();
        let nr = Request::from_parts(parts, body);
        let client = ProxyClient::new(nr);
        Ok(client.exec_request().await.unwrap())
    }
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
