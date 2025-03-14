use std::sync::{LazyLock, OnceLock};
pub static TESTING_ENDPOINT: OnceLock<&'static str> = OnceLock::new();
pub static ETHEREUM_ENDPOINT: LazyLock<[InternalEndpoints; 1]> = LazyLock::new(|| {
    [InternalEndpoints::Optimism(
        dotenvy::var("ETHEREUM_ENDPOINT").unwrap().leak(),
    )]
});

#[derive(Copy, Clone, Debug)]
pub enum InternalEndpoints {
    Optimism(&'static str),
    Arbitrum(&'static str),
    Polygon(&'static str),
    Base(&'static str),
    Anvil(&'static str),
}

impl InternalEndpoints {
    pub fn as_str(&self) -> &str {
        match self {
            InternalEndpoints::Optimism(o) => o,
            InternalEndpoints::Arbitrum(a) => a,
            InternalEndpoints::Polygon(p) => p,
            InternalEndpoints::Base(b) => b,
            InternalEndpoints::Anvil(a) => a,
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
