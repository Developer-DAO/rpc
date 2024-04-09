use super::errors::ApiError;
use crate::database::types::{Chain as Chainlist, FromHexStr};
use crate::eth_rpc::types::{Chains, Endpoints, GetTransactionByHash, Receipt, ETHEREUM_ENDPOINT};
use crate::{
    database::types::{Payments, RELATIONAL_DATABASE},
    eth_rpc::types::Provider,
};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use crypto_bigint::Uint;
use crypto_bigint::{Encoding, Limb};
use ethers::signers::yubihsm::Uuid;
use hex;
use hex::decode;
use jwt_simple::reexports::anyhow::Chain;
use num::traits::SaturatingMul;
use num::{BigInt, Num};
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use std::borrow::{Borrow, BorrowMut};
use std::str::FromStr;

pub fn convert_hex_to_dec(hex_str: &str) -> String {
    BigInt::from_str_radix(hex_str, 16).unwrap().to_string()
}

use core::str;
use ethers::core::utils::hex::FromHex;

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
//Previous arg payload): Json<Payments>)
async fn process_payment(txhash: &str) -> Result<impl IntoResponse, Box<dyn std::error::Error>> {
    // Take in mind the struct that will be in the db
    let transaction = GetTransactionByHash::new(txhash.to_owned()); // Assuming it addes it automatically
    let provider = ETHEREUM_ENDPOINT.get().unwrap();
    let tx = provider.get_transaction_by_hash(&transaction).await?;
    let chain = Chainlist::from_hex(&tx.chain_id);
    println!("Chain id {:?}", chain);
    let tx_input = tx.input;
    println!("{:?}", tx_input);
    //let tx_decode = hex::decode(tx_input).unwrap();
    let buffer = <[u8; 12]>::from_hex(tx_input)?;
    let string = str::from_utf8(&buffer).expect("invalid buffer length");

    println!("decoded {:?}", string);
    // If not 0 in value check call inpit to extract usdc info
    //Amount for ether is the value and for token should be in call data
    //println!("This is the transaction by hash Response {:?}", &tx);
    let tx_value = convert_hex_to_dec(&tx.value.trim_start_matches("0x"));
    println!("Tx value {:?}", tx_value); // wei value 10.^18
                                         // We need to construct the receipt first
    let receipt = Receipt(transaction);
    //println!("This is the receipt of the transaction {:?}" , response);
    let response = provider.get_transaction_receipt(receipt).await?;
    // let input = response.
    println!("This is the receipt of the transaction {:?}", response);
    Ok((StatusCode::OK, "User payment submitted").into_response())
    // Should be corrected to handle the response tho
}

//async fn insert_payment(){}
// fn submit_payment(){}

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

//Error handling for submitPayment
#[derive(Debug)]
pub enum SubmitPaymentError {
    TxDataError,
    AmountError,
    DatabaseError(sqlx::Error),
}

impl std::fmt::Display for SubmitPaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitPaymentError::TxDataError => write!(f, "Can't get data from Tx"),
            SubmitPaymentError::DatabaseError(e) => write!(f, "{}", e),
            SubmitPaymentError::AmountError => write!(f, "Can't parse value from Tx"),
        }
    }
}

impl std::error::Error for SubmitPaymentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SubmitPaymentError::TxDataError => None,
            SubmitPaymentError::DatabaseError(e) => Some(e),
            SubmitPaymentError::AmountError => None,
        }
    }
}

impl From<sqlx::Error> for ApiError<SubmitPaymentError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(SubmitPaymentError::DatabaseError(value))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        database::types::{Asset, Chain, Payments},
        eth_rpc::types::Endpoints,
        routes::payment::process_payment,
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
        // I would treat this like the struc is the input of my function
        let payment = Payments {
            customer_email: "customer@example.com".to_string(),
            transaction_hash: "0xc9abd0b9745ca40417bad813cc012114b81f043ee7215db168f28f21abf7bafe"
                .to_string(),
            asset: Asset::USDC,
            amount: 1000,
            chain: Chain::Optimism,
            date: OffsetDateTime::now_utc(),
        };
        println!("Sending payment"); // Ensure submit_payment accepts axum::Json<Payments>
        let arg1 = "0xc9abd0b9745ca40417bad813cc012114b81f043ee7215db168f28f21abf7bafe";
        process_payment(arg1).await?;

        Ok(())
    }
}
