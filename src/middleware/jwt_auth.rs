use crate::routes::types::{Claims, JWT_KEY};
use axum::{
    extract::Request,
    http::{header::COOKIE, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jwt_simple::{algorithms::MACLike, common::VerificationOptions};
use thiserror::Error;

pub async fn verify_jwt(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, JwtAuthError> {
    let jwt = headers
        .get(COOKIE)
        .ok_or_else(|| JwtAuthError::InvalidHeader)?;
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
        .ok_or_else(|| JwtAuthError::TokenParsingError)?[4..];

    let decoded_token = JWT_KEY
        .get()
        .unwrap()
        .verify_token::<Claims>(token, Some(VerificationOptions::default()))?;
    request.extensions_mut().insert(decoded_token);
    Ok(next.run(request).await)
}

#[derive(Debug, Error)]
pub enum JwtAuthError {
    #[error("Failed to parse JWT from header value.")]
    TokenParsingError,
    #[error("Missing header: Authorization")]
    InvalidHeader,
    #[error(transparent)]
    JwtVerificationFailed(#[from] jwt_simple::Error),
    #[error(transparent)]
    HeaderParsingError(#[from] axum::http::header::ToStrError),
}

impl IntoResponse for JwtAuthError {
    fn into_response(self) -> Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            self.to_string(),
        )
            .into_response()
    }
}
