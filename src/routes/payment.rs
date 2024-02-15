use std::borrow::{Borrow, BorrowMut};

use super::errors::ApiError;
use crate::{database::types::{Payments, RELATIONAL_DATABASE}, eth_rpc::types::Provider};
use axum::{extract::Path, http::StatusCode, response::IntoResponse , Json};
use sqlx::types::time::OffsetDateTime;
use crate::eth_rpc::types::{Endpoints, GetTransactionByHash, ETHEREUM_ENDPOINT, Receipt};

#[tracing::instrument]
pub async fn verify_subscription(
    Path(email_address): Path<String>,
) -> Result<impl IntoResponse, ApiError<PaymentError>> {
    let payment_validation: PaymentValidation = sqlx::query_as!(
        PaymentValidation,
        "SELECT PaymentInfo.planExpiration AS plan_expiration
        FROM PaymentInfo
        WHERE PaymentInfo.customerEmail = $1",
        email_address
    )
    .fetch_optional(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .ok_or_else(|| ApiError::new(PaymentError::PaymentNotFound))?;

    if payment_validation.plan_expiration < OffsetDateTime::now_utc() {
        Err(ApiError::new(PaymentError::PaymentExpired))?
    }
    
    Ok((StatusCode::OK, "User payment is valid").into_response())
}

async fn submit_payment(Json(payload): Json<Payments>)-> Result<() , Box<dyn std::error::Error>>{
    let hash = payload.transaction_hash;
    let transaction = GetTransactionByHash::new(hash.to_owned());
    let amount_paid = transaction.data;
    println!("{:?}", amount_paid);
    let provider = ETHEREUM_ENDPOINT.get().unwrap();
    let args = GetTransactionByHash::new(hash.to_owned());
    let transaction = provider.get_transaction_by_hash(args).await?;
    println!("{}", transaction.value);
    Ok(())
    // Should be corrected to handle the response tho 
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


#[cfg(test)]
mod tests {
    use super::submit_payment;
    use crate::{database::types::{Asset, Chain, Payments}, eth_rpc::types::Endpoints};
    use sqlx::types::time::OffsetDateTime;
    use std::error::Error;
    // Assuming axum::Json is required for submit_payment signature
    use axum::Json;

    #[tokio::test]
    async fn get_tx_by_hash() -> Result<(), Box<dyn Error>> {
        // Assuming Endpoints::init() exists and is necessary
        // Replace with actual initialization if required
        Endpoints::init()?;

        let payment = Payments {
            customer_email: "customer@example.com".to_string(),
            transaction_hash: "0x10d26a9726e85f6bd33b5a1455219d8d56dd53d105e69e1be062119e8c7808a2".to_string(),
            asset: Asset::USDC,
            amount: 1000,
            chain: Chain::Optimism,
            date: OffsetDateTime::now_utc(),
        };

        // Ensure submit_payment accepts axum::Json<Payments>
        submit_payment(Json(payment)).await?;

        Ok(())
    }
}

