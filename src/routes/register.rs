use crate::database::types::{Customers, RELATIONAL_DATABASE};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use axum::{http::StatusCode, response::IntoResponse, Json};
use rand::{rngs::ThreadRng, Rng};

use super::{errors::ApiError, types::RegisterUser};

pub async fn register_user(
    Json(payload): Json<RegisterUser>,
) -> Result<impl IntoResponse, ApiError> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();

    let account: Option<Customers> = sqlx::query_as!(
        Customers,
        "SELECT * FROM Customers WHERE email = $1",
        &payload.email
    )
    .fetch_optional(db_connection)
    .await?;

    if let Some(_) = account {
        return Err(ApiError::new(Box::new(RegisterUserError::AlreadyRegistered)));
    }

    let hashed_pass: String = {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(&payload.password.as_bytes(), &salt)?
            .to_string()
    };
    
    let verification_code: u32 = ThreadRng::default().gen();

    sqlx::query!("INSERT INTO Customers(email, wallet, password, verificationCode, activated) 
            VALUES ($1, $2, $3, $4, $5)",
            payload.email, payload.wallet, hashed_pass, verification_code.to_string(), false,            
    ).execute(db_connection).await?;

    Ok((StatusCode::OK, "User was successfully registered").into_response())
}

#[derive(Debug)]
pub enum RegisterUserError {
    AlreadyRegistered,
    DatabaseError(sqlx::Error),
    HashingError(argon2::password_hash::Error)
}

impl std::fmt::Display for RegisterUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterUserError::AlreadyRegistered => write!(
                f,
                "The user you are attempting to register already exists. Please try logging in."
            ),
            RegisterUserError::DatabaseError(e) => write!(f, "{}", e),
            RegisterUserError::HashingError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for RegisterUserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RegisterUserError::AlreadyRegistered => None,
            RegisterUserError::DatabaseError(e) => Some(e),
            RegisterUserError::HashingError(_) => None
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(Box::new(RegisterUserError::DatabaseError(value)))
    }
}

impl From<argon2::password_hash::Error> for ApiError {
    fn from(value: argon2::password_hash::Error) -> Self {
        ApiError::new(Box::new(RegisterUserError::HashingError(value)))
    }
}
