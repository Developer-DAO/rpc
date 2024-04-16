use serde::{Deserialize, Serialize};
use sqlx::{migrate, types::time::OffsetDateTime, FromRow, PgPool, Pool, Postgres};
use std::fmt::Display;
use std::str::FromStr;
use std::sync::OnceLock;

use super::errors::{ChainidError, ParsingError};

pub static RELATIONAL_DATABASE: OnceLock<Pool<Postgres>> = OnceLock::new();

pub struct Database;

impl Database {
    pub async fn init(test: Option<()>) -> Result<(), Box<dyn std::error::Error>> {
        let pool = match test {
            Some(_) => PgPool::connect(&dotenvy::var("TESTING_DATABASE_URL").unwrap())
                .await
                .unwrap(),
            None => PgPool::connect(&dotenvy::var("DATABASE_URL").unwrap())
                .await
                .unwrap(),
        };
        migrate!("./migrations").run(&pool).await.unwrap();
        RELATIONAL_DATABASE.get_or_init(|| pool);
        Ok(())
    }
}

#[derive(FromRow, Debug)]
pub struct Customers {
    pub email: String,
    pub wallet: String,
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

#[derive(FromRow, Debug)]
pub struct Payments {
    pub customer_email: String,
    pub transaction_hash: String,
    pub asset: Asset,
    pub amount: i64,
    pub chain: Chain,
    pub date: OffsetDateTime,
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct Api {
    pub customeremail: String,
    pub apikey: String,
}

#[derive(Debug, Clone)]
pub enum Plan {
    None,
    Based,
    Premier,
    Gigachad,
}

#[derive(Debug, Clone, Copy , Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase", type_name = "chain")]
pub enum Chain {
    Optimism,
    Polygon,
    Arbitrum,
    Base,
}

#[derive(Debug, Clone , Copy , Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase", type_name = "chain")]
pub enum Asset {
    Ether,
    USDC,
}

impl Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Plan::None => write!(f, "none"),
            Plan::Based => write!(f, "based"),
            Plan::Premier => write!(f, "premier"),
            Plan::Gigachad => write!(f, "gigachad"),
        }
    }
}

impl FromStr for Plan {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let plan = match s {
            "based" => Plan::Based,
            "premier" => Plan::Premier,
            "gigachad" => Plan::Gigachad,
            _ => Err(ParsingError(s.to_string(), "Plan"))?,
        };

        Ok(plan)
    }
}

impl Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Base => write!(f, "base"),
            Chain::Polygon => write!(f, "polygon"),
            Chain::Optimism => write!(f, "optimism"),
            Chain::Arbitrum => write!(f, "arbitrum"),
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
            _ => return Err(ChainidError(s.to_string(), "Invalid ChainId")),
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
            "ether" => Asset::Ether,
            "usdc" => Asset::USDC,
            _ => Err(ParsingError(s.to_string(), "Asset"))?,
        };

        Ok(plan)
    }
}


