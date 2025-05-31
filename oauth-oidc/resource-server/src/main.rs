use axum::{
    http::{self, HeaderValue, Method},
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;

mod errors;
mod handlers;

#[derive(Clone)]
struct AppState {
    auth_server_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let state = AppState {
        auth_server_url: "http://localhost:3123".to_string(),
    };

    let cors = CorsLayer::new()
        .allow_origin(["http://localhost:5173".parse::<HeaderValue>()?])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([http::header::AUTHORIZATION])
        .allow_credentials(true);

    let app = Router::new()
        .route("/resource", get(handlers::resource))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:6244").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
