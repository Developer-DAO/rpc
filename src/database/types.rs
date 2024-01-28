use sqlx::{migrate, PgPool, Pool, Postgres, types::time::OffsetDateTime, FromRow};
use std::fmt::Display;
use std::str::FromStr;
use std::sync::OnceLock;

use super::errors::ParsingError;

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
    email: String,
    wallet: String,
    password: String,
}

#[derive(FromRow, Debug)]
pub struct PaymentInfo {
    customer_email: String,
    call_count: i32,
    plan_expiration: i64,
    subscription: Plan,
}

#[derive(FromRow, Debug)]
pub struct Payments {
    customer_email: String,
    transaction_hash: String,
    asset: Asset, 
    amount: i64,
    chain: Chain, 
    date: OffsetDateTime,
}

#[derive(FromRow, Debug)]
pub struct Api {
    customer_email: String, 
    api_key: String,
}

#[derive(Debug, Clone)]
pub enum Plan {
    Based,
    Premier,
    Gigachad,
}

#[derive(Debug, Clone)]
pub enum Chain {
    Optimism, 
    Polygon, 
    Arbitrum,
    Base,
}

#[derive(Debug, Clone)]
pub enum Asset {
    Ether,
    USDC
}

impl Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Plan {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let plan = match s {
            "based" => Plan::Based,
            "premier" => Plan::Premier,
            "gigachad" => Plan::Gigachad,
            _ => return Err(ParsingError(s.to_string(), "Plan")),
        };

        Ok(plan)
    }
}

impl Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
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
            _ => return Err(ParsingError(s.to_string(), "Chain")),
        };

        Ok(plan)
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Asset {
    type Err = ParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let plan = match s {
            "ether" => Asset::Ether,
            "usdc" => Asset::USDC,
            _ => return Err(ParsingError(s.to_string(), "Asset")),
        };

        Ok(plan)
    }
}
