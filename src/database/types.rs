use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres, migrate, postgres::PgPoolOptions};
use std::fmt::Display;
use std::str::FromStr;
use std::sync::OnceLock;
use time::OffsetDateTime;

use super::errors::{ChainidError, ParsingError};

pub static RELATIONAL_DATABASE: OnceLock<Pool<Postgres>> = OnceLock::new();

pub struct Database;

impl Database {
    pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
        let pool = PgPoolOptions::new()
            // todo: fix this for prod
            .after_release(|_, _| Box::pin(async move { Ok(false) }))
            .connect(&dotenvy::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        migrate!("./migrations").run(&pool).await.unwrap();
        RELATIONAL_DATABASE.get_or_init(|| pool);
        Ok(())
    }
}

#[derive(FromRow, Debug)]
pub struct Customers {
    pub email: String,
    pub wallet: Option<String>,
    pub role: Role,
    pub password: String,
    pub verificationcode: String,
    pub activated: bool,
}

#[derive(FromRow, Debug)]
pub struct PaymentInfo {
    pub customer_email: String,
    pub call_count: i64,
    pub plan_expiration: i64,
    pub subscription: Plan,
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct Payments {
    pub customeremail: String,
    pub transactionhash: String,
    pub asset: Asset,
    pub amount: String,
    pub chain: Chain,
    pub date: OffsetDateTime,
    pub usdvalue: i64,
    pub decimals: i32,
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct Api {
    pub customeremail: String,
    pub apikey: String,
}

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "PLAN", rename_all = "lowercase")]
pub enum Plan {
    Free,
    Tier1,
    Tier2,
    Tier3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase", type_name = "chain")]
pub enum Chain {
    Optimism,
    Polygon,
    Arbitrum,
    Base,
    Anvil,
    Sepolia,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase", type_name = "asset")]
pub enum Asset {
    Ether,
    USDC,
}

impl Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Plan::Free => write!(f, "free"),
            Plan::Tier1 => write!(f, "tier1"),
            Plan::Tier2 => write!(f, "tier2"),
            Plan::Tier3 => write!(f, "tier3"),
        }
    }
}

impl FromStr for Plan {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let plan = match s {
            "tier1" => Plan::Tier1,
            "premier" => Plan::Tier2,
            "tier3" => Plan::Tier3,
            _ => Err(ParsingError(s.to_string(), "Plan"))?,
        };

        Ok(plan)
    }
}

impl Plan {
    pub const FREE_TIER_LIMIT: u32 = 3_100_000;
    // Free Tier: 3.1M requests per month
    pub const TIER_ONE: u32 = 30_000_000;
    pub const TIER_ONE_COST: f64 = 50.0;
    // Tier 1: 30M requests per month
    // price: $50/mo
    pub const TIER_TWO: u32 = 125_000_000;
    pub const TIER_TWO_COST: f64 = 20.0;
    // Tier 2: 125M requests per month
    // price: $200/mo
    pub const TIER_THREE: u32 = 500_000_000;
    pub const TIER_THREE_COST: f64 = 875.0;
    // Tier 3: 500M requests per month
    // price: $875/mo

    pub fn get_cost(&self) -> f64 {
        match self {
            Plan::Free => 0.0,
            // $50
            Plan::Tier1 => Self::TIER_ONE_COST,
            // $200
            Plan::Tier2 => Self::TIER_TWO_COST,
            // $875
            Plan::Tier3 => Self::TIER_THREE_COST,
        }
    }

    pub fn get_plan_limit(&self) -> u32 {
        match self {
            Plan::Free => Self::FREE_TIER_LIMIT,
            Plan::Tier1 => Self::TIER_ONE,
            Plan::Tier2 => Self::TIER_TWO,
            Plan::Tier3 => Self::TIER_THREE,
        }
    }
}

impl Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Base => write!(f, "base"),
            Chain::Polygon => write!(f, "polygon"),
            Chain::Optimism => write!(f, "optimism"),
            Chain::Arbitrum => write!(f, "arbitrum"),
            Chain::Anvil => write!(f, "anvil"),
            Chain::Sepolia => write!(f, "sepolia"),
        }
    }
}

impl FromStr for Chain {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let plan = match s {
            "optimism" => Chain::Optimism,
            "polygon" => Chain::Polygon,
            "base" => Chain::Base,
            "arbitrum" => Chain::Arbitrum,
            _ => Err(ParsingError(s.to_string(), "Chain"))?,
        };
        Ok(plan)
    }
}
// Define the trait FromHexStr
pub trait FromHexStr {
    type Err;

    fn from_hex(s: &str) -> Result<Self, Self::Err>
    where
        Self: Sized;
}

impl FromHexStr for Chain {
    type Err = ChainidError;

    fn from_hex(s: &str) -> Result<Self, Self::Err> {
        let chain = match s {
            "0xa" => Chain::Optimism,
            "0x89" => Chain::Polygon,
            "0x2105" => Chain::Base,
            "0xa4b1" => Chain::Arbitrum,
            _ => Err(ChainidError(s.to_string(), "Invalid ChainId"))?,
        };

        Ok(chain)
    }
}

#[derive(Debug, Clone, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "ROLE", rename_all = "lowercase")]
pub enum Role {
    Normie,
    Admin,
}

impl From<String> for Role {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_ref() {
            "normie" => Role::Normie,
            "admin" => Role::Admin,
            _ => Role::Normie,
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Normie => write!(f, "Normie"),
            Role::Admin => write!(f, "Admin"),
        }
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Asset::Ether => write!(f, "ether"),
            Asset::USDC => write!(f, "usdc"),
        }
    }
}

impl FromStr for Asset {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let plan = match s {
            "ETHER" | "ether" => Asset::Ether,
            "USDC" | "usdc" => Asset::USDC,
            _ => Err(ParsingError(s.to_string(), "Asset"))?,
        };

        Ok(plan)
    }
}
