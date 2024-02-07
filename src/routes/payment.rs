use super::errors::ApiError;
use crate::database::types::RELATIONAL_DATABASE;
use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use sqlx::types::time::OffsetDateTime;

#[tracing::instrument]
pub async fn verify_subscription(
    Path(email_address): Path<String>,
) -> Result<impl IntoResponse, ApiError<PaymentError>> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let utc_now = OffsetDateTime::now_utc();
    let payment_validation: Option<PaymentValidation> = sqlx::query_as!(
        PaymentValidation,
        "SELECT PaymentInfo.planExpiration AS plan_expiration
        FROM PaymentInfo
        WHERE PaymentInfo.customerEmail = $1",
        email_address
    )
    .fetch_optional(db_connection)
    .await?;

    match payment_validation {
        Some(payment_validation) => {
            if payment_validation.plan_expiration >= utc_now {
                // The payment is valid
                Ok((StatusCode::OK, "User payment is valid").into_response())
            } else {
                // The payment is invalid (expired)
                Err(ApiError::new(PaymentError::PaymentExpired))
            }
        }
        None => {
            // Payment information not found
            Err(ApiError::new(PaymentError::PaymentNotFound))
        }
    }
}

#[derive(Debug)]
struct PaymentValidation {
    plan_expiration: OffsetDateTime,
}

// Error handling
#[derive(Debug)]
pub enum PaymentError {
    PaymentExpired,
    PaymentNotFound,
    DatabaseError(sqlx::Error),
}

impl std::fmt::Display for PaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentError::PaymentNotFound => write!(f, "The user doesn't have an active payment"),
            PaymentError::DatabaseError(e) => write!(f, "{}", e),
            PaymentError::PaymentExpired => write!(f, "The user payment is expired"),
        }
    }
}

impl std::error::Error for PaymentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PaymentError::PaymentExpired => None,
            PaymentError::DatabaseError(e) => Some(e),
            PaymentError::PaymentNotFound => None,
        }
    }
}

impl From<sqlx::Error> for ApiError<PaymentError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(PaymentError::DatabaseError(value))
    }
}
