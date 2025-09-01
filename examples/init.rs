use tinkoff_acquiring::{Config, TinkoffClient, Receipt, Item, Taxation, Tax};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::new("YOUR_TERMINAL_KEY", "YOUR_PASSWORD");
    let client = TinkoffClient::new(cfg);

    let receipt = Receipt {
        email: Some("buyer@example.com".into()),
        phone: None,
        taxation: Taxation::Osn,
        items: vec![Item { name: "DDRW Hoodie".into(), price: 199_900, quantity: 1.0, amount: 199_900, tax: Tax::Vat20 }],
        currency: None,
    };

    let init = client.init(
        199_900,
        "ORDER-123",
        Some("Оплата худи DDRW"),
        Some("O"),
        Some(receipt),
        Some(json!({"meta":"abc"})),
        Some("https://your.site/pay/success"),
        Some("https://your.site/pay/fail"),
        None,
        Some("idem-ORDER-123"),
    ).await?;

    println!("Payment URL: {:?}", init.payment_url);
    Ok(())
}
