use super::{
    errors::ApiError,
    types::{Claims, JWT_KEY},
};
use crate::{
    database::{
        errors::ParsingError,
        types::{Customers, Role, RELATIONAL_DATABASE},
    },
    eth_rpc::types::Address,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use jwt_simple::{algorithms::MACLike, reexports::coarsetime::Duration};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[tracing::instrument]
pub async fn user_login(
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError<LoginError>> {
    let user = sqlx::query_as!(
        Customers,
        r#"SELECT email, wallet, password, role as "role!:Role", verificationCode, activated FROM Customers 
        WHERE email = $1"#,
        &payload.email,
    )
    .fetch_optional(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .ok_or_else(|| ApiError::new(LoginError::InvalidEmailOrPassword))?;

    let plaintext_password = payload.password.as_bytes();
    let hashed_password = PasswordHash::new(&user.password)?;
    Argon2::default().verify_password(plaintext_password, &hashed_password)?;

    if !user.activated {
        Err(ApiError::new(LoginError::AccountNotActivated))?
    }

    let user_info = Claims {
        role: user.role,
        email: user.email,
        wallet: user.wallet.parse::<Address>()?,
    };
    let claims = jwt_simple::claims::Claims::with_custom_claims(user_info, Duration::from_hours(2));
    let key = JWT_KEY.get().unwrap();
    let auth = key.authenticate(claims)?;

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, HeaderValue::from_str(&format!("jwt={}", auth)).unwrap());
    headers.append(SET_COOKIE, HeaderValue::from_str("Secure").unwrap());   
    headers.append(SET_COOKIE, HeaderValue::from_str("HttpOnly").unwrap());
    headers.append(SET_COOKIE, HeaderValue::from_str("SameSite=Strict").unwrap());

    Ok((StatusCode::OK, headers, "login successful!"))
}

#[derive(Debug)]
pub enum LoginError {
    InvalidEmailOrPassword,
    DatabaseError(sqlx::Error),
    HashingError(argon2::password_hash::Error),
    AccountNotActivated,
    JwtCreationError(jwt_simple::Error),
    AddressParsingError(ParsingError),
    BuilderResponseError(axum::http::Error),
}

impl Display for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::InvalidEmailOrPassword => {
                write!(f, "The email or password used to login is invalid")
            }
            LoginError::DatabaseError(e) => {
                write!(f, "Something went wrong while querying the database: {}", e)
            }
            LoginError::HashingError(e) => write!(f, "An error occurred while hashing: {}", e),
            LoginError::AccountNotActivated => write!(f, "This account is not yet activated!"),
            LoginError::JwtCreationError(e) => {
                write!(f, "There was an error creating a JWT: {}", e)
            }
            LoginError::AddressParsingError(e) => {
                write!(f, "There was an error parsing input as an Address: {}", e)
            }
            LoginError::BuilderResponseError(e) => write!(
                f,
                "An error occured while building a response from the server {}",
                e
            ),
        }
    }
}

impl Error for LoginError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LoginError::InvalidEmailOrPassword => None,
            LoginError::DatabaseError(e) => Some(e),
            LoginError::HashingError(_) => None,
            LoginError::AccountNotActivated => None,
            LoginError::JwtCreationError(e) => e.source(),
            LoginError::AddressParsingError(e) => Some(e),
            LoginError::BuilderResponseError(e) => Some(e),
        }
    }
}

impl From<axum::http::Error> for ApiError<LoginError> {
    fn from(value: axum::http::Error) -> Self {
        ApiError::new(LoginError::BuilderResponseError(value))
    }
}

impl From<sqlx::Error> for ApiError<LoginError> {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(LoginError::DatabaseError(value))
    }
}

impl From<argon2::password_hash::Error> for ApiError<LoginError> {
    fn from(value: argon2::password_hash::Error) -> Self {
        ApiError::new(LoginError::HashingError(value))
    }
}

impl From<jwt_simple::Error> for ApiError<LoginError> {
    fn from(value: jwt_simple::Error) -> Self {
        ApiError::new(LoginError::JwtCreationError(value))
    }
}

impl From<ParsingError> for ApiError<LoginError> {
    fn from(value: ParsingError) -> Self {
        ApiError::new(LoginError::AddressParsingError(value))
    }
}

#[cfg(test)]
pub mod tests {

    use hex::ToHex;
    use jwt_simple::algorithms::{HS256Key, MACLike};

    #[test]
    fn get_key() -> Result<(), Box<dyn std::error::Error>> {
        let key: String = HS256Key::generate().key().encode_hex();
        // println!("{key:?}");
        HS256Key::from_bytes(&hex::decode(key)?);
        Ok(())
    }

    use crate::{
        database::types::RELATIONAL_DATABASE,
        middleware::jwt_auth::verify_jwt,
        register_user,
        routes::{
            activate::{activate_account, ActivationRequest},
            api_keys::generate_api_keys,
            login::LoginRequest,
            types::RegisterUser,
        },
        user_login, Database, Email, JWTKey, TcpListener,
    };
    use axum::{http::StatusCode, middleware::from_fn, routing::post, Router};
    use dotenvy::dotenv;

    #[tokio::test]
    async fn login() -> Result<(), Box<dyn std::error::Error>> {
        dotenv().unwrap();
        JWTKey::init().unwrap();
        Database::init(None).await.unwrap();
        Email::init().unwrap();
        let to = dotenvy::var("SMTP_USERNAME")?;

        tokio::spawn(async move {
            let app = Router::new()
                .route("/api/register", post(register_user))
                .route("/api/activate", post(activate_account))
                .route("/api/login", post(user_login))
                .route(
                    "/api/keys",
                    post(generate_api_keys).route_layer(from_fn(verify_jwt)),
                );
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
            .await?;

        pub struct Code {
            verificationcode: String,
        }

        let code = sqlx::query_as!(
            Code,
            "SELECT verificationCode FROM Customers WHERE email = $1",
            &to
        )
        .fetch_one(RELATIONAL_DATABASE.get().unwrap())
        .await?;

        let ar = ActivationRequest {
            code: code.verificationcode,
            email: to.to_string(),
        };

        reqwest::Client::new()
            .post("http://localhost:3000/api/activate")
            .json(&ar)
            .send()
            .await?;

        let lr = LoginRequest {
            email: to.to_string(),
            password: "test".to_string(),
        };

        let ddrpc_client = reqwest::Client::builder()
            .cookie_store(true)
            .build()?;

        ddrpc_client 
            .post("http://localhost:3000/api/login")
            .json(&lr)
            .send()
            .await?;

        let keygen = ddrpc_client 
            .post("http://localhost:3000/api/keys")
            .send()
            .await?;

        assert_eq!(&keygen.status().to_string(), &StatusCode::OK.to_string());
        println!("key: {}", keygen.text().await?);

        sqlx::query!("DELETE FROM Customers WHERE email = $1", &to)
            .execute(RELATIONAL_DATABASE.get().unwrap())
            .await?;

        Ok(())
    }
}
