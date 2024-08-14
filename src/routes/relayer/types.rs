use crate::json_rpc::types::JsonRpcRequest;
use reqwest::Client;
use std::{
    fmt::{self, Display, Formatter},
    future::Future,
    str::FromStr,
    sync::LazyLock,
};

pub static GATEWAY_URL: LazyLock<&'static str> =
    LazyLock::new(|| dotenvy::var("GATEWAY_URL").unwrap().leak());

pub trait Relayer {
    fn relay_transaction(
        &self,
        body: &JsonRpcRequest,
    ) -> impl Future<Output = Result<String, RelayErrors>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoktChains {
    Arbitrum,
    ArbitrumSepoliaArchival,
    AmoyTestnetArchival,
    AVAX,
    AVAXArchival,
    BOBA,
    Base,
    BaseTestnet,
    BinanceSmartChain,
    BinanceSmartChainArchival,
    BnbArchivalOp,
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
    //    Experimental,
    Fantom,
    FraxArchival,
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
    OptimismSepoliaArchival,
    Osmosis,
    Pocket,
    PocketArchival,
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
    Taiko,
    TaikoHeklaTestnet,
    Velas,
    VelasArchival,
    ZkSync,
    PoktTestnetEthereumMock,
}

impl fmt::Display for PoktChains {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoktChains::Arbitrum => write!(f, "0066"),
            PoktChains::ArbitrumSepoliaArchival => write!(f, "A086"),
            PoktChains::AmoyTestnetArchival => write!(f, "A085"),
            PoktChains::OptimismSepoliaArchival => write!(f, "A087"),
            PoktChains::AVAX => write!(f, "0003"),
            PoktChains::AVAXArchival => write!(f, "A003"),
            PoktChains::BOBA => write!(f, "0048"),
            PoktChains::Base => write!(f, "0079"),
            PoktChains::BaseTestnet => write!(f, "0080"),
            PoktChains::BinanceSmartChain => write!(f, "0004"),
            PoktChains::BinanceSmartChainArchival => write!(f, "0010"),
            PoktChains::BnbArchivalOp => write!(f, "A089"),
            PoktChains::CelestiaArchival => write!(f, "A0CA"),
            PoktChains::Celo => write!(f, "0065"),
            PoktChains::DFKchainSubnet => write!(f, "03DF"),
            PoktChains::Dogechain => write!(f, "0059"),
            PoktChains::EthereumBeacon => write!(f, "B021"),
            PoktChains::Ethereum => write!(f, "0021"),
            PoktChains::EthereumArchival => write!(f, "0022"),
            PoktChains::EthereumArchivalTrace => write!(f, "0028"),
            PoktChains::EthereumHighGas => write!(f, "0062"),
            PoktChains::Evmos => write!(f, "0046"),
            //Experimental = write!(f, "BE2A"),
            PoktChains::Fantom => write!(f, "0049"),
            PoktChains::FraxArchival => write!(f, "A088"),
            PoktChains::Fuse => write!(f, "0005"),
            PoktChains::FuseArchival => write!(f, "000A"),
            PoktChains::Gnosis => write!(f, "0027"),
            PoktChains::GnosisArchival => write!(f, "000C"),
            PoktChains::Goerli => write!(f, "0026"),
            PoktChains::GoerliArchival => write!(f, "0063"),
            PoktChains::HarmonyShard0 => write!(f, "0040"),
            PoktChains::HoleskyBeacon => write!(f, "B081"),
            PoktChains::HoleskyTestnet => write!(f, "0081"),
            PoktChains::IoTeX => write!(f, "0044"),
            PoktChains::Kava => write!(f, "0071"),
            PoktChains::KavaArchival => write!(f, "0072"),
            PoktChains::Klatyn => write!(f, "0056"),
            PoktChains::Kovan => write!(f, "0024"),
            PoktChains::Meter => write!(f, "0057"),
            PoktChains::Metis => write!(f, "0058"),
            PoktChains::Moonbeam => write!(f, "0050"),
            PoktChains::Moonriver => write!(f, "0051"),
            PoktChains::Near => write!(f, "0052"),
            PoktChains::OKC => write!(f, "0047"),
            PoktChains::Oasys => write!(f, "0070"),
            PoktChains::OasysArchival => write!(f, "0069"),
            PoktChains::Optimism => write!(f, "0053"),
            PoktChains::OptimismArchival => write!(f, "A053"),
            PoktChains::Osmosis => write!(f, "0054"),
            PoktChains::Pocket => write!(f, "0001"),
            PoktChains::PocketArchival => write!(f, "A001"),
            PoktChains::PolygonMatic => write!(f, "0009"),
            PoktChains::PolygonMaticArchival => write!(f, "000B"),
            PoktChains::PolygonMumbai => write!(f, "000F"),
            PoktChains::PolygonZkEVM => write!(f, "0074"),
            PoktChains::Radix => write!(f, "0083"),
            PoktChains::Scroll => write!(f, "0082"),
            PoktChains::ScrollTestnet => write!(f, "0075"),
            PoktChains::SepoliaTestnet => write!(f, "0077"),
            PoktChains::SepoliaArchival => write!(f, "0078"),
            PoktChains::Solana => write!(f, "0006"),
            PoktChains::SolanaCustom => write!(f, "C006"),
            PoktChains::Starknet => write!(f, "0060"),
            PoktChains::StarknetTestnet => write!(f, "0061"),
            PoktChains::Sui => write!(f, "0076"),
            PoktChains::Taiko => write!(f, "7A00"),
            PoktChains::TaikoHeklaTestnet => write!(f, "7A10"),
            PoktChains::Velas => write!(f, "0067"),
            PoktChains::VelasArchival => write!(f, "0068"),
            PoktChains::ZkSync => write!(f, "0084"),
            PoktChains::PoktTestnetEthereumMock => write!(f, "0007"),
        }
    }
}

impl PoktChains {
    fn id(&self) -> &'static str {
        match self {
            PoktChains::Arbitrum => "0066",
            PoktChains::ArbitrumSepoliaArchival => "A086",
            PoktChains::AmoyTestnetArchival => "A085",
            PoktChains::OptimismSepoliaArchival => "A087",
            PoktChains::AVAX => "0003",
            PoktChains::AVAXArchival => "A003",
            PoktChains::BOBA => "0048",
            PoktChains::Base => "0079",
            PoktChains::BaseTestnet => "0080",
            PoktChains::BinanceSmartChain => "0004",
            PoktChains::BinanceSmartChainArchival => "0010",
            PoktChains::BnbArchivalOp => "A089",
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
            //Experimental = "BE2A",
            PoktChains::Fantom => "0049",
            PoktChains::FraxArchival => "A088",
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
            PoktChains::PocketArchival => "A001",
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
            PoktChains::Taiko => "7A00",
            PoktChains::TaikoHeklaTestnet => "7A10",
            PoktChains::Velas => "0067",
            PoktChains::VelasArchival => "0068",
            PoktChains::ZkSync => "0084",
            PoktChains::PoktTestnetEthereumMock => "0007",
        }
    }

    pub fn get_endpoint(&self) -> String {
        format!("{}/relay/{}", *GATEWAY_URL, self.id())
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
        match value.to_lowercase().as_ref() {
            "arbitrum" => Ok(PoktChains::Arbitrum),
            "arbitrumsepoliaarchival" => Ok(PoktChains::ArbitrumSepoliaArchival),
            "amoytestnetarchival" => Ok(PoktChains::AmoyTestnetArchival),
            "avax" => Ok(PoktChains::AVAX),
            "avaxarchival" => Ok(PoktChains::AVAXArchival),
            "boba" => Ok(PoktChains::BOBA),
            "base" => Ok(PoktChains::Base),
            "basetestnet" => Ok(PoktChains::BaseTestnet),
            "binancesmartchain" => Ok(PoktChains::BinanceSmartChain),
            "binancesmartchainarchival" => Ok(PoktChains::BinanceSmartChainArchival),
            "bnbarchivalop" => Ok(PoktChains::BnbArchivalOp),
            "celestiaarchival" => Ok(PoktChains::CelestiaArchival),
            "celo" => Ok(PoktChains::Celo),
            "dfkchainsubnet" => Ok(PoktChains::DFKchainSubnet),
            "dogechain" => Ok(PoktChains::Dogechain),
            "ethereumbeacon" => Ok(PoktChains::EthereumBeacon),
            "ethereum" => Ok(PoktChains::Ethereum),
            "ethereumarchival" => Ok(PoktChains::EthereumArchival),
            "ethereumarchivaltrace" => Ok(PoktChains::EthereumArchivalTrace),
            "ethereumhighgas" => Ok(PoktChains::EthereumHighGas),
            // "experimental"
            "evmos" => Ok(PoktChains::Evmos),
            "fantom" => Ok(PoktChains::Fantom),
            "fraxarchival" => Ok(PoktChains::FraxArchival),
            "fuse" => Ok(PoktChains::Fuse),
            "fusearchival" => Ok(PoktChains::FuseArchival),
            "gnosis" => Ok(PoktChains::Gnosis),
            "gnosisarchival" => Ok(PoktChains::GnosisArchival),
            "goerli" => Ok(PoktChains::Goerli),
            "goerliarchival" => Ok(PoktChains::GoerliArchival),
            "harmonyshard0" => Ok(PoktChains::HarmonyShard0),
            "holeskybeacon" => Ok(PoktChains::HoleskyBeacon),
            "holeskytestnet" => Ok(PoktChains::HoleskyTestnet),
            "iotex" => Ok(PoktChains::IoTeX),
            "kava" => Ok(PoktChains::Kava),
            "kavaarchival" => Ok(PoktChains::KavaArchival),
            "klatyn" => Ok(PoktChains::Klatyn),
            "kovan" => Ok(PoktChains::Kovan),
            "meter" => Ok(PoktChains::Meter),
            "metis" => Ok(PoktChains::Metis),
            "moonbeam" => Ok(PoktChains::Moonbeam),
            "moonriver" => Ok(PoktChains::Moonriver),
            "near" => Ok(PoktChains::Near),
            "okc" => Ok(PoktChains::OKC),
            "oasys" => Ok(PoktChains::Oasys),
            "oasysarchival" => Ok(PoktChains::OasysArchival),
            "optimism" => Ok(PoktChains::Optimism),
            "optimismarchival" => Ok(PoktChains::OptimismArchival),
            "optimismsepoliaarchival" => Ok(PoktChains::OptimismSepoliaArchival),
            "osmosis" => Ok(PoktChains::Osmosis),
            "pocket" => Ok(PoktChains::Pocket),
            "pocketarchival" => Ok(PoktChains::PocketArchival),
            "polygonmatic" => Ok(PoktChains::PolygonMatic),
            "polygonmaticarchival" => Ok(PoktChains::PolygonMaticArchival),
            "polygonmumbai" => Ok(PoktChains::PolygonMumbai),
            "polygonzkevm" => Ok(PoktChains::PolygonZkEVM),
            "radix" => Ok(PoktChains::Radix),
            "scroll" => Ok(PoktChains::Scroll),
            "scrolltestnet" => Ok(PoktChains::ScrollTestnet),
            "sepoliatestnet" => Ok(PoktChains::SepoliaTestnet),
            "sepoliaarchival" => Ok(PoktChains::SepoliaArchival),
            "solana" => Ok(PoktChains::Solana),
            "solanacustom" => Ok(PoktChains::SolanaCustom),
            "starknet" => Ok(PoktChains::Starknet),
            "starknettestnet" => Ok(PoktChains::StarknetTestnet),
            "sui" => Ok(PoktChains::Sui),
            "taiko" => Ok(PoktChains::Taiko),
            "taikoheklatestnet" => Ok(PoktChains::TaikoHeklaTestnet),
            "velas" => Ok(PoktChains::Velas),
            "velasarchival" => Ok(PoktChains::VelasArchival),
            "zksync" => Ok(PoktChains::ZkSync),
            "pokttestnetethereummock" => Ok(PoktChains::PoktTestnetEthereumMock),
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
