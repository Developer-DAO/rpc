use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::database::types::RELATIONAL_DATABASE;
use axum::{http::StatusCode, response::IntoResponse, Json};
use rand::{rngs::ThreadRng, Rng};
use serde::{Deserialize, Serialize};

use super::errors::ApiError;

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivationRequest {
    pub email: String,
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivationCode {
    pub verificationcode: String,
    pub activated: bool,
}

#[tracing::instrument]
pub async fn activate_account(
    Json(payload): Json<ActivationRequest>,
) -> Result<impl IntoResponse, ApiError<ActivationError>> {
    let db = RELATIONAL_DATABASE.get().unwrap(); 
    let code = sqlx::query_as!(
        ActivationCode,
        "SELECT verificationCode, activated FROM Customers where email = $1",
        &payload.email
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ApiError::new(ActivationError::UserNotFound))?;

    if code.activated {
        Err(ApiError::new(ActivationError::AlreadyActivated))?
    }

    if payload.code != code.verificationcode {
        Err(ApiError::new(ActivationError::InvalidCode))?
    }

    let new_code: u32 = ThreadRng::default().gen_range(10000000..99999999);
    sqlx::query!(
        "UPDATE Customers SET activated = true, verificationCode = $1 WHERE email = $2",
        new_code.to_string(),
        &payload.email
    )
    .execute(db)
    .await?;

    Ok((StatusCode::OK, "Account activated successfully").into_response())
}

#[derive(Debug)]
pub enum ActivationError {
    DatabaseError(sqlx::Error),
    UserNotFound,
    InvalidCode,
    AlreadyActivated,
}

impl Display for ActivationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ActivationError::DatabaseError(e) => {
                write!(f, "An error occured while querying the database: {}", e)
            }
            ActivationError::UserNotFound => write!(f, "User registration not found"),
            ActivationError::InvalidCode => {
                write!(f, "The code received was invalid, please try again")
            }
            ActivationError::AlreadyActivated => write!(f, "This account was already activated"),
        }
    }
}

impl Error for ActivationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ActivationError::DatabaseError(e) => Some(e),
            ActivationError::UserNotFound => None,
            ActivationError::InvalidCode => None,
            ActivationError::AlreadyActivated => None,
        }
    }
}

impl From<sqlx::Error> for ApiError<ActivationError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(ActivationError::DatabaseError(value))
    }
}
