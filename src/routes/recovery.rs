use crate::database::types::RELATIONAL_DATABASE;
use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use core::fmt;
use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use rand::{rngs::ThreadRng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{Display, Formatter},
    num::ParseIntError,
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};

use super::{
    errors::ApiError,
    types::{Email, SERVER_EMAIL},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResetPasswordByEmail {
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResetResponse {
    pub activated: bool,
    pub verificationcode: String,
}

#[tracing::instrument]
pub async fn recover_password_email(
    Path(payload): Path<String>,
) -> Result<impl IntoResponse, ApiError<RecoveryError>> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let email = payload;
    let user_email = email.parse::<Mailbox>()?;
    sqlx::query!(
        "SELECT email FROM Customers WHERE email = $1",
        email
    )
    .fetch_optional(db_connection)
    .await?
    .ok_or_else(|| ApiError::new(RecoveryError::UserNotFound))?;

    let server_email_info: &'static Email = SERVER_EMAIL.get().unwrap();
    let email_credentials = Credentials::new(
        server_email_info.address.to_string(),
        server_email_info.password.to_string(),
    );

    let server_mailbox: Mailbox =
        format!("Developer DAO RPC <{}>", server_email_info.address).parse()?;

    let verification_code: u32 = ThreadRng::default().gen_range(10000000..99999999);
    sqlx::query!(
        "UPDATE Customers SET verificationCode = $1 WHERE email = $2",
        verification_code.to_string(),
        &email
    )
    .execute(db_connection)
    .await?;

    let email = Message::builder()
        .from(server_mailbox)
        .to(user_email)
        .subject("D_D RPC Password Reset Code")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("Your reset code is: {}", verification_code))?;

    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")?
        .credentials(email_credentials)
        .build();

    mailer.send(&email)?;

    Ok((
        StatusCode::OK,
        "An email has been sent to the email provided.",
    ))
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdatePassword {
    code: String,
    email: String,
    password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdatePasswordResponse {
    verificationcode: String,
}

#[tracing::instrument]
pub async fn update_password(
    Json(payload): Json<UpdatePassword>,
) -> Result<impl IntoResponse, ApiError<RecoveryError>> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let account: UpdatePasswordResponse = sqlx::query_as!(
        UpdatePasswordResponse,
        "SELECT verificationCode FROM Customers WHERE email = $1",
        &payload.email
    )
    .fetch_optional(db_connection)
    .await?
    .ok_or_else(|| ApiError::new(RecoveryError::UserNotFound))?;

    if payload.code != account.verificationcode {
        Err(ApiError::new(RecoveryError::IncorrectCode(
            payload.code.parse::<u32>()?,
        )))?
    }
    
    //update password with new hash 
    let hashed_pass: String = {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)?
            .to_string()
    };

    let verification_code: u32 = ThreadRng::default().gen_range(10000000..99999999);
    sqlx::query!(
        "UPDATE Customers SET verificationCode = $1, password = $3, activated = true WHERE email = $2",
        verification_code.to_string(),
        &payload.email,
        hashed_pass,
    )
    .execute(db_connection)
    .await?;

    Ok((StatusCode::OK, "Password changed successfully"))
}

#[derive(Debug)]
pub enum RecoveryError {
    DatabaseError(sqlx::Error),
    UserNotFound,
    AccountNotActivated,
    EmailTransportError(lettre::transport::smtp::Error),
    EmailError(lettre::error::Error),
    EmailAddressError(AddressError),
    IncorrectCode(u32),
    ParsingError(ParseIntError),
    RouteArgumentsIncorrect,
    PasswordHashingError(argon2::password_hash::Error),
}

impl Display for RecoveryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryError::RouteArgumentsIncorrect => {
                write!(f, "Error: no path argument was provided to this route.")
            }
            RecoveryError::DatabaseError(_) => {
                write!(f, "An issue occurred while querying the database.")
            }
            RecoveryError::UserNotFound => {
                write!(f, "The user could not be found with the given email!")
            }
            RecoveryError::AccountNotActivated => write!(
                f,
                "This user has not yet completed the registration process."
            ),
            RecoveryError::EmailAddressError(_) => write!(
                f,
                "An error occurred while parsing the provided email address."
            ),
            RecoveryError::EmailTransportError(_) => write!(
                f,
                "An error occurred while sending your email through SMTP."
            ),
            RecoveryError::EmailError(_) => {
                write!(f, "An error occurred while bulding your email.")
            }
            RecoveryError::IncorrectCode(num) => {
                write!(f, "The submitted code was incorrect: {}", num)
            }
            RecoveryError::ParsingError(_) => write!(f, "Failed to parse string into an u32"),
            RecoveryError::PasswordHashingError(e) => write!(f, "Failed to hash password: {}", e)
        }
    }
}

impl Error for RecoveryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RecoveryError::RouteArgumentsIncorrect => None,
            RecoveryError::DatabaseError(e) => Some(e),
            RecoveryError::UserNotFound => None,
            RecoveryError::AccountNotActivated => None,
            RecoveryError::EmailTransportError(e) => Some(e),
            RecoveryError::EmailError(e) => Some(e),
            RecoveryError::EmailAddressError(e) => Some(e),
            RecoveryError::IncorrectCode(_) => None,
            RecoveryError::ParsingError(e) => Some(e),
            RecoveryError::PasswordHashingError(_) => None,
        }
    }
}

impl From<sqlx::Error> for ApiError<RecoveryError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(RecoveryError::DatabaseError(value))
    }
}

impl From<lettre::transport::smtp::Error> for ApiError<RecoveryError> {
    fn from(value: lettre::transport::smtp::Error) -> Self {
        ApiError::new(RecoveryError::EmailTransportError(value))
    }
}

impl From<lettre::error::Error> for ApiError<RecoveryError> {
    fn from(value: lettre::error::Error) -> Self {
        ApiError::new(RecoveryError::EmailError(value))
    }
}

impl From<AddressError> for ApiError<RecoveryError> {
    fn from(value: AddressError) -> Self {
        ApiError::new(RecoveryError::EmailAddressError(value))
    }
}

impl From<ParseIntError> for ApiError<RecoveryError> {
    fn from(value: ParseIntError) -> Self {
        ApiError::new(RecoveryError::ParsingError(value))
    }
}

impl From<argon2::password_hash::Error> for ApiError<RecoveryError> {
    fn from(value: argon2::password_hash::Error) -> Self {
        ApiError::new(RecoveryError::PasswordHashingError(value))
    }
}
