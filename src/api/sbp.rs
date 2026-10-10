//! СБП и цифровой рубль: `GetQr`, `QrMembersList`, `GetQrState`,
//! `GetQrBankList`, `SbpPayTest` и автоплатежи по привязанному счёту
//! (`AddAccountQr`, `GetAddAccountQrState`, `GetAccountQrList`, `ChargeQr`).

use crate::enums::{
    AccountQrStatus, DeviceType, PaymentStatus, QrDataType, QrPaymentMethod, QrScenario,
};
use crate::Amount;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// ---------------------------------------------------------------- GetQr

/// Сформировать QR или ссылку для оплаты по СБП / цифровым рублём
/// ([`v2/GetQr`](https://developer.tbank.ru/eacq/api/get-qr)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetQr {
    /// Идентификатор платежа из `Init`.
    pub payment_id: String,
    /// `PAYLOAD` — ссылка, `IMAGE` — SVG.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<QrDataType>,
    /// Банк покупателя (из [`GetQrBankList`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_id: Option<String>,
    /// `SBP` или `DR` (цифровой рубль).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<QrPaymentMethod>,
}

impl GetQr {
    /// QR для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            data_type: None,
            bank_id: None,
            payment_method: None,
        }
    }
}

setters!(GetQr {
    /// Формат данных.
    data_type: QrDataType,
    /// Банк покупателя.
    bank_id: String,
    /// Способ оплаты.
    payment_method: QrPaymentMethod,
});

impl_request!(GetQr => GetQrResponse, "v2/GetQr");

response! {
    /// Ответ `GetQr`.
    pub struct GetQrResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// Payload (ссылка `https://qr.nspk.ru/…`) или SVG — по `DataType`.
        #[serde(default)]
        pub data: String,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
        /// Идентификатор привязки счёта (оплата с привязкой).
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub request_key: Option<String>,
    }
}

// ---------------------------------------------------------------- QrMembersList

/// Банки, в которые можно вернуть платёж по СБП
/// ([`v2/QrMembersList`](https://developer.tbank.ru/eacq/api/qr-members-list)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct QrMembersList {
    /// Идентификатор платежа.
    pub payment_id: String,
}

impl QrMembersList {
    /// Банки для возврата по платежу `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

impl_request!(QrMembersList => QrMembersListResponse, "v2/QrMembersList", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Ответ `QrMembersList`.
    pub struct QrMembersListResponse {
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// Банки (только если возврат возможен).
        #[serde(default)]
        pub members: Vec<QrMember>,
    }
}

/// Банк-участник СБП.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct QrMember {
    /// Идентификатор участника (передаётся в `Cancel.QrMemberId`).
    #[serde(deserialize_with = "crate::de::string")]
    pub member_id: String,
    /// Название банка.
    #[serde(default)]
    pub member_name: String,
    /// Этот банк был получателем платежа.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub is_payee: bool,
}

// ---------------------------------------------------------------- GetQrState

/// Статус возврата по СБП
/// ([`v2/GetQrState`](https://developer.tbank.ru/eacq/api/get-qr-state)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetQrState {
    /// Идентификатор платежа.
    pub payment_id: String,
}

impl GetQrState {
    /// Статус возврата по платежу `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
        }
    }
}

impl_request!(GetQrState => GetQrStateResponse, "v2/GetQrState", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Ответ `GetQrState`.
    pub struct GetQrStateResponse {
        /// Статус платежа.
        #[serde(default)]
        pub status: Option<PaymentStatus>,
        /// Код ошибки возврата от СБП.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub qr_cancel_code: Option<String>,
        /// Описание ошибки возврата.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub qr_cancel_message: Option<String>,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub order_id: Option<String>,
        /// Сумма отмены в копейках.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub amount: Option<Amount>,
    }
}

// ---------------------------------------------------------------- GetQrBankList

/// Устройство покупателя для [`GetQrBankList`].
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Device {
    /// `desktop` или `mobile`.
    #[serde(rename = "Type")]
    pub kind: DeviceType,
    /// ОС устройства.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
}

impl Device {
    /// Устройство типа `kind`.
    pub fn new(kind: DeviceType) -> Self {
        Self { kind, os: None }
    }
}

setters!(Device {
    /// ОС устройства.
    os: String,
});

/// Банки СБП для выбора покупателем
/// ([`v2/GetQrBankList`](https://developer.tbank.ru/eacq/api/get-qr-bank-list)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetQrBankList {
    /// Устройство покупателя.
    pub device: Device,
    /// `qr` — оплата (по умолчанию), `sub` — привязка счёта.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_type: Option<QrScenario>,
    /// `SBP` или `DR`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<QrPaymentMethod>,
}

impl GetQrBankList {
    /// Список банков для устройства.
    pub fn new(device: Device) -> Self {
        Self {
            device,
            scenario_type: None,
            payment_method: None,
        }
    }
}

setters!(GetQrBankList {
    /// Сценарий.
    scenario_type: QrScenario,
    /// Способ оплаты.
    payment_method: QrPaymentMethod,
});

impl_request!(GetQrBankList => GetQrBankListResponse, "v2/GetQrBankList", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Ответ `GetQrBankList`.
    pub struct GetQrBankListResponse {
        /// Банки от НСПК.
        #[serde(default)]
        pub bank_list: Vec<SbpBank>,
    }
}

/// Банк из списка НСПК.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct SbpBank {
    /// Внутренний идентификатор банка (для `GetQr.BankId`).
    #[serde(deserialize_with = "crate::de::string")]
    pub bank_id: String,
    /// Идентификатор банка в НСПК.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub nspk_bank_id: String,
    /// Название банка.
    #[serde(default)]
    pub bank_name: String,
    /// Ссылка на логотип.
    #[serde(default)]
    pub bank_logo: Option<String>,
    /// Порядок сортировки.
    #[serde(default, deserialize_with = "crate::de::opt_u64")]
    pub bank_order: Option<u64>,
}

// ---------------------------------------------------------------- SbpPayTest

/// Эмулировать оплату по СБП на тестовом терминале
/// ([`v2/SbpPayTest`](https://developer.tbank.ru/eacq/api/sbp-pay-test)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct SbpPayTest {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// Эмулировать отказ по таймауту.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_deadline_expired: Option<bool>,
    /// Эмулировать отказ банка.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_rejected: Option<bool>,
}

impl SbpPayTest {
    /// Успешная тестовая оплата платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            is_deadline_expired: None,
            is_rejected: None,
        }
    }
}

setters!(SbpPayTest {
    /// Эмулировать отказ по таймауту.
    is_deadline_expired: bool,
    /// Эмулировать отказ банка.
    is_rejected: bool,
});

impl_request!(SbpPayTest => SbpPayTestResponse, "v2/SbpPayTest");

response! {
    /// Ответ `SbpPayTest`.
    pub struct SbpPayTestResponse {}
}

// ---------------------------------------------------------------- AddAccountQr

/// Привязать счёт покупателя для автоплатежей по СБП
/// ([`v2/AddAccountQr`](https://developer.tbank.ru/eacq/api/add-account-qr)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AddAccountQr {
    /// Описание (отображается покупателю).
    pub description: String,
    /// `PAYLOAD` или `IMAGE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_type: Option<QrDataType>,
    /// Банк покупателя.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_id: Option<String>,
    /// Дополнительные параметры (`Data`).
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub data: Map<String, Value>,
    /// Срок жизни QR, `2026-10-31T23:59:59+03:00`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_due_date: Option<String>,
}

impl AddAccountQr {
    /// Привязка счёта с описанием `description`.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            data_type: None,
            bank_id: None,
            data: Map::new(),
            redirect_due_date: None,
        }
    }

    /// Параметр в `Data`.
    #[must_use]
    pub fn data(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

setters!(AddAccountQr {
    /// Формат данных.
    data_type: QrDataType,
    /// Банк покупателя.
    bank_id: String,
    /// Срок жизни QR.
    redirect_due_date: String,
});

impl_request!(AddAccountQr => AddAccountQrResponse, "v2/AddAccountQr");

response! {
    /// Ответ `AddAccountQr`.
    pub struct AddAccountQrResponse {
        /// Payload или SVG.
        #[serde(default)]
        pub data: String,
        /// Идентификатор запроса на привязку.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub request_key: String,
    }
}

/// Статус привязки счёта
/// ([`v2/GetAddAccountQrState`](https://developer.tbank.ru/eacq/api/get-add-account-qr-state)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetAddAccountQrState {
    /// Идентификатор запроса на привязку.
    pub request_key: String,
}

impl GetAddAccountQrState {
    /// Статус привязки `request_key`.
    pub fn new(request_key: impl Into<String>) -> Self {
        Self {
            request_key: request_key.into(),
        }
    }
}

impl_request!(GetAddAccountQrState => AccountQr, "v2/GetAddAccountQrState", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Привязанный счёт СБП (ответ `GetAddAccountQrState`, элемент
    /// `GetAccountQrList`).
    pub struct AccountQr {
        /// Идентификатор запроса на привязку.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub request_key: String,
        /// Статус привязки.
        pub status: AccountQrStatus,
        /// Идентификатор привязки для [`ChargeQr`].
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub account_token: Option<String>,
        /// Банк покупателя.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub bank_member_id: Option<String>,
        /// Название банка покупателя.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub bank_member_name: Option<String>,
    }
}

/// Все счета, привязанные к терминалу
/// ([`v2/GetAccountQrList`](https://developer.tbank.ru/eacq/api/get-account-qr-list)).
#[derive(Clone, Debug, Default, Serialize)]
#[non_exhaustive]
pub struct GetAccountQrList {}

impl GetAccountQrList {
    /// Запрос списка.
    pub fn new() -> Self {
        Self {}
    }
}

impl_request!(GetAccountQrList => GetAccountQrListResponse, "v2/GetAccountQrList", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Ответ `GetAccountQrList`.
    pub struct GetAccountQrListResponse {
        /// Привязанные счета.
        #[serde(default)]
        pub account_tokens: Vec<AccountQr>,
    }
}

// ---------------------------------------------------------------- ChargeQr

/// Автоплатёж по привязанному счёту СБП
/// ([`v2/ChargeQr`](https://developer.tbank.ru/eacq/api/charge-qr)).
///
/// Как и для карт, сначала нужен новый платёж через [`Init`](crate::Init).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct ChargeQr {
    /// Идентификатор платежа из `Init`.
    pub payment_id: String,
    /// Идентификатор привязки счёта.
    pub account_token: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Отправить письмо об оплате.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_email: Option<bool>,
    /// Email для письма.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_email: Option<String>,
    /// Банк плательщика.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_member_id: Option<String>,
}

impl ChargeQr {
    /// Списание по платежу `payment_id` со счёта `account_token`.
    pub fn new(payment_id: impl Into<String>, account_token: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            account_token: account_token.into(),
            ip: None,
            send_email: None,
            info_email: None,
            bank_member_id: None,
        }
    }
}

setters!(ChargeQr {
    /// IP-адрес покупателя.
    ip: String,
    /// Отправить письмо об оплате.
    send_email: bool,
    /// Email для письма.
    info_email: String,
    /// Банк плательщика.
    bank_member_id: String,
});

impl_request!(ChargeQr => ChargeQrResponse, "v2/ChargeQr", {
    fn validate(&self) -> Result<(), String> {
        if self.send_email == Some(true) && self.info_email.is_none() {
            return Err("при SendEmail=true нужен InfoEmail".into());
        }
        Ok(())
    }
});

response! {
    /// Ответ `ChargeQr`.
    pub struct ChargeQrResponse {
        /// Сумма в копейках.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub amount: Option<Amount>,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// `CONFIRMED`, `REJECTED` или `FORM_SHOWED` (ждём НСПК).
        #[serde(default)]
        pub status: Option<PaymentStatus>,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
        /// Код валюты ISO 4217.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub currency: Option<String>,
    }
}
