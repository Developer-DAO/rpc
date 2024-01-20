use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use database::types::Database;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

pub mod database;

#[tokio::main]
async fn main() {

    Database::init(None).await.unwrap();
    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers([header::CONTENT_TYPE])
        .allow_methods(Any);

    let app = Router::new()
        .route(
            "/rpc/checkhealth",
            get(|| async { (StatusCode::OK, "GM, we are fully operational").into_response() }),
        )
        .layer(cors);
    info!("Initialized D_D RPC on 0.0.0.0:3000");
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// routes:
// checkhealth
// register
// login (for client side applications)
// protected routes:
// rpc_request
// keys
