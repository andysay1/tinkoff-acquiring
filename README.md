# tinkoff-acquiring

Rust-клиент к [Tinkoff (T-Банк) Acquiring REST API v2](https://www.tbank.ru/business/documents/oplata-api/).

## Возможности

-   Инициализация платежа: `Init`, `GetState`, `Confirm`, `Cancel`, `Charge`
-   Работа с клиентами: `AddCustomer`, `GetCustomer`, `RemoveCustomer`
-   Карты: `GetCardList`, `RemoveCard`
-   Привязка и 3DS: `BindCard`, `FinishAuthorize`
-   QR: `GetQr`
-   Верификация вебхука (токен только по корневым полям)
-   Ретраи с экспоненциальным backoff
-   Идемпотентные ключи через заголовок
-   `blocking`-режим (опция через feature)

## Быстрый старт

```rust
use tinkoff_acquiring::{Config, TinkoffClient, Receipt, Item, Taxation, Tax};
use serde_json::json;

# #[tokio::main]
# async fn main() -> anyhow::Result<()> {
let cfg = Config::new("YOUR_TERMINAL_KEY", "YOUR_PASSWORD");
let client = TinkoffClient::new(cfg);

let receipt = Receipt {
  email: Some("buyer@example.com".into()),
  phone: None,
  taxation: Taxation::Osn,
  items: vec![Item {
      name: "Hoodie".into(),
      price: 199_900,
      quantity: 1.0,
      amount: 199_900,
      tax: Tax::Vat20,
  }],
  currency: None,
};

let init = client.init(
  199_900,
  "ORDER-123",
  Some("Оплата худи"),
  Some("O"),
  Some(receipt),
  Some(json!({"meta":"abc"})),
  Some("https://your.site/success"),
  Some("https://your.site/fail"),
  None,
  Some("order-123-key"), // идемпотентность
).await?;

println!("Payment URL: {:?}", init.payment_url);
# Ok(()) }
```
