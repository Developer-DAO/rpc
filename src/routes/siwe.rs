use crate::{database::types::RELATIONAL_DATABASE, eth_rpc::types::ETHEREUM_ENDPOINT};
use alloy::{primitives::Address, providers::ProviderBuilder};
use axum::{
    extract::{Extension, Json},
    http::StatusCode,
    response::IntoResponse,
};
use jwt_simple::claims::JWTClaims;
use serde::{Deserialize, Serialize};
use siwe::{Message, VerificationError, VerificationOpts, generate_nonce};
use sqlx::prelude::FromRow;
use thiserror::Error;
use time::OffsetDateTime;

use super::types::{Claims, SiweNonce};

#[derive(Debug, Serialize, Deserialize)]
pub struct Siwe {
    pub message: String,
    pub signature: Vec<u8>,
}

pub struct Nonce<'a> {
    nonce: SiweNonce<'a>,
}

pub async fn siwe_add_wallet(
    Extension(jwt): Extension<JWTClaims<Claims<'_>>>,
    Json(payload): Json<Siwe>,
) -> Result<impl IntoResponse, SiweError> {
    let msg: Message = payload.message.parse()?;

    let nonce = sqlx::query_as!(
        Nonce,
        "SELECT nonce FROM Customers where email = $1",
        jwt.custom.email.as_str(),
    )
    .fetch_one(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .nonce;

    let rpc = ProviderBuilder::new().on_http(ETHEREUM_ENDPOINT[0].as_str().parse().unwrap());

    let domain = if cfg!(feature = "dev") {
        "localhost:5173"
    } else {
        "Developer DAO Cloud"
    };

    let verification_opts = VerificationOpts {
        domain: Some(domain.parse().unwrap()),
        nonce: Some(nonce.to_string()),
        timestamp: Some(OffsetDateTime::now_utc()),
        rpc_provider: Some(rpc),
    };

    msg.verify(&payload.signature, &verification_opts).await?;

    let address = Address::from(msg.address).to_string();

    sqlx::query!(
        "UPDATE Customers SET wallet = $1 where email = $2",
        address,
        jwt.custom.email.as_str()
    )
    .execute(RELATIONAL_DATABASE.get().unwrap())
    .await?;

    Ok((StatusCode::OK, address).into_response())
}

pub async fn get_siwe_nonce(
    Extension(jwt): Extension<JWTClaims<Claims<'_>>>,
) -> Result<impl IntoResponse, SiweError> {
    let nonce = generate_nonce();
    sqlx::query!(
        "UPDATE Customers SET nonce = $1 where email = $2",
        nonce,
        jwt.custom.email.as_str()
    )
    .execute(RELATIONAL_DATABASE.get().unwrap())
    .await?;
    Ok((StatusCode::OK, nonce).into_response())
}

#[derive(Debug, Error)]
pub enum SiweError {
    #[error(transparent)]
    VerificationFailed(#[from] VerificationError),
    #[error("Incorrect siwe nonce for user")]
    IncorrectNonce,
    #[error("An error ocurred while querying the database")]
    QueryError(#[from] sqlx::Error),
    #[error(transparent)]
    ParseError(#[from] siwe::ParseError),
}

impl IntoResponse for SiweError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            self.to_string(),
        )
            .into_response()
    }
}
