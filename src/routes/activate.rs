use crate::{
    EmailLogin,
    database::types::RELATIONAL_DATABASE,
    routes::{register::generate_verification_code, types::SERVER_EMAIL},
};
use axum::{Json, extract::Path, http::StatusCode, response::IntoResponse};
use lettre::{
    Message, SmtpTransport, Transport,
    message::{Mailbox, header::ContentType},
    transport::smtp::{self, authentication::Credentials},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivationRequest {
    pub email: sstr::Str<256>,
    pub code: sstr::Str<8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivationCode {
    pub verificationcode: String,
    pub activated: bool,
}

#[tracing::instrument]
pub async fn new_activation_code(
    Path(email): Path<sstr::Str<256>>,
) -> Result<impl IntoResponse, ActivationError> {
    let db = RELATIONAL_DATABASE.get().unwrap();

    let code = sqlx::query_as!(
        ActivationCode,
        "SELECT verificationCode, activated FROM Customers where email = $1",
        &email
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ActivationError::UserNotFound)?;

    if code.activated {
        Err(ActivationError::AlreadyActivated)?;
    }

    let new_code: String = generate_verification_code(8);
    sqlx::query!(
        "UPDATE Customers SET verificationCode = $1 WHERE email = $2",
        &new_code,
        &email
    )
    .execute(db)
    .await?;

    let server_email_info: &'static EmailLogin = SERVER_EMAIL.get().unwrap();
    let email_credentials = Credentials::new(
        server_email_info.address.to_string(),
        server_email_info.password.to_string(),
    );

    let server_mailbox: Mailbox =
        format!("Developer DAO RPC <{}>", server_email_info.address).parse()?;
    let user_email = email.parse()?;

    let email = Message::builder()
        .from(server_mailbox)
        .to(user_email)
        .subject("D_D RPC Verification Code")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("Your verification code is: {new_code}"))?;

    let mailer = SmtpTransport::starttls_relay("smtp.gmail.com")?
        .credentials(email_credentials)
        .build();

    let _: smtp::response::Response = mailer
        .send(&email)
        .expect("Failed to send verification email)");

    Ok((StatusCode::OK).into_response())
}

#[tracing::instrument]
pub async fn activate_account(
    Json(payload): Json<ActivationRequest>,
) -> Result<impl IntoResponse, ActivationError> {
    let db = RELATIONAL_DATABASE.get().unwrap();
    let code = sqlx::query_as!(
        ActivationCode,
        "SELECT verificationCode, activated FROM Customers where email = $1",
        &payload.email
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ActivationError::UserNotFound)?;

    if code.activated {
        Err(ActivationError::AlreadyActivated)?
    }

    if payload.code.as_str() != code.verificationcode.as_str() {
        Err(ActivationError::InvalidCode)?
    }

    let new_code: String = generate_verification_code(8);
    sqlx::query!(
        "UPDATE Customers SET activated = true, verificationCode = $1 WHERE email = $2",
        new_code.to_string(),
        &payload.email
    )
    .execute(db)
    .await?;

    Ok((StatusCode::OK, "Account activated successfully").into_response())
}

#[derive(Error, Debug)]
pub enum ActivationError {
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("User registration not found.")]
    UserNotFound,
    #[error("The code received was invalid, please try again.")]
    InvalidCode,
    #[error("This account is already activated. Please login.")]
    AlreadyActivated,
    #[error(transparent)]
    EmailError(#[from] lettre::transport::smtp::Error),
    #[error(transparent)]
    AddressError(#[from] lettre::address::AddressError),
    #[error(transparent)]
    LettreError(#[from] lettre::error::Error),
}

impl IntoResponse for ActivationError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ActivationError::UserNotFound => {
                (StatusCode::NOT_FOUND, self.to_string()).into_response()
            }
            ActivationError::InvalidCode => {
                (StatusCode::UNAUTHORIZED, self.to_string()).into_response()
            }
            ActivationError::AlreadyActivated => {
                (axum::http::StatusCode::FORBIDDEN, self.to_string()).into_response()
            }
            _ => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                self.to_string(),
            )
                .into_response(),
        }
    }
}
