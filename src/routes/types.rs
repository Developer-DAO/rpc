use crate::database::types::Role;
use alloy::primitives::Address;
use axum::{http::StatusCode, response::IntoResponse};
use jwt_simple::algorithms::HS256Key;
use serde::{Deserialize, Serialize};
use sstr::Str;
use std::error::Error;
use std::sync::OnceLock;

pub static JWT_KEY: OnceLock<HS256Key> = OnceLock::new();
pub static SERVER_EMAIL: OnceLock<EmailLogin> = OnceLock::new();

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, sqlx::Type)]
pub struct EmailAddress(pub String);

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, sqlx::Type)]
pub struct Password(pub String);

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone, sqlx::Type)]
pub struct SiweNonce(pub String);

impl Password {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// an email address that is not guarateed to be valid
impl EmailAddress {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for EmailAddress {
    fn from(value: String) -> Self {
        EmailAddress(value)
    }
}

impl From<String> for Password {
    fn from(value: String) -> Self {
        Password(value)
    }
}

impl From<String> for SiweNonce {
    fn from(value: String) -> Self {
        SiweNonce(value)
    }
}

pub struct JWTKey;

impl JWTKey {
    pub fn init() -> Result<(), Box<dyn std::error::Error>> {
        let hex_string = dotenvy::var("JWT_KEY")?;
        let key = HS256Key::from_bytes(&hex::decode(hex_string)?);
        JWT_KEY.get_or_init(|| key);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub role: Role,
    pub email: EmailAddress,
    pub wallet: Option<Address>,
}

pub struct EmailLogin {
    pub address: &'static str,
    pub password: &'static str,
}

impl EmailLogin {
    pub fn new(address: &'static str, password: &'static str) -> Self {
        Self { address, password }
    }

    pub fn init() -> Result<(), Box<dyn Error>> {
        let email = dotenvy::var("SMTP_USERNAME")?;
        let password = dotenvy::var("SMTP_PASSWORD")?;
        SERVER_EMAIL.get_or_init(|| EmailLogin::new(email.leak(), password.leak()));
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUser {
    pub email: Str<256>,
    pub password: Str<256>,
}

impl IntoResponse for RegisterUser {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, self).into_response()
    }
}
