use hex::FromHexError;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum EthCallError {
    RequestError(reqwest::Error),
    HexDecodingError(FromHexError),
    JsonDecodingError(serde_json::Error),
}

impl Display for EthCallError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EthCallError::RequestError(e) => write!(f, "{}", e),
            EthCallError::HexDecodingError(e) => write!(f, "{}", e),
            EthCallError::JsonDecodingError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for EthCallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EthCallError::RequestError(e) => Some(e),
            EthCallError::HexDecodingError(e) => Some(e), 
            EthCallError::JsonDecodingError(e) => Some(e), 
        }
    }
}

impl From<reqwest::Error> for EthCallError {
    fn from(value: reqwest::Error) -> Self {
        EthCallError::RequestError(value)
    }
}

impl From<FromHexError> for EthCallError {
    fn from(value: FromHexError) -> Self {
        EthCallError::HexDecodingError(value)
    }
}

impl From<serde_json::Error> for EthCallError {
    fn from(value: serde_json::Error) -> Self {
        EthCallError::JsonDecodingError(value)
    }
}
