//! Оплата через приложения банков: T-Pay, SberPay, Mir Pay, Alfa Pay.
//!
//! Сначала создайте платёж через [`Init`](crate::Init), затем получите ссылку
//! для перехода в приложение банка.

use crate::request::{path_segment, Encoding, Signing};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Параметры ответа со ссылкой.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct LinkParams {
    /// Ссылка для перехода в приложение банка.
    #[serde(default)]
    pub redirect_url: String,
    /// Ссылка на QR для оплаты с десктопа (T-Pay).
    #[serde(rename = "WebQR", default)]
    pub web_qr: Option<String>,
}

response! {
    /// Ответ методов, которые возвращают ссылку на оплату.
    pub struct LinkResponse {
        /// Параметры со ссылкой.
        #[serde(default)]
        pub params: LinkParams,
    }
}

impl LinkResponse {
    /// Ссылка для перехода.
    pub fn redirect_url(&self) -> &str {
        &self.params.redirect_url
    }
}

// ---------------------------------------------------------------- T-Pay

/// Доступен ли T-Pay на терминале
/// ([`GET v2/TinkoffPay/terminals/{TerminalKey}/status`](https://developer.tbank.ru/eacq/api/status)).
#[derive(Clone, Debug, Default, Serialize)]
#[non_exhaustive]
pub struct TPayStatus {}

impl TPayStatus {
    /// Запрос для терминала клиента.
    pub fn new() -> Self {
        Self {}
    }
}

impl_request!(TPayStatus => TPayStatusResponse, "v2/TinkoffPay/terminals/{TerminalKey}/status", {
    const ENCODING: Encoding = Encoding::Get;
    const SIGNING: Signing = Signing::None;
    fn path(&self, terminal_key: &str) -> Cow<'static, str> {
        Cow::Owned(format!(
            "v2/TinkoffPay/terminals/{}/status",
            path_segment(terminal_key)
        ))
    }
    fn retry_safe(&self) -> bool { true }
});

/// Параметры ответа [`TPayStatus`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct TPayStatusParams {
    /// T-Pay по API доступен.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub allowed: bool,
    /// Версия: `1.0` — e-invoice, `2.0` — T-Pay.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub version: Option<String>,
}

response! {
    /// Ответ [`TPayStatus`].
    pub struct TPayStatusResponse {
        /// Параметры.
        #[serde(default)]
        pub params: TPayStatusParams,
    }
}

/// Ссылка на оплату через T-Pay
/// ([`GET v2/TinkoffPay/transactions/{paymentId}/versions/{version}/link`](https://developer.tbank.ru/eacq/api/link)).
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct TPayLink {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// Версия T-Pay из [`TPayStatus`], обычно `2.0`.
    pub version: String,
}

impl TPayLink {
    /// Ссылка T-Pay версии `2.0` для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            version: "2.0".into(),
        }
    }

    /// Версия T-Pay.
    #[must_use]
    pub fn version(mut self, v: impl Into<String>) -> Self {
        self.version = v.into();
        self
    }
}

impl_request!(TPayLink => LinkResponse, "v2/TinkoffPay/transactions/{paymentId}/versions/{version}/link", {
    const ENCODING: Encoding = Encoding::Get;
    const SIGNING: Signing = Signing::None;
    fn path(&self, _terminal_key: &str) -> Cow<'static, str> {
        Cow::Owned(format!(
            "v2/TinkoffPay/transactions/{}/versions/{}/link",
            path_segment(&self.payment_id),
            path_segment(&self.version)
        ))
    }
    fn retry_safe(&self) -> bool { true }
});

// ---------------------------------------------------------------- SberPay

/// Создать заказ SberPay и получить ссылку
/// ([`v2/SberPay/link/get`](https://developer.tbank.ru/eacq/api/sberpay-link-get)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct SberPayLinkGet {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// Телефон покупателя (обязателен при оплате с десктопа).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

impl SberPayLinkGet {
    /// Ссылка SberPay для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            phone: None,
        }
    }
}

setters!(SberPayLinkGet {
    /// Телефон покупателя.
    phone: String,
});

impl_request!(SberPayLinkGet => LinkResponse, "v2/SberPay/link/get");

/// Ссылка SberPay для мобильного браузера
/// ([`GET v2/SberPay/transactions/{paymentId}/link`](https://developer.tbank.ru/eacq/api/sber-paylink)).
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct SberPayLink {
    /// Идентификатор платежа.
    pub payment_id: String,
}

impl SberPayLink {
    /// Ссылка для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

impl_request!(SberPayLink => LinkResponse, "v2/SberPay/transactions/{paymentId}/link", {
    const ENCODING: Encoding = Encoding::Get;
    const SIGNING: Signing = Signing::None;
    fn path(&self, _terminal_key: &str) -> Cow<'static, str> {
        Cow::Owned(format!(
            "v2/SberPay/transactions/{}/link",
            path_segment(&self.payment_id)
        ))
    }
    fn retry_safe(&self) -> bool { true }
});

// ---------------------------------------------------------------- Mir Pay

/// DeepLink для оплаты через Mir Pay
/// ([`v2/MirPay/GetDeepLink`](https://developer.tbank.ru/eacq/api/get-deep-link)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct MirPayDeepLink {
    /// Идентификатор платежа.
    pub payment_id: String,
}

impl MirPayDeepLink {
    /// DeepLink для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

impl_request!(MirPayDeepLink => MirPayDeepLinkResponse, "v2/MirPay/GetDeepLink");

response! {
    /// Ответ `MirPay/GetDeepLink`.
    pub struct MirPayDeepLinkResponse {
        /// DeepLink, подписанный JWT.
        #[serde(default)]
        pub deeplink: Option<String>,
    }
}

// ---------------------------------------------------------------- Alfa Pay

/// Ссылка на оплату через Alfa Pay
/// ([`v2/AlfaPay/link/get`](https://developer.tbank.ru/eacq/api/alfa-pay-link)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AlfaPayLink {
    /// Идентификатор платежа.
    pub payment_id: String,
}

impl AlfaPayLink {
    /// Ссылка для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

impl_request!(AlfaPayLink => LinkResponse, "v2/AlfaPay/link/get");
