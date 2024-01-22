use sqlx::FromRow;
use sqlx::{migrate, PgPool, Pool, Postgres};
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
    wallet: [u8; 32],
    password: String,
}

#[derive(FromRow, Debug)]
pub struct Payments {
    customer_email: String,
    call_count: i32,
    subscription: Plan,
}

#[derive(Debug, Clone)]
pub enum Plan {
    Based,
    Premier,
    Gigachad,
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
            _ => return Err(ParsingError),
        };

        Ok(plan)
    }
}
