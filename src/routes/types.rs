use crate::database::types::Role;
use alloy::primitives::Address;
use axum::{http::StatusCode, response::IntoResponse};
use jwt_simple::algorithms::HS256Key;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::error::Error;
use std::sync::OnceLock;
pub static JWT_KEY: OnceLock<HS256Key> = OnceLock::new();
pub static SERVER_EMAIL: OnceLock<EmailLogin> = OnceLock::new();

/// Does not support generic or regular structs
/// Works for new type structs with one inner type regardless of lifetimes
/// Inner types must implement sqlx::Type
/// Supports doc comments and derive macros
macro_rules! PgNewType {
    ($($(#[doc = $doc:expr])* $(#[derive($($trait:ty),+)])? $visibility:vis struct $newtype:ident$(<$lt:lifetime>)?($inner_vis:vis $type:ty);)*) => {
        use sqlx::{Type, Decode};
        use std::fmt::Display;
        $(
             $(#[doc = $doc])*
             $(#[derive($($trait,)+)])?
             $visibility struct $newtype$(<$lt>)?($inner_vis $type);

             impl$(<$lt>)? Display for $newtype$(<$lt>)? {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.0.fmt(f)
                }
             }

             impl<'a, DB> sqlx::Decode<'a, DB> for $newtype$(<$lt>)?
                where DB: sqlx::Database,
                      $type: Decode<'a, DB>
             {
                fn decode(arg: <DB as sqlx::Database>::ValueRef<'a>) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
                    <$type as sqlx::Decode<DB>>::decode(arg).map(|t| $newtype(t))
                }
             }

             impl<'a, DB> sqlx::Encode<'a, DB> for $newtype$(<$lt>)?
                 where
                     DB: sqlx::Database,
                    $type: sqlx::Encode<'a, DB>,
             {
                fn encode(
                    self,
                    buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'a>,
                ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
                where
                    Self: Sized,
                {
                    <$type as sqlx::Encode<DB>>::encode(self.0, buf)
                }

                fn encode_by_ref(
                    &self,
                    buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'a>,
                ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                    <$type as sqlx::Encode<DB>>::encode(self.0.clone(), buf)
                }
            }

            impl<$($lt,)?DB> Type<DB> for $newtype$(<$lt>)?
                where DB: sqlx::Database,
                    $type: Type<DB>
            {
                fn type_info() -> <DB as sqlx::Database>::TypeInfo {
                    <$type as Type<DB>>::type_info()
                }
            }

        )+
    }
}

PgNewType! {
    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
    pub struct EmailAddress<'a>(pub Cow<'a, str>);

    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
    pub struct Password<'a>(pub Cow<'a, str>);

    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
    pub struct SiweNonce<'a>(pub Cow<'a, str>);
}

// impl From<String> for PraiseKek {
//     fn from(value: String) -> Self {
//         PraiseKek(
//             OffsetDateTime::parse(&value, &Rfc3339)
//                 .expect("Date is not Rfc3339 compliant"),
//         )
//     }
// }

impl<'a> Password<'a> {
    pub fn as_bytes(&'a self) -> &'a [u8] {
        self.0.as_bytes()
    }
    pub fn as_str(&'a self) -> &'a str {
        &self.0
    }
}

/// an email address that is not guarateed to be valid
impl<'a> EmailAddress<'a> {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'a> From<String> for EmailAddress<'a> {
    fn from(value: String) -> Self {
        EmailAddress(value.into())
    }
}

impl<'a> From<String> for Password<'a> {
    fn from(value: String) -> Self {
        Password(value.into())
    }
}

impl<'a> From<String> for SiweNonce<'a> {
    fn from(value: String) -> Self {
        SiweNonce(value.into())
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
pub struct Claims<'a> {
    pub role: Role,
    pub email: EmailAddress<'a>,
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
    pub email: String,
    pub password: String,
}

impl IntoResponse for RegisterUser {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, self).into_response()
    }
}
