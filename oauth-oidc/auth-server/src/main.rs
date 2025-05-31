use std::sync::Arc;

use axum::{
    http::{self, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use redis;
use tower_http::cors::CorsLayer;

mod errors;
mod handlers;

#[derive(Clone)]
struct AppState {
    redis: Arc<redis::Client>,
    public_key: String,
    private_key: String,
    // Public/Private key の識別子
    key_id: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(["http://localhost:5173".parse::<HeaderValue>()?])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let redis_client = Arc::new(redis::Client::open("redis://localhost:6379")?);
    let state = AppState {
        redis: redis_client,
        public_key: include_str!("../public.pem").to_string(),
        private_key: include_str!("../private.pem").to_string(),
        key_id: uuid::Uuid::new_v4().to_string(),
    };
    let app = Router::new()
        .route("/authorize", get(handlers::authorize))
        .route("/token", post(handlers::token))
        .route("/clients", post(handlers::create_client))
        .route("/.well-known/jwks.json", get(handlers::jwks_handler))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3123").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
