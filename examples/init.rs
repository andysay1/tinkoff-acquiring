//! Платёж с чеком ФФД 1.2 и ссылкой на платёжную форму.
//!
//! ```sh
//! TBANK_TERMINAL_KEY=... TBANK_PASSWORD=... cargo run --example init
//! ```

use tinkoff_acquiring::{Client, Init, ItemFfd12, PaymentObject, ReceiptFfd12, Tax, Taxation};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new(
        std::env::var("TBANK_TERMINAL_KEY")?,
        std::env::var("TBANK_PASSWORD")?,
    )?;

    let receipt = ReceiptFfd12::new(
        Taxation::Osn,
        vec![
            ItemFfd12::new("Худи DDRW", 199_900, 1.0, Tax::Vat22),
            ItemFfd12::new("Доставка", 35_000, 1.0, Tax::Vat22)
                .payment_object(PaymentObject::Service)
                .measurement_unit("усл"),
        ],
    )
    .email("buyer@example.com");

    let order_id = format!(
        "order-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );
    let init = Init::new(234_900, order_id)
        .description("Оплата заказа")
        .receipt(receipt)
        .success_url("https://example.com/pay/success")
        .fail_url("https://example.com/pay/fail")
        .notification_url("https://example.com/tbank/notify");

    let payment = client.init(&init).await?;
    println!("PaymentId: {}", payment.payment_id);
    println!("Ссылка:    {}", payment.payment_url.unwrap_or_default());
    Ok(())
}
