use actix_web::{post, web, App, HttpResponse, HttpServer};
use serde_json::Value;
use tinkoff_acquiring::verify_webhook_token;

#[post("/payments/tinkoff/callback")]
async fn tinkoff_cb(body: web::Json<Value>) -> actix_web::Result<HttpResponse> {
    if verify_webhook_token("YOUR_TERMINAL_KEY", "YOUR_PASSWORD", &body.0).is_err() {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    Ok(HttpResponse::Ok().body("OK"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(tinkoff_cb))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
