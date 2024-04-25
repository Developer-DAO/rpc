use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use axum::{
    extract::Request,
    http::{header::COOKIE, HeaderMap},
    middleware::Next,
    response::Response,
};
use jwt_simple::{algorithms::MACLike, common::VerificationOptions};

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
        .get(COOKIE)
        .ok_or_else(|| ApiError::new(JwtAuthError::InvalidHeader))?;
    let token = &jwt
        .to_str()?
        .split(';')
        .filter_map(|t| {
            let t = t.trim_start();
            if t.starts_with("jwt") {
                Some(t)
            } else {
                None
            }
        })
        .collect::<Vec<&str>>()
        .first()
        .ok_or_else(|| ApiError::new(JwtAuthError::TokenParsingError))?[4..];

    let decoded_token = JWT_KEY
        .get()
        .unwrap()
        .verify_token::<Claims>(token, Some(VerificationOptions::default()))?;
    request.extensions_mut().insert(decoded_token);
    Ok(next.run(request).await)
}

#[derive(Debug)]
pub enum JwtAuthError {
    TokenParsingError,
    InvalidHeader,
    JwtVerificationFailed(jwt_simple::Error),
    HeaderParsingError(axum::http::header::ToStrError),
}

impl Display for JwtAuthError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JwtAuthError::InvalidHeader => write!(f, "Missing header: Authorization"),
            JwtAuthError::JwtVerificationFailed(_) => write!(f, "Jwt is expired or invalid"),
            JwtAuthError::HeaderParsingError(_) => write!(f, "Couldn't parse headers into &str"),
            JwtAuthError::TokenParsingError => write!(f, "Failed to parse JWT from header value."),
        }
    }
}

impl Error for JwtAuthError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            JwtAuthError::InvalidHeader => None,
            JwtAuthError::JwtVerificationFailed(e) => Some(e.deref()),
            JwtAuthError::HeaderParsingError(e) => Some(e),
            JwtAuthError::TokenParsingError => None,
        }
    }
}

impl From<jwt_simple::Error> for ApiError<JwtAuthError> {
    fn from(value: jwt_simple::Error) -> Self {
        ApiError::new(JwtAuthError::JwtVerificationFailed(value))
    }
}

impl From<axum::http::header::ToStrError> for ApiError<JwtAuthError> {
    fn from(value: axum::http::header::ToStrError) -> Self {
        ApiError::new(JwtAuthError::HeaderParsingError(value))
    }
}
