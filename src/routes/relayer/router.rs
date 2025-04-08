use super::types::RelayErrors;
use crate::routes::relayer::types::{PoktChains, Relayer};
use axum::{
    body::Body,
    extract::{Path, Request},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use hyper::{Request as HyperRequest, body::Incoming, header::HOST};
use hyper_util::rt::TokioIo;
use thiserror::Error;
use tracing::info;

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

impl Relayer for PoktChains {
    async fn relay_transaction(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Incoming>, RelayErrors> {
        let (mut parts, body) = req.into_parts();

        // Parse our URL...
        let url = dotenvy::var("SEPOLIA_PROVIDER")
            .unwrap()
            .parse::<hyper::Uri>()
            .unwrap();

        // Get the host and the port
        let host = url.host().expect("uri has no host");
        let port = url.port_u16().unwrap_or(80);

        let address = format!("{}:{}", host, port);

        // Open a TCP connection to the remote host
        let stream = tokio::net::TcpStream::connect(address).await.unwrap();

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Create the Hyper client
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();

        // Spawn a task to poll the connection, driving the HTTP state
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        let authority = url.authority().unwrap().clone();

        // Create an HTTP request with an empty body and a HOST header
        let mut req = HyperRequest::builder()
            .method(parts.method)
            .uri(url)
            .body(body)
            .unwrap();

        parts
            .headers
            .insert(HOST, HeaderValue::from_str(authority.as_str()).unwrap());
        *req.headers_mut() = parts.headers;

        info!(name = "RequestInfo", "{:?}", req);

        // Await the response...
        Ok(sender.send_request(req).await.unwrap())
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
