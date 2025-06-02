use axum::{
    http::{self, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use redis;
use store::{Store, UserData};
use tower_http::cors::CorsLayer;

mod errors;
mod handlers;
mod store;

#[derive(Clone)]
struct AppState {
    store: Store,
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

    let redis_client = redis::Client::open("redis://localhost:6379")?;
    let mut store = Store::new(redis_client.clone());
    let state = AppState {
        store: store.clone(),
        public_key: include_str!("../public.pem").to_string(),
        private_key: include_str!("../private.pem").to_string(),
        // 本当は起動時に生成するより、public_key と private_key をもとに生成するのが望ましい
        key_id: uuid::Uuid::new_v4().to_string(),
    };

    // 初期ユーザを登録しておく
    store
        .write_user_data(&UserData {
            name: "sample".to_string(),
            password: "sample".to_string(),
        })
        .await?;

    let app = Router::new()
        .route("/authorize", get(handlers::authorize))
        .route("/token", post(handlers::token))
        .route("/clients", post(handlers::create_client))
        .route("/introspect", post(handlers::introspect))
        .route("/.well-known/jwks.json", get(handlers::jwks))
        .route(
            "/.well-known/openid-configuration",
            get(handlers::openid_configuration),
        )
        .route("/login", get(handlers::login_form))
        .route("/login", post(handlers::login_action))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3123").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
