use std::error::Error;

use axum::{http::StatusCode, response::IntoResponse};
use serde::{Serialize, Deserialize};

use crate::eth_rpc::types::Address;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUser {
    pub email: String, 
    pub wallet: String, 
    pub password: String,
}

impl IntoResponse for RegisterUser {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self).into_response()
    }
}

trait Normalize {
    fn normalize(&self) -> Result<(), Box<dyn Error>>;
}

impl Normalize for RegisterUser {
    fn normalize(&self) -> Result<(), Box<dyn Error>> {
        Address::try_address(&self.wallet)?;
        Ok(())
    }
}
