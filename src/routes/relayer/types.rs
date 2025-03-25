use crate::json_rpc::types::JsonRpcRequest;
use reqwest::Client;
use std::{
    fmt::{self, Display, Formatter},
    future::Future,
    str::FromStr,
    sync::LazyLock,
};

pub static GATEWAY_ENDPOINT: LazyLock<&'static str> =
    LazyLock::new(|| format!("{}/v1", dotenvy::var("GATEWAY_URL").unwrap()).leak());

pub trait Relayer {
    fn relay_transaction(
        &self,
        body: &JsonRpcRequest,
    ) -> impl Future<Output = Result<String, RelayErrors>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoktChains {
    ArbitrumOne,
    ArbitrumSepoliaTestnet,
    Avalanche,
    AvalancheDFK,
    Base,
    BaseSepoliaTestnet,
    Bitcoin,
    Blast,
    BNBChain,
    Boba,
    CelestiaConsensus,
    CelestiaConsensusTestnet,
    CelestiaDA,
    CelestiaDATestnet,
    Celo,
    Ethereum,
    EthereumHoleskyTestnet,
    EthereumSepoliaTestnet,
    Evmos,
    Fantom,
    Fraxtal,
    Fuse,
    Gnosis,
    Harmony0,
    IoTeX,
    Kaia,
    Kava,
    Metis,
    Moonbeam,
    Moonriver,
    Near,
    OasysMainnet,
    OpBNB,
    Optimism,
    OptimismSepolia,
    Osmosis,
    PocketNetwork,
    Polygon,
    PolygonAmoyTestnet,
    PolygonzkEVM,
    Radix,
    Scroll,
    Solana,
    Sui,
    Taiko,
    TaikoHeklaTestnet,
    ZkLink,
    ZkSyncEra,
}

impl fmt::Display for PoktChains {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoktChains::ArbitrumOne => write!(f, "F001"),
            PoktChains::ArbitrumSepoliaTestnet => write!(f, "F002"),
            PoktChains::Avalanche => write!(f, "F003"),
            PoktChains::AvalancheDFK => write!(f, "F004"),
            PoktChains::Base => write!(f, "F005"),
            PoktChains::BaseSepoliaTestnet => write!(f, "F006"),
            PoktChains::Bitcoin => write!(f, "F007"),
            PoktChains::Blast => write!(f, "F008"),
            PoktChains::BNBChain => write!(f, "F009"),
            PoktChains::Boba => write!(f, "F00A"),
            PoktChains::CelestiaConsensus => write!(f, "A0CB"),
            PoktChains::CelestiaConsensusTestnet => write!(f, "A0CC"),
            PoktChains::CelestiaDA => write!(f, "A0CA"),
            PoktChains::CelestiaDATestnet => write!(f, "A0CD"),
            PoktChains::Celo => write!(f, "F00B"),
            PoktChains::Ethereum => write!(f, "F00C"),
            PoktChains::EthereumHoleskyTestnet => write!(f, "F00D"),
            PoktChains::EthereumSepoliaTestnet => write!(f, "F00E"),
            PoktChains::Evmos => write!(f, "F00F"),
            PoktChains::Fantom => write!(f, "F010"),
            PoktChains::Fraxtal => write!(f, "F011"),
            PoktChains::Fuse => write!(f, "F012"),
            PoktChains::Gnosis => write!(f, "F013"),
            PoktChains::Harmony0 => write!(f, "F014"),
            PoktChains::IoTeX => write!(f, "F015"),
            PoktChains::Kaia => write!(f, "F016"),
            PoktChains::Kava => write!(f, "F017"),
            PoktChains::Metis => write!(f, "F018"),
            PoktChains::Moonbeam => write!(f, "F019"),
            PoktChains::Moonriver => write!(f, "F01A"),
            PoktChains::Near => write!(f, "F01B"),
            PoktChains::OasysMainnet => write!(f, "F01C"),
            PoktChains::OpBNB => write!(f, "F01F"),
            PoktChains::Optimism => write!(f, "F01D"),
            PoktChains::OptimismSepolia => write!(f, "F01E"),
            PoktChains::Osmosis => write!(f, "F020"),
            PoktChains::PocketNetwork => write!(f, "F000"),
            PoktChains::Polygon => write!(f, "F021"),
            PoktChains::PolygonAmoyTestnet => write!(f, "F022"),
            PoktChains::PolygonzkEVM => write!(f, "F029"),
            PoktChains::Radix => write!(f, "F023"),
            PoktChains::Scroll => write!(f, "F024"),
            PoktChains::Solana => write!(f, "F025"),
            PoktChains::Sui => write!(f, "F026"),
            PoktChains::Taiko => write!(f, "F027"),
            PoktChains::TaikoHeklaTestnet => write!(f, "F028"),
            PoktChains::ZkLink => write!(f, "F02A"),
            PoktChains::ZkSyncEra => write!(f, "F02B"),
        }
    }
}

impl PoktChains {
    pub const fn id(&self) -> &'static str {
        match self {
            PoktChains::ArbitrumOne => "F001",
            PoktChains::ArbitrumSepoliaTestnet => "F002",
            PoktChains::Avalanche => "F003",
            PoktChains::AvalancheDFK => "F004",
            PoktChains::Base => "F005",
            PoktChains::BaseSepoliaTestnet => "F006",
            PoktChains::Bitcoin => "F007",
            PoktChains::Blast => "F008",
            PoktChains::BNBChain => "F009",
            PoktChains::Boba => "F00A",
            PoktChains::CelestiaConsensus => "A0CB",
            PoktChains::CelestiaConsensusTestnet => "A0CC",
            PoktChains::CelestiaDA => "A0CA",
            PoktChains::CelestiaDATestnet => "A0CD",
            PoktChains::Celo => "F00B",
            PoktChains::Ethereum => "F00C",
            PoktChains::EthereumHoleskyTestnet => "F00D",
            PoktChains::EthereumSepoliaTestnet => "F00E",
            PoktChains::Evmos => "F00F",
            PoktChains::Fantom => "F010",
            PoktChains::Fraxtal => "F011",
            PoktChains::Fuse => "F012",
            PoktChains::Gnosis => "F013",
            PoktChains::Harmony0 => "F014",
            PoktChains::IoTeX => "F015",
            PoktChains::Kaia => "F016",
            PoktChains::Kava => "F017",
            PoktChains::Metis => "F018",
            PoktChains::Moonbeam => "F019",
            PoktChains::Moonriver => "F01A",
            PoktChains::Near => "F01B",
            PoktChains::OasysMainnet => "F01C",
            PoktChains::OpBNB => "F01F",
            PoktChains::Optimism => "F01D",
            PoktChains::OptimismSepolia => "F01E",
            PoktChains::Osmosis => "F020",
            PoktChains::PocketNetwork => "F000",
            PoktChains::Polygon => "F021",
            PoktChains::PolygonAmoyTestnet => "F022",
            PoktChains::PolygonzkEVM => "F029",
            PoktChains::Radix => "F023",
            PoktChains::Scroll => "F024",
            PoktChains::Solana => "F025",
            PoktChains::Sui => "F026",
            PoktChains::Taiko => "F027",
            PoktChains::TaikoHeklaTestnet => "F028",
            PoktChains::ZkLink => "F02A",
            PoktChains::ZkSyncEra => "F02B",
        }
    }
}

impl Relayer for PoktChains {
    async fn relay_transaction(&self, body: &JsonRpcRequest) -> Result<String, RelayErrors> {
        Ok(Client::new()
            .post(*GATEWAY_ENDPOINT)
            .header("target-service-id", self.id())
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
            "arbitrum" => Ok(PoktChains::ArbitrumOne),
            "arbitrumsepolia" => Ok(PoktChains::ArbitrumSepoliaTestnet),
            "avalanche" => Ok(PoktChains::Avalanche),
            "avalanchedfk" => Ok(PoktChains::AvalancheDFK),
            "base" => Ok(PoktChains::Base),
            "basesepolia" => Ok(PoktChains::BaseSepoliaTestnet),
            "bitcoin" => Ok(PoktChains::Bitcoin),
            "blast" => Ok(PoktChains::Blast),
            "bsc" => Ok(PoktChains::BNBChain),
            "boba" => Ok(PoktChains::Boba),
            "celestiaconsensus" => Ok(PoktChains::CelestiaConsensus),
            "celestiaconsensustestnet" => Ok(PoktChains::CelestiaConsensusTestnet),
            "celestia" => Ok(PoktChains::CelestiaDA),
            "celestiatestnet" => Ok(PoktChains::CelestiaDATestnet),
            "celo" => Ok(PoktChains::Celo),
            "ethereum" => Ok(PoktChains::Ethereum),
            "holesky" => Ok(PoktChains::EthereumHoleskyTestnet),
            "sepolia" => Ok(PoktChains::EthereumSepoliaTestnet),
            "evmos" => Ok(PoktChains::Evmos),
            "fantom" => Ok(PoktChains::Fantom),
            "fraxtal" => Ok(PoktChains::Fraxtal),
            "fuse" => Ok(PoktChains::Fuse),
            "gnosis" => Ok(PoktChains::Gnosis),
            "harmony0" => Ok(PoktChains::Harmony0),
            "iotex" => Ok(PoktChains::IoTeX),
            "kaia" => Ok(PoktChains::Kaia),
            "kava" => Ok(PoktChains::Kava),
            "metis" => Ok(PoktChains::Metis),
            "moonbeam" => Ok(PoktChains::Moonbeam),
            "moonriver" => Ok(PoktChains::Moonriver),
            "near" => Ok(PoktChains::Near),
            "oasysmainnet" => Ok(PoktChains::OasysMainnet),
            "opbnb" => Ok(PoktChains::OpBNB),
            "optimism" => Ok(PoktChains::Optimism),
            "optimismsepolia" => Ok(PoktChains::OptimismSepolia),
            "osmosis" => Ok(PoktChains::Osmosis),
            "pocketnetwork" => Ok(PoktChains::PocketNetwork),
            "polygon" => Ok(PoktChains::Polygon),
            "polygonamoytestnet" => Ok(PoktChains::PolygonAmoyTestnet),
            "polygonzkevm" => Ok(PoktChains::PolygonzkEVM),
            "radix" => Ok(PoktChains::Radix),
            "scroll" => Ok(PoktChains::Scroll),
            "solana" => Ok(PoktChains::Solana),
            "sui" => Ok(PoktChains::Sui),
            "taiko" => Ok(PoktChains::Taiko),
            "taikoheklatestnet" => Ok(PoktChains::TaikoHeklaTestnet),
            "zklink" => Ok(PoktChains::ZkLink),
            "zksyncera" => Ok(PoktChains::ZkSyncEra),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoktTestnetChains {
    Holesky,
    PocketTestnet,
}

impl fmt::Display for PoktTestnetChains {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoktTestnetChains::Holesky => write!(f, "0081"),
            PoktTestnetChains::PocketTestnet => write!(f, "0002"),
        }
    }
}

impl PoktTestnetChains {
    pub fn id(&self) -> &'static str {
        match self {
            PoktTestnetChains::Holesky => "0081",
            PoktTestnetChains::PocketTestnet => "0002",
        }
    }
}

impl FromStr for PoktTestnetChains {
    type Err = RelayErrors;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "holesky" => Ok(PoktTestnetChains::Holesky),
            "pockettestnet" => Ok(PoktTestnetChains::PocketTestnet),
            _ => Err(RelayErrors::PoktChainIdParsingError),
        }
    }
}
