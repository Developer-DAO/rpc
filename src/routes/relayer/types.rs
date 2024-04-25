use crate::json_rpc::types::JsonRpcRequest;
use core::fmt;
use reqwest::Client;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
    sync::OnceLock,
};

pub static GATEWAY_URL: OnceLock<&'static str> = OnceLock::new();

pub enum PoktChains {
    Arbitrum,
    ArbitrumSepoliaArchival,
    AmoyTestnetArchival,
    OptimismSepoliaArchival,
    AVAX,
    AVAXArchival,
    BOBA,
    Base,
    BaseTestnet,
    BinanceSmartChain,
    BinanceSmartChainArchival,
    CelestiaArchival,
    Celo,
    DFKchainSubnet,
    Dogechain,
    EthereumBeacon,
    Ethereum,
    EthereumArchival,
    EthereumArchivalTrace,
    EthereumHighGas,
    Evmos,
    Fantom,
    Fuse,
    FuseArchival,
    Gnosis,
    GnosisArchival,
    Goerli,
    GoerliArchival,
    HarmonyShard0,
    HoleskyBeacon,
    HoleskyTestnet,
    IoTeX,
    Kava,
    KavaArchival,
    Klatyn,
    Kovan,
    Meter,
    Metis,
    Moonbeam,
    Moonriver,
    Near,
    OKC,
    Oasys,
    OasysArchival,
    Optimism,
    OptimismArchival,
    Osmosis,
    Pocket,
    PolygonMatic,
    PolygonMaticArchival,
    PolygonMumbai,
    PolygonZkEVM,
    Radix,
    Scroll,
    ScrollTestnet,
    SepoliaTestnet,
    SepoliaArchival,
    Solana,
    SolanaCustom,
    Starknet,
    StarknetTestnet,
    Sui,
    Velas,
    VelasArchival,
    ZkSync,
    PoktTestnetEthereumMock,
}

impl PoktChains {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::PoktTestnetEthereumMock => "0007",
            PoktChains::Arbitrum => "0066",
            PoktChains::AmoyTestnetArchival => "A085",
            PoktChains::OptimismSepoliaArchival => "A087",
            PoktChains::ArbitrumSepoliaArchival => "A086",
            PoktChains::AVAX => "0003",
            PoktChains::AVAXArchival => "A003",
            PoktChains::BOBA => "0048",
            PoktChains::Base => "0079",
            PoktChains::BaseTestnet => "0080",
            PoktChains::BinanceSmartChain => "0004",
            PoktChains::BinanceSmartChainArchival => "0010",
            PoktChains::CelestiaArchival => "A0CA",
            PoktChains::Celo => "0065",
            PoktChains::DFKchainSubnet => "03DF",
            PoktChains::Dogechain => "0059",
            PoktChains::EthereumBeacon => "B021",
            PoktChains::Ethereum => "0021",
            PoktChains::EthereumArchival => "0022",
            PoktChains::EthereumArchivalTrace => "0028",
            PoktChains::EthereumHighGas => "0062",
            PoktChains::Evmos => "0046",
            PoktChains::Fantom => "0049",
            PoktChains::Fuse => "0005",
            PoktChains::FuseArchival => "000A",
            PoktChains::Gnosis => "0027",
            PoktChains::GnosisArchival => "000C",
            PoktChains::Goerli => "0026",
            PoktChains::GoerliArchival => "0063",
            PoktChains::HarmonyShard0 => "0040",
            PoktChains::HoleskyBeacon => "B081",
            PoktChains::HoleskyTestnet => "0081",
            PoktChains::IoTeX => "0044",
            PoktChains::Kava => "0071",
            PoktChains::KavaArchival => "0072",
            PoktChains::Klatyn => "0056",
            PoktChains::Kovan => "0024",
            PoktChains::Meter => "0057",
            PoktChains::Metis => "0058",
            PoktChains::Moonbeam => "0050",
            PoktChains::Moonriver => "0051",
            PoktChains::Near => "0052",
            PoktChains::OKC => "0047",
            PoktChains::Oasys => "0070",
            PoktChains::OasysArchival => "0069",
            PoktChains::Optimism => "0053",
            PoktChains::OptimismArchival => "A053",
            PoktChains::Osmosis => "0054",
            PoktChains::Pocket => "0001",
            PoktChains::PolygonMatic => "0009",
            PoktChains::PolygonMaticArchival => "000B",
            PoktChains::PolygonMumbai => "000F",
            PoktChains::PolygonZkEVM => "0074",
            PoktChains::Radix => "0083",
            PoktChains::Scroll => "0082",
            PoktChains::ScrollTestnet => "0075",
            PoktChains::SepoliaTestnet => "0077",
            PoktChains::SepoliaArchival => "0078",
            PoktChains::Solana => "0006",
            PoktChains::SolanaCustom => "C006",
            PoktChains::Starknet => "0060",
            PoktChains::StarknetTestnet => "0061",
            PoktChains::Sui => "0076",
            PoktChains::Velas => "0067",
            PoktChains::VelasArchival => "0068",
            PoktChains::ZkSync => "0084",
        }
    }

    pub fn init_deployment_url() {
        GATEWAY_URL
            .set(dotenvy::var("GATEWAY_URL").unwrap().leak())
            .unwrap()
    }

    pub fn get_endpoint(&self) -> String {
        format!("{}/relay/{}", GATEWAY_URL.get().unwrap(), Self::id(self))
    }

    pub async fn relay_pokt_transaction(
        &self,
        body: &JsonRpcRequest,
    ) -> Result<String, RelayErrors> {
        Ok(Client::new()
            .post(self.get_endpoint())
            .json(body)
            .send()
            .await?
            .text()
            .await?)
    }
}

impl FromStr for PoktChains {
    type Err = RelayErrors;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pokt_test" => Ok(PoktChains::PoktTestnetEthereumMock),
            "arbitrum" => Ok(PoktChains::Arbitrum),
            "amoy_a" => Ok(PoktChains::AmoyTestnetArchival),
            "arbitrum_sepolia_a" => Ok(PoktChains::ArbitrumSepoliaArchival),
            "optimism_sepolia_a" => Ok(PoktChains::OptimismSepoliaArchival),
            "avax" => Ok(PoktChains::AVAX),
            "avax_a" => Ok(PoktChains::AVAXArchival),
            "boba" => Ok(PoktChains::BOBA),
            "base" => Ok(PoktChains::Base),
            "base_t" => Ok(PoktChains::BaseTestnet),
            "bsc" => Ok(PoktChains::BinanceSmartChain),
            "bsc_a" => Ok(PoktChains::BinanceSmartChainArchival),
            "celestia_a" => Ok(PoktChains::CelestiaArchival),
            "celo" => Ok(PoktChains::Celo),
            "dfk" => Ok(PoktChains::DFKchainSubnet),
            "doge" => Ok(PoktChains::Dogechain),
            "eth_beacon" => Ok(PoktChains::EthereumBeacon),
            "ethereum" => Ok(PoktChains::Ethereum),
            "ethereum_a" => Ok(PoktChains::EthereumArchival),
            "eth_trace_a" => Ok(PoktChains::EthereumArchivalTrace),
            "eth_high_gas" => Ok(PoktChains::EthereumHighGas),
            "evmos" => Ok(PoktChains::Evmos),
            "fantom" => Ok(PoktChains::Fantom),
            "fuse" => Ok(PoktChains::Fuse),
            "fuse_a" => Ok(PoktChains::FuseArchival),
            "gnosis" => Ok(PoktChains::Gnosis),
            "gnosis_a" => Ok(PoktChains::GnosisArchival),
            "goerli" => Ok(PoktChains::Goerli),
            "goerli_a" => Ok(PoktChains::GoerliArchival),
            "harmony_0" => Ok(PoktChains::HarmonyShard0),
            "hol_beacon" => Ok(PoktChains::HoleskyBeacon),
            "holesky" => Ok(PoktChains::HoleskyTestnet),
            "iotex" => Ok(PoktChains::IoTeX),
            "kava" => Ok(PoktChains::Kava),
            "kava_a" => Ok(PoktChains::KavaArchival),
            "klatyn" => Ok(PoktChains::Klatyn),
            "kovan" => Ok(PoktChains::Kovan),
            "meter" => Ok(PoktChains::Meter),
            "metis" => Ok(PoktChains::Metis),
            "moonbeam" => Ok(PoktChains::Moonbeam),
            "moonriver" => Ok(PoktChains::Moonriver),
            "near" => Ok(PoktChains::Near),
            "okc" => Ok(PoktChains::OKC),
            "oasys" => Ok(PoktChains::Oasys),
            "oasys_a" => Ok(PoktChains::OasysArchival),
            "optimism" => Ok(PoktChains::Optimism),
            "optimism_a" => Ok(PoktChains::OptimismArchival),
            "osmosis" => Ok(PoktChains::Osmosis),
            "pocket" => Ok(PoktChains::Pocket),
            "matic" => Ok(PoktChains::PolygonMatic),
            "matic_a" => Ok(PoktChains::PolygonMaticArchival),
            "mumbai_t" => Ok(PoktChains::PolygonMumbai),
            "polygon_zkevm" => Ok(PoktChains::PolygonZkEVM),
            "radix" => Ok(PoktChains::Radix),
            "scroll" => Ok(PoktChains::Scroll),
            "scroll_t" => Ok(PoktChains::ScrollTestnet),
            "sepolia_t" => Ok(PoktChains::SepoliaTestnet),
            "sepolia_a" => Ok(PoktChains::SepoliaArchival),
            "solana" => Ok(PoktChains::Solana),
            "solana_custom" => Ok(PoktChains::SolanaCustom),
            "starknet" => Ok(PoktChains::Starknet),
            "starknet_t" => Ok(PoktChains::StarknetTestnet),
            "sui" => Ok(PoktChains::Sui),
            "velas" => Ok(PoktChains::Velas),
            "velas_a" => Ok(PoktChains::VelasArchival),
            "zksync" => Ok(PoktChains::ZkSync),
            _ => Err(RelayErrors::PoktChainIdParsingError),
        }
    }
}

#[derive(Debug)]
pub enum RelayErrors {
    RelayError(reqwest::Error),
    PoktChainIdParsingError,
}

impl From<reqwest::Error> for RelayErrors {
    fn from(value: reqwest::Error) -> Self {
        RelayErrors::RelayError(value)
    }
}

impl Display for RelayErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RelayErrors::RelayError(_) => {
                write!(f, "Failed to submit transaction or parse the response")
            }
            RelayErrors::PoktChainIdParsingError => write!(f, "Could not identify chain by id"),
        }
    }
}

impl std::error::Error for RelayErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RelayErrors::RelayError(e) => Some(e),
            RelayErrors::PoktChainIdParsingError => None,
        }
    }
}
