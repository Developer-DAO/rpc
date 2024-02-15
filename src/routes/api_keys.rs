use super::{errors::ApiError, types::Claims};
use crate::database::types::{Api, RELATIONAL_DATABASE};
use axum::{
    extract::{Extension, Query},
    http::StatusCode,
    response::IntoResponse,
};
use core::fmt;
use jwt_simple::claims::JWTClaims;
use secp256k1::generate_keypair;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

pub async fn generate_api_keys(
    Extension(jwt): Extension<JWTClaims<Claims>>,
) -> Result<impl IntoResponse, ApiError<ApiKeyError>> {
    let (secret_key, _) = generate_keypair(&mut rand::thread_rng());
    let key_string = hex::encode(secret_key.secret_bytes());
    sqlx::query!(
        "INSERT INTO Api (customerEmail, apiKey) VALUES ($1, $2)",
        jwt.custom.email,
        &key_string
    )
    .execute(RELATIONAL_DATABASE.get().unwrap())
    .await?;

    Ok((StatusCode::OK, key_string).into_response())
}

pub async fn get_all_api_keys(
    Extension(jwt): Extension<JWTClaims<Claims>>,
) -> Result<impl IntoResponse, ApiError<ApiKeyError>> {
    let keys: Vec<Api> = sqlx::query_as!(
        Api,
        "SELECT * FROM Api where customerEmail = $1",
        jwt.custom.email
    )
    .fetch_all(RELATIONAL_DATABASE.get().unwrap())
    .await?;

    Ok((StatusCode::OK, serde_json::to_string(&keys)?).into_response())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteKey {
    key: String,
}

pub async fn delete_key(
    // http://example.com/api/?key="..."
    Query(params): Query<String>,
) -> Result<impl IntoResponse, ApiError<ApiKeyError>> {
    sqlx::query_as!(Api, "DELETE FROM Api where apiKey = $1", params)
        .execute(RELATIONAL_DATABASE.get().unwrap())
        .await?;

    Ok((StatusCode::OK, "Key successfully deleted").into_response())
}

#[derive(Debug)]
pub enum ApiKeyError {
    DatabaseError(sqlx::Error),
    JsonError(serde_json::Error),
    KeyNotFound,
}

impl Display for ApiKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ApiKeyError::DatabaseError(_) => {
                write!(f, "An error has occured while querying the database")
            }
            ApiKeyError::JsonError(_) => write!(f, "Failed to serialize value into JSON"),
            ApiKeyError::KeyNotFound => write!(f, "Failed to find key in database"),
        }
    }
}

impl Error for ApiKeyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ApiKeyError::DatabaseError(e) => Some(e),
            ApiKeyError::JsonError(e) => Some(e),
            ApiKeyError::KeyNotFound => None,
        }
    }
}

impl From<sqlx::Error> for ApiError<ApiKeyError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(ApiKeyError::DatabaseError(value))
    }
}

impl From<serde_json::Error> for ApiError<ApiKeyError> {
    fn from(value: serde_json::Error) -> Self {
        ApiError::new(ApiKeyError::JsonError(value))
    }
}
