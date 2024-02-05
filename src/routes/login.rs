use super::{errors::ApiError, types::JWT_KEY};
use crate::{
    database::{errors::ParsingError, types::{Customers, Role, RELATIONAL_DATABASE}},
    eth_rpc::types::Address,
};
use argon2::PasswordHash;
use axum::{http::StatusCode, response::IntoResponse, Json};
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    role: Role,
    email: String,
    wallet: Address,
}

pub async fn user_login(
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError<LoginError>> {
    let user = sqlx::query_as!(
        Customers,
        r#"SELECT email, wallet, password, role as "role!:Role", verificationCode, activated FROM Customers 
        WHERE email = $1 
        AND password = $2"#,
        &payload.email,
        PasswordHash::new(&payload.password)?.to_string()
    )
    .fetch_optional(RELATIONAL_DATABASE.get().unwrap())
    .await?
    .ok_or_else(|| ApiError::new(LoginError::InvalidEmailOrPassword))?;

    if !user.activated {
        return Err(ApiError::new(LoginError::AccountNotActivated));
    }

    let user_info = Claims {
        role: user.role,
        email: user.email,
        wallet: user.wallet.parse::<Address>()?,
    };
    let claims = jwt_simple::claims::Claims::with_custom_claims(user_info, Duration::from_hours(2));

    let key = JWT_KEY.get().unwrap();
    let jwt = key.authenticate(claims)?;
    Ok((StatusCode::OK, jwt).into_response())
}

#[derive(Debug)]
pub enum LoginError {
    InvalidEmailOrPassword,
    DatabaseError(sqlx::Error),
    HashingError(argon2::password_hash::Error),
    AccountNotActivated,
    JwtCreationError(jwt_simple::Error),
    AddressParsingError(ParsingError),
}

impl Display for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LoginError::InvalidEmailOrPassword => write!(f, "The email or password used to login is invalid"),
            LoginError::DatabaseError(e) => write!(f, "Something went wrong while querying the database: {}", e),
            LoginError::HashingError(e) => write!(f, "An error occurred while hashing: {}", e),
            LoginError::AccountNotActivated => write!(f, "This account is not yet activated!"),
            LoginError::JwtCreationError(e) => write!(f, "There was an error creating a JWT: {}", e),
            LoginError::AddressParsingError(e) => write!(f, "There was an error parsing input as an Address: {}", e),
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
        }
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
        println!("{key:?}");
        HS256Key::from_bytes(&hex::decode(key)?);
        Ok(())
    }
}
