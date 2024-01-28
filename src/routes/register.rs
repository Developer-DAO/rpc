use crate::database::types::{Customers, RELATIONAL_DATABASE};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use axum::{response::IntoResponse, Json};
use serde_json::json;

use super::{errors::ApiError, types::RegisterUser}; 

pub async fn register_user(Json(payload): Json<RegisterUser>) -> Result<impl IntoResponse, ApiError> {

    Ok(payload)

}
