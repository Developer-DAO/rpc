use crate::middleware::{
    jwt_auth::verify_jwt, rpc_service::validate_subscription_and_update_user_calls,
};
use crate::routes::types::{Email, JWTKey};
use crate::routes::{
    activate::activate_account,
    api_keys::{delete_key, generate_api_keys, get_all_api_keys},
    login::user_login,
    payment::verify_subscription,
    pk_login::{pk_login_challenge, pk_login_response},
    recovery::{recover_password_email, update_password},
    register::register_user,
    relayer::router::route_call,
};
use axum::{
    http::{header, StatusCode},
    middleware::from_fn,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use database::types::Database;
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
pub mod database;
pub mod eth_rpc;
pub mod json_rpc;
pub mod middleware;
pub mod routes;

#[tokio::main]
async fn main() {
    dotenv().unwrap();
    JWTKey::init().unwrap();
    Database::init(None).await.unwrap();
    Email::init().unwrap();
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_methods(Any);
    let relayer = Router::new()
        .route("/rpc/:chain/:api_key", post(route_call))
        .route_layer(from_fn(validate_subscription_and_update_user_calls));
    let api_keys = Router::new()
        .route(
            "/api/keys",
            get(get_all_api_keys)
                .post(generate_api_keys)
                .delete(delete_key),
        )
        .route_layer(from_fn(verify_jwt));

    let app = Router::new()
        .route(
            "/rpc/checkhealth",
            get(|| async { (StatusCode::OK, "GM, we are fully operational").into_response() }),
        )
        .route("/api/register", post(register_user))
        .route("/api/verifypayment/:emailaddress", get(verify_subscription))
        .route("/activate", post(activate_account))
        .route("/login", post(user_login))
        .route("/pk_login", get(pk_login_challenge).post(pk_login_response))
        .route(
            "/recovery",
            get(recover_password_email).post(update_password),
        )
        .merge(api_keys)
        .merge(relayer)
        .layer(cors);
    info!("Initialized D_D RPC on 0.0.0.0:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

// routes:
// checkhealth
// register
// verify_payment
// login (for client side applications)
// protected routes:
// rpc_request
// keys
