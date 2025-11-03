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
    Poly,
    ArbSepoliaTestnet,
    ArbOne,
    ZksyncEra,
    Solana,
    Osmosis,
    Gnosis,
    Sui,
    Bera,
    Harmony,
    XrplEvmTestnet,
    Metis,
    Base,
    ZkLinkNova,
    Kaia,
    Op,
    Scroll,
    Taiko,
    Pocket,
    PocketBeta,
    OpSepoliaTestnet,
    //    Fantom,
    Moonbeam,
    Ink,
    Evmos,
    BaseSepoliaTestnet,
    Sei,
    Kava,
    Oasys,
    Tron,
    Sonic,
    Near,
    AvaxDFK,
    Bsc,
    Polyzkevm,
    Linea,
    Celo,
    Fraxtal,
    Fuse,
    Avax,
    Iotex,
    Moonriver,
    Boba,
    EthSepoliaTestnet,
    Radix,
    Xrplevm,
    EthHoleskyTestnet,
    OpBNB,
    Blast,
    Mantle,
    Eth,
    TaikoHeklaTestnet,
    PolyAmoyTestnet,
}

impl fmt::Display for PoktChains {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoktChains::ArbOne => write!(f, "arb-one"),
            PoktChains::Avax => write!(f, "avax"),
            PoktChains::AvaxDFK => write!(f, "avax-dfk"),
            PoktChains::Base => write!(f, "base"),
            PoktChains::BaseSepoliaTestnet => write!(f, "base-sepolia-testnet"),
            PoktChains::Bera => write!(f, "bera"),
            PoktChains::Blast => write!(f, "blast"),
            PoktChains::Bsc => write!(f, "bsc"),
            PoktChains::Boba => write!(f, "boba"),
            PoktChains::Celo => write!(f, "celo"),
            PoktChains::Eth => write!(f, "eth"),
            PoktChains::EthHoleskyTestnet => write!(f, "eth-holesky-testnet"),
            PoktChains::EthSepoliaTestnet => write!(f, "eth-sepolia-testnet"),
            PoktChains::Evmos => write!(f, "evmos"),
            PoktChains::Fraxtal => write!(f, "fraxtal"),
            PoktChains::Fuse => write!(f, "fuse"),
            PoktChains::Gnosis => write!(f, "gnosis"),
            PoktChains::Harmony => write!(f, "harmony"),
            PoktChains::Iotex => write!(f, "iotex"),
            PoktChains::Kaia => write!(f, "kaia"),
            PoktChains::Kava => write!(f, "kava"),
            PoktChains::Metis => write!(f, "metis"),
            PoktChains::Moonbeam => write!(f, "moonbeam"),
            PoktChains::Moonriver => write!(f, "moonriver"),
            PoktChains::Near => write!(f, "near"),
            PoktChains::Oasys => write!(f, "oasys"),
            PoktChains::OpBNB => write!(f, "opbnb"),
            PoktChains::Op => write!(f, "op"),
            PoktChains::OpSepoliaTestnet => write!(f, "op-sepolia-testnet"),
            PoktChains::Osmosis => write!(f, "osmosis"),
            PoktChains::Pocket => write!(f, "pocket"),
            PoktChains::PocketBeta => write!(f, "pocket-beta"),
            PoktChains::Poly => write!(f, "poly"),
            PoktChains::PolyAmoyTestnet => write!(f, "poly-amoy-testnet"),
            PoktChains::Polyzkevm => write!(f, "poly-zkevm"),
            PoktChains::Radix => write!(f, "radix"),
            PoktChains::Scroll => write!(f, "scroll"),
            PoktChains::Solana => write!(f, "solana"),
            PoktChains::Sui => write!(f, "sui"),
            PoktChains::Taiko => write!(f, "taiko"),
            PoktChains::TaikoHeklaTestnet => write!(f, "taiko-hekla-testnet"),
            PoktChains::ZkLinkNova => write!(f, "zklink-nova"),
            PoktChains::ZksyncEra => write!(f, "zksync-era"),
            #[cfg(any(test, feature = "dev"))]
            PoktChains::Anvil => write!(f, "anvil"),
            PoktChains::XrplEvmTestnet => write!(f, "xrplevm-testnet"),
            //           PoktChains::Fantom => write!(f, "fantom"),
            PoktChains::Ink => write!(f, "ink"),
            PoktChains::Sei => write!(f, "sei"),
            PoktChains::Tron => write!(f, "tron"),
            PoktChains::Sonic => write!(f, "sonic"),
            PoktChains::Linea => write!(f, "linea"),
            PoktChains::ArbSepoliaTestnet => write!(f, "arb-sepolia-testnet"),
            PoktChains::Xrplevm => write!(f, "xrplevm"),
            PoktChains::Mantle => write!(f, "mantle"),
        }
    }
}

impl PoktChains {
    pub const fn id(&self) -> &'static str {
        match self {
            PoktChains::ArbOne => "arb-one",
            PoktChains::Avax => "avax",
            PoktChains::AvaxDFK => "avax-dfk",
            PoktChains::Base => "base",
            PoktChains::BaseSepoliaTestnet => "base-sepolia-testnet",
            PoktChains::Bera => "bera",
            PoktChains::Blast => "blast",
            PoktChains::Bsc => "bsc",
            PoktChains::Boba => "boba",
            PoktChains::Celo => "celo",
            PoktChains::Eth => "eth",
            PoktChains::EthHoleskyTestnet => "eth-holesky-testnet",
            PoktChains::EthSepoliaTestnet => "eth-sepolia-testnet",
            PoktChains::Evmos => "evmos",
            PoktChains::Fraxtal => "fraxtal",
            PoktChains::Fuse => "fuse",
            PoktChains::Gnosis => "gnosis",
            PoktChains::Harmony => "harmony",
            PoktChains::Iotex => "iotex",
            PoktChains::Kaia => "kaia",
            PoktChains::Kava => "kava",
            PoktChains::Metis => "metis",
            PoktChains::Moonbeam => "moonbeam",
            PoktChains::Moonriver => "moonriver",
            PoktChains::Near => "near",
            PoktChains::Oasys => "oasys",
            PoktChains::OpBNB => "opbnb",
            PoktChains::Op => "op",
            PoktChains::OpSepoliaTestnet => "op-sepolia-testnet",
            PoktChains::Osmosis => "osmosis",
            PoktChains::Pocket => "pocket",
            PoktChains::PocketBeta => "pocket-beta",
            PoktChains::Poly => "poly",
            PoktChains::PolyAmoyTestnet => "poly-amoy-testnet",
            PoktChains::Polyzkevm => "poly-zkevm",
            PoktChains::Radix => "radix",
            PoktChains::Scroll => "scroll",
            PoktChains::Solana => "solana",
            PoktChains::Sui => "sui",
            PoktChains::Taiko => "taiko",
            PoktChains::TaikoHeklaTestnet => "taiko-hekla-testnet",
            PoktChains::ZkLinkNova => "zklink-nova",
            PoktChains::ZksyncEra => "zksync-era",
            #[cfg(any(test, feature = "dev"))]
            PoktChains::Anvil => "anvil",
            PoktChains::XrplEvmTestnet => "xrplevm-testnet",
            //         PoktChains::Fantom => "fantom",
            PoktChains::Ink => "ink",
            PoktChains::Sei => "sei",
            PoktChains::Tron => "tron",
            PoktChains::Sonic => "sonic",
            PoktChains::Linea => "linea",
            PoktChains::ArbSepoliaTestnet => "arb-sepolia-testnet",
            PoktChains::Xrplevm => "xrplevm",
            PoktChains::Mantle => "mantle",
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
            "avax" => Ok(PoktChains::Avax),
            "avax-dfk" => Ok(PoktChains::AvaxDFK),
            "base" => Ok(PoktChains::Base),
            "base-sepolia-testnet" => Ok(PoktChains::BaseSepoliaTestnet),
            "bera" => Ok(PoktChains::Bera),
            "blast" => Ok(PoktChains::Blast),
            "bsc" => Ok(PoktChains::Bsc),
            "boba" => Ok(PoktChains::Boba),
            "celo" => Ok(PoktChains::Celo),
            "eth" => Ok(PoktChains::Eth),
            "eth-holesky-testnet" => Ok(PoktChains::EthHoleskyTestnet),
            "eth-sepolia-testnet" => Ok(PoktChains::EthSepoliaTestnet),
            "evmos" => Ok(PoktChains::Evmos),
            "fraxtal" => Ok(PoktChains::Fraxtal),
            "fuse" => Ok(PoktChains::Fuse),
            "gnosis" => Ok(PoktChains::Gnosis),
            "harmony" => Ok(PoktChains::Harmony),
            "iotex" => Ok(PoktChains::Iotex),
            "kaia" => Ok(PoktChains::Kaia),
            "kava" => Ok(PoktChains::Kava),
            "metis" => Ok(PoktChains::Metis),
            "moonbeam" => Ok(PoktChains::Moonbeam),
            "moonriver" => Ok(PoktChains::Moonriver),
            "near" => Ok(PoktChains::Near),
            "oasys" => Ok(PoktChains::Oasys),
            "opbnb" => Ok(PoktChains::OpBNB),
            "op" => Ok(PoktChains::Op),
            "op-sepolia-testnet" => Ok(PoktChains::OpSepoliaTestnet),
            "osmosis" => Ok(PoktChains::Osmosis),
            "pocket" => Ok(PoktChains::Pocket),
            "pocket-beta" => Ok(PoktChains::PocketBeta),
            "poly" => Ok(PoktChains::Poly),
            "poly-amoy-testnet" => Ok(PoktChains::PolyAmoyTestnet),
            "poly-zkevm" => Ok(PoktChains::Polyzkevm),
            "radix" => Ok(PoktChains::Radix),
            "scroll" => Ok(PoktChains::Scroll),
            "solana" => Ok(PoktChains::Solana),
            "sui" => Ok(PoktChains::Sui),
            "taiko" => Ok(PoktChains::Taiko),
            "taiko-hekla-testnet" => Ok(PoktChains::TaikoHeklaTestnet),
            "zklink-nova" => Ok(PoktChains::ZkLinkNova),
            "zksync-era" => Ok(PoktChains::ZksyncEra),
            #[cfg(any(test, feature = "dev"))]
            "anvil" => Ok(PoktChains::Anvil),
            "xrplevm-testnet" => Ok(PoktChains::XrplEvmTestnet),
            //         PoktChains::Fantom => "fantom",
            "ink" => Ok(PoktChains::Ink),
            "sei" => Ok(PoktChains::Sei),
            "tron" => Ok(PoktChains::Tron),
            "sonic" => Ok(PoktChains::Sonic),
            "linea" => Ok(PoktChains::Linea),
            "xrplevm" => Ok(PoktChains::Xrplevm),
            "mantle" => Ok(PoktChains::Mantle),
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
