use crate::middleware::{
    jwt_auth::verify_jwt, rpc_service::validate_subscription_and_update_user_calls,
};
// use crate::routes::relayer::types::PoktChains;
use crate::routes::types::{EmailLogin, JWTKey};
use crate::routes::{
    activate::activate_account,
    api_keys::{delete_key, generate_api_keys, get_all_api_keys},
    login::user_login,
    recovery::{recover_password_email, update_password},
    register::register_user,
    relayer::router::route_call,
};
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
use dotenvy::dotenv;
// use middleware::rpc_service::{RpcAuthErrors, refill_calls_and_renew_plans};
use mimalloc::MiMalloc;
use routes::login::{refresh, user_login_siwe};
use routes::payment::{
    apply_payment_to_plan, get_calls_and_balance, get_payments, process_ethereum_payment,
};
use routes::siwe::{get_siwe_nonce, jwt_get_siwe_nonce, siwe_add_wallet};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use crate::routes::event_tracking::tracker::{get_events, subscribe_to_event, unsubscribe_from_event};
use tokio::spawn;

pub mod database;
pub mod eth_rpc;
pub mod middleware;
pub mod routes;

struct BlockchainIndexer;

impl BlockchainIndexer {
    fn new() -> Self {
        Self
    }

    async fn run(&self) {
        info!("Starting blockchain indexer...");
        loop {
            // TODO: Implement indexing logic
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
}

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() {
    //    PoktChains::init_deployment_url();
    JWTKey::init().unwrap();
    Database::init().await.unwrap();
    EmailLogin::init().unwrap();

    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let cors_api = CorsLayer::new()
        .allow_credentials(true)
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE]);

    let cors_rpc = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let relayer = Router::new()
        .route("/rpc/{chain}/{api_key}", post(route_call))
        .route_layer(from_fn(validate_subscription_and_update_user_calls))
        .layer(cors_rpc);

    let event_tracker = Router::new()
        .route("/api/event",
               get(get_events)
               .post(subscribe_to_event)
               .delete(unsubscribe_from_event));
    // .route_layer(from_fn(verify_jwt));

    let api_keys = Router::new()
        .route("/api/keys", get(get_all_api_keys).post(generate_api_keys))
        .route("/api/keys/{key}", delete(delete_key))
        .route_layer(from_fn(verify_jwt));
    let payments = Router::new()
        .route("/api/pay/eth", post(process_ethereum_payment))
        .route("/api/pay/apply", post(apply_payment_to_plan))
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
        .merge(event_tracker)
        .layer(cors_api)
        .merge(relayer);

    // tokio::spawn(async move {
    //     refill_calls_and_renew_plans().await?;
    //     Ok::<(), RpcAuthErrors>(())
    // });

    // /// The indexer to manage the subscribed events.
    // TODO! allow for issuing commands to the indexer from elsewhere to manage it at runtime
    // let indexer = BlockchainIndexer::new();
    // spawn(async move {
    //     indexer.run().await;
    // });


    info!("Initialized D_D RPC on 0.0.0.0:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
