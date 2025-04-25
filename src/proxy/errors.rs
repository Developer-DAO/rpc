use hyper::{
    header::{InvalidHeaderValue, ToStrError},
    http::uri::InvalidUri,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpClientErrors {
    #[error("Only one redirect is allowed by this policy")]
    RedirectMaxDepthExceeded,
    #[error("Failed to parse Uri from Location Header")]
    UriParsingError(#[from] InvalidUri),
    #[error(transparent)]
    InvalidHeader(#[from] InvalidHeaderValue),
    #[error(transparent)]
    ToStrError(#[from] ToStrError),
    #[error("No Location header found for redirect / Invalid status code")]
    NoRedirect,
    #[error(transparent)]
    HyperError(#[from] hyper::Error),
    #[error("External host redirect not allowed")]
    ExternalHostRedirect
}
