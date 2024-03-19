use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use axum::{extract::Request, http::{HeaderMap, HeaderName}, middleware::Next, response::Response};
use jwt_simple::algorithms::MACLike;

use crate::routes::{
    errors::ApiError,
    types::{Claims, JWT_KEY},
};

pub async fn verify_jwt(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError<JwtAuthError>> {
    let jwt = headers
        .get(HeaderName::from_static("Authorization"))
        .ok_or_else(|| ApiError::new(JwtAuthError::InvalidHeader))?;

    let decoded_token = JWT_KEY
        .get()
        .unwrap()
        .verify_token::<Claims>(jwt.to_str()?, None)?;
    request.extensions_mut().insert(decoded_token);
    Ok(next.run(request).await)
}

#[derive(Debug)]
pub enum JwtAuthError {
    InvalidHeader,
    InvalidJwt(jwt_simple::Error),
    HeaderParsingError(axum::http::header::ToStrError),
}

impl Display for JwtAuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JwtAuthError::InvalidHeader => write!(f, "Missing header: Authorization"),
            JwtAuthError::InvalidJwt(_) => write!(f, "Jwt is expired or invalid"),
            JwtAuthError::HeaderParsingError(_) => write!(f, "Couldn't parse headers into &str"),
        }
    }
}

impl Error for JwtAuthError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            JwtAuthError::InvalidHeader => None,
            JwtAuthError::InvalidJwt(e) => Some(e.deref()),
            JwtAuthError::HeaderParsingError(e) => Some(e),
        }
    }
}

impl From<jwt_simple::Error> for ApiError<JwtAuthError> {
    fn from(value: jwt_simple::Error) -> Self {
        ApiError::new(JwtAuthError::InvalidJwt(value))
    }
}

impl From<axum::http::header::ToStrError> for ApiError<JwtAuthError> {
    fn from(value: axum::http::header::ToStrError) -> Self {
        ApiError::new(JwtAuthError::HeaderParsingError(value))
    }
}
