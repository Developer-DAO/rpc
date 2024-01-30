use super::errors::ApiError;
use crate::database::types::RELATIONAL_DATABASE;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use sqlx::types::time::OffsetDateTime;

pub async fn verify_payment(
    Path(user_address): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();

    let payment_validation: Option<PaymentValidation> = sqlx::query_as!(
        PaymentValidation,
        "SELECT PaymentInfo.planExpiration AS plan_expiration, Payments.date AS payment_date
         FROM PaymentInfo
         JOIN Payments ON PaymentInfo.customerEmail = Payments.customerEmail
         WHERE Payments.customerEmail = $1",
        user_address
    )
    .fetch_optional(db_connection)
    .await
    .map_err(|e| ApiError::new(Box::new(PaymentError::DatabaseError(e))))?; // Handle database errors
    match payment_validation {
        Some(payment_validation) => {
            if payment_validation.plan_expiration >= payment_validation.payment_date {
                // The payment is valid
                Ok((StatusCode::OK, "User payment is valid").into_response())
            } else {
                // The payment is invalid (expired)
                Err(ApiError::new(Box::new(PaymentError::PaymentInvalid)))
            }
        }
        None => {
            // Payment information not found
            Err(ApiError::new(Box::new(PaymentError::PaymentNotFound)))
        }
    }
}

#[derive(Debug)]
struct PaymentValidation {
    plan_expiration: OffsetDateTime,
    payment_date: OffsetDateTime,
}

// Error handling
#[derive(Debug)]
pub enum PaymentError {
    PaymentInvalid,
    PaymentNotFound,
    DatabaseError(sqlx::Error),
}

impl std::fmt::Display for PaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentError::PaymentNotFound => write!(f, "The user doesn't have an active payment"),
            PaymentError::DatabaseError(e) => write!(f, "{}", e),
            PaymentError::PaymentInvalid => write!(f, "The user payment is expired"),
        }
    }
}

impl std::error::Error for PaymentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PaymentError::PaymentInvalid => None,
            PaymentError::DatabaseError(e) => Some(e),
            PaymentError::PaymentNotFound => None,
        }
    }
}