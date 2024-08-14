use std::sync::OnceLock;

use alloy::node_bindings::Anvil;

pub static ETHEREUM_ENDPOINT: OnceLock<&'static str> = OnceLock::new();

#[derive(Debug, Clone, Copy)]
pub struct Endpoints;

pub enum EndpointType {
    Testing,
    Production,
}

impl Endpoints {
    fn init(testing: EndpointType) {
        match testing {
            EndpointType::Testing => {
                let endpoint = Anvil::new()
                .block_time(1)
                .try_spawn()
                .unwrap()
                .endpoint_url()
                .to_string()
                .leak();
                ETHEREUM_ENDPOINT.get_or_init(|| endpoint);
            },
            EndpointType::Production => {
                let endpoint = dotenvy::var("ETHEREUM_ENDPOINT").unwrap().leak();
                ETHEREUM_ENDPOINT.get_or_init(|| endpoint);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Chains {
    Ethereum,
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
