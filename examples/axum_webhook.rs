use axum::{routing::post, Json, Router};
use serde_json::Value;
use tinkoff_acquiring::verify_webhook_token;
use tokio::net::TcpListener;

async fn tinkoff_cb(Json(payload): Json<Value>) -> String {
    match verify_webhook_token("YOUR_TERMINAL_KEY", "YOUR_PASSWORD", &payload) {
        Ok(_) => "OK".to_string(),
        Err(_) => "Unauthorized".to_string(),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/payments/tinkoff/callback", post(tinkoff_cb));
    let listener = TcpListener::bind("127.0.0.1:8081").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
