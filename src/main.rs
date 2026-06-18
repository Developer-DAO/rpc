use crate::middleware::ingress_limits::body_size_limiter;
use crate::middleware::jwt_auth::verify_jwt;
#[cfg(not(feature = "dev"))]
use crate::middleware::rpc_service::validate_subscription_and_update_user_calls;
use crate::routes::payment::{cancel, downgrade, upgrade};
use crate::routes::relayer::router::{
    route_arb, route_base, route_bsc, route_eth, route_op, route_poly, route_solana, route_sui,
};
use crate::routes::relayer::websockets::ws_handler;
use crate::routes::token_queries::{
    aggregate_balances, aggregate_single_token_bals, aggregate_token_bals_for_user,
    get_batch_nft_info,
};
use crate::routes::types::{EmailLogin, JWTKey};
use crate::routes::{
    activate::activate_account,
    api_keys::{delete_key, generate_api_keys, get_all_api_keys},
    login::user_login,
    recovery::{recover_password_email, update_password},
    register::register_user,
};
use axum::extract::DefaultBodyLimit;
use axum::http::HeaderValue;
use axum::http::Method;
use axum::routing::delete;
use axum::{
    Router,
    http::{StatusCode, header},
    middleware::from_fn,
    response::IntoResponse,
    routing::{get, post},
};
use database::types::Database;
use mimalloc::MiMalloc;
use routes::login::{refresh, user_login_siwe};
use routes::payment::{get_calls_and_balance, get_payments, process_ethereum_payment};
use routes::siwe::{get_siwe_nonce, jwt_get_siwe_nonce, siwe_add_wallet};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

pub mod database;
pub mod eth_rpc;
pub mod middleware;
pub mod routes;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() {
    JWTKey::init().unwrap();
    Database::init().await.unwrap();
    EmailLogin::init().unwrap();

    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let origin = if cfg!(feature = "dev") {
        "http://localhost:5173"
    } else {
        "https://cloud.developerdao.com"
    };

    let cors_api = CorsLayer::new()
        .allow_credentials(true)
        .allow_origin(origin.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE]);

    let relayer = Router::new()
        .route("/rpc/poly/{api_key}", post(route_poly))
        .route("/rpc/bsc/{api_key}", post(route_bsc))
        .route("/rpc/arb-one/{api_key}", post(route_arb))
        .route("/rpc/base/{api_key}", post(route_base))
        .route("/rpc/op/{api_key}", post(route_op))
        .route("/rpc/eth/{api_key}", post(route_eth))
        .route("/rpc/solana/{api_key}", post(route_solana))
        .route("/rpc/sui/{api_key}", post(route_sui))
        .route("/ws/{chain}/{api_key}", axum::routing::any(ws_handler));

    #[cfg(not(feature = "dev"))]
    let relayer = relayer.route_layer(from_fn(validate_subscription_and_update_user_calls));

    let token_queries = Router::new()
        .route(
            "/v1/tokens/balances/{chain}/{api_key}",
            get(aggregate_balances),
        )
        .route(
            "/v1/tokens/single_token_balances/{chain}/{api_key}",
            get(aggregate_single_token_bals),
        )
        .route(
            "/v1/tokens/many_token_balances/{chain}/{api_key}",
            get(aggregate_token_bals_for_user),
        )
        .route(
            "/v1/nfts/ownership/{chain}/{api_key}",
            get(get_batch_nft_info),
        );

    let api_keys = Router::new()
        .route("/api/keys", get(get_all_api_keys).post(generate_api_keys))
        .route("/api/keys/{key}", delete(delete_key))
        .route_layer(from_fn(verify_jwt));

    let payments = Router::new()
        .route("/api/pay/eth", post(process_ethereum_payment))
        .route("/api/upgrade", post(upgrade))
        .route("/api/downgrade", post(downgrade))
        .route("/api/cancel", post(cancel))
        .route("/api/balances", get(get_calls_and_balance))
        .route("/api/payments", get(get_payments))
        .route_layer(from_fn(verify_jwt));

    let siwe = Router::new()
        .route("/api/refresh", post(refresh))
        .route("/api/siwe/add_wallet", post(siwe_add_wallet))
        .route("/api/siwe/nonce/jwt", get(jwt_get_siwe_nonce))
        .route_layer(from_fn(verify_jwt))
        .route("/api/siwe/nonce/{wallet}", get(get_siwe_nonce));

    let app = Router::new()
        .route(
            "/api/checkhealth",
            get(|| async { (StatusCode::OK, "GM, we are fully operational").into_response() }),
        )
        .route("/api/register", post(register_user))
        .route("/api/activate", post(activate_account))
        .route("/api/login", post(user_login))
        .route("/api/login/siwe", post(user_login_siwe))
        .route("/api/recovery", post(update_password))
        .route("/api/recovery/{email}", get(recover_password_email))
        .merge(api_keys)
        .merge(siwe)
        .merge(payments)
        .layer(cors_api)
        .merge(token_queries)
        .merge(relayer)
        .layer(DefaultBodyLimit::disable())
        .layer(from_fn(body_size_limiter));

    info!("Initialized D_D RPC on 0.0.0.0:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
