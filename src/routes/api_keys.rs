use super::types::Claims;
use crate::database::types::RELATIONAL_DATABASE;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::IntoResponse,
};
use jwt_simple::claims::JWTClaims;
use secp256k1::generate_keypair;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Default)]
pub struct KeygenLimit {
    count: Option<i64>,
}

#[tracing::instrument]
pub async fn generate_api_keys(
    Extension(jwt): Extension<JWTClaims<Claims>>,
) -> Result<impl IntoResponse, ApiKeyError> {
    let keys = sqlx::query_as!(
        KeygenLimit,
        "SELECT COUNT(*) FROM Api where customerEmail = $1",
        jwt.custom.email
    )
    .fetch_one(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .count
    .unwrap_or_default();

    if keys >= 10i64 {
        Err(ApiKeyError::RateLimit)?
    }

    let (secret_key, _) = generate_keypair(&mut rand::thread_rng());
    let key_string = hex::encode(secret_key.secret_bytes());
    sqlx::query!(
        "INSERT INTO Api (customerEmail, apiKey) VALUES ($1, $2)",
        jwt.custom.email,
        &key_string
    )
    .execute(RELATIONAL_DATABASE.get().unwrap())
    .await?;
    println!("{key_string}");
    Ok((StatusCode::OK, key_string))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Keys {
    apikey: String,
}

#[tracing::instrument]
pub async fn get_all_api_keys(
    Extension(jwt): Extension<JWTClaims<Claims>>,
) -> Result<impl IntoResponse, ApiKeyError> {
    let keys: Vec<Keys> = sqlx::query_as!(
        Keys,
        "SELECT apiKey FROM Api where customerEmail = $1",
        jwt.custom.email
    )
    .fetch_all(RELATIONAL_DATABASE.get().unwrap())
    .await?;

    Ok((StatusCode::OK, serde_json::to_string(&keys)?))
}

#[tracing::instrument]
pub async fn delete_key(Path(params): Path<String>) -> Result<impl IntoResponse, ApiKeyError> {
    sqlx::query_as!(Api, "DELETE FROM Api where apiKey = $1", params)
        .execute(RELATIONAL_DATABASE.get().unwrap())
        .await?;

    Ok((StatusCode::OK, "Key successfully deleted"))
}

#[derive(Debug, Error)]
pub enum ApiKeyError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("Failed to find key in database.")]
    KeyNotFound,
    #[error("You have reached your maximum allocation of API keys.")]
    RateLimit,
}

impl IntoResponse for ApiKeyError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            self.to_string(),
        )
            .into_response()
    }
}

// limits for API key generation to avoid abuse
//
// maybe scope api key permissions in the future
