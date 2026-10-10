//! Рекуррентные платежи (подписка) по карте.
//!
//! 1. Родительский платёж: `Init` с `Recurrent=Y`, `CustomerKey` и
//!    `OperationInitiatorType=1`. Покупатель платит на форме.
//! 2. В уведомлении `CONFIRMED`/`AUTHORIZED` приходит `RebillId` — сохраните его.
//! 3. Следующие списания: новый `Init` (без `Recurrent`) и `Charge` с `RebillId`.
//!
//! ```sh
//! TBANK_TERMINAL_KEY=... TBANK_PASSWORD=... cargo run --example recurrent            # шаг 1
//! TBANK_TERMINAL_KEY=... TBANK_PASSWORD=... cargo run --example recurrent -- <RebillId>  # шаг 3
//! ```
//!
//! На DEMO-терминале `Charge` обычно отключён (ошибка 10) — его включает банк.

use tinkoff_acquiring::{Charge, Client, Init, OperationInitiatorType, PaymentStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new(
        std::env::var("TBANK_TERMINAL_KEY")?,
        std::env::var("TBANK_PASSWORD")?,
    )?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let customer_key = "customer-42";

    match std::env::args().nth(1) {
        None => {
            let parent = client
                .init(
                    &Init::new(49_900, format!("sub-{stamp}"))
                        .description("Подписка, первый месяц")
                        .customer_key(customer_key)
                        .recurrent()
                        .operation_initiator_type(OperationInitiatorType::CitCredentialCapture),
                )
                .await?;
            println!("Оплатите: {}", parent.payment_url.unwrap_or_default());
            println!("RebillId придёт в уведомлении на NotificationURL.");
        }
        Some(rebill_id) => {
            let next = client
                .init(
                    &Init::new(49_900, format!("sub-{stamp}"))
                        .description("Подписка, следующий месяц")
                        .customer_key(customer_key)
                        .operation_initiator_type(OperationInitiatorType::MitRecurring),
                )
                .await?;
            match client
                .charge(&Charge::new(&next.payment_id, rebill_id))
                .await
            {
                Ok(r) if r.status == PaymentStatus::Confirmed => {
                    println!("Списано: {}", r.payment_id)
                }
                Ok(r) => println!("Статус: {}", r.status),
                Err(e) => println!("Не списано: {e}"),
            }
        }
    }
    Ok(())
}
