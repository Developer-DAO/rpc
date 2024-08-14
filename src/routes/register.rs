use super::types::{Email, RegisterUser, SERVER_EMAIL};
use crate::database::types::{Customers, Role, RELATIONAL_DATABASE};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use axum::{http::StatusCode, response::IntoResponse, Json};
use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::{self, authentication::Credentials},
    Message, Transport,
};
use rand::{rngs::ThreadRng, Rng};
use thiserror::Error;
use tokio::{join, task::JoinError};

#[tracing::instrument]
pub async fn register_user(
    Json(payload): Json<RegisterUser>,
) -> Result<impl IntoResponse, RegisterUserError> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let transaction = db_connection.begin().await?;
    let account: Option<Customers> = sqlx::query_as!(
        Customers,
        r#"SELECT email, wallet, password, activated, verificationCode, role as "role!: Role" FROM Customers WHERE email = $1"#,
        &payload.email
    )
    .fetch_optional(db_connection)
    .await?;

    if account.is_some() {
        Err(RegisterUserError::AlreadyRegistered)?
    }

    let hashed_pass: String = {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt)?
            .to_string()
    };

    let verification_code: u32 = ThreadRng::default().gen_range(10000000..99999999);

    let server_email_info: &'static Email = SERVER_EMAIL.get().unwrap();
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
        .body(format!("Your verification code is: {}", verification_code))?;

    let mailer = smtp::SmtpTransport::starttls_relay("smtp.gmail.com")?
        .credentials(email_credentials)
        .build();

    let send_mail: tokio::task::JoinHandle<Result<(), RegisterUserError>> =
        tokio::spawn(async move {
            let _: smtp::response::Response = mailer.send(&email)?;
            Ok(())
        });

    let db_write = tokio::spawn(async move {
        let res = sqlx::query!(
            "INSERT INTO Customers(email, wallet, role, password, verificationCode, credits, activated) 
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &payload.email,
            payload.wallet,
            Role::Normie as Role,
            hashed_pass,
            verification_code.to_string(),
            0,
            false,
        )
        .execute(db_connection)
        .await;

        transaction.commit().await?;

        res
    });

    let (res_mail, res_db_write) = join!(send_mail, db_write);
    res_mail??;
    res_db_write??;

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
        database::types::RELATIONAL_DATABASE, register_user, routes::types::RegisterUser, Database,
        Email, JWTKey, TcpListener,
    };
    use argon2::{
        password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    };
    use axum::{routing::post, Router};
    use dotenvy::dotenv;
    use lettre::{
        message::{header::ContentType, Mailbox},
        transport::smtp::{self, authentication::Credentials},
        Message, Transport,
    };
    use rand::rngs::OsRng;
    use rand::{rngs::ThreadRng, Rng};

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
        Email::init().unwrap();
        let to = dotenvy::var("SMTP_USERNAME").unwrap();

        tokio::spawn(async move {
            let app = Router::new().route("/api/register", post(register_user));
            let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        reqwest::Client::new()
            .post("http://localhost:3000/api/register")
            .json(&RegisterUser {
                email: to.to_string(),
                wallet: "0x0c5E7D8C1494B74891e4c6539Be96C8e2402dcEF".to_string(),
                password: "test".to_string(),
            })
            .send()
            .await
            .unwrap();

        // sqlx::query!("DELETE FROM Customers WHERE email = $1", &to)
        //     .execute(RELATIONAL_DATABASE.get().unwrap())
        //     .await
        //     .unwrap();
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
