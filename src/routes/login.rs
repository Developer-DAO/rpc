use super::{
    siwe::Siwe,
    types::{Claims, JWT_KEY},
};
use crate::{
    database::{
        errors::ParsingError,
        types::{Customers, RELATIONAL_DATABASE, Role},
    },
    eth_rpc::types::ETHEREUM_ENDPOINT,
};
use alloy::{primitives::Address, providers::ProviderBuilder};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json,
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::IntoResponse,
};
use jwt_simple::{algorithms::MACLike, reexports::coarsetime::Duration};
use serde::{Deserialize, Serialize};
use siwe::{Message, VerificationError, VerificationOpts};
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoginRequest {
    pub(crate) email: String,
    pub(crate) password: String,
}

pub struct SiweLogin {
    pub email: String,
    pub wallet: Option<String>,
    pub role: Role,
    pub nonce: Option<String>,
}

#[tracing::instrument]
pub async fn user_login_siwe(Json(payload): Json<Siwe>) -> Result<impl IntoResponse, LoginError> {
    let msg: Message = payload.message.parse()?;
    let address = Address::new(msg.address);

    let customer = sqlx::query_as!(
        SiweLogin,
        r#"SELECT email, wallet, nonce, role as "role!: Role" FROM Customers where wallet = $1"#,
        address.to_string(),
    )
    .fetch_optional(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .ok_or_else(|| LoginError::InvalidAddress)?;

    let nonce = customer.nonce.ok_or_else(|| LoginError::MissingNonce)?;

    let rpc = ProviderBuilder::new().on_http(ETHEREUM_ENDPOINT[0].as_str().parse().unwrap());

    let verification_opts = VerificationOpts {
        domain: Some("Developer DAO Cloud".parse().unwrap()),
        nonce: Some(nonce),
        timestamp: Some(OffsetDateTime::now_utc()),
        rpc_provider: Some(rpc),
    };

    msg.verify(&payload.signature, &verification_opts).await?;

    let user_info = Claims {
        role: customer.role,
        email: customer.email,
        wallet: Some(address),
    };
    let claims = jwt_simple::claims::Claims::with_custom_claims(user_info, Duration::from_hours(2));
    let key = JWT_KEY.get().unwrap();
    let auth = key.authenticate(claims)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!("jwt={}", auth)).unwrap(),
    );
    headers.append(SET_COOKIE, HeaderValue::from_str("Secure").unwrap());
    headers.append(SET_COOKIE, HeaderValue::from_str("HttpOnly").unwrap());
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str("SameSite=Strict").unwrap(),
    );

    Ok((StatusCode::OK, headers))
}

#[tracing::instrument]
pub async fn user_login(
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, LoginError> {
    let user = sqlx::query_as!(
        Customers,
        r#"SELECT email, wallet, password, role as "role!:Role", verificationCode, activated FROM Customers 
        WHERE email = $1"#,
        &payload.email,
    )
    .fetch_optional(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .ok_or_else(|| LoginError::InvalidEmailOrPassword)?;

    let plaintext_password = payload.password.as_bytes();
    let hashed_password =
        PasswordHash::new(&user.password).map_err(|_| LoginError::HashingError)?;
    Argon2::default()
        .verify_password(plaintext_password, &hashed_password)
        .map_err(|_| LoginError::InvalidEmailOrPassword)?;

    if !user.activated {
        Err(LoginError::AccountNotActivated)?
    }

    let user_info = Claims {
        role: user.role,
        email: user.email,
        wallet: user.wallet.map(|w| w.parse::<Address>().unwrap()),
    };
    let claims = jwt_simple::claims::Claims::with_custom_claims(user_info, Duration::from_hours(2));
    let key = JWT_KEY.get().unwrap();
    let auth = key.authenticate(claims)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!("jwt={}", auth)).unwrap(),
    );
    headers.append(SET_COOKIE, HeaderValue::from_str("Secure").unwrap());
    headers.append(SET_COOKIE, HeaderValue::from_str("HttpOnly").unwrap());
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str("SameSite=Strict").unwrap(),
    );

    Ok((StatusCode::OK, headers, "login successful!"))
}

#[derive(Debug, Error)]
pub enum LoginError {
    #[error(transparent)]
    VerificationError(#[from] VerificationError),
    #[error("User did not generate nonce")]
    MissingNonce,
    #[error("The email or password you provided is invalid.")]
    InvalidEmailOrPassword,
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error("An error occurred while hashing.")]
    HashingError,
    #[error("The account you are trying to login with is not activated.")]
    AccountNotActivated,
    #[error(transparent)]
    JwtCreationError(#[from] jwt_simple::Error),
    #[error(transparent)]
    AddressParsingError(#[from] ParsingError),
    #[error(transparent)]
    BuilderResponseError(#[from] axum::http::Error),
    #[error("No account found for address")]
    InvalidAddress,
    #[error(transparent)]
    ParseError(#[from] siwe::ParseError),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
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
        Database, Email, JWTKey, TcpListener,
        database::types::RELATIONAL_DATABASE,
        middleware::jwt_auth::verify_jwt,
        register_user,
        routes::{
            activate::{ActivationRequest, activate_account},
            api_keys::generate_api_keys,
            login::LoginRequest,
            types::RegisterUser,
        },
        user_login,
    };
    use axum::{Router, middleware::from_fn, routing::post};
    use dotenvy::dotenv;

    #[tokio::test]
    async fn login() {
        dotenv().unwrap();
        JWTKey::init().unwrap();
        Database::init().await.unwrap();
        Email::init().unwrap();

        tokio::spawn(async move {
            let app = Router::new()
                .route("/api/register", post(register_user))
                .route("/api/activate", post(activate_account))
                .route("/api/login", post(user_login))
                .route(
                    "/api/keys",
                    post(generate_api_keys).route_layer(from_fn(verify_jwt)),
                );
            let listener = TcpListener::bind("0.0.0.0:3030").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        reqwest::Client::new()
            .post("http://localhost:3030/api/register")
            .json(&RegisterUser {
                email: "abc@aol.com".to_string(),
                password: "test".to_string(),
            })
            .send()
            .await
            .unwrap();

        pub struct Code {
            verificationcode: String,
        }

        let code = sqlx::query_as!(
            Code,
            "SELECT verificationCode FROM Customers WHERE email = $1",
            "abc@aol.com"
        )
        .fetch_one(RELATIONAL_DATABASE.get().unwrap())
        .await
        .unwrap();

        let ar = ActivationRequest {
            code: code.verificationcode,
            email: "abc@aol.com".to_string(),
        };

        reqwest::Client::new()
            .post("http://localhost:3030/api/activate")
            .json(&ar)
            .send()
            .await
            .unwrap();

        let lr = LoginRequest {
            email: "abc@aol.com".to_string(),
            password: "test".to_string(),
        };

        let ddrpc_client = reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap();

        ddrpc_client
            .post("http://localhost:3030/api/login")
            .json(&lr)
            .send()
            .await
            .unwrap();

        // let keygen = ddrpc_client
        //     .post("http://localhost:3030/api/keys")
        //     .send()
        //     .await
        //     .unwrap();
        //
        // assert_eq!(&keygen.status().to_string(), &StatusCode::OK.to_string());

        // sqlx::query!("DELETE FROM Customers WHERE email = $1", "abc@aol.com")
        //     .execute(RELATIONAL_DATABASE.get().unwrap())
        //     .await
        //     .unwrap();
    }
}
