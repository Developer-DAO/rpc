use super::errors::ApiError;
use crate::database::types::{Asset, Chain as Chainlist, Database, FromHexStr};
use crate::eth_rpc::errors::EthCallError;
use crate::eth_rpc::types::{Chains, Endpoints, GetTransactionByHash, Receipt, Transfer, ETHEREUM_ENDPOINT};
use crate::{
    database::types::{Payments, RELATIONAL_DATABASE},
    eth_rpc::types::Provider,
};
use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
use crypto_bigint::Uint;
use crypto_bigint::{Encoding, Limb};
use dotenvy::dotenv;
use hex;
use hex::decode;
use jwt_simple::reexports::anyhow::Chain;
use num::{BigInt, Num};
use serde::Serialize;
use serde_json::from_str;
use sqlx::types::time::OffsetDateTime;
use std::borrow::{Borrow, BorrowMut};
use std::error::Error;
use std::io;
use std::ops::Sub;
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
async fn process_payment(customer_mail: &str , txhash: &str) -> Result<impl IntoResponse, Box<dyn Error>> {
    // Take in mind the struct that will be in the db

    //Note: Do we need to check the input of the chain to match with the chain id?
    let transaction = GetTransactionByHash::new(txhash.to_owned()); // Assuming it addes it automatically
    let provider = ETHEREUM_ENDPOINT.get().unwrap();
    let tx = provider.get_transaction_by_hash(&transaction).await?;
    let to = tx.to.as_str();
    println!("{:?}" ,TOKENS_SUPPORTED);

    //Can we compare bytes?
    //Lowercase for compatibility
    if TOKENS_SUPPORTED.iter().any(|e| to.to_lowercase().contains(&e.to_lowercase())) {
        println!("This stuff worked");
    } else {
        println!("This don't");
        //return Err(Box::new(ApiError::new(PaymentError::UnsupportedToken)));
    }
    //Necesito los checkeos antes de parsearlos por que si no para que tomarme el tiempo
    println!("This is the transaction hash {:?}" , tx );
    if tx.input == "0x"{
        //Let value , tx , amount 
        let tx_value = convert_hex_to_dec(&tx.value.trim_start_matches("0x"));
        let tx_asset = Asset::Ether;
        println!("Tx value {:?}", tx_value); // wei value 10.^18
        // We should get the two from the receipt 
        let chain = Chainlist::from_hex(&tx.chain_id)?;
        let payment = Payments {
            customer_email: customer_mail.to_owned(),
            transaction_hash: txhash.to_owned(),
            asset: tx_asset,
            amount: from_str(&tx.value)?,
            chain: chain,
            date: OffsetDateTime::now_utc(),
        };
        insert_payment(axum::Json(payment)).await?;
    
    } else {
         // Check later if this checks the method id 0xa9059cbb
        println!("Entering the else statement");
        let res = Transfer::from_str(&tx.input)?;
        let to = res.to;
        let value = res.amount.to_string();
        let z = i64::from_str_radix(&value, 16)?;
        let chain = Chainlist::from_hex(&tx.chain_id)?;
        println!("To {:?} , value {:?}" , to , &value);

        let payment = Payments {
            customer_email: customer_mail.to_owned(),
            transaction_hash: txhash.to_owned(),
            asset: Asset::USDC,
            amount: z,
            chain: chain,
            date: OffsetDateTime::now_utc(),
        };
        println!("Inserting payment");
        insert_payment(axum::Json(payment)).await?;
    }

    let receipt = Receipt(transaction);
    let receipt_response = provider.get_transaction_receipt(receipt).await?;
    if receipt_response.status == "0x1"{
        Ok((StatusCode::OK, "User payment submitted").into_response())
    } else {
        Ok((StatusCode::FORBIDDEN , "Payment request cannot be processed").into_response())
    }
}

async fn insert_payment(payment : Json<Payments>) -> Result<impl IntoResponse, ApiError<SubmitPaymentError>>{
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
    AmountError,
    DatabaseError(sqlx::Error),
}

impl std::fmt::Display for SubmitPaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitPaymentError::TxDataError => write!(f, "Can't get data from Tx"),
            SubmitPaymentError::DatabaseError(e) => write!(f, "{}", e),
            SubmitPaymentError::AmountError => write!(f, "Can't parse value from Tx"),
            SubmitPaymentError::TxhashError(e) => write!(f, "Transaction error: {}", e),
        }
    }
}


impl std::error::Error for SubmitPaymentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SubmitPaymentError::TxDataError | SubmitPaymentError::AmountError => None,
            SubmitPaymentError::DatabaseError(e) => Some(e),
            SubmitPaymentError::TxhashError(e) => Some(e),
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
    use crate::{
        database::types::{Asset, Chain, Payments},
        eth_rpc::types::Endpoints,
        routes::{errors::ApiError, payment::{process_payment, SubmitPaymentError}},
    };
    use super::insert_payment;
    use jwt_simple::reexports::anyhow::Ok;
    use sqlx::types::time::OffsetDateTime;
    use std::error::Error;
    use axum::response::IntoResponse;
    use serde::ser::StdError;
    //use std::result::Result::Ok;
    // Assuming axum::Json is required for submit_payment signature
 

    #[tokio::test]
    async fn usdc_tx() -> Result<(), jwt_simple::Error> {
        // Assuming Endpoints::init() exists and is necessary
        // Replace with actual initialization if required

        Endpoints::init().unwrap();
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
        let arg1 = "0x8215cabb4634fac018ce551b20b381c62a6c808510e60eb0595f580fd8b8bf34";
        process_payment(&payment.customer_email , arg1).await.unwrap();
        Ok(())
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
        insert_payment(axum::Json(payment)).await.unwrap();
        Ok(())
        
    }

}
