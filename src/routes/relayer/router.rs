use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};

use super::types::RelayErrors;
use crate::{json_rpc::types::JsonRpcRequest, routes::relayer::types::PoktChains};
use thiserror::Error;

pub async fn route_call(
    Path(route_info): Path<[String; 2]>,
    Json(body): Json<JsonRpcRequest>,
) -> Result<impl IntoResponse, RouterErrors> {
    let raw_destination = route_info
        .first()
        .ok_or_else(|| RouterErrors::DestinationError)?;
    let dest = raw_destination.parse::<PoktChains>()?;
    let result = dest.relay_pokt_transaction(&body).await?;
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
