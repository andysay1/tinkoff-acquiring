//! Покупатели и сохранённые карты: `AddCustomer`, `GetCustomer`,
//! `RemoveCustomer`, `AddCard`, `AttachCard`, `GetAddCardState`,
//! `GetCardList`, `RemoveCard`.

use crate::enums::{AddCardStatus, CardStatus, CardType, CheckType, DeviceChannel};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

// ---------------------------------------------------------------- AddCustomer

/// Зарегистрировать покупателя
/// ([`v2/AddCustomer`](https://developer.tbank.ru/eacq/api/add-customer)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AddCustomer {
    /// Идентификатор покупателя в системе мерчанта.
    pub customer_key: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Email покупателя.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Телефон покупателя (`+79991234567`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

impl AddCustomer {
    /// Покупатель `customer_key`.
    pub fn new(customer_key: impl Into<String>) -> Self {
        Self {
            customer_key: customer_key.into(),
            ip: None,
            email: None,
            phone: None,
        }
    }
}

setters!(AddCustomer {
    /// IP-адрес покупателя.
    ip: String,
    /// Email покупателя.
    email: String,
    /// Телефон покупателя.
    phone: String,
});

impl_request!(AddCustomer => CustomerResponse, "v2/AddCustomer");

response! {
    /// Ответ `AddCustomer`, `GetCustomer`, `RemoveCustomer`.
    pub struct CustomerResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор покупателя.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub customer_key: String,
        /// Email покупателя (`GetCustomer`).
        #[serde(default)]
        pub email: Option<String>,
        /// Телефон покупателя (`GetCustomer`).
        #[serde(default)]
        pub phone: Option<String>,
    }
}

/// Получить данные покупателя
/// ([`v2/GetCustomer`](https://developer.tbank.ru/eacq/api/get-customer)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetCustomer {
    /// Идентификатор покупателя.
    pub customer_key: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
}

impl GetCustomer {
    /// Покупатель `customer_key`.
    pub fn new(customer_key: impl Into<String>) -> Self {
        Self {
            customer_key: customer_key.into(),
            ip: None,
        }
    }
}

setters!(GetCustomer {
    /// IP-адрес покупателя.
    ip: String,
});

impl_request!(GetCustomer => CustomerResponse, "v2/GetCustomer", {
    fn retry_safe(&self) -> bool { true }
});

/// Удалить покупателя
/// ([`v2/RemoveCustomer`](https://developer.tbank.ru/eacq/api/remove-customer)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct RemoveCustomer {
    /// Идентификатор покупателя.
    pub customer_key: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
}

impl RemoveCustomer {
    /// Покупатель `customer_key`.
    pub fn new(customer_key: impl Into<String>) -> Self {
        Self {
            customer_key: customer_key.into(),
            ip: None,
        }
    }
}

setters!(RemoveCustomer {
    /// IP-адрес покупателя.
    ip: String,
});

impl_request!(RemoveCustomer => CustomerResponse, "v2/RemoveCustomer");

// ---------------------------------------------------------------- AddCard

/// Начать привязку карты на форме банка
/// ([`v2/AddCard`](https://developer.tbank.ru/eacq/api/add-card)).
///
/// Перенаправьте покупателя на `PaymentURL` из ответа. Результат придёт
/// уведомлением `NotificationType=LINKCARD` и доступен через [`GetAddCardState`].
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AddCard {
    /// Идентификатор покупателя (сначала [`AddCustomer`]).
    pub customer_key: String,
    /// Проверка карты; по умолчанию `NO`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_type: Option<CheckType>,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// `true` — только карты РФ, `false` — только зарубежные, не задано — любые.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resident_state: Option<bool>,
}

impl AddCard {
    /// Привязка карты покупателю `customer_key`.
    pub fn new(customer_key: impl Into<String>) -> Self {
        Self {
            customer_key: customer_key.into(),
            check_type: None,
            ip: None,
            resident_state: None,
        }
    }
}

setters!(AddCard {
    /// Проверка карты.
    check_type: CheckType,
    /// IP-адрес покупателя.
    ip: String,
    /// Резидентность карты.
    resident_state: bool,
});

impl_request!(AddCard => AddCardResponse, "v2/AddCard");

response! {
    /// Ответ `AddCard`.
    pub struct AddCardResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор покупателя.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub customer_key: String,
        /// Идентификатор запроса на привязку.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub request_key: String,
        /// Идентификатор платежа (проверочного списания).
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub payment_id: Option<String>,
        /// Ссылка на форму привязки.
        #[serde(rename = "PaymentURL", default)]
        pub payment_url: Option<String>,
    }
}

// ---------------------------------------------------------------- AttachCard

/// Привязать карту через свою форму (PCI DSS)
/// ([`v2/AttachCard`](https://developer.tbank.ru/eacq/api/attach-card)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AttachCard {
    /// Идентификатор запроса из ответа [`AddCard`].
    pub request_key: String,
    /// Данные карты, зашифрованные открытым ключом терминала, в Base64
    /// (см. [`CardData`](crate::CardData)).
    pub card_data: String,
    /// Канал устройства для 3DS.
    #[serde(rename = "deviceChannel")]
    pub device_channel: DeviceChannel,
    /// Дополнительные параметры, для 3DS v2 — данные браузера
    /// ([`ThreeDsBrowserData`](crate::ThreeDsBrowserData)).
    #[serde(rename = "DATA", skip_serializing_if = "Map::is_empty")]
    pub data: Map<String, Value>,
}

impl AttachCard {
    /// Привязка карты по `request_key` с зашифрованными данными `card_data`.
    pub fn new(request_key: impl Into<String>, card_data: impl Into<String>) -> Self {
        Self {
            request_key: request_key.into(),
            card_data: card_data.into(),
            device_channel: DeviceChannel::Browser,
            data: Map::new(),
        }
    }

    /// Канал устройства.
    #[must_use]
    pub fn device_channel(mut self, v: DeviceChannel) -> Self {
        self.device_channel = v;
        self
    }

    /// Параметры `DATA`.
    #[must_use]
    pub fn data(mut self, data: impl Into<Map<String, Value>>) -> Self {
        self.data = data.into();
        self
    }
}

impl_request!(AttachCard => AttachCardResponse, "v2/AttachCard");

response! {
    /// Ответ `AttachCard`.
    pub struct AttachCardResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор покупателя.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub customer_key: String,
        /// Идентификатор запроса на привязку.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub request_key: String,
        /// Идентификатор карты.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub card_id: Option<String>,
        /// Статус привязки.
        #[serde(default)]
        pub status: Option<AddCardStatus>,
        /// Идентификатор реквизитов для `Charge`.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub rebill_id: Option<String>,
        /// Адрес ACS для 3DS (при `3DS_CHECKING`).
        #[serde(rename = "ACSUrl", default)]
        pub acs_url: Option<String>,
        /// `MD` для 3DS v1.
        #[serde(rename = "MD", default)]
        pub md: Option<String>,
        /// `PaReq` для 3DS v1.
        #[serde(default)]
        pub pa_req: Option<String>,
        /// Идентификатор 3DS Server (3DS v2, для [`three_ds_creq`](crate::three_ds_creq)).
        #[serde(default)]
        pub tds_server_trans_id: Option<String>,
        /// Идентификатор ACS (3DS v2).
        #[serde(default)]
        pub acs_trans_id: Option<String>,
    }
}

// ---------------------------------------------------------------- GetAddCardState

/// Статус привязки карты
/// ([`v2/GetAddCardState`](https://developer.tbank.ru/eacq/api/get-add-card-state)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetAddCardState {
    /// Идентификатор запроса на привязку.
    pub request_key: String,
}

impl GetAddCardState {
    /// Статус привязки `request_key`.
    pub fn new(request_key: impl Into<String>) -> Self {
        Self {
            request_key: request_key.into(),
        }
    }
}

impl_request!(GetAddCardState => GetAddCardStateResponse, "v2/GetAddCardState", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Ответ `GetAddCardState`.
    pub struct GetAddCardStateResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор запроса на привязку.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub request_key: String,
        /// Статус привязки.
        pub status: AddCardStatus,
        /// Идентификатор карты.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub card_id: Option<String>,
        /// Идентификатор реквизитов для `Charge`.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub rebill_id: Option<String>,
        /// Идентификатор покупателя.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub customer_key: Option<String>,
    }
}

// ---------------------------------------------------------------- GetCardList

/// Список карт покупателя
/// ([`v2/GetCardList`](https://developer.tbank.ru/eacq/api/get-card-list)).
///
/// Ответ — массив карт, а не объект.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetCardList {
    /// Идентификатор покупателя.
    pub customer_key: String,
    /// Только карты, сохранённые для оплаты в один клик.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_card: Option<bool>,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
}

impl GetCardList {
    /// Карты покупателя `customer_key`.
    pub fn new(customer_key: impl Into<String>) -> Self {
        Self {
            customer_key: customer_key.into(),
            saved_card: None,
            ip: None,
        }
    }
}

setters!(GetCardList {
    /// Только карты для оплаты в один клик.
    saved_card: bool,
    /// IP-адрес покупателя.
    ip: String,
});

impl_request!(GetCardList => Vec<Card>, "v2/GetCardList", {
    fn retry_safe(&self) -> bool { true }
});

/// Сохранённая карта.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Card {
    /// Идентификатор карты.
    #[serde(deserialize_with = "crate::de::string")]
    pub card_id: String,
    /// Маскированный номер карты.
    #[serde(default)]
    pub pan: String,
    /// `A` — активная, `D` — удалённая.
    pub status: CardStatus,
    /// Идентификатор реквизитов для `Charge`.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub rebill_id: Option<String>,
    /// Тип карты.
    #[serde(default)]
    pub card_type: Option<CardType>,
    /// Срок действия `MMYY`.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub exp_date: Option<String>,
    /// Поля, которых нет в этой версии библиотеки.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

// ---------------------------------------------------------------- RemoveCard

/// Удалить карту покупателя
/// ([`v2/RemoveCard`](https://developer.tbank.ru/eacq/api/remove-card)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct RemoveCard {
    /// Идентификатор покупателя.
    pub customer_key: String,
    /// Идентификатор карты.
    pub card_id: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
}

impl RemoveCard {
    /// Удалить карту `card_id` покупателя `customer_key`.
    pub fn new(customer_key: impl Into<String>, card_id: impl Into<String>) -> Self {
        Self {
            customer_key: customer_key.into(),
            card_id: card_id.into(),
            ip: None,
        }
    }
}

setters!(RemoveCard {
    /// IP-адрес покупателя.
    ip: String,
});

impl_request!(RemoveCard => RemoveCardResponse, "v2/RemoveCard");

response! {
    /// Ответ `RemoveCard`.
    pub struct RemoveCardResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Статус карты (`D`).
        #[serde(default)]
        pub status: Option<CardStatus>,
        /// Идентификатор покупателя.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub customer_key: String,
        /// Идентификатор карты.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub card_id: String,
        /// Тип карты.
        #[serde(default)]
        pub card_type: Option<CardType>,
    }
}
