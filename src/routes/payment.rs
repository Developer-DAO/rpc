use super::types::{Claims, EmailAddress};
use crate::database::types::{Asset, Chain, Payments, Plan, RELATIONAL_DATABASE};
#[cfg(test)]
use crate::eth_rpc::types::TESTING_ENDPOINT;
use alloy::consensus::Transaction;
use alloy::eips::BlockId;
use alloy::primitives::ruint::ParseError;
use alloy::primitives::utils::{UnitsError, parse_units};
use alloy::rpc::types::TransactionReceipt;
use alloy::transports::{RpcError, TransportErrorKind};
use alloy::{
    network::ReceiptResponse,
    primitives::{Address, Bytes, FixedBytes, U256, address, hex, utils::format_units},
    providers::{Provider, ProviderBuilder},
    sol,
    sol_types::SolCall,
};
use axum::Extension;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use jwt_simple::claims::JWTClaims;
use serde::{Deserialize, Serialize};
use sqlx::types::time::OffsetDateTime;
use sqlx::types::Uuid;
#[cfg(test)]
use std::collections::HashMap;
use std::num::ParseFloatError;
use std::sync::LazyLock;
#[cfg(test)]
use std::sync::OnceLock;
use thiserror::Error;
use tokio::task::JoinError;
use tracing::info;

#[derive(Serialize, Deserialize, Debug)]
pub struct PriceData {
    data: AssetData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AssetData {
    pub amount: String,
    pub base: String,
    pub currency: String,
}

static WALLET: Address = address!("0x65C67Befc1AE667E538a588295070E5d5f478B2C");

#[derive(Debug, Clone, Copy)]
pub struct TokenDetails {
    pub decimals: u8,
    pub network: Chain,
    pub asset: Asset,
}

impl TokenDetails {
    pub fn new(decimals: u8, network: Chain, asset: Asset) -> TokenDetails {
        TokenDetails {
            decimals,
            network,
            asset,
        }
    }
}

#[cfg(test)]
static TEST_TOKEN: OnceLock<HashMap<Address, TokenDetails>> = OnceLock::new();
#[cfg(not(test))]
#[cfg(not(feature = "dev"))]
static TOKENS: std::sync::LazyLock<std::collections::HashMap<Address, TokenDetails>> =
    std::sync::LazyLock::new(|| {
        let mut map = std::collections::HashMap::new();
        map.insert(
            address!("af88d065e77c8cc2239327c5edb3a432268e5831"),
            TokenDetails::new(6, Chain::Arbitrum, Asset::USDC),
        );
        map.insert(
            address!("833589fcd6edb6e08f4c7c32d4f71b54bda02913"),
            TokenDetails::new(6, Chain::Base, Asset::USDC),
        );
        map.insert(
            address!("3c499c542cEF5E3811e1192ce70d8cC03d5c3359"),
            TokenDetails::new(6, Chain::Polygon, Asset::USDC),
        );
        map.insert(
            // USDC.e is the bridged version of USDC pre-circle native issuance
            address!("2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
            TokenDetails::new(6, Chain::Polygon, Asset::USDC),
        );
        map.insert(
            address!("0b2C639c533813f4Aa9D7837CAf62653d097Ff85"),
            TokenDetails::new(6, Chain::Optimism, Asset::USDC),
        );
        map
    });

sol! {
    #[sol(rpc, bytecode="608060405234801561000f575f80fd5b506040518060400160405280600781526020017f4d79546f6b656e000000000000000000000000000000000000000000000000008152506040518060400160405280600381526020017f4d544b0000000000000000000000000000000000000000000000000000000000815250816003908161008b919061059a565b50806004908161009b919061059a565b5050506100bd336e13426172c74d822b878fe8000000006100c260201b60201c565b61077e565b5f73ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff1603610132575f6040517fec442f0500000000000000000000000000000000000000000000000000000000815260040161012991906106a8565b60405180910390fd5b6101435f838361014760201b60201c565b5050565b5f73ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff1603610197578060025f82825461018b91906106ee565b92505081905550610265565b5f805f8573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f2054905081811015610220578381836040517fe450d38c00000000000000000000000000000000000000000000000000000000815260040161021793929190610730565b60405180910390fd5b8181035f808673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f2081905550505b5f73ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff16036102ac578060025f82825403925050819055506102f6565b805f808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f82825401925050819055505b8173ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040516103539190610765565b60405180910390a3505050565b5f81519050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52604160045260245ffd5b7f4e487b71000000000000000000000000000000000000000000000000000000005f52602260045260245ffd5b5f60028204905060018216806103db57607f821691505b6020821081036103ee576103ed610397565b5b50919050565b5f819050815f5260205f209050919050565b5f6020601f8301049050919050565b5f82821b905092915050565b5f600883026104507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff82610415565b61045a8683610415565b95508019841693508086168417925050509392505050565b5f819050919050565b5f819050919050565b5f61049e61049961049484610472565b61047b565b610472565b9050919050565b5f819050919050565b6104b783610484565b6104cb6104c3826104a5565b848454610421565b825550505050565b5f90565b6104df6104d3565b6104ea8184846104ae565b505050565b5b8181101561050d576105025f826104d7565b6001810190506104f0565b5050565b601f82111561055257610523816103f4565b61052c84610406565b8101602085101561053b578190505b61054f61054785610406565b8301826104ef565b50505b505050565b5f82821c905092915050565b5f6105725f1984600802610557565b1980831691505092915050565b5f61058a8383610563565b9150826002028217905092915050565b6105a382610360565b67ffffffffffffffff8111156105bc576105bb61036a565b5b6105c682546103c4565b6105d1828285610511565b5f60209050601f831160018114610602575f84156105f0578287015190505b6105fa858261057f565b865550610661565b601f198416610610866103f4565b5f5b8281101561063757848901518255600182019150602085019450602081019050610612565b868310156106545784890151610650601f891682610563565b8355505b6001600288020188555050505b505050505050565b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f61069282610669565b9050919050565b6106a281610688565b82525050565b5f6020820190506106bb5f830184610699565b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f6106f882610472565b915061070383610472565b925082820190508082111561071b5761071a6106c1565b5b92915050565b61072a81610472565b82525050565b5f6060820190506107435f830186610699565b6107506020830185610721565b61075d6040830184610721565b949350505050565b5f6020820190506107785f830184610721565b92915050565b610de18061078b5f395ff3fe608060405234801561000f575f80fd5b5060043610610091575f3560e01c8063313ce56711610064578063313ce5671461013157806370a082311461014f57806395d89b411461017f578063a9059cbb1461019d578063dd62ed3e146101cd57610091565b806306fdde0314610095578063095ea7b3146100b357806318160ddd146100e357806323b872dd14610101575b5f80fd5b61009d6101fd565b6040516100aa9190610a5a565b60405180910390f35b6100cd60048036038101906100c89190610b0b565b61028d565b6040516100da9190610b63565b60405180910390f35b6100eb6102af565b6040516100f89190610b8b565b60405180910390f35b61011b60048036038101906101169190610ba4565b6102b8565b6040516101289190610b63565b60405180910390f35b6101396102e6565b6040516101469190610c0f565b60405180910390f35b61016960048036038101906101649190610c28565b6102ee565b6040516101769190610b8b565b60405180910390f35b610187610333565b6040516101949190610a5a565b60405180910390f35b6101b760048036038101906101b29190610b0b565b6103c3565b6040516101c49190610b63565b60405180910390f35b6101e760048036038101906101e29190610c53565b6103e5565b6040516101f49190610b8b565b60405180910390f35b60606003805461020c90610cbe565b80601f016020809104026020016040519081016040528092919081815260200182805461023890610cbe565b80156102835780601f1061025a57610100808354040283529160200191610283565b820191905f5260205f20905b81548152906001019060200180831161026657829003601f168201915b5050505050905090565b5f80610297610467565b90506102a481858561046e565b600191505092915050565b5f600254905090565b5f806102c2610467565b90506102cf858285610480565b6102da858585610512565b60019150509392505050565b5f6012905090565b5f805f8373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f20549050919050565b60606004805461034290610cbe565b80601f016020809104026020016040519081016040528092919081815260200182805461036e90610cbe565b80156103b95780601f10610390576101008083540402835291602001916103b9565b820191905f5260205f20905b81548152906001019060200180831161039c57829003601f168201915b5050505050905090565b5f806103cd610467565b90506103da818585610512565b600191505092915050565b5f60015f8473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f8373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f2054905092915050565b5f33905090565b61047b8383836001610602565b505050565b5f61048b84846103e5565b90507fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff811461050c57818110156104fd578281836040517ffb8f41b20000000000000000000000000000000000000000000000000000000081526004016104f493929190610cfd565b60405180910390fd5b61050b84848484035f610602565b5b50505050565b5f73ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff1603610582575f6040517f96c6fd1e0000000000000000000000000000000000000000000000000000000081526004016105799190610d32565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff16036105f2575f6040517fec442f050000000000000000000000000000000000000000000000000000000081526004016105e99190610d32565b60405180910390fd5b6105fd8383836107d1565b505050565b5f73ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff1603610672575f6040517fe602df050000000000000000000000000000000000000000000000000000000081526004016106699190610d32565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff16036106e2575f6040517f94280d620000000000000000000000000000000000000000000000000000000081526004016106d99190610d32565b60405180910390fd5b8160015f8673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f8573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f208190555080156107cb578273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff167f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925846040516107c29190610b8b565b60405180910390a35b50505050565b5f73ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff1603610821578060025f8282546108159190610d78565b925050819055506108ef565b5f805f8573ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f20549050818110156108aa578381836040517fe450d38c0000000000000000000000000000000000000000000000000000000081526004016108a193929190610cfd565b60405180910390fd5b8181035f808673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f2081905550505b5f73ffffffffffffffffffffffffffffffffffffffff168273ffffffffffffffffffffffffffffffffffffffff1603610936578060025f8282540392505081905550610980565b805f808473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f82825401925050819055505b8173ffffffffffffffffffffffffffffffffffffffff168373ffffffffffffffffffffffffffffffffffffffff167fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef836040516109dd9190610b8b565b60405180910390a3505050565b5f81519050919050565b5f82825260208201905092915050565b8281835e5f83830152505050565b5f601f19601f8301169050919050565b5f610a2c826109ea565b610a3681856109f4565b9350610a46818560208601610a04565b610a4f81610a12565b840191505092915050565b5f6020820190508181035f830152610a728184610a22565b905092915050565b5f80fd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f610aa782610a7e565b9050919050565b610ab781610a9d565b8114610ac1575f80fd5b50565b5f81359050610ad281610aae565b92915050565b5f819050919050565b610aea81610ad8565b8114610af4575f80fd5b50565b5f81359050610b0581610ae1565b92915050565b5f8060408385031215610b2157610b20610a7a565b5b5f610b2e85828601610ac4565b9250506020610b3f85828601610af7565b9150509250929050565b5f8115159050919050565b610b5d81610b49565b82525050565b5f602082019050610b765f830184610b54565b92915050565b610b8581610ad8565b82525050565b5f602082019050610b9e5f830184610b7c565b92915050565b5f805f60608486031215610bbb57610bba610a7a565b5b5f610bc886828701610ac4565b9350506020610bd986828701610ac4565b9250506040610bea86828701610af7565b9150509250925092565b5f60ff82169050919050565b610c0981610bf4565b82525050565b5f602082019050610c225f830184610c00565b92915050565b5f60208284031215610c3d57610c3c610a7a565b5b5f610c4a84828501610ac4565b91505092915050565b5f8060408385031215610c6957610c68610a7a565b5b5f610c7685828601610ac4565b9250506020610c8785828601610ac4565b9150509250929050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52602260045260245ffd5b5f6002820490506001821680610cd557607f821691505b602082108103610ce857610ce7610c91565b5b50919050565b610cf781610a9d565b82525050565b5f606082019050610d105f830186610cee565b610d1d6020830185610b7c565b610d2a6040830184610b7c565b949350505050565b5f602082019050610d455f830184610cee565b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f610d8282610ad8565b9150610d8d83610ad8565b9250828201905080821115610da557610da4610d4b565b5b9291505056fea2646970667358221220a3c84ce57f4a6659703f00784344c7abe2aadadd6dd2e165fdb9cc4af220202264736f6c634300081a0033")]
    contract ERC20 {
        #[allow(missing_docs)]
        function transfer(
            address to,
            uint256 amount
        ) public returns (bool success);

        #[allow(missing_docs)]
        function transferFrom(
            address from,
            address to,
            uint256 amount
        ) public returns (bool success);
    }
}

pub enum Transfer {
    Transfer(ERC20::transferCall),
    TransferFrom(ERC20::transferFromCall),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EthereumPayment {
    pub chain: Chain,
    pub hash: String,
    pub plan: Option<Plan>,
}

pub struct Balances {
    pub balance: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Upgrade {
    plan: Plan
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Downgrade {
    plan: Plan
}

#[derive(Deserialize)]
pub struct Pagination {
    page: usize,
    per_page: usize,
}

#[derive(Debug, Serialize)]
pub struct UserBalances {
    calls: i64,
    balance: i64,
}

pub async fn get_calls_and_balance<'a>(
    Extension(jwt): Extension<JWTClaims<Claims<'a>>>,
) -> Result<impl IntoResponse, PaymentError> {
    let res = sqlx::query_as!(
        UserBalances,
        "SELECT calls, balance 
        FROM Customers, RpcPlans 
        where Customers.email = $1 
        AND 
        RpcPlans.email = $1",
        jwt.custom.email.as_str()
    )
    .fetch_one(RELATIONAL_DATABASE.get().unwrap())
    .await?;

    Ok((StatusCode::OK, serde_json::to_string(&res)?).into_response())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentData<'a> {
    pub customeremail: EmailAddress<'a>,
    pub transactionhash: String,
    pub asset: Asset,
    pub amount: String,
    pub chain: Chain,
    pub date: i64,
    pub usdvalue: i64,
    pub decimals: i32,
}

pub async fn get_payments<'a>(
    Query(params): Query<Pagination>,
    Extension(jwt): Extension<JWTClaims<Claims<'a>>>,
) -> Result<impl IntoResponse, PaymentError> {
    let res: Vec<Payments> = sqlx::query_as!(
        Payments,
        r#"SELECT customerEmail, transactionHash, asset as "asset!: Asset",
        amount, decimals, chain as "chain!: Chain", date, usdValue
        FROM Payments WHERE customerEmail = $1 
        LIMIT $2 
        OFFSET $3
        "#,
        jwt.custom.email.as_str(),
        params.per_page as i64,
        params.page as i64,
    )
    .fetch_all(RELATIONAL_DATABASE.get().unwrap())
    .await?;

    let res: Vec<PaymentData> = res
        .into_iter()
        .map(|e| PaymentData {
            customeremail: e.customeremail,
            transactionhash: e.transactionhash,
            asset: e.asset,
            amount: e.amount,
            date: e.date.unix_timestamp(),
            chain: e.chain,
            usdvalue: e.usdvalue,
            decimals: e.decimals,
        })
        .collect();

    Ok((StatusCode::OK, serde_json::to_string(&res)?).into_response())
}

pub struct Cancel { 
    pub id: Uuid,
}

/// cancels a user's subscription
pub async fn cancel(
    Extension(jwt): Extension<JWTClaims<Claims<'_>>>,
) -> Result<impl IntoResponse, PaymentError> {

    let mut tx = RELATIONAL_DATABASE.get().unwrap().begin().await?;

    sqlx::query!(
        r#"
        UPDATE RpcPlans
        SET 
        downgradeto = 'free'
        WHERE 
            $1 = email 
        "#,
        jwt.custom.email.as_str()
    )
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok((StatusCode::OK, "Plan succesfully cancelled").into_response())
}

pub struct RpcPlan {
    pub email: String,
    pub calls: i64,
    pub plan: Plan,
    pub created: OffsetDateTime,
    pub expires: OffsetDateTime,
    pub downgradeto: Option<Plan>
}

/// Downgrades the service tier for active plan next cycle
pub async fn downgrade(
    Extension(jwt): Extension<JWTClaims<Claims<'_>>>,
    Json(payload): Json<Downgrade>,
) -> Result<impl IntoResponse, PaymentError> {
    let mut tx = RELATIONAL_DATABASE.get().unwrap().begin().await?;
    // get user plans
    let plan = sqlx::query_as!(RpcPlan, 
        r#"SELECT email, calls, created, expires, plan as "plan!: Plan", downgradeto as "downgradeto!: Plan" FROM RpcPlans 
        WHERE $1 = email 
        "#,  
        jwt.custom.email.as_str(),
    )
        .fetch_one(&mut *tx)
        .await?;

    if payload.plan >= plan.plan || matches!(plan.plan, Plan::Free) {
        return Ok((StatusCode::FORBIDDEN, "Not a downgrade").into_response())
    }

    // update tier for next cycle
    sqlx::query!( 
        r#"UPDATE RpcPlans set downgradeTo = $1 
        WHERE $2 = email"#,  
        payload.plan as Plan,
        jwt.custom.email.as_str()
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::OK, "Downgrade successful").into_response())
}


pub async fn upgrade(
    Extension(jwt): Extension<JWTClaims<Claims<'_>>>,
    Json(payload): Json<Upgrade>,
) -> Result<impl IntoResponse, PaymentError> {
    let mut tx = RELATIONAL_DATABASE.get().unwrap().begin().await?;
    // get user plan
    let plan = sqlx::query_as!(RpcPlan, 
        r#"SELECT email, calls, created, expires, plan as "plan!: Plan", downgradeto as "downgradeto!: Plan" FROM RpcPlans 
        WHERE $1 = email 
        "#,  
        jwt.custom.email.as_str()
    )
        .fetch_one(&mut *tx)
        .await?;

    if payload.plan <= plan.plan {
        return Ok((StatusCode::FORBIDDEN, "Not an upgrade").into_response())
    }

    // cost of new plan - cost of old plan
    let total_cost = (payload.plan.get_cost() - plan.plan.get_cost()) as i64  * 100;

    // deduct balance from account
    let mut tx = RELATIONAL_DATABASE.get().unwrap().begin().await?;
    let r = sqlx::query!(
        "UPDATE Customers SET balance = balance - $1 WHERE Customers.email = $2",
        total_cost,
        jwt.custom.email.as_str()
    )
    .execute(&mut *tx)
    .await;

    if let Err(e) = r {
        match e.as_database_error() {
            Some(db) => { 
                // there is a check on user balances that asserts a non-zero balance, if this
                // check fails then the DB will not allow the write
                // allows for a fairly optimistic system & cuts out 1 round trip 
                if db.is_check_violation() {
                    return Ok((StatusCode::PAYMENT_REQUIRED, "Insufficient funds for plan").into_response());
                }
            },
            None => Err(e)?,
        }
    }

    // plan gets written to DB row
    // calls is set to 0 because we prorated the total number of calls made by the user previously
    sqlx::query!(
        r#"UPDATE RpcPlans SET plan = $1, calls = 0, downgradeto=NULL WHERE email = $2"#,
        payload.plan as Plan,
        jwt.custom.email.as_str(),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::OK, "Successfully applied payment").into_response())
}

pub static D_D_CLOUD_API_KEY: LazyLock<&'static str> = LazyLock::new(|| {
    dotenvy::var("D_D_CLOUD_API_KEY").unwrap().leak()
});

#[tracing::instrument]
pub async fn process_ethereum_payment(
    Extension(jwt): Extension<JWTClaims<Claims<'_>>>,
    Json(payload): Json<EthereumPayment>,
) -> Result<impl IntoResponse, PaymentError> {
    #[cfg(not(test))]
    #[cfg(not(feature = "dev"))]
    let _endpoint: String = format!("https://api.cloud.developerdao.com/rpc/{}/{}", payload.chain.pokt_id(), *D_D_CLOUD_API_KEY);

    #[cfg(test)]
    let _endpoint: &'static str = TESTING_ENDPOINT.get().unwrap();

    #[cfg(feature = "dev")]
    let _endpoint: &'static str = {
        matches!(payload.chain, Chain::Sepolia)
            .then(|| ())
            .ok_or_else(|| PaymentError::InvalidNetwork)?;
        dotenvy::var("SEPOLIA_PROVIDER").unwrap().leak()
    };

    let hash = hex::decode(&payload.hash)?;
    let mut fixed = [0u8; 32];
    fixed.copy_from_slice(&hash);

    #[allow(clippy::needless_borrow)]
    let eth = reqwest::Url::parse(&_endpoint).unwrap();
    let provider = ProviderBuilder::new().connect_http(eth);

    let p1 = provider.clone();
    let res = tokio::spawn(async move {
            p1 
                .get_transaction_by_hash(FixedBytes::from(&fixed))
                .await?
                .ok_or_else(|| PaymentError::TxNotFound)
        });
    
    let p2 = provider.clone();
    let receipt: tokio::task::JoinHandle<Result<TransactionReceipt, PaymentError>> = {
        tokio::spawn(async move {
            p2
                .get_transaction_receipt(FixedBytes::from(&fixed))
                .await?
                .ok_or_else(|| PaymentError::TxNotFound)
        })
    };

    let p3 = provider.clone();
    let last_safe_block: tokio::task::JoinHandle<Result<u64, PaymentError>> = {
        tokio::spawn(async move {
            Ok(p3
                .get_block(BlockId::safe())
                .await?
                .ok_or_else(|| PaymentError::TxNotFinalized)?
                .header
                .number)
        })
    };

    let (res, receipt, last_safe_block) = tokio::join!(res, receipt, last_safe_block);

    let tx = receipt??;

    if !tx.status() {
        Err(PaymentError::TxFailed)?
    }

    if tx.block_number().is_none() {
        Err(PaymentError::TxNotFinalized)?
    }

    if tx.block_number().unwrap() >= last_safe_block?? {
        Err(PaymentError::TxNotFinalized)?
    }

    let res = &res??;

    if jwt.custom.wallet.is_none_or(|e| res.inner.signer() != e) {
        Err(PaymentError::SenderWalletMismatch)?
    }

    let payment: Payments = match res.inner.input() == &Bytes::new() {
        // ether
        true => {
            Err(PaymentError::UnsupportedToken)?
            // if res.from != jwt.custom.wallet {
            //     Err(PaymentError::AddressMismatch)?
            // }
            // let to = res.to.ok_or_else(|| PaymentError::NoDestination)?;
            // if to != WALLET {
            //     Err(PaymentError::IncorrectRecipient)?
            // }
            // let value: U256 = calculate_eth_value(res.value).await?;
            // info!("USD amount paid: {}", value,);
            // let usd_value: String = format_units(value, 18)?;
            // let usd_value = (usd_value.parse::<f64>()? * 100.0) as i64;
            //     Payments {
            //         customer_email: jwt.custom.email,
            //         transaction_hash: hex::encode(hash),
            //         asset: Asset::Ether,
            //         amount: format_ether(res.value),
            //         chain: payload.chain,
            //         date: OffsetDateTime::now_utc(),
            //         usd_value,
            //         decimals: 18
            //     }
        }
        // token handling
        false => {
            let decoded = if let Ok(d) = ERC20::transferCall::abi_decode(res.input()) {
                Transfer::Transfer(d)
            } else if let Ok(tf) = ERC20::transferFromCall::abi_decode(res.input()) {
                Transfer::TransferFrom(tf)
            } else {
                Err(PaymentError::AbiDecodingError)?
            };

            #[cfg(not(feature = "dev"))]
            let _token_address = res.inner.to().ok_or_else(|| PaymentError::UnsupportedToken)?;

            let (amount, to) = match decoded {
                Transfer::Transfer(tx) => (tx.amount, tx.to),
                Transfer::TransferFrom(tx) => match jwt.custom.wallet {
                    Some(wallet) => {
                        if tx.from != wallet {
                            Err(PaymentError::AddressMismatch)?
                        }
                        (tx.amount, tx.to)
                    }
                    None => Err(PaymentError::AddressMismatch)?,
                },
            };

            if to != WALLET {
                Err(PaymentError::IncorrectRecipient)?
            }

            #[cfg(feature = "dev")]
            let _token_address = Address::new([0u8; 20]);

            #[cfg(not(test))]
            #[cfg(not(feature = "dev"))]
            let _token = *TOKENS
                .get(&_token_address)
                .ok_or_else(|| PaymentError::UnsupportedToken)?;

            #[cfg(test)]
            let _token = *TEST_TOKEN
                .get()
                .unwrap()
                .get(&_token_address)
                .ok_or_else(|| PaymentError::UnsupportedToken)?;

            #[cfg(feature = "dev")]
            let _token = TokenDetails {
                decimals: 18,
                network: Chain::Sepolia,
                asset: Asset::USDC,
            };

            match _token.asset {
                Asset::Ether => Err(PaymentError::UnsupportedToken)?,
                Asset::USDC => {}
            }

            let value = stablecoin_fixedpoint_conversion(amount, _token.decimals).await?;

            info!("USD amount paid: {value}");

            let usdvalue: String = format_units(value, _token.decimals)?;
            println!("USD_VALUE {usdvalue}");
            let usdvalue = (usdvalue.parse::<f64>()? * 100.0) as i64;

            Payments {
                customeremail: jwt.custom.email,
                transactionhash: hex::encode(hash),
                asset: _token.asset,
                amount: amount.to_string(),
                chain: payload.chain,
                date: OffsetDateTime::now_utc(),
                decimals: _token.decimals as i32,
                usdvalue,
            }
        }
    };

    credit_account(&payment.customeremail, payment.usdvalue, payload.plan).await?;
    insert_payment(&payment).await?;

    Ok((StatusCode::OK, payment.usdvalue.to_string()).into_response())
}

// async fn calculate_eth_value(eth_amount: U256) -> Result<U256, PaymentError> {
//     let price: PriceData = reqwest::Client::new()
//         .get("https://api.coinbase.com/v2/prices/ETH-USD/spot")
//         .send()
//         .await?
//         .json::<PriceData>()
//         .await?;
//     println!("USD Price of Assets as string: {}", price.data.amount);
//     let usd_price_ether_repr = parse_units(&price.data.amount, 18)?.get_absolute();
//     let value = (usd_price_ether_repr * eth_amount) / U256::from(10).pow(U256::from(18));
//     Ok(value)
// }

async fn stablecoin_fixedpoint_conversion(
    amount: U256,
    decimals: u8,
) -> Result<U256, PaymentError> {
    // let price: PriceData = reqwest::Client::new()
    //     .get(format!(
    //         "https://api.coinbase.com/v2/prices/{asset}-USD/spot"
    //     ))
    //     .send()
    //     .await?
    //     .json::<PriceData>()
    //     .await?;
    let price = 1.00;
    let usd_price_repr = parse_units(&price.to_string(), decimals)?.get_absolute();
    let value = (usd_price_repr * amount) / U256::from(10).pow(U256::from(decimals));
    Ok(value)
}

async fn insert_payment(payment: &Payments<'_>) -> Result<(), PaymentError> {
    // append only -- isolated
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    sqlx::query!(
        "INSERT INTO Payments(customerEmail, transactionHash, asset, amount, chain, decimals, usdValue) 
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        payment.customeremail.as_str(),
        payment.transactionhash,
        payment.asset as crate::database::types::Asset,
        payment.amount,
        payment.chain as crate::database::types::Chain,
        payment.decimals as i32,
        payment.usdvalue
        
    )
    .execute(db_connection)
    .await?;
    Ok(())
}

async fn credit_account(email: &EmailAddress<'_>, mut amount: i64, plan: Option<Plan>) -> Result<(), PaymentError> {
    // only updates account balance based on transaction
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let mut transaction = db_connection.begin().await?;

    if let Some(plan) = plan && plan.get_cost() <= amount as f64 / 100.0
    {
        amount -= (plan.get_cost() * 100.0) as i64;
        
        sqlx::query!(
            "UPDATE RpcPlans SET plan = $1 where email = $2",
            plan as Plan,
            email.as_str(),
        )
        .execute(&mut *transaction)
        .await?;

    }

    sqlx::query!(
        "UPDATE Customers SET balance = balance + $1 where email = $2",
        amount,
        email.as_str(),
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(())
}

//Error handling for submitPayment
#[derive(Error, Debug)]
pub enum PaymentError {
    #[error("Balance is zero or negative")]
    ZeroBalance,
    #[error(
        "An error occured while adjusting expiry dates. Please try again and please notify us if this occurs again."
    )]
    OverflowExpiry,
    #[error(
        "The sender of the transaction does not match the account's wallet on file. Please change the wallet associated with your account if this is in error."
    )]
    SenderWalletMismatch,
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    UnitsError(#[from] UnitsError),
    #[error(
        "Failed to decode the transaction's calldata into ERC20::Transfer or ERC20::TransferFrom"
)]
    AbiDecodingError,
    #[error("No destination for tx")]
    NoDestination,
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("Transaction failed")]
    TxFailed,
    #[error(transparent)]
    RpcError(#[from] RpcError<TransportErrorKind>),
    #[error(transparent)]
    JoinError(#[from] JoinError),
    #[error(transparent)]
    PriceFetchError(#[from] reqwest::Error),
    #[error(transparent)]
    HexError(#[from] hex::FromHexError),
    #[error("Tx destination not valid")]
    AddressMismatch,
    #[error("Value not sent to any of our wallets")]
    IncorrectRecipient,
    #[error("Token is not supported")]
    UnsupportedToken,
    #[error("Tx not finalized")]
    TxNotFinalized,
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
    #[error("Transaction not found")]
    TxNotFound,
    #[error("Insufficient payment for plan and duration specified in call")]
    InsufficientFunds,
    #[error("Invalid duration, must be greater than 0")]
    InvalidDuration,
    #[error("Network is not currently supported")]
    InvalidNetwork,
}

impl IntoResponse for PaymentError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Database, EmailLogin, JWTKey, TcpListener,
        database::types::RELATIONAL_DATABASE,
        middleware::jwt_auth::verify_jwt,
        register_user,
        routes::{
            activate::{ActivationRequest, activate_account},
            api_keys::generate_api_keys,
            login::LoginRequest,
            types::{EmailAddress, Password, RegisterUser},
        },
        user_login,
    };
    use alloy::{network::EthereumWallet, node_bindings::Anvil, signers::local::PrivateKeySigner};
    use axum::{Router, middleware::from_fn, routing::post};
    use dotenvy::dotenv;
    use std::time::Duration;

    #[tokio::test]
    async fn test_payment() {
        let _ = dotenv();
        JWTKey::init().unwrap();
        Database::init().await.unwrap();
        EmailLogin::init().unwrap();
        let anvil = Anvil::new().block_time_f64(0.001).try_spawn().unwrap();
        let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
        let wallet = EthereumWallet::from(signer.clone());
        let rpc_url = anvil.endpoint().parse().unwrap();
        TESTING_ENDPOINT.get_or_init(|| anvil.endpoint().leak());
        let provider = ProviderBuilder::new()
            .wallet(wallet)
            .connect_http(rpc_url);

        // let eth_tx = provider
        //     .transaction_request()
        //     .with_value(U256::from(1000000000000000000u128))
        //     .with_to(WALLET);
        //
        // let eth_tx_hash = provider
        //     .send_transaction(eth_tx)
        //     .await
        //     .unwrap()
        //     .with_required_confirmations(24)
        //     .watch()
        //     .await
        //     .unwrap();
        // println!("eth tx hash: {}", &eth_tx_hash);

        let contract = ERC20::deploy(&provider).await.unwrap();
        let addy = *contract.address();
        println!("Contract address: {addy}");
        TEST_TOKEN.get_or_init(|| {
            let mut map = HashMap::new();
            map.insert(addy, TokenDetails::new(18, Chain::Optimism, Asset::USDC));
            map
        });
        let usdc_tx_hash = contract
            .transfer(WALLET, U256::from(1000u128 * (10u128.pow(18u32))))
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap()
            .transaction_hash;

        std::thread::sleep(Duration::new(1, 0));

        tokio::spawn(async move {
            let app = Router::new()
                .route("/api/register", post(register_user))
                .route("/api/activate", post(activate_account))
                .route(
                    "/api/pay",
                    post(process_ethereum_payment).route_layer(from_fn(verify_jwt)),
                )
                .route("/api/login", post(user_login))
                .route(
                    "/api/keys",
                    post(generate_api_keys).route_layer(from_fn(verify_jwt)),
                );
            let listener = TcpListener::bind("0.0.0.0:3072").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });

        let _reg_res = reqwest::Client::new()
            .post("http://localhost:3072/api/register")
            .json(&RegisterUser {
                email: "cloud@developerdao.com".to_string(),
                password: "test".to_string(),
            })
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        // assert_eq!(reg_res, "User was successfully registered");

        pub struct Code {
            verificationcode: String,
        }

        let code = sqlx::query_as!(
            Code,
            "SELECT verificationCode FROM Customers WHERE email = $1",
            "cloud@developerdao.com"
        )
        .fetch_one(RELATIONAL_DATABASE.get().unwrap())
        .await
        .unwrap();

        let ar = ActivationRequest {
            code: code.verificationcode,
            email: "cloud@developerdao.com".to_string(),
        };
        println!("Signer Address: {}", signer.address());
        sqlx::query!(
            "UPDATE Customers SET wallet = $1 where email = $2",
            signer.address().to_string(),
            "cloud@developerdao.com"
        )
        .execute(RELATIONAL_DATABASE.get().unwrap())
        .await
        .unwrap();

        reqwest::Client::new()
            .post("http://localhost:3072/api/activate")
            .json(&ar)
            .send()
            .await
            .unwrap();

        let lr = LoginRequest {
            email: EmailAddress("cloud@developerdao.com".into()),
            password: Password("test".into()),
        };

        let ddrpc_client = reqwest::Client::builder()
            .use_rustls_tls()
            .cookie_store(true)
            .build()
            .unwrap();

        ddrpc_client
            .post("http://localhost:3072/api/login")
            .json(&lr)
            .send()
            .await
            .unwrap();

        let usdc_payment = EthereumPayment {
            chain: Chain::Anvil,
            hash: usdc_tx_hash.to_string(),
            plan: None,
        };

        // let eth_payment = EthereumPayment {
        //     chain: Chain::Anvil,
        //     hash: eth_tx_hash.to_string(),
        // };

        let res = ddrpc_client
            .post("http://localhost:3072/api/pay")
            .json(&usdc_payment)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        println!("{res}");
        assert_eq!(res.parse::<i64>().unwrap(), 100000);

        // let res = ddrpc_client
        //     .post("http://localhost:3072/api/pay")
        //     .json(&eth_payment)
        //     .send()
        //     .await
        //     .unwrap()
        //     .text()
        //     .await
        //     .unwrap();
        //
        // println!("Credits from payment: {:?}", res);
        //
        // assert!(res.parse::<i64>().unwrap() > 0);

        sqlx::query!(
            "DELETE FROM Customers WHERE email = $1",
            "cloud@developerdao.com"
        )
        .execute(RELATIONAL_DATABASE.get().unwrap())
        .await
        .unwrap();

        sqlx::query!(
            "DELETE FROM Payments WHERE customerEmail = $1",
            "cloud@developerdao.com"
        )
        .execute(RELATIONAL_DATABASE.get().unwrap())
        .await
        .unwrap();

        sqlx::query!(
            "DELETE FROM RpcPlans WHERE email = $1",
            "cloud@developerdao.com"
        ) 
        .execute(RELATIONAL_DATABASE.get().unwrap())
        .await
        .unwrap();
    }
}
