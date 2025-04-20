use axum::{extract::Query, response::Html};
use std::collections::HashMap;

pub async fn authorize(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    Html(String::from("authorize endpoint"))
}

pub async fn token() -> Html<&'static str> {
    Html("<h1>Token endpoint</h1>")
}
