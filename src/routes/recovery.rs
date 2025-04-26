use crate::database::types::RELATIONAL_DATABASE;
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};
use lettre::{
    Message, SmtpTransport, Transport,
    address::AddressError,
    message::{Mailbox, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use rand::{Rng, rngs::ThreadRng};
use serde::{Deserialize, Serialize};
use std::num::ParseIntError;
use thiserror::Error;

use super::types::{Email, SERVER_EMAIL};

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
) -> Result<impl IntoResponse, RecoveryError> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let email: String = payload;
    let user_email = email.parse::<Mailbox>()?;
    sqlx::query!("SELECT email FROM Customers WHERE email = $1", email)
        .fetch_optional(db_connection)
        .await?
        .ok_or_else(|| RecoveryError::UserNotFound)?;

    let server_email_info: &'static Email = SERVER_EMAIL.get().unwrap();
    let email_credentials = Credentials::new(
        server_email_info.address.to_string(),
        server_email_info.password.to_string(),
    );

    let server_mailbox: Mailbox =
        format!("Developer DAO RPC <{}>", server_email_info.address).parse()?;

    let verification_code: u32 = ThreadRng::default().gen_range(10000000..99999999);

    let res1: tokio::task::JoinHandle<Result<(), RecoveryError>> = tokio::spawn(async move {
        sqlx::query!(
            "UPDATE Customers SET verificationCode = $1 WHERE email = $2",
            verification_code.to_string(),
            &email
        )
        .execute(db_connection)
        .await?;
        Ok(())
    });

    let res2: tokio::task::JoinHandle<Result<(), RecoveryError>> = tokio::spawn(async move {
        let email = Message::builder()
            .from(server_mailbox)
            .to(user_email)
            .subject("D_D RPC Password Reset Code")
            .header(ContentType::TEXT_PLAIN)
            .body(format!("Your reset code is: {verification_code}"))?;

        let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")?
            .credentials(email_credentials)
            .build();

        mailer.send(&email)?;
        Ok(())
    });

    let (one, two) = tokio::join!(res1, res2);
    one??;
    two??;

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
) -> Result<impl IntoResponse, RecoveryError> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let account: UpdatePasswordResponse = sqlx::query_as!(
        UpdatePasswordResponse,
        "SELECT verificationCode FROM Customers WHERE email = $1",
        &payload.email
    )
    .fetch_optional(db_connection)
    .await?
    .ok_or_else(|| RecoveryError::UserNotFound)?;

    if payload.code != account.verificationcode {
        Err(RecoveryError::IncorrectCode(payload.code.parse::<u32>()?))?
    }

    //update password with new hash
    let hashed_pass: String = {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)
            .map_err(|_| RecoveryError::PasswordHashingError)?
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

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("The user could not be found with the given email!")]
    UserNotFound,
    #[error("This user has not yet completed the registration process.")]
    AccountNotActivated,
    #[error(transparent)]
    EmailTransportError(#[from] lettre::transport::smtp::Error),
    #[error(transparent)]
    EmailError(#[from] lettre::error::Error),
    #[error(transparent)]
    EmailAddressError(#[from] AddressError),
    #[error("The submitted code was incorrect: {0}")]
    IncorrectCode(u32),
    #[error(transparent)]
    ParsingError(#[from] ParseIntError),
    #[error("Failed to parse string into an u32")]
    RouteArgumentsIncorrect,
    #[error("Failed to hash password")]
    PasswordHashingError,
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}

impl IntoResponse for RecoveryError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
