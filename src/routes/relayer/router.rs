use super::types::RelayErrors;
use crate::routes::relayer::types::{PoktChains, Relayer};
use axum::{body::Bytes, http::StatusCode, response::IntoResponse};
use thiserror::Error;

macro_rules! route {
    ($($i:ident = $t:expr)+) => {
        $(
             pub async fn $i(
                body: Bytes,
            ) -> Result<impl IntoResponse, RouterErrors> {
                Ok((StatusCode::OK, $t.relay_transaction(body).await?))
            }
        )+
    }
}

route! {
    route_arb = PoktChains::ArbOne
    route_op = PoktChains::Op
    route_sui = PoktChains::Sui
    route_poly = PoktChains::Poly
    route_solana = PoktChains::Solana
    route_base = PoktChains::Base
    route_eth = PoktChains::Eth
    route_bsc = PoktChains::Bsc
}

#[derive(Debug, Error)]
pub enum RouterErrors {
    #[error(transparent)]
    Relay(#[from] RelayErrors),
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
        let chain = "base";
        let dest = chain.parse::<PoktChains>().unwrap();
        let res = dest.relay_transaction(bytes).await;
        assert!(res.is_ok());
        let text = res.unwrap().collect().await.unwrap().to_bytes().to_vec();
        let text = String::from_utf8(text).unwrap();
        println!("{text:?}");
    }
}
