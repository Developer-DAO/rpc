use super::errors::ApiError;
use crate::database::types::{ Asset, Chain as Chainlist, Database, FromHexStr};
use crate::eth_rpc::errors::EthCallError;
use crate::eth_rpc::types::{ GetTransactionByHash, RawGetTransactionByHashResponse, Receipt, Transfer, ETHEREUM_ENDPOINT};
use crate::
    database::types::{Payments, RELATIONAL_DATABASE}
;
use axum::http::Response;
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use dotenvy::dotenv;
use num::{BigInt, Num};
use sqlx::types::time::OffsetDateTime;
use std::error::Error;
use std::env;
use std::str::FromStr;

pub fn convert_hex_to_dec(hex_str: &str) -> String {
    BigInt::from_str_radix(hex_str, 16).unwrap().to_string()
}

use core::str;
//use ethers::core::utils::hex::FromHex;


const TOKENS_SUPPORTED: [&str; 8] = [
    "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
    "0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
    "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
    "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619",
    "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
    "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
    "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
];



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
async fn process_payment(customer_mail: &str, txhash: &str) -> Result<Response<axum::body::Body>, Box<dyn Error>> {
    dotenv().ok();
    let addy = env::var("PAYMENT_ADDY").unwrap();

    let transaction = GetTransactionByHash::new(txhash.to_owned());
    let provider = ETHEREUM_ENDPOINT.get().unwrap();
    let tx = provider.get_transaction_by_hash(&transaction).await?;
    let response = match tx.input.as_str() {
        "0x" => process_ether_transfer(&addy, &tx, customer_mail).await,
        _ => process_token_transfer(&addy, &tx, customer_mail).await,
    };

    // Convert all responses to a common type
    response.map(|inner_response| inner_response.into_response())
}

async fn process_ether_transfer(addy: &str, tx: &RawGetTransactionByHashResponse, customer_mail: &str) -> Result<Json<Payments>, Box<dyn Error>> {
    let to = tx.to.as_str();
    println!("{:?}" , to);
    if addy.to_lowercase() == to.to_lowercase() {
        let tx_value = tx.value.trim_start_matches("0x");
        let z = i64::from_str_radix(tx_value.into(), 16)?;
        let payment = Payments {
            customer_email: customer_mail.to_owned(),
            transaction_hash: tx.hash.to_owned(),
            asset: Asset::Ether,
            amount: z,
            chain: Chainlist::from_hex(&tx.chain_id)?,
            date: OffsetDateTime::now_utc(),
        };
        println!("{:?}" , payment.amount);
        is_txfinalized(&payment.transaction_hash).await?;
        insert_payment(Json(&payment)).await?;
        Ok(Json(payment))
    } else {
        Err(Box::new(ApiError::new(SubmitPaymentError::AddressMismatch)))
    }
}

async fn process_token_transfer(addy: &str, tx: &RawGetTransactionByHashResponse, customer_mail: &str) -> Result<Json<Payments>, Box<dyn Error>> {
    if TOKENS_SUPPORTED.iter().any(|e| tx.to.as_str().to_lowercase().contains(&e.to_lowercase())) {
        let res = Transfer::from_str(&tx.input)?;
        if addy.to_lowercase() == res.to.to_string().to_lowercase() {
            let z = i64::from_str_radix(&res.amount.to_string(), 16)?;
            let payment = Payments {
                customer_email: customer_mail.to_owned(),
                transaction_hash: tx.hash.to_owned(),
                asset: Asset::USDC,
                amount: z,
                chain: Chainlist::from_hex(&tx.chain_id)?,
                date: OffsetDateTime::now_utc(),
            };
            is_txfinalized(&payment.transaction_hash).await?;
            insert_payment(Json(&payment)).await?;
            Ok(Json(payment))
        } else {
            Err(Box::new(ApiError::new(SubmitPaymentError::AddressMismatch)))
        }
    } else {
        Err(Box::new(ApiError::new(SubmitPaymentError::UnsupportedToken)))
    }
}

async fn is_txfinalized(tx : &str) -> Result<bool , Box< dyn Error>>{
    dotenv().ok();
    let transaction = GetTransactionByHash::new(tx.to_owned());
    let provider = ETHEREUM_ENDPOINT.get().unwrap();
    let receipt = Receipt(transaction);
    let tx = provider.get_transaction_receipt(receipt).await?;
    if tx.status == "0x1"{
        Ok(true)
    } else {
        Err(Box::new(ApiError::new(SubmitPaymentError::TxNotFinalized)))
    }
}

async fn insert_payment(payment : Json<&Payments>) -> Result<impl IntoResponse, ApiError<SubmitPaymentError>>{
    println!("Testing insert payment");
    dotenv().unwrap();
    Database::init(None).await.unwrap();
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    sqlx::query!(
        "INSERT INTO Payments(customerEmail, transactionHash, asset  , amount, chain, date) 
            VALUES ($1, $2, $3, $4, $5, $6)",
        payment.customer_email,
        payment.transaction_hash,
        payment.asset as crate::database::types::Asset,
        payment.amount,
        payment.chain as crate::database::types::Chain,
        payment.date,
    )
    .execute(db_connection)
    .await?;
    println!("Finish insert");
    Ok(())
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

//Error handling for submitPayment
#[derive(Debug)]
pub enum SubmitPaymentError {
    TxhashError(EthCallError),
    TxDataError,
    AddressMismatch,
    UnsupportedToken,
    TxNotFinalized,
    DatabaseError(sqlx::Error),
}

impl std::fmt::Display for SubmitPaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitPaymentError::TxDataError => write!(f, "Can't get data from Tx"),
            SubmitPaymentError::DatabaseError(e) => write!(f, "{}", e),
            SubmitPaymentError::AddressMismatch => write!(f, "Tx destinatary is not valid"),
            SubmitPaymentError::TxhashError(e) => write!(f, "Transaction error: {}", e),
            SubmitPaymentError::UnsupportedToken =>write!(f, "Token not supported") , 
            SubmitPaymentError::TxNotFinalized => write!(f , "Tx not finalized")
        }
    }
}


impl std::error::Error for SubmitPaymentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SubmitPaymentError::TxDataError | SubmitPaymentError::AddressMismatch => None,
            SubmitPaymentError::DatabaseError(e) => Some(e),
            SubmitPaymentError::TxhashError(e) => Some(e),
            SubmitPaymentError::UnsupportedToken => None,
            SubmitPaymentError::TxNotFinalized => None
        }
    }
}


impl From<EthCallError> for SubmitPaymentError {
    fn from(error: EthCallError) -> Self {
        SubmitPaymentError::TxhashError(error)
    }
}
impl From<sqlx::Error> for ApiError<SubmitPaymentError> {
    fn from(error: sqlx::Error) -> Self {
        ApiError::new(SubmitPaymentError::DatabaseError(error))
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::types::{Asset, Chain, Payments};
    use crate::eth_rpc::types::Endpoints;
    use sqlx::types::time::OffsetDateTime;
    

    use std::error::Error as StdError;

    #[tokio::test]
    async fn usdc_tx() -> Result<(), Box<dyn StdError>> {
        // Initialize endpoints and handle potential errors explicitly
        Endpoints::init()?;

        println!("Sending payment");
        let arg1 = "0x8215cabb4634fac018ce551b20b381c62a6c808510e60eb0595f580fd8b8bf34";

        // Process the payment and handle results explicitly
        let response = process_payment("customer2@example.com", arg1).await;
        match response {
            Ok(_) => Ok(()),
            Err(e) => Err(e),  // Assuming e is already Box<dyn StdError>
        }
    }
    #[tokio::test]
    async fn eth_tx() -> Result<(), Box<dyn StdError>> {
        // Initialize endpoints and handle potential errors explicitly
        Endpoints::init()?;
    
        println!("Sending payment");
        let arg1 = "0x8fca1317d09136312b6edc742dd868e6d9e16982a8544d60d8d2ee79b304db0e";

        // Process the payment and handle results explicitly
        let response = process_payment("customer@example.com", arg1).await;
        match response {
            Ok(_) => Ok(()),
            Err(e) => Err(e),  // Assuming e is already Box<dyn StdError>
        }
    }

    #[tokio::test]
    async fn payment_insert() ->Result<(), jwt_simple::Error>{
        let payment = Payments {
            customer_email: "customer@example.com".to_string(),
            transaction_hash: "0x8215cabb4634fac018ce551b20b381c62a6c808510e60eb0595f580fd8b8bf34"
                .to_string(),
            asset: Asset::USDC,
            amount: 1000,
            chain: Chain::Optimism,
            date: OffsetDateTime::now_utc(),
        };
        println!("{:?}" , payment.asset);
        insert_payment(axum::Json(&payment)).await.unwrap();
        Ok(())
        
    }

}
