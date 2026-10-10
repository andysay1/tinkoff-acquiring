//! HTTP-уведомления (вебхуки) Т-Банка
//! ([документация](https://developer.tbank.ru/eacq/intro/developer/notification)).
//!
//! Подпись проверяется по исходным байтам тела: так числа участвуют в
//! подписи ровно в той записи, в какой их прислал банк. На успешно
//! обработанное уведомление ответьте `200` с телом [`NOTIFICATION_OK`],
//! иначе банк будет повторять его раз в час в течение суток, затем раз в
//! сутки в течение месяца.
//!
//! ```
//! use tinkoff_acquiring::{Notification, NotificationVerifier, NOTIFICATION_OK};
//!
//! fn handle(body: &[u8]) -> (u16, &'static str) {
//!     let verifier = NotificationVerifier::new("TerminalKey", "Password");
//!     match verifier.verify(body) {
//!         Ok(Notification::Payment(p)) => {
//!             println!("{} → {}", p.order_id, p.status);
//!             (200, NOTIFICATION_OK)
//!         }
//!         Ok(_) => (200, NOTIFICATION_OK),
//!         Err(_) => (400, "bad notification"),
//!     }
//! }
//! ```

use crate::enums::{AccountQrStatus, AddCardStatus, NotificationType, PaymentStatus};
use crate::token;
use crate::Amount;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt;

/// Тело ответа на успешно обработанное уведомление.
pub const NOTIFICATION_OK: &str = "OK";

/// Ошибка проверки уведомления.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NotificationError {
    /// Тело — не JSON-объект.
    #[error("тело уведомления не является JSON-объектом: {0}")]
    Json(#[source] serde_json::Error),
    /// Нет поля `Token`.
    #[error("в уведомлении нет Token")]
    MissingToken,
    /// Подпись не совпала.
    #[error("неверная подпись уведомления")]
    InvalidToken,
    /// Уведомление для другого терминала.
    #[error("уведомление для другого терминала: {0}")]
    TerminalMismatch(String),
    /// В уведомлении нет `TerminalKey`.
    #[error("в уведомлении нет TerminalKey")]
    MissingTerminalKey,
    /// Подпись верна, но структуру разобрать не удалось.
    #[error("не удалось разобрать уведомление: {0}")]
    Decode(#[source] serde_json::Error),
}

/// Проверка подписи и разбор уведомлений.
#[derive(Clone)]
pub struct NotificationVerifier {
    terminal_key: String,
    password: String,
}

impl fmt::Debug for NotificationVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotificationVerifier")
            .field("terminal_key", &self.terminal_key)
            .field("password", &"***")
            .finish()
    }
}

impl NotificationVerifier {
    /// Проверка для терминала `terminal_key` с паролем `password`.
    pub fn new(terminal_key: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            terminal_key: terminal_key.into(),
            password: password.into(),
        }
    }

    /// Проверить только подпись по исходному телу запроса.
    pub fn verify_token(&self, body: &[u8]) -> Result<(), NotificationError> {
        let fields: BTreeMap<String, Box<RawValue>> =
            serde_json::from_slice(body).map_err(NotificationError::Json)?;
        let got = match fields.get("Token") {
            Some(raw) => serde_json::from_str::<String>(raw.get())
                .map_err(|_| NotificationError::MissingToken)?,
            None => return Err(NotificationError::MissingToken),
        };
        let mut pairs = Vec::with_capacity(fields.len());
        for (k, raw) in &fields {
            let s = raw.get().trim();
            if s.starts_with('{') || s.starts_with('[') || s == "null" {
                continue;
            }
            let v = if s.starts_with('"') {
                serde_json::from_str::<String>(s).map_err(NotificationError::Json)?
            } else {
                s.to_owned()
            };
            pairs.push((k.clone(), v));
        }
        let expected = token::sign_pairs(&self.password, pairs);
        if token::constant_time_eq(expected.as_bytes(), got.to_ascii_lowercase().as_bytes()) {
            Ok(())
        } else {
            Err(NotificationError::InvalidToken)
        }
    }

    /// Проверить подпись и `TerminalKey`, затем разобрать уведомление.
    pub fn verify(&self, body: &[u8]) -> Result<Notification, NotificationError> {
        self.verify_token(body)?;
        let value: Value = serde_json::from_slice(body).map_err(NotificationError::Json)?;
        self.check_terminal(&value)?;
        Notification::from_value(value).map_err(NotificationError::Decode)
    }

    /// То же для уже разобранного JSON (например, `axum::Json<Value>`).
    ///
    /// Надёжнее передавать исходные байты в [`verify`](Self::verify): при
    /// повторной сериализации дробные числа могут поменять запись.
    pub fn verify_value(&self, value: &Value) -> Result<Notification, NotificationError> {
        let obj = value
            .as_object()
            .ok_or_else(|| NotificationError::Json(serde::de::Error::custom("ожидался объект")))?;
        let got = obj
            .get("Token")
            .and_then(Value::as_str)
            .ok_or(NotificationError::MissingToken)?;
        let expected = token::sign(&self.password, obj);
        if !token::constant_time_eq(expected.as_bytes(), got.to_ascii_lowercase().as_bytes()) {
            return Err(NotificationError::InvalidToken);
        }
        self.check_terminal(value)?;
        Notification::from_value(value.clone()).map_err(NotificationError::Decode)
    }

    fn check_terminal(&self, value: &Value) -> Result<(), NotificationError> {
        match value.get("TerminalKey").and_then(Value::as_str) {
            Some(tk) if tk == self.terminal_key => Ok(()),
            Some(tk) => Err(NotificationError::TerminalMismatch(tk.to_owned())),
            None => Err(NotificationError::MissingTerminalKey),
        }
    }
}

/// Уведомление после проверки подписи.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Notification {
    /// Статус платежа (карта, СБП, T-Pay, SberPay, Mir Pay, Alfa Pay,
    /// цифровой рубль, «Долями»).
    Payment(PaymentNotification),
    /// Привязка карты (`NotificationType=LINKCARD`).
    CardBinding(CardBindingNotification),
    /// Привязка счёта СБП (`NotificationType=LINKACCOUNT`).
    AccountBinding(AccountBindingNotification),
    /// Фискализация чека.
    Fiscalization(Box<FiscalizationNotification>),
}

impl Notification {
    /// Разобрать JSON уведомления без проверки подписи.
    pub fn from_value(value: Value) -> Result<Self, serde_json::Error> {
        let kind = value
            .get("NotificationType")
            .and_then(Value::as_str)
            .map(NotificationType::from);
        let is_fiscal = value.get("Status").and_then(Value::as_str) == Some("RECEIPT")
            || value.get("FiscalNumber").is_some()
            || value.get("FiscalDocumentNumber").is_some();
        Ok(match kind {
            Some(NotificationType::LinkCard) => {
                Notification::CardBinding(serde_json::from_value(value)?)
            }
            Some(NotificationType::LinkAccount) => {
                Notification::AccountBinding(serde_json::from_value(value)?)
            }
            _ if is_fiscal => Notification::Fiscalization(Box::new(serde_json::from_value(value)?)),
            _ => Notification::Payment(serde_json::from_value(value)?),
        })
    }

    /// `TerminalKey` уведомления.
    pub fn terminal_key(&self) -> &str {
        match self {
            Notification::Payment(n) => &n.terminal_key,
            Notification::CardBinding(n) => &n.terminal_key,
            Notification::AccountBinding(n) => &n.terminal_key,
            Notification::Fiscalization(n) => &n.terminal_key,
        }
    }
}

/// Уведомление о статусе платежа.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct PaymentNotification {
    /// Идентификатор терминала.
    #[serde(default)]
    pub terminal_key: String,
    /// Идентификатор заказа.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub order_id: String,
    /// Успешность операции.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub success: bool,
    /// Новый статус платежа.
    pub status: PaymentStatus,
    /// Идентификатор платежа.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub payment_id: String,
    /// Код ошибки (`"0"` — успех).
    #[serde(default, deserialize_with = "crate::de::string")]
    pub error_code: String,
    /// Сумма в копейках.
    #[serde(default, deserialize_with = "crate::de::opt_u64")]
    pub amount: Option<Amount>,
    /// Идентификатор карты.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub card_id: Option<String>,
    /// Маскированный номер карты (для СБП — телефон, для T-Pay — `T-Pay_account`).
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub pan: Option<String>,
    /// Срок действия карты `MMYY`.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub exp_date: Option<String>,
    /// Идентификатор реквизитов для `Charge` (родительский рекуррентный платёж).
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub rebill_id: Option<String>,
    /// Сообщение.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub message: Option<String>,
    /// Подробности ошибки.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub details: Option<String>,
    /// Дополнительные параметры (`Data`). Вложенные объекты не входят в
    /// подпись банка, поэтому их содержимое подписью не защищено.
    #[serde(default, alias = "DATA", deserialize_with = "crate::de::map_or_null")]
    pub data: Map<String, Value>,
    /// Поля, которых нет в этой версии библиотеки.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Уведомление о привязке карты.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct CardBindingNotification {
    /// Идентификатор терминала.
    #[serde(default)]
    pub terminal_key: String,
    /// Идентификатор покупателя.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub customer_key: String,
    /// Идентификатор запроса на привязку.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub request_key: String,
    /// Успешность.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub success: bool,
    /// Статус привязки (`COMPLETED` — карта привязана).
    pub status: AddCardStatus,
    /// Идентификатор проверочного платежа.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub payment_id: Option<String>,
    /// Код ошибки.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub error_code: String,
    /// Идентификатор карты.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub card_id: Option<String>,
    /// Маскированный номер карты.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub pan: Option<String>,
    /// Срок действия `MMYY`.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub exp_date: Option<String>,
    /// Идентификатор реквизитов для `Charge`.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub rebill_id: Option<String>,
    /// Поля, которых нет в этой версии библиотеки.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Уведомление о привязке счёта СБП (приходит для `ACTIVE` и `INACTIVE`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AccountBindingNotification {
    /// Идентификатор терминала.
    #[serde(default)]
    pub terminal_key: String,
    /// Идентификатор запроса на привязку.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub request_key: String,
    /// Статус привязки.
    pub status: AccountQrStatus,
    /// Успешность.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub success: bool,
    /// Код ошибки.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub error_code: String,
    /// Сообщение.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub message: Option<String>,
    /// Идентификатор привязки для `ChargeQr`.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub account_token: Option<String>,
    /// Банк покупателя.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub bank_member_id: Option<String>,
    /// Название банка.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub bank_member_name: Option<String>,
    /// Поля, которых нет в этой версии библиотеки.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Уведомление о фискализации (приходит примерно через 10 минут после
/// ответа онлайн-кассы, включается через поддержку банка).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct FiscalizationNotification {
    /// Идентификатор терминала.
    #[serde(default)]
    pub terminal_key: String,
    /// Идентификатор заказа.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub order_id: String,
    /// Успешность.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub success: bool,
    /// `RECEIPT`.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub status: String,
    /// Идентификатор платежа.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub payment_id: String,
    /// Код ошибки.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub error_code: String,
    /// Описание ошибки кассы.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub error_message: Option<String>,
    /// Сумма в копейках.
    #[serde(default, deserialize_with = "crate::de::opt_u64")]
    pub amount: Option<Amount>,
    /// Фискальный номер чека.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub fiscal_number: Option<String>,
    /// Номер смены.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub shift_number: Option<String>,
    /// Дата и время чека.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub receipt_datetime: Option<String>,
    /// Номер фискального накопителя.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub fn_number: Option<String>,
    /// Регистрационный номер ККТ.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub ecr_reg_number: Option<String>,
    /// Номер фискального документа.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub fiscal_document_number: Option<String>,
    /// Фискальный признак документа.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub fiscal_document_attribute: Option<String>,
    /// Чек в том виде, в каком его прислал банк. Вложенные объекты не входят
    /// в подпись, поэтому чек подписью не защищён.
    #[serde(default)]
    pub receipt: Option<Value>,
    /// Тип чека.
    #[serde(rename = "Type", default, deserialize_with = "crate::de::opt_string")]
    pub kind: Option<String>,
    /// ОФД.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub ofd: Option<String>,
    /// Ссылка на чек.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub url: Option<String>,
    /// Ссылка на QR чека.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub qr_code_url: Option<String>,
    /// Место расчётов.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub calculation_place: Option<String>,
    /// Кассир.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub cashier_name: Option<String>,
    /// Адрес расчётов.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub settle_place: Option<String>,
    /// Поля, которых нет в этой версии библиотеки.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
