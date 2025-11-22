use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use super::types::RelayErrors;
use crate::routes::relayer::types::{PoktChains, Relayer};
use axum::{body::Bytes, extract::Path, http::StatusCode, response::IntoResponse};
use thiserror::Error;

pub struct Cache {
    // api_key => calls
    pub entries: Arc<RwLock<HashMap<String, Arc<AtomicU64>>>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

/// REMEMBER: DON'T HOLD THE LOCK FOR LONGER THAN NECESSARY
impl Cache {
    pub fn new() -> Cache {
        Cache {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// returns old value before incrementing or inserting
    pub fn fetch_incr_or_insert(&self, key: String) -> u64 {
        match self.count_ref(&key) {
            Some(e) => e.fetch_add(1, Ordering::Acquire),
            None => self.insert(key),
        }
    }

    pub fn insert(&self, key: String) -> u64 {
        let mut hm = self.entries.write().unwrap();
        // catch all in case multiple requests are in flight and cache isn't populated
        match hm.get_mut(&key) {
            Some(e) => e.fetch_add(1, Ordering::Acquire),
            None => {
                hm.insert(key, Arc::new(AtomicU64::new(1)));
                1
            }
        }
    }

    /// CRITICAL: THIS DOES NOT CONTEND RW LOCK AS WRITER
    pub fn count_ref(&self, key: &str) -> Option<Arc<AtomicU64>> {
        // operate on a value without holding onto the lock
        // drops at the end of the scope
        let hm = self.entries.read().unwrap();
        let key = hm.get(key);
        key.cloned()
    }

    pub fn refresh_entire_cache(&self, new_hm: HashMap<String, Arc<AtomicU64>>) {
        let mut lock = self.entries.write().unwrap();
        *lock = new_hm;
    }
}

pub async fn route_call(
    Path(route_info): Path<[String; 2]>,
    body: Bytes,
) -> Result<impl IntoResponse, RouterErrors> {
    let raw_destination = route_info
        .first()
        .ok_or_else(|| RouterErrors::DestinationError)?;
    let dest = raw_destination.parse::<PoktChains>()?;
    let result = dest.relay_transaction(body).await?;
    Ok((StatusCode::OK, result))
}

#[derive(Debug, Error)]
pub enum RouterErrors {
    #[error("Could not parse destination from the first Path parameter")]
    DestinationError,
    #[error(transparent)]
    Relay(#[from] RelayErrors),
    #[error("malformed payload")]
    NotJsonRpc,
}

impl IntoResponse for RouterErrors {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[cfg(test)]
pub mod test {
    use crate::routes::relayer::types::{PoktChains, Relayer};
    use http_body_util::BodyExt;
    use serde_json::json;

    #[tokio::test]
    async fn relay_test() {
        let body = json!({
            "jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id": 1
        })
        .to_string()
        .leak()
        .as_bytes();

        let bytes = axum::body::Bytes::from_static(body);
        let chain = "anvil";
        let dest = chain.parse::<PoktChains>().unwrap();
        let res = dest.relay_transaction(bytes).await;
        assert!(res.is_ok());
        let text = res.unwrap().collect().await.unwrap().to_bytes().to_vec();
        let text = String::from_utf8(text).unwrap();
        println!("{text:?}");
    }
}
