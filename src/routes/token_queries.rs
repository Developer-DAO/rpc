use alloy::{
    primitives::{Address, U256, address},
    providers::ProviderBuilder,
    sol,
};
use axum::{
    Json,
    extract::{Path, Query},
    response::IntoResponse,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::routes::{
    relayer::types::PoktChains,
    token_queries::TokenQueryContract::{TokenBalanceQuery, TokenQueryContractInstance},
};

static TOKEN_QUERY_UTIL_DEPLOYMENTS: std::sync::LazyLock<
    std::collections::HashMap<PoktChains, Address>,
> = std::sync::LazyLock::new(|| {
    let mut map = std::collections::HashMap::new();
    map.insert(
        PoktChains::Eth,
        address!("0x96a7B30FD0B97BfF5bEdB343049b378011Cc62fd"),
    );
    map.insert(
        PoktChains::Base,
        address!("0x8B52358d9d2651f9264Df0ceA60333263427b86F"),
    );
    map.insert(
        PoktChains::Poly,
        address!("0x87bd0e6aA53B21A9FB8f465cd90801a479321048"),
    );
    map.insert(
        PoktChains::ArbOne,
        address!("2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
    );
    map
});

sol! {
    #[sol(rpc)]
    contract TokenQueryContract {
        #[derive(Debug, Serialize, Deserialize)]
        struct TokenBalanceQuery {
            address contract_addr;
            address user;
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct TokenBalance {
            address contract_addr;
            uint256 amount;
            address user;
            uint8 decimals;
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct NftInfo {
            address owner;
            uint256 token_num;
        }

        function getBatchNftInfo(address, uint256, uint256) external view returns (NftInfo[] memory);
        function aggregateTokenBalsForUser(address[] memory, address) external view returns (TokenBalance[] memory);
        function aggregateSingleTokenBals(address[] memory, address) external view returns (TokenBalance[] memory);
        function aggregateBalances(TokenBalanceQuery[] memory) external view returns(TokenBalance[] memory);
    }
}

// Many tokens, many users
pub async fn aggregate_balances(
    Path((chain, api_key)): Path<(PoktChains, String)>,
    Json(payload): Json<Vec<TokenBalanceQuery>>,
) -> Result<impl IntoResponse, QueryError> {
    if payload.len() > 1000 {
        Err(QueryError::ERC20QueryLimit)?
    }

    if matches!(
        chain,
        PoktChains::Op | PoktChains::Bsc | PoktChains::Sui | PoktChains::Solana
    ) {
        return Err(QueryError::ChainError)?;
    }

    let endpoint: String = format!("https://api.cloud.developerdao.com/rpc/{chain}/{api_key}");
    let eth = reqwest::Url::parse(&endpoint)?;
    let provider = ProviderBuilder::new().connect_http(eth);
    let c_addr = *TOKEN_QUERY_UTIL_DEPLOYMENTS
        .get(&chain)
        .ok_or_else(|| QueryError::ChainError)?;

    let contract = TokenQueryContractInstance::new(c_addr, provider);
    let res = contract.aggregateBalances(payload).call().await?;

    Ok((StatusCode::OK, serde_json::to_string(&res)?).into_response())
}

// one token, many users
pub async fn aggregate_token_bals_for_user(
    Path((chain, api_key)): Path<(PoktChains, String)>,
    Query((address, tokens)): Query<(Address, Vec<Address>)>,
) -> Result<impl IntoResponse, QueryError> {
    if tokens.len() > 1000 {
        Err(QueryError::ERC20QueryLimit)?
    }

    if matches!(
        chain,
        PoktChains::Op | PoktChains::Bsc | PoktChains::Sui | PoktChains::Solana
    ) {
        return Err(QueryError::ChainError)?;
    }

    let endpoint: String = format!("https://api.cloud.developerdao.com/rpc/{chain}/{api_key}");
    let eth = reqwest::Url::parse(&endpoint)?;

    let provider = ProviderBuilder::new().connect_http(eth);
    let contract = TokenQueryContractInstance::new(
        address!("0x96a7B30FD0B97BfF5bEdB343049b378011Cc62fd"),
        provider,
    );
    let res = contract
        .aggregateTokenBalsForUser(tokens, address)
        .call()
        .await?;

    Ok((StatusCode::OK, serde_json::to_string(&res)?).into_response())
}

// many users, one token
pub async fn aggregate_single_token_bals(
    Path((chain, api_key)): Path<(PoktChains, String)>,
    Query((token_address, users)): Query<(Address, Vec<Address>)>,
) -> Result<impl IntoResponse, QueryError> {
    if users.len() > 1000 {
        Err(QueryError::ERC20QueryLimit)?
    }

    if matches!(
        chain,
        PoktChains::Op | PoktChains::Bsc | PoktChains::Sui | PoktChains::Solana
    ) {
        return Err(QueryError::ChainError)?;
    }

    let endpoint: String = format!("https://api.cloud.developerdao.com/rpc/{chain}/{api_key}");
    let eth = reqwest::Url::parse(&endpoint)?;
    let provider = ProviderBuilder::new().connect_http(eth);
    let c_addr = *TOKEN_QUERY_UTIL_DEPLOYMENTS
        .get(&chain)
        .ok_or_else(|| QueryError::ChainError)?;
    let contract = TokenQueryContractInstance::new(c_addr, provider);
    let res = contract
        .aggregateSingleTokenBals(users, token_address)
        .call()
        .await?;

    Ok((StatusCode::OK, serde_json::to_string(&res)?).into_response())
}

pub async fn get_batch_nft_info(
    Path((chain, api_key)): Path<(PoktChains, String)>,
    Query((collection, offset, limit)): Query<(Address, u16, u16)>,
) -> Result<impl IntoResponse, QueryError> {
    if limit > 10000 {
        Err(QueryError::NFTQueryLimit)?
    }

    if matches!(
        chain,
        PoktChains::Op | PoktChains::Bsc | PoktChains::Sui | PoktChains::Solana
    ) {
        return Err(QueryError::ChainError)?;
    }

    let endpoint: String = format!("https://api.cloud.developerdao.com/rpc/{chain}/{api_key}");
    let eth = reqwest::Url::parse(&endpoint)?;
    let provider = ProviderBuilder::new().connect_http(eth);
    let c_addr = *TOKEN_QUERY_UTIL_DEPLOYMENTS
        .get(&chain)
        .ok_or_else(|| QueryError::ChainError)?;
    let contract = TokenQueryContractInstance::new(c_addr, provider);

    let res = contract
        .getBatchNftInfo(collection, U256::from(offset), U256::from(limit))
        .call()
        .await?;

    Ok((StatusCode::OK, serde_json::to_string(&res)?).into_response())
}

// nft token owners

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("The max number of entries you can request for NFT owners is 10,000")]
    NFTQueryLimit,
    #[error("The max number of entries you can request for ERC20 balances is 1,000")]
    ERC20QueryLimit,
    #[error(transparent)]
    ParseError(#[from] url::ParseError),
    #[error(transparent)]
    ContractError(#[from] alloy::contract::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("This chain is not yet supported for the token query endpoints.")]
    ChainError,
}

impl IntoResponse for QueryError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[cfg(test)]
pub mod test {

    use crate::routes::payment::D_D_CLOUD_API_KEY;

    pub use super::*;
    #[tokio::test]
    async fn basic() {
        let endpoint: String = format!(
            "https://api.cloud.developerdao.com/rpc/{}/{}",
            PoktChains::Eth,
            *D_D_CLOUD_API_KEY
        );
        let eth = reqwest::Url::parse(&endpoint).unwrap();
        let provider = ProviderBuilder::new().connect_http(eth);
        let contract = TokenQueryContractInstance::new(
            // mainnet
            address!("0x96a7B30FD0B97BfF5bEdB343049b378011Cc62fd"),
            provider,
        );
        let res = contract
            .aggregateBalances(vec![TokenBalanceQuery {
                contract_addr: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(),
                user: "0x940ACd9375b46EC2FA7C0E8aAd9D7241fb01e205"
                    .parse()
                    .unwrap(),
            }])
            .call()
            .await
            .unwrap();
        println!("{res:?}");
    }
}
