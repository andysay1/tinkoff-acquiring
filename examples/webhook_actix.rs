//! Приём HTTP-уведомлений Т-Банка на actix-web.
//!
//! ```sh
//! TBANK_TERMINAL_KEY=... TBANK_PASSWORD=... cargo run --example webhook_actix
//! ```

use actix_web::{post, web, App, HttpResponse, HttpServer};
use tinkoff_acquiring::{Notification, NotificationVerifier, NOTIFICATION_OK};

#[post("/tbank/notify")]
async fn notify(verifier: web::Data<NotificationVerifier>, body: web::Bytes) -> HttpResponse {
    match verifier.verify(&body) {
        Ok(Notification::Payment(p)) => {
            println!("заказ {}: {}", p.order_id, p.status);
            HttpResponse::Ok().body(NOTIFICATION_OK)
        }
        Ok(_) => HttpResponse::Ok().body(NOTIFICATION_OK),
        Err(e) => {
            eprintln!("отклонено: {e}");
            HttpResponse::BadRequest().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let verifier = web::Data::new(NotificationVerifier::new(
        std::env::var("TBANK_TERMINAL_KEY").expect("TBANK_TERMINAL_KEY"),
        std::env::var("TBANK_PASSWORD").expect("TBANK_PASSWORD"),
    ));
    println!("слушаю http://127.0.0.1:8080/tbank/notify");
    HttpServer::new(move || App::new().app_data(verifier.clone()).service(notify))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
