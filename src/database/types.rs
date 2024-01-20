use std::sync::OnceLock;

use sqlx::{Pool, Postgres, PgPool, migrate};

pub static RELATIONAL_DATABASE: OnceLock<Pool<Postgres>> = OnceLock::new();


pub struct Database;

impl Database {
    pub async fn init(test: Option<()>) -> Result<(), Box<dyn std::error::Error>> {
        let pool = match test {
            Some(_) => PgPool::connect(&dotenvy::var("TESTING_DATABASE_URL").unwrap()).await.unwrap(),
            None => PgPool::connect(&dotenvy::var("DATABASE_URL").unwrap()).await.unwrap(),
        };
        migrate!("./migrations").run(&pool).await.unwrap();
        RELATIONAL_DATABASE.get_or_init(|| pool);
        Ok(())
    }
}
