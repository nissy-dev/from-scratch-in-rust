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
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cors = CorsLayer::new()
        .allow_origin(["http://localhost:5173".parse::<HeaderValue>().unwrap()])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([http::header::CONTENT_TYPE])
        .allow_credentials(true);

    let redis_client = Arc::new(redis::Client::open("redis://localhost:6379").unwrap());
    let state = AppState {
        redis: redis_client,
    };
    let app = Router::new()
        .route("/authorize", get(handlers::authorize))
        .route("/token", post(handlers::token))
        .route("/clients", post(handlers::create_client))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3123").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
