# tinkoff-acquiring

[![crates.io](https://img.shields.io/crates/v/tinkoff-acquiring.svg)](https://crates.io/crates/tinkoff-acquiring)
[![docs.rs](https://docs.rs/tinkoff-acquiring/badge.svg)](https://docs.rs/tinkoff-acquiring)

Rust-клиент интернет-эквайринга Т-Банка (Tinkoff Acquiring API v2). Покрывает все
методы [OpenAPI-спецификации Т-Банка](https://developer.tbank.ru/eacq/intro/developer/openapi)
версии 1.36 (сентябрь 2026) и проверен на живом DEMO-терминале.

## Возможности

| Раздел | Методы |
|---|---|
| Платежи | `Init`, `GetState`, `Confirm`, `Cancel`, `Charge`, `CheckOrder` |
| Чеки 54-ФЗ | ФФД 1.05 и 1.2, НДС 22 % (с 01.01.2026), 5 %, 7 %, маркировка, агенты, `SendClosingReceipt` |
| Покупатели и карты | `AddCustomer`, `GetCustomer`, `RemoveCustomer`, `AddCard`, `AttachCard`, `GetAddCardState`, `GetCardList`, `RemoveCard` |
| Своя форма и 3DS | `Check3dsVersion`, `FinishAuthorize`, `Submit3DSAuthorization`, `Submit3DSAuthorizationV2`, помощники для 3DS Method и CReq |
| СБП и цифровой рубль | `GetQr`, `QrMembersList`, `GetQrState`, `GetQrBankList`, `SbpPayTest`, `AddAccountQr`, `GetAddAccountQrState`, `GetAccountQrList`, `ChargeQr` |
| Кошельки | T-Pay, SberPay, Mir Pay, Alfa Pay |
| Справки | `getConfirmOperation` |
| Уведомления | проверка подписи по исходным байтам и типизированный разбор: платёж, привязка карты, привязка счёта, фискализация |

Клиент сам:

- добавляет `TerminalKey` и считает `Token` по правилам банка;
- превращает `Success: false` в `Error::Api` с кодом, сообщением и полным телом ответа;
- повторяет запросы безопасно: при ошибке соединения — всегда, при таймаутах и 5xx — только для методов без побочных эффектов;
- доверяет сертификату НУЦ Минцифры (подробнее ниже).

## Установка

```toml
[dependencies]
tinkoff-acquiring = "0.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Минимальная версия Rust — 1.85.

## Быстрый старт

```rust,no_run
use tinkoff_acquiring::{Client, GetState, Init, ItemFfd12, ReceiptFfd12, Tax, Taxation};

#[tokio::main]
async fn main() -> tinkoff_acquiring::Result<()> {
    let client = Client::new("1234567890DEMO", "password")?;

    let receipt = ReceiptFfd12::new(
        Taxation::Osn,
        vec![ItemFfd12::new("Худи", 199_900, 1.0, Tax::Vat22)],
    )
    .email("buyer@example.com");

    let payment = client
        .init(
            &Init::new(199_900, "order-42")
                .description("Оплата заказа 42")
                .receipt(receipt)
                .notification_url("https://example.com/tbank/notify"),
        )
        .await?;
    println!("Ссылка на оплату: {}", payment.payment_url.unwrap_or_default());

    let state = client.get_state(&GetState::new(&payment.payment_id)).await?;
    println!("Статус: {}", state.status);
    Ok(())
}
```

Суммы задаются в копейках (`Amount = u64`). `ItemFfd12::new` считает `Amount` позиции
как `round(Price × Quantity)` и по умолчанию ставит `full_payment`, `commodity` и `шт`.
Перед отправкой `Init` проверяет, что сумма позиций чека равна `Amount`.

## Сертификаты Минцифры

Т-Банк [переводит API на сертификаты НУЦ Минцифры](https://developer.tbank.ru/eacq/intro/certificates/migration-russian-trusted-ca)
(Russian Trusted CA). Их нет в стандартных хранилищах ОС и в `webpki-roots`, поэтому
без настройки запросы падают с ошибкой проверки TLS.

Feature `russian-trusted-ca` (включена по умолчанию) встраивает корневой сертификат
Russian Trusted Root CA (SHA-256 `D2:6D:2D:…:CF:31`, действителен до 2032 года) и
добавляет его **только в HTTP-клиент библиотеки**: системное хранилище и другие
соединения приложения это не затрагивает. Отключить можно так:
`Client::builder(..).russian_trusted_ca(false)`. Если вы передаёте свой
`reqwest::Client`, добавьте сертификат сами — PEM лежит в `RUSSIAN_TRUSTED_ROOT_CA_PEM`.

TLS-бэкенд по умолчанию — `rustls`. С `native-tls` на macOS SecureTransport отвергает
цепочку Минцифры (`bad certificate format`), на Linux с OpenSSL всё работает.

## Двухстадийная оплата

```rust,no_run
# use tinkoff_acquiring::*;
# async fn f(client: Client) -> Result<()> {
let p = client.init(&Init::new(100_000, "order-7").two_stage()).await?;
// … покупатель оплатил, пришло уведомление AUTHORIZED …
client.confirm(&Confirm::new(&p.payment_id)).await?;          // списать
// или client.cancel(&Cancel::new(&p.payment_id)).await?;     // разблокировать
# Ok(()) }
```

## Возвраты

```rust,no_run
# use tinkoff_acquiring::*;
# async fn f(client: Client, payment_id: String) -> Result<()> {
let refund = client
    .cancel(
        &Cancel::new(&payment_id)
            .amount(50_000) // частичный возврат
            .receipt(ReceiptFfd12::new(Taxation::Osn, vec![ItemFfd12::new("Худи", 50_000, 1.0, Tax::Vat22)]).email("buyer@example.com"))
            .external_request_id("refund-order-42-1"), // ключ идемпотентности
    )
    .await?;
println!("{} → {:?}", refund.status, refund.new_amount);
# Ok(()) }
```

С `ExternalRequestId` повтор запроса не создаст второй возврат, поэтому клиент сам
повторяет такой `Cancel` при таймауте.

## Рекуррентные платежи

```rust,no_run
# use tinkoff_acquiring::*;
# async fn f(client: Client, rebill_id: String) -> Result<()> {
// 1. Родительский платёж: покупатель платит на форме, карта сохраняется.
client.init(
    &Init::new(49_900, "sub-1")
        .customer_key("customer-42")
        .recurrent()
        .operation_initiator_type(OperationInitiatorType::CitCredentialCapture),
).await?;
// 2. RebillId приходит в уведомлении (PaymentNotification::rebill_id).
// 3. Следующее списание: новый Init и Charge.
let next = client
    .init(&Init::new(49_900, "sub-2").operation_initiator_type(OperationInitiatorType::MitRecurring))
    .await?;
client.charge(&Charge::new(&next.payment_id, rebill_id)).await?;
# Ok(()) }
```

Для автоплатежей по СБП: `AddAccountQr` → уведомление `LINKACCOUNT` с `AccountToken` → `Init` + `ChargeQr`.

## СБП

```rust,no_run
# use tinkoff_acquiring::*;
# async fn f(client: Client) -> Result<()> {
let p = client.init(&Init::new(10_000, "sbp-1")).await?;
let qr = client.get_qr(&GetQr::new(&p.payment_id)).await?;
println!("{}", qr.data); // https://qr.nspk.ru/…  (или SVG с .data_type(QrDataType::Image))

// На тестовом терминале оплату можно сэмулировать:
client.sbp_pay_test(&SbpPayTest::new(&p.payment_id)).await?;
# Ok(()) }
```

## Уведомления

Проверяйте подпись по **исходным байтам** тела и отвечайте `200` с телом `OK`:

```rust,no_run
use axum::{body::Bytes, extract::State, http::StatusCode};
use tinkoff_acquiring::{Notification, NotificationVerifier, NOTIFICATION_OK};

async fn notify(State(v): State<NotificationVerifier>, body: Bytes) -> (StatusCode, &'static str) {
    match v.verify(&body) {
        Ok(Notification::Payment(p)) => {
            println!("{} → {} (RebillId: {:?})", p.order_id, p.status, p.rebill_id);
            (StatusCode::OK, NOTIFICATION_OK)
        }
        Ok(_) => (StatusCode::OK, NOTIFICATION_OK),
        Err(_) => (StatusCode::BAD_REQUEST, "invalid"),
    }
}
```

`verify` проверяет подпись (сравнение за постоянное время) и `TerminalKey` и
определяет тип уведомления: `Payment`, `CardBinding` (`LINKCARD`),
`AccountBinding` (`LINKACCOUNT`) или `Fiscalization`. Полные примеры:
`examples/webhook_axum.rs` и `examples/webhook_actix.rs`.

## Ошибки

```rust,no_run
# use tinkoff_acquiring::*;
# async fn f(client: Client) {
match client.charge(&Charge::new("123", "456")).await {
    Ok(r) => println!("{}", r.status),
    Err(Error::Api(e)) => println!("банк ответил {}: {:?}, статус {:?}", e.code, e.message, e.status()),
    Err(e) if e.is_transient() => println!("сеть/5xx, проверьте статус через GetState: {e}"),
    Err(e) => println!("{e}"),
}
# }
```

Неизвестные значения перечислений (новые статусы, ставки и т. п.) не ломают разбор
ответа и попадают в вариант `Other(String)`. Поля ответа, которых нет в типах,
сохраняются в `extra`.

## Метод, которого нет в библиотеке

```rust,no_run
# use tinkoff_acquiring::*;
# async fn f(client: Client) -> Result<()> {
let mut body = serde_json::Map::new();
body.insert("PaymentId".into(), "123".into());
let raw = client.execute_raw("v2/SomeNewMethod", body).await?; // подпись и TerminalKey добавятся сами
# Ok(()) }
```

Можно и типизированно: реализуйте трейт `Request` для своей структуры и вызовите
`client.execute(&req)`.

## Синхронный клиент

```toml
tinkoff-acquiring = { version = "0.3", features = ["blocking"] }
```

```rust,ignore
let client = tinkoff_acquiring::blocking::Client::new("TerminalKey", "Password")?;
let state = client.get_state(&GetState::new("1234567890"))?;
```

## Features

| Feature | По умолчанию | Что делает |
|---|---|---|
| `rustls` | да | TLS через rustls с системной проверкой сертификатов |
| `russian-trusted-ca` | да | доверять сертификату Минцифры в клиенте библиотеки |
| `native-tls` | нет | TLS через OpenSSL / SChannel / SecureTransport |
| `blocking` | нет | синхронный `blocking::Client` |

## Тесты

```sh
cargo test --all-features                       # модульные, мок-сервер, сверка со спецификацией
TBANK_TERMINAL_KEY=…DEMO TBANK_PASSWORD=… \
  cargo test --all-features --test live -- --ignored --test-threads=1   # живой DEMO-терминал
```

`tests/spec.rs` сверяет типы с OpenAPI-спецификацией Т-Банка: каждый пример ответа
должен разбираться, каждое поле запроса должно быть в спецификации, каждый метод
спецификации должен быть покрыт.

## Переход с 0.2

Версия 0.3 — полная переработка, API несовместим с 0.2. См. [CHANGELOG.md](CHANGELOG.md).

## Лицензия

MIT или Apache-2.0 на ваш выбор.
