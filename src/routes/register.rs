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

use super::{
    errors::ApiError,
    types::{Email, RegisterUser, SERVER_EMAIL},
};

#[tracing::instrument]
pub async fn register_user(
    Json(payload): Json<RegisterUser>,
) -> Result<impl IntoResponse, ApiError<RegisterUserError>> {
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let account: Option<Customers> = sqlx::query_as!(
        Customers,
        r#"SELECT email, wallet, password, activated, verificationCode, role as "role!: Role" FROM Customers WHERE email = $1"#,
        &payload.email
    )
    .fetch_optional(db_connection)
    .await?;

    if account.is_some() {
        Err(ApiError::new(RegisterUserError::AlreadyRegistered))?
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

    mailer.send(&email)?;

    sqlx::query!(
        "INSERT INTO Customers(email, wallet, role, password, verificationCode, activated) 
            VALUES ($1, $2, $3, $4, $5, $6)",
        &payload.email,
        payload.wallet,
        Role::Normie as Role,
        hashed_pass,
        verification_code.to_string(),
        false,
    )
    .execute(db_connection)
    .await?;

    Ok((StatusCode::OK, "User was successfully registered").into_response())
}

#[derive(Debug)]
pub enum RegisterUserError {
    AlreadyRegistered,
    DatabaseError(sqlx::Error),
    HashingError(argon2::password_hash::Error),
    EmailAddressParsingError(AddressError),
    EmailBuilderError(lettre::error::Error),
    SmtpError(lettre::transport::smtp::Error),
}

impl std::fmt::Display for RegisterUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterUserError::AlreadyRegistered => write!(
                f,
                "The user you are attempting to register already exists. Please try logging in."
            ),
            RegisterUserError::DatabaseError(e) => write!(f, "{}", e),
            RegisterUserError::HashingError(e) => write!(f, "{}", e),
            RegisterUserError::EmailAddressParsingError(e) => write!(f, "{}", e),
            RegisterUserError::EmailBuilderError(e) => write!(f, "{}", e),
            RegisterUserError::SmtpError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for RegisterUserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RegisterUserError::AlreadyRegistered => None,
            RegisterUserError::DatabaseError(e) => Some(e),
            RegisterUserError::HashingError(_) => None,
            RegisterUserError::EmailAddressParsingError(e) => Some(e),
            RegisterUserError::EmailBuilderError(e) => Some(e),
            RegisterUserError::SmtpError(e) => Some(e),
        }
    }
}

impl From<sqlx::Error> for ApiError<RegisterUserError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(RegisterUserError::DatabaseError(value))
    }
}

impl From<argon2::password_hash::Error> for ApiError<RegisterUserError> {
    fn from(value: argon2::password_hash::Error) -> Self {
        ApiError::new(RegisterUserError::HashingError(value))
    }
}

impl From<AddressError> for ApiError<RegisterUserError> {
    fn from(value: AddressError) -> Self {
        ApiError::new(RegisterUserError::EmailAddressParsingError(value))
    }
}

impl From<lettre::error::Error> for ApiError<RegisterUserError> {
    fn from(value: lettre::error::Error) -> Self {
        ApiError::new(RegisterUserError::EmailBuilderError(value))
    }
}

impl From<lettre::transport::smtp::Error> for ApiError<RegisterUserError> {
    fn from(value: lettre::transport::smtp::Error) -> Self {
        ApiError::new(RegisterUserError::SmtpError(value))
    }
}

#[cfg(test)]
pub mod test {
    use crate::{
        database::types::RELATIONAL_DATABASE, register_user, routes::types::RegisterUser, Database,
        Email, JWTKey, TcpListener,
    };
    use axum::{routing::post, Router};
    use dotenvy::dotenv;
    use lettre::{
        message::{header::ContentType, Mailbox},
        transport::smtp::{self, authentication::Credentials},
        Message, Transport,
    };
    use rand::{rngs::ThreadRng, Rng};

    #[tokio::test]
    async fn register() -> Result<(), Box<dyn std::error::Error>> {
        dotenv().unwrap();
        JWTKey::init().unwrap();
        Database::init(None).await.unwrap();
        Email::init().unwrap();
        let to = dotenvy::var("SMTP_USERNAME")?;

        tokio::spawn(async move {
            let app = Router::new().route("/api/register", post(register_user));
            let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();
        let res = client
            .post("http://localhost:3000/api/register")
            .json(&RegisterUser {
                email: to.to_string(),
                wallet: "0x0c5E7D8C1494B74891e4c6539Be96C8e2402dcEF".to_string(),
                password: "test".to_string(),
            })
            .send()
            .await?;

        println!("{res:#?}");

        sqlx::query!("DELETE FROM Customers WHERE email = $1", &to)
            .execute(RELATIONAL_DATABASE.get().unwrap())
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn mail() -> Result<(), Box<dyn std::error::Error>> {
        dotenv().unwrap();

        let verification_code: u32 = ThreadRng::default().gen_range(10000000..99999999);

        let username = dotenvy::var("SMTP_USERNAME")?;
        let password = dotenvy::var("SMTP_PASSWORD")?;

        let user_email = username.parse()?;
        let server_mailbox: Mailbox = format!("Developer DAO RPC <{}>", &username).parse()?;

        let email_credentials = Credentials::new(username, password);

        let email = Message::builder()
            .from(server_mailbox)
            .to(user_email)
            .subject("D_D RPC Verification Code")
            .header(ContentType::TEXT_PLAIN)
            .body(format!("Your verification code is: {}", verification_code))?;
        let mailer = smtp::SmtpTransport::starttls_relay("smtp.gmail.com")?
            .credentials(email_credentials)
            .build();

        mailer.send(&email)?;

        Ok(())
    }
}
