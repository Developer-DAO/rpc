use super::types::{EmailLogin, RegisterUser, SERVER_EMAIL};
use crate::database::types::{RELATIONAL_DATABASE, Role};
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{Json, http::StatusCode, response::IntoResponse};
use lettre::{
    Message, Transport,
    address::AddressError,
    message::{Mailbox, header::ContentType},
    transport::smtp::{self, authentication::Credentials},
};
use rand::{Rng, rngs::ThreadRng};
use thiserror::Error;
// use time::OffsetDateTime;
use siwe::generate_nonce;
use tokio::task::JoinError;

pub struct Dedup {
    pub email: String,
}

#[tracing::instrument(skip(payload), fields(email = %payload.email))]
pub async fn register_user(
    Json(payload): Json<RegisterUser>,
) -> Result<impl IntoResponse, RegisterUserError> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let exists: Option<Dedup> = sqlx::query_as!(
        Dedup,
        "SELECT email FROM Customers WHERE email = $1",
        &payload.email
    )
    .fetch_optional(db_connection)
    .await?;

    if exists.is_some() {
        Err(RegisterUserError::AlreadyRegistered)?
    }

    let hashed_pass: String = {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)?
            .to_string()
    };

    let verification_code: u32 = ThreadRng::default().gen_range(10000000..99999999);

    let server_email_info: &'static EmailLogin = SERVER_EMAIL.get().unwrap();
    let email_credentials = Credentials::new(
        server_email_info.address.to_string(),
        server_email_info.password.to_string(),
    );

    let server_mailbox: Mailbox =
        format!("Developer DAO RPC <{}>", server_email_info.address).parse()?;
    let user_email = payload.email.parse()?;

    let email = Message::builder()
        .from(server_mailbox)
        .to(user_email)
        .subject("D_D RPC Verification Code")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("Your verification code is: {verification_code}"))?;

    let mailer = smtp::SmtpTransport::starttls_relay("smtp.gmail.com")?
        .credentials(email_credentials)
        .build();

    let _: smtp::response::Response = mailer
        .send(&email)
        .expect("Failed to send verification email)");
    let mut transaction = db_connection.begin().await?;
    sqlx::query!(
        r#"INSERT INTO Customers(
                email, 
                password, 
                role,
                verificationcode, 
                nonce,
                balance,
                activated
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        &payload.email,
        hashed_pass,
        Role::Normie as Role,
        verification_code.to_string(),
        generate_nonce(),
        0,
        false,
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query!("INSERT INTO RpcPlans(email) VALUES ($1)", &payload.email)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok((StatusCode::OK, "User was successfully registered").into_response())
}

#[derive(Error, Debug)]
pub enum RegisterUserError {
    #[error("This user is already registered")]
    AlreadyRegistered,
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("An error occured while hashing a password")]
    HashingError,
    #[error(transparent)]
    EmailAddressParsingError(#[from] AddressError),
    #[error(transparent)]
    EmailBuilderError(#[from] lettre::error::Error),
    #[error(transparent)]
    SmtpError(#[from] lettre::transport::smtp::Error),
    #[error(transparent)]
    JoinError(#[from] JoinError),
}

impl IntoResponse for RegisterUserError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

impl From<argon2::password_hash::Error> for RegisterUserError {
    fn from(_: argon2::password_hash::Error) -> Self {
        RegisterUserError::HashingError
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        Database, EmailLogin, JWTKey, TcpListener, database::types::RELATIONAL_DATABASE,
        register_user, routes::types::RegisterUser,
    };
    use argon2::{
        Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString,
    };
    use axum::{Router, routing::post};
    use dotenvy::dotenv;
    use lettre::{
        Message, Transport,
        message::{Mailbox, header::ContentType},
        transport::smtp::{self, authentication::Credentials},
    };
    use rand::rngs::OsRng;
    use rand::{Rng, rngs::ThreadRng};

    #[test]
    fn hash_test() {
        let hashed_pass: String = {
            let salt = SaltString::generate(&mut OsRng);
            Argon2::default()
                .hash_password("testing_password".as_bytes(), &salt)
                .unwrap()
                .to_string()
        };

        Argon2::default()
            .verify_password(
                "testing_password".as_bytes(),
                &PasswordHash::new(&hashed_pass).unwrap(),
            )
            .unwrap();
    }

    #[tokio::test]
    async fn register() {
        dotenv().unwrap();
        JWTKey::init().unwrap();
        Database::init().await.unwrap();
        EmailLogin::init().unwrap();
        let to = dotenvy::var("SMTP_USERNAME").unwrap();

        tokio::spawn(async move {
            let app = Router::new().route("/api/register", post(register_user));
            let listener = TcpListener::bind("0.0.0.0:3111").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        let res = reqwest::Client::new()
            .post("http://localhost:3000/api/register")
            .json(&RegisterUser {
                email: to.to_string(),
                password: "test".to_string(),
            })
            .send()
            .await
            .unwrap()
            .text()
            .await
            .inspect(|e| println!("{e}"))
            .unwrap();

        assert_eq!(&res, "User was successfully registered");

        sqlx::query!("DELETE FROM Customers WHERE email = $1", &to)
            .execute(RELATIONAL_DATABASE.get().unwrap())
            .await
            .unwrap();

        sqlx::query!("DELETE FROM RpcPlans WHERE email = $1", &to)
            .execute(RELATIONAL_DATABASE.get().unwrap())
            .await
            .unwrap();

    }



    #[tokio::test]
    async fn mail() {
        dotenv().unwrap();

        let verification_code: u32 = ThreadRng::default().gen_range(10000000..99999999);

        let username = dotenvy::var("SMTP_USERNAME").unwrap();
        let password = dotenvy::var("SMTP_PASSWORD").unwrap();

        let user_email = username.parse().unwrap();
        let server_mailbox: Mailbox = format!("Developer DAO RPC <{}>", &username)
            .parse()
            .unwrap();

        let email_credentials = Credentials::new(username, password);

        let email = Message::builder()
            .from(server_mailbox)
            .to(user_email)
            .subject("D_D RPC Verification Code")
            .header(ContentType::TEXT_PLAIN)
            .body(format!("Your verification code is: {}", verification_code))
            .unwrap();
        let mailer = smtp::SmtpTransport::starttls_relay("smtp.gmail.com")
            .unwrap()
            .credentials(email_credentials)
            .build();

        mailer.send(&email).unwrap();
    }
}
