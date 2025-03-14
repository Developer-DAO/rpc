use super::types::Claims;
use crate::database::types::{Asset, Chain};
use crate::database::types::{Payments, RELATIONAL_DATABASE};
use crate::eth_rpc::types::{ETHEREUM_ENDPOINT, TESTING_ENDPOINT};
use alloy::eips::BlockId;
use alloy::primitives::ruint::ParseError;
use alloy::primitives::utils::{UnitsError, parse_units};
use alloy::rpc::types::{BlockTransactionsKind, Transaction, TransactionReceipt};
use alloy::transports::{RpcError, TransportErrorKind};
use alloy::{
    network::ReceiptResponse,
    primitives::{Address, Bytes, FixedBytes, U256, address, hex, utils::format_units},
    providers::{Provider, ProviderBuilder},
    sol,
    sol_types::SolCall,
};
use axum::Extension;
use axum::http::StatusCode;
use axum::{Json, response::IntoResponse};
use jwt_simple::claims::JWTClaims;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::types::time::OffsetDateTime;
use std::num::ParseFloatError;
use std::sync::OnceLock;
use std::{collections::HashMap, sync::LazyLock};
use thiserror::Error;
use tokio::task::JoinError;
use tracing::info;

// const TOKENS_SUPPORTED: [&str; 8] = [
//     "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
//     "0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
//     "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
//     "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
//     "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619",
//     "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
//     "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
//     "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
// ];

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

static WALLET: Address = address!("0b2C639c533813f4Aa9D7837CAf62653d097Ff85");

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

static TEST_TOKEN: OnceLock<HashMap<Address, TokenDetails>> = OnceLock::new();

static TOKENS: LazyLock<HashMap<Address, TokenDetails>> = LazyLock::new(|| {
    let mut map = HashMap::new();
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Months(pub u8);

#[derive(Serialize, Deserialize, Debug)]
pub struct EthereumPayment {
    pub chain: Chain,
    pub hash: String,
}

pub async fn process_ethereum_payment(
    Extension(jwt): Extension<JWTClaims<Claims>>,
    Json(payload): Json<EthereumPayment>,
) -> Result<impl IntoResponse, PaymentError> {
    let endpoint = if cfg!(test) {
        TESTING_ENDPOINT.get().unwrap()
    } else {
        ETHEREUM_ENDPOINT
            .iter()
            .find(|e| match (e, payload.chain) {
                (crate::eth_rpc::types::InternalEndpoints::Optimism(_), Chain::Optimism) => true,
                (crate::eth_rpc::types::InternalEndpoints::Arbitrum(_), Chain::Arbitrum) => true,
                (crate::eth_rpc::types::InternalEndpoints::Polygon(_), Chain::Polygon) => true,
                (crate::eth_rpc::types::InternalEndpoints::Base(_), Chain::Base) => true,
                _ => false,
            })
            .ok_or_else(|| PaymentError::InvalidNetwork)?
            .as_str()
    };

    let hash = hex::decode(&payload.hash)?;
    let mut fixed = [0u8; 32];
    fixed.copy_from_slice(&hash);

    let res: tokio::task::JoinHandle<Result<Transaction, PaymentError>> = {
        tokio::spawn(async move {
            let eth = reqwest::Url::parse(&endpoint).unwrap();
            let provider = ProviderBuilder::new().on_http(eth);
            provider
                .get_transaction_by_hash(FixedBytes::from(&fixed))
                .await?
                .ok_or_else(|| PaymentError::TxNotFound)
        })
    };

    let receipt: tokio::task::JoinHandle<Result<TransactionReceipt, PaymentError>> = {
        tokio::spawn(async move {
            let eth = reqwest::Url::parse(&endpoint).unwrap();
            let provider = ProviderBuilder::new().on_http(eth);
            provider
                .get_transaction_receipt(FixedBytes::from(&fixed))
                .await?
                .ok_or_else(|| PaymentError::TxNotFound)
        })
    };

    let last_safe_block: tokio::task::JoinHandle<Result<u64, PaymentError>> = {
        tokio::spawn(async move {
            let eth = reqwest::Url::parse(&endpoint).unwrap();
            let provider = ProviderBuilder::new().on_http(eth);
            provider
                .get_block(BlockId::safe(), BlockTransactionsKind::Full)
                .await?
                .ok_or_else(|| PaymentError::TxNotFinalized)?
                .header
                .number
                .ok_or_else(|| PaymentError::TxNotFinalized)
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

    let res: &Transaction = &res??;

    // todo: uncomment once SIWE is implemented
    // assert!(jwt.custom.wallet.is_some_and(|e| res.from == e));

    let payment: Payments = match res.input == Bytes::new() {
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
            let decoded = if let Ok(d) = ERC20::transferCall::abi_decode(&res.input, true) {
                Transfer::Transfer(d)
            } else if let Ok(tf) = ERC20::transferFromCall::abi_decode(&res.input, true) {
                Transfer::TransferFrom(tf)
            } else {
                Err(PaymentError::AbiDecodingError)?
            };
            let token_address = res.to.ok_or_else(|| PaymentError::UnsupportedToken)?;
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

            let token = if cfg!(test) {
                TEST_TOKEN
                    .get()
                    .unwrap()
                    .get(&token_address)
                    .ok_or_else(|| PaymentError::UnsupportedToken)?
            } else {
                TOKENS
                    .get(&token_address)
                    .ok_or_else(|| PaymentError::UnsupportedToken)?
            };

            match token.asset {
                Asset::Ether => Err(PaymentError::UnsupportedToken)?,
                Asset::USDC => {}
            }

            println!("Calculating credits from token ...");
            let value = calculate_token_value(token.asset, amount, token.decimals).await?;
            println!("Raw Value {}", value);

            info!("USD amount paid: {}", value,);

            let usd_value: String = format_units(value, token.decimals)?;
            println!("USD_VALUE {}", usd_value);
            let usd_value = (usd_value.parse::<f64>()? * 100.0) as i64;

            Payments {
                customer_email: jwt.custom.email,
                transaction_hash: hex::encode(hash),
                asset: token.asset,
                amount: amount.to_string(),
                chain: payload.chain,
                date: OffsetDateTime::now_utc(),
                decimals: token.decimals as i8,
                usd_value,
            }
        }
    };

    credit_account(&payment.customer_email, payment.usd_value).await?;
    insert_payment(&payment).await?;
    println!("End of function");
    Ok((
        StatusCode::OK,
        json!({"paid": payment.usd_value}).to_string(),
    )
        .into_response())
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

async fn calculate_token_value(
    asset: Asset,
    amount: U256,
    decimals: u8,
) -> Result<U256, PaymentError> {
    let price: PriceData = reqwest::Client::new()
        .get(format!(
            "https://api.coinbase.com/v2/prices/{}-USD/spot",
            asset,
        ))
        .send()
        .await?
        .json::<PriceData>()
        .await?;
    let usd_price_repr = parse_units(&price.data.amount, decimals)?.get_absolute();
    let value = (usd_price_repr * amount) / U256::from(10).pow(U256::from(decimals));
    Ok(value)
}

async fn insert_payment(payment: &Payments) -> Result<(), PaymentError> {
    // append only -- isolated
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    sqlx::query!(
        "INSERT INTO Payments(customerEmail, transactionHash, asset, amount, chain, date, decimals, usdValue) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        payment.customer_email,
        payment.transaction_hash,
        payment.asset as crate::database::types::Asset,
        payment.amount,
        payment.chain as crate::database::types::Chain,
        payment.date,
        payment.decimals as i32,
        payment.usd_value
        
    )
    .execute(db_connection)
    .await?;
    Ok(())
}

async fn credit_account(email: &str, amount: i64) -> Result<(), PaymentError> {
    // only updates account balance based on transaction
    let db_connection = RELATIONAL_DATABASE.get().unwrap();
    let transaction = db_connection.begin().await?;
    sqlx::query!(
        "UPDATE Customers SET balance = balance + $1 where email = $2",
        amount,
        email,
    )
    .execute(db_connection)
    .await?;
    transaction.commit().await?;

    Ok(())
}

//Error handling for submitPayment
#[derive(Error, Debug)]
pub enum PaymentError {
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
    InsufficientPayment,
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
    use std::time::Duration;

    use super::*;
    use alloy::{network::EthereumWallet, node_bindings::Anvil, signers::local::PrivateKeySigner};

    use crate::{
        Database, Email, JWTKey, TcpListener,
        database::types::RELATIONAL_DATABASE,
        middleware::jwt_auth::verify_jwt,
        register_user,
        routes::{
            activate::{ActivationRequest, activate_account},
            api_keys::generate_api_keys,
            login::LoginRequest,
            types::RegisterUser,
        },
        user_login,
    };
    use axum::{Router, middleware::from_fn, routing::post};
    use dotenvy::dotenv;

    #[tokio::test]
    async fn test_payment() {
        // todo: make test not flaky by deleting inserted data
        // in pg

        dotenv().unwrap();
        JWTKey::init().unwrap();
        Database::init().await.unwrap();
        Email::init().unwrap();
        let anvil = Anvil::new().block_time_f64(0.001).try_spawn().unwrap();
        let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
        let wallet = EthereumWallet::from(signer.clone());
        let rpc_url = anvil.endpoint().parse().unwrap();
        TESTING_ENDPOINT.get_or_init(|| anvil.endpoint().leak());
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url);

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

        let reg_res = reqwest::Client::new()
            .post("http://localhost:3072/api/register")
            .json(&RegisterUser {
                email: "0xe3024@gmail.com".to_string(),
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
            "0xe3024@gmail.com"
        )
        .fetch_one(RELATIONAL_DATABASE.get().unwrap())
        .await
        .unwrap();

        let ar = ActivationRequest {
            code: code.verificationcode,
            email: "0xe3024@gmail.com".to_string(),
        };

        reqwest::Client::new()
            .post("http://localhost:3072/api/activate")
            .json(&ar)
            .send()
            .await
            .unwrap();

        let lr = LoginRequest {
            email: "0xe3024@gmail.com".to_string(),
            password: "test".to_string(),
        };

        let ddrpc_client = reqwest::Client::builder()
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
            .json::<serde_json::Value>()
            .await
            .unwrap();
        println!("{res}");
        assert_eq!(res["paid"], 100000);

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
    }
}
