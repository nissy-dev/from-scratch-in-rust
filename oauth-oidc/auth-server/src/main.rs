use axum::{routing::get, Router};

mod handlers;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/authorize", get(handlers::authorize))
        .route("/token", get(handlers::token));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
