use crate::database::types::RELATIONAL_DATABASE;
use axum::{http::StatusCode, response::IntoResponse, Json};
use rand::{rngs::ThreadRng, Rng};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
) -> Result<impl IntoResponse, ActivationError> {
    let db = RELATIONAL_DATABASE.get().unwrap();
    let code = sqlx::query_as!(
        ActivationCode,
        "SELECT verificationCode, activated FROM Customers where email = $1",
        &payload.email
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ActivationError::UserNotFound)?;

    if code.activated {
        Err(ActivationError::AlreadyActivated)?
    }

    if payload.code != code.verificationcode {
        Err(ActivationError::InvalidCode)?
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

#[derive(Error, Debug)]
pub enum ActivationError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("User registration not found.")]
    UserNotFound,
    #[error("The code received was invalid, please try again.")]
    InvalidCode,
    #[error("This account is already activated. Please login.")]
    AlreadyActivated,
}

impl IntoResponse for ActivationError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            self.to_string(),
        )
            .into_response()
    }
}
