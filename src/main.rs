use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use database::types::Database;
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use crate::routes::{register::register_user, payment::verify_subscription};
use crate::routes::types::Email;
pub mod database;
pub mod eth_rpc;
pub mod json_rpc;
pub mod routes; 

#[tokio::main]
async fn main() {
    dotenv().unwrap();
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

    let app = Router::new()
        .route(
            "/rpc/checkhealth",
            get(|| async { (StatusCode::OK, "GM, we are fully operational").into_response() }),
        )
        .route("/api/register", get(()).post(register_user))
        .route("/api/verifypayment/:emailaddress" , get(verify_subscription))
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
