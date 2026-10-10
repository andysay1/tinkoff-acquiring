//! Клиент интернет-эквайринга Т-Банка (Tinkoff Acquiring API v2).
//!
//! Покрывает все методы [спецификации Т-Банка](https://developer.tbank.ru/eacq/intro/developer/openapi)
//! версии 1.36: платежи и возвраты, чеки ФФД 1.05/1.2 (НДС 22 % с 2026 года),
//! рекуррентные платежи, покупатели и сохранённые карты, своя платёжная
//! форма и 3-D Secure, СБП и автоплатежи по СБП, цифровой рубль, T-Pay,
//! SberPay, Mir Pay, Alfa Pay, а также проверку HTTP-уведомлений.
//!
//! # Быстрый старт
//!
//! ```no_run
//! use tinkoff_acquiring::{Client, Init, ItemFfd12, ReceiptFfd12, Tax, Taxation};
//!
//! # async fn run() -> tinkoff_acquiring::Result<()> {
//! let client = Client::new("1234567890DEMO", "password")?;
//!
//! let receipt = ReceiptFfd12::new(
//!     Taxation::UsnIncome,
//!     vec![ItemFfd12::new("Худи", 199_900, 1.0, Tax::None)],
//! )
//! .email("buyer@example.com");
//!
//! let payment = client
//!     .init(
//!         &Init::new(199_900, "order-42")
//!             .description("Оплата заказа 42")
//!             .receipt(receipt)
//!             .notification_url("https://example.com/tbank/notify"),
//!     )
//!     .await?;
//!
//! println!("Ссылка на оплату: {}", payment.payment_url.unwrap_or_default());
//! # Ok(()) }
//! ```
//!
//! # Что делает клиент за вас
//!
//! * добавляет `TerminalKey` и считает `Token` по правилам Т-Банка
//!   (только корневые поля, вложенные `DATA`/`Receipt` не участвуют);
//! * превращает `Success: false` в [`Error::Api`] с кодом, сообщением и
//!   полным телом ответа;
//! * повторяет запрос, если он не дошёл до сервера, а таймауты и 5xx —
//!   только для методов без побочных эффектов (см. [`RetryPolicy`]);
//! * доверяет сертификату НУЦ Минцифры, на который Т-Банк переводит API
//!   (feature `russian-trusted-ca`, включена по умолчанию; системное
//!   хранилище сертификатов не меняется).
//!
//! # Features
//!
//! * `rustls` (по умолчанию) — TLS через rustls с системной проверкой
//!   сертификатов; если включены обе TLS-фичи, используется rustls;
//! * `native-tls` — TLS через OpenSSL / SChannel / SecureTransport. На macOS
//!   SecureTransport отвергает цепочку Минцифры («bad certificate format»),
//!   поэтому там используйте `rustls`; на Linux с OpenSSL всё работает;
//! * `russian-trusted-ca` (по умолчанию) — сертификат Минцифры для клиента;
//! * `blocking` — синхронный клиент `blocking::Client`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
mod macros;
#[macro_use]
mod request;

pub mod api;
#[cfg(feature = "blocking")]
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
pub mod blocking;
mod client;
mod de;
mod enums;
mod error;
pub mod notification;
pub mod receipt;
pub mod token;
mod transport;

/// Сумма в копейках.
pub type Amount = u64;

pub use api::*;
pub use client::{Client, ClientBuilder, Environment, RUSSIAN_TRUSTED_ROOT_CA_PEM};
pub use enums::*;
pub use error::{ApiError, Error, Result};
pub use notification::{
    AccountBindingNotification, CardBindingNotification, FiscalizationNotification, Notification,
    NotificationError, NotificationVerifier, PaymentNotification, NOTIFICATION_OK,
};
pub use receipt::{
    AddUserProp, AgentData, AgentSign, ClientInfo, ItemFfd105, ItemFfd12, MarkCode, MarkCodeType,
    MarkQuantity, OperatingCheckProps, PaymentMethod, PaymentObject, Payments, Receipt,
    ReceiptFfd105, ReceiptFfd12, SectoralProps, SupplierInfo, Tax, Taxation,
};
pub use request::{Encoding, Request, Signing};
pub use transport::{RetryPolicy, PRODUCTION_URL, TEST_URL};

/// Реэкспорт `reqwest` для [`ClientBuilder::http_client`] и сертификатов.
pub use reqwest;

/// Примеры из README компилируются вместе с doc-тестами.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
