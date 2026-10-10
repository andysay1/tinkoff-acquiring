//! Быстрая проверка связи с API: создать платёж, получить статус, отменить.
//!
//! ```sh
//! TBANK_TERMINAL_KEY=... TBANK_PASSWORD=... cargo run --example smoke
//! ```

use tinkoff_acquiring::{Cancel, CheckOrder, Client, GetState, Init};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new(
        std::env::var("TBANK_TERMINAL_KEY")?,
        std::env::var("TBANK_PASSWORD")?,
    )?;
    let order_id = format!(
        "smoke-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    );

    let payment = client
        .init(&Init::new(10_000, &order_id).description("smoke test"))
        .await?;
    println!(
        "Init:       PaymentId={} status={:?}",
        payment.payment_id, payment.status
    );
    println!(
        "            {}",
        payment.payment_url.as_deref().unwrap_or("-")
    );

    let state = client
        .get_state(&GetState::new(&payment.payment_id))
        .await?;
    println!("GetState:   {}", state.status);

    let order = client.check_order(&CheckOrder::new(&order_id)).await?;
    println!("CheckOrder: {} платеж(ей)", order.payments.len());

    let cancel = client.cancel(&Cancel::new(&payment.payment_id)).await?;
    println!("Cancel:     {}", cancel.status);
    Ok(())
}
