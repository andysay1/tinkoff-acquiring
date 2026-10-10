//! Синхронный клиент: создать платёж и сразу узнать его статус.
//!
//! ```sh
//! TBANK_TERMINAL_KEY=... TBANK_PASSWORD=... cargo run --example blocking --features blocking
//! ```

use tinkoff_acquiring::{blocking, GetState, Init};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let terminal_key = std::env::var("TBANK_TERMINAL_KEY")?;
    let password = std::env::var("TBANK_PASSWORD")?;
    let client = blocking::Client::new(terminal_key, password)?;

    let order_id = format!(
        "blocking-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );
    let payment = client.init(&Init::new(10_000, order_id).description("Пример blocking"))?;
    println!("PaymentId: {}", payment.payment_id);
    println!("Ссылка:    {}", payment.payment_url.unwrap_or_default());

    let state = client.get_state(&GetState::new(&payment.payment_id))?;
    println!("Статус:    {}", state.status);
    Ok(())
}
