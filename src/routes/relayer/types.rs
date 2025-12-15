use http::{HeaderValue, header::CONTENT_TYPE};
use axum::body::{Body, Bytes};
use reqwest::Client;
use std::{
    fmt::{self, Display, Formatter},
    future::Future,
    str::FromStr,
    sync::LazyLock,
};
pub static GATEWAY_ENDPOINT: LazyLock<&'static str> = LazyLock::new(|| {
    format!(
        "{}/v1",
        dotenvy::var("GATEWAY_URL").unwrap_or_else(|_| "http://localhost:3069".to_string())
    )
    .leak()
});

pub trait Relayer {
    fn relay_transaction(
        &self,
        body: Bytes,
    ) -> impl Future<Output = Result<Body, RelayErrors>>;
}

impl From<PoktChains> for HeaderValue {
    fn from(value: PoktChains) -> Self {
        HeaderValue::from_static(value.id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoktChains {
    #[cfg(any(test, feature = "dev"))]
    Anvil,
    Base,
    Eth,
    ArbOne,
    Solana,
    Sui,
    Bsc,
    Poly,
    Op,
}

impl fmt::Display for PoktChains {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoktChains::ArbOne => write!(f, "arb-one"),
            PoktChains::Base => write!(f, "base"),
            PoktChains::Bsc => write!(f, "bsc"),
            PoktChains::Eth => write!(f, "eth"),
            PoktChains::Op => write!(f, "op"),
            PoktChains::Poly => write!(f, "poly"),
            PoktChains::Solana => write!(f, "solana"),
            PoktChains::Sui => write!(f, "sui"),
            #[cfg(any(test, feature = "dev"))]
            PoktChains::Anvil => write!(f, "anvil"),
        }
    }
}

impl PoktChains {
    pub const fn id(&self) -> &'static str {
        match self {
            PoktChains::ArbOne => "arb-one",
            PoktChains::Base => "base",
            PoktChains::Bsc => "bsc",
            PoktChains::Eth => "eth",
            PoktChains::Op => "op",
            PoktChains::Poly => "poly",
            PoktChains::Solana => "solana",
            PoktChains::Sui => "sui",
            #[cfg(any(test, feature = "dev"))]
            PoktChains::Anvil => "anvil",
        }
    }
}

impl Relayer for PoktChains {
    async fn relay_transaction(&self, body: Bytes) -> Result<Body, RelayErrors> {
        if cfg!(test) {
            let provider = dotenvy::var("SEPOLIA_PROVIDER").expect("SEPOLIA_PROVIDER not found");
            let byte_stream = Client::new()
                .post(&provider)
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .body(body)
                .send()
                .await?
                .error_for_status()?
                .bytes_stream();
            let body = Body::from_stream(byte_stream);
            Ok(body)
        } else {
            let byte_stream = Client::new()
                .post(*GATEWAY_ENDPOINT)
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .header("target-service-id", self.id())
                .body(body)
                .send()
                .await?
                .error_for_status()?
                .bytes_stream();
            let body = Body::from_stream(byte_stream);
            Ok(body)
        }
    }
}

impl FromStr for PoktChains {
    type Err = RelayErrors;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "arb-one" => Ok(PoktChains::ArbOne),
            "base" => Ok(PoktChains::Base),
            "bsc" => Ok(PoktChains::Bsc),
            "eth" => Ok(PoktChains::Eth),
            "op" => Ok(PoktChains::Op),
            "poly" => Ok(PoktChains::Poly),
            "solana" => Ok(PoktChains::Solana),
            "sui" => Ok(PoktChains::Sui),
            #[cfg(any(test, feature = "dev"))]
            "anvil" => Ok(PoktChains::Anvil),
            _ => Err(RelayErrors::PoktChainIdParsingError),
        }
    }
}

#[derive(Debug)]
pub enum RelayErrors {
    PoktRelayError(reqwest::Error),
    PoktChainIdParsingError,
}

impl From<reqwest::Error> for RelayErrors {
    fn from(value: reqwest::Error) -> Self {
        RelayErrors::PoktRelayError(value)
    }
}

impl Display for RelayErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RelayErrors::PoktRelayError(_) => {
                write!(f, "Failed to submit transaction or parse the response")
            }
            RelayErrors::PoktChainIdParsingError => write!(f, "Could not identify chain by id"),
        }
    }
}

impl std::error::Error for RelayErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RelayErrors::PoktRelayError(e) => Some(e),
            RelayErrors::PoktChainIdParsingError => None,
        }
    }
}
