use std::fmt::{self, Display, Formatter};

use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};

use crate::{
    json_rpc::types::JsonRpcRequest,
    routes::{errors::ApiError, relayer::types::PoktChains},
};

use super::types::RelayErrors;

pub async fn route_call(
    Path(route_info): Path<[String; 2]>,
    Json(body): Json<JsonRpcRequest>,
) -> Result<impl IntoResponse, ApiError<RouterErrors>> {
    let raw_destination = route_info
        .first()
        .ok_or_else(|| ApiError::new(RouterErrors::DestinationError))?;
    let dest = raw_destination.parse::<PoktChains>()?;
    let result = dest.relay_pokt_transaction(&body).await?;
    Ok((StatusCode::OK, result))
}

#[derive(Debug)]
pub enum RouterErrors {
    DestinationError,
    Relay(RelayErrors),
}

impl Display for RouterErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RouterErrors::DestinationError => write!(
                f,
                "Could not parse destination from the first Path parameter"
            ),
            RouterErrors::Relay(_) => write!(f, "An error occured while relaying the transaction"),
        }
    }
}

impl std::error::Error for RouterErrors {

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RouterErrors::DestinationError => None,
            RouterErrors::Relay(e) => Some(e),
        }
    }

}

impl From<RelayErrors> for ApiError<RouterErrors> {
    fn from(value: RelayErrors) -> Self {
        ApiError::new(RouterErrors::Relay(value))
    }
}
