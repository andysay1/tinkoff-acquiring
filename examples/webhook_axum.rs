//! Приём HTTP-уведомлений Т-Банка на axum.
//!
//! ```sh
//! TBANK_TERMINAL_KEY=... TBANK_PASSWORD=... cargo run --example webhook_axum
//! ```
//!
//! Подпись проверяется по исходным байтам тела, поэтому обработчик принимает
//! `Bytes`, а не `Json<Value>`.

use axum::{body::Bytes, extract::State, http::StatusCode, routing::post, Router};
use tinkoff_acquiring::{Notification, NotificationVerifier, PaymentStatus, NOTIFICATION_OK};

async fn notify(
    State(verifier): State<NotificationVerifier>,
    body: Bytes,
) -> (StatusCode, &'static str) {
    match verifier.verify(&body) {
        Ok(Notification::Payment(p)) => {
            match p.status {
                PaymentStatus::Confirmed => println!("заказ {} оплачен", p.order_id),
                PaymentStatus::Authorized => println!("заказ {}: деньги заблокированы", p.order_id),
                PaymentStatus::Rejected => {
                    println!("заказ {}: отказ ({})", p.order_id, p.error_code)
                }
                ref other => println!("заказ {}: {other}", p.order_id),
            }
            if let Some(rebill) = &p.rebill_id {
                println!("RebillId для рекуррентов: {rebill}");
            }
            (StatusCode::OK, NOTIFICATION_OK)
        }
        Ok(other) => {
            println!("уведомление: {other:?}");
            (StatusCode::OK, NOTIFICATION_OK)
        }
        Err(e) => {
            eprintln!("отклонено: {e}");
            (StatusCode::BAD_REQUEST, "invalid notification")
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let verifier = NotificationVerifier::new(
        std::env::var("TBANK_TERMINAL_KEY")?,
        std::env::var("TBANK_PASSWORD")?,
    );
    let app = Router::new()
        .route("/tbank/notify", post(notify))
        .with_state(verifier);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8081").await?;
    println!("слушаю http://127.0.0.1:8081/tbank/notify");
    axum::serve(listener, app).await?;
    Ok(())
}
