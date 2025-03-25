use std::sync::LazyLock;
#[cfg(test)]
use std::sync::OnceLock;
#[cfg(test)]
pub static TESTING_ENDPOINT: OnceLock<&'static str> = OnceLock::new();
pub static ETHEREUM_ENDPOINT: LazyLock<[InternalEndpoints; 1]> = LazyLock::new(|| {
    [InternalEndpoints::Ethereum(
        dotenvy::var("ETHEREUM_ENDPOINT").unwrap().leak(),
    )]
});

#[derive(Copy, Clone, Debug)]
pub enum InternalEndpoints {
    Optimism(&'static str),
    Arbitrum(&'static str),
    Polygon(&'static str),
    Base(&'static str),
    Ethereum(&'static str),
}

impl InternalEndpoints {
    pub fn as_str(&self) -> &str {
        match self {
            InternalEndpoints::Optimism(o) => o,
            InternalEndpoints::Arbitrum(a) => a,
            InternalEndpoints::Polygon(p) => p,
            InternalEndpoints::Base(b) => b,
            InternalEndpoints::Ethereum(e) => e,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    pub url: reqwest::Url,
}

impl Provider {
    pub fn new(url: reqwest::Url) -> Provider {
        Self { url }
    }
}
