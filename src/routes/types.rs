use std::error::Error;

use axum::{http::StatusCode, response::IntoResponse};
use serde::{Serialize, Deserialize};
use crate::eth_rpc::types::Address;
use std::sync::OnceLock;

pub static SERVER_EMAIL: OnceLock<Email> = OnceLock::new();

pub struct Email {
    pub address: &'static str,
    pub password: &'static str,
}

impl Email {
    
    pub fn new(address: &'static str, password: &'static str) -> Self {
        Self {
            address, 
            password
        }
    }

    pub fn init() -> Result<(), Box<dyn Error>> {
       let email = dotenvy::var("SMTP_USERNAME")?;
       let password = dotenvy::var("SMTP_PASSWORD")?;
       SERVER_EMAIL.get_or_init(|| Email::new(email.leak(), password.leak()));
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUser {
    pub email: String, 
    pub wallet: String, 
    pub password: String,
}

impl IntoResponse for RegisterUser {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self).into_response()
    }
}

trait Normalize {
    fn normalize(&self) -> Result<(), Box<dyn Error>>;
}

impl Normalize for RegisterUser {
    fn normalize(&self) -> Result<(), Box<dyn Error>> {
        Address::try_address(&self.wallet)?;
        Ok(())
    }
}