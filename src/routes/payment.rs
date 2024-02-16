use super::errors::ApiError;
use crate::eth_rpc::types::{Endpoints, GetTransactionByHash, Receipt, ETHEREUM_ENDPOINT};
use crate::{
    database::types::{Payments, RELATIONAL_DATABASE},
    eth_rpc::types::Provider,
};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use crypto_bigint::U256;
use crypto_bigint::{Encoding, Limb};
use hex;
use num::{BigInt, Num};
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use std::borrow::{Borrow, BorrowMut};
use std::str::FromStr;

pub fn convert_hex_to_dec(hex_str: &str) -> String {
    BigInt::from_str_radix(hex_str, 16).unwrap().to_string()
}

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

async fn submit_payment(Json(payload): Json<Payments>) -> Result<(), Box<dyn std::error::Error>> {
    // The scope of dis function will be only to recieve the paylad an put that in the db
    // We will recieve a tx hash so we need to parse that to the correct elements that will be in the database
    //let hash = payload.transaction_hash; // Just accessing the transa
    let transaction = GetTransactionByHash::new(payload.transaction_hash.to_owned()); // Assuming it addes it automatically
    println!(" The hash of the transaction is {:?}", transaction);
    let provider = ETHEREUM_ENDPOINT.get().unwrap();
    let transaction = provider.get_transaction_by_hash(transaction).await?;
    let dec_str = convert_hex_to_dec(&transaction.value.trim_start_matches("0x"));
    println!("{:?}", dec_str);
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
    use crate::{
        database::types::{Asset, Chain, Payments},
        eth_rpc::types::Endpoints,
    };
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
            transaction_hash: "0x10d26a9726e85f6bd33b5a1455219d8d56dd53d105e69e1be062119e8c7808a2"
                .to_string(),
            asset: Asset::USDC,
            amount: 1000,
            chain: Chain::Optimism,
            date: OffsetDateTime::now_utc(),
        };
        println!("Sending payment"); // Ensure submit_payment accepts axum::Json<Payments>
        submit_payment(Json(payment)).await?;

        Ok(())
    }
}
