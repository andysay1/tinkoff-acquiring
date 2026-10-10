//! Проведение платежа: `Init`, `Confirm`, `Cancel`, `Charge`, `GetState`,
//! `CheckOrder`, `SendClosingReceipt`, `getConfirmOperation`.

use crate::enums::{Language, OperationInitiatorType, PayType, PaymentStatus, Route, Source};
use crate::receipt::Receipt;
use crate::request::Signing;
use crate::Amount;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Распределение суммы между магазинами маркетплейса.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Shop {
    /// Код магазина (`Submerchant_ID`).
    pub shop_code: String,
    /// Сумма в копейках, относящаяся к магазину.
    pub amount: Amount,
    /// Наименование товара.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Комиссия маркетплейса в копейках (только в `Init`/`Confirm`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<String>,
}

impl Shop {
    /// Новая строка распределения.
    pub fn new(shop_code: impl Into<String>, amount: Amount) -> Self {
        Self {
            shop_code: shop_code.into(),
            amount,
            name: None,
            fee: None,
        }
    }
}

setters!(Shop {
    /// Наименование товара.
    name: String,
    /// Комиссия маркетплейса в копейках.
    fee: String,
});

/// Пара ключ–значение из `Params` ответа `GetState`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Param {
    /// `Route`, `Source`, `CreditAmount`, `DrPaymentId`, …
    #[serde(deserialize_with = "crate::de::string")]
    pub key: String,
    /// Значение.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub value: String,
}

// ---------------------------------------------------------------- Init

/// Инициировать платёж ([`v2/Init`](https://developer.tbank.ru/eacq/api/init)).
///
/// ```
/// use tinkoff_acquiring::{Init, ItemFfd12, ReceiptFfd12, Tax, Taxation};
///
/// let init = Init::new(199_900, "order-42")
///     .description("Худи")
///     .receipt(
///         ReceiptFfd12::new(Taxation::UsnIncome, vec![ItemFfd12::new("Худи", 199_900, 1.0, Tax::None)])
///             .email("buyer@example.com"),
///     );
/// assert!(tinkoff_acquiring::Request::validate(&init).is_ok());
/// ```
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Init {
    /// Сумма в копейках. Должна совпадать с суммой позиций чека.
    pub amount: Amount,
    /// Идентификатор заказа (до 50 символов, для цифрового рубля — до 35).
    pub order_id: String,
    /// Описание заказа (до 140 символов, для СБП — до 140, отображается на форме).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Идентификатор покупателя в системе мерчанта.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_key: Option<String>,
    /// `"Y"` — родительский рекуррентный платёж (сохранить карту/счёт).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrent: Option<String>,
    /// Одно- или двухстадийная оплата.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pay_type: Option<PayType>,
    /// Язык платёжной формы.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    /// URL для HTTP-уведомлений (иначе берётся из настроек терминала).
    #[serde(rename = "NotificationURL", skip_serializing_if = "Option::is_none")]
    pub notification_url: Option<String>,
    /// Куда вернуть покупателя после успешной оплаты.
    #[serde(rename = "SuccessURL", skip_serializing_if = "Option::is_none")]
    pub success_url: Option<String>,
    /// Куда вернуть покупателя после неуспешной оплаты.
    #[serde(rename = "FailURL", skip_serializing_if = "Option::is_none")]
    pub fail_url: Option<String>,
    /// Срок жизни ссылки/QR, `YYYY-MM-DDTHH24:MI:SS+GMT`, например
    /// `2026-10-31T23:59:59+03:00`. По умолчанию 24 часа, максимум 90 дней.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_due_date: Option<String>,
    /// Дополнительные параметры `DATA` (до 20 пар, строковое значение — до
    /// 100 символов).
    #[serde(rename = "DATA", skip_serializing_if = "Map::is_empty")]
    pub data: Map<String, Value>,
    /// Чек. Обязателен, если к терминалу подключена онлайн-касса.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    /// Распределение по магазинам (маркетплейсы).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shops: Option<Vec<Shop>>,
}

impl Init {
    /// Платёж на `amount` копеек по заказу `order_id`.
    pub fn new(amount: Amount, order_id: impl Into<String>) -> Self {
        Self {
            amount,
            order_id: order_id.into(),
            description: None,
            customer_key: None,
            recurrent: None,
            pay_type: None,
            language: None,
            notification_url: None,
            success_url: None,
            fail_url: None,
            redirect_due_date: None,
            data: Map::new(),
            receipt: None,
            shops: None,
        }
    }

    /// Родительский рекуррентный платёж (`Recurrent=Y`): после оплаты в
    /// уведомлении придёт `RebillId` для [`Charge`]. Для карт также нужен
    /// [`customer_key`](Self::customer_key).
    #[must_use]
    pub fn recurrent(mut self) -> Self {
        self.recurrent = Some("Y".into());
        self
    }

    /// Двухстадийная оплата: деньги блокируются, списываются через [`Confirm`].
    #[must_use]
    pub fn two_stage(mut self) -> Self {
        self.pay_type = Some(PayType::TwoStage);
        self
    }

    /// Чек.
    #[must_use]
    pub fn receipt(mut self, receipt: impl Into<Receipt>) -> Self {
        self.receipt = Some(receipt.into());
        self
    }

    /// Параметр в `DATA`.
    #[must_use]
    pub fn data(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    /// Признак инициатора операции (`DATA.OperationInitiatorType`).
    #[must_use]
    pub fn operation_initiator_type(self, v: OperationInitiatorType) -> Self {
        self.data("OperationInitiatorType", v.as_str())
    }
}

setters!(Init {
    /// Описание заказа.
    description: String,
    /// Идентификатор покупателя.
    customer_key: String,
    /// Тип оплаты.
    pay_type: PayType,
    /// Язык формы.
    language: Language,
    /// URL уведомлений.
    notification_url: String,
    /// URL успешной оплаты.
    success_url: String,
    /// URL неуспешной оплаты.
    fail_url: String,
    /// Срок жизни ссылки, например `2026-10-31T23:59:59+03:00`.
    redirect_due_date: String,
    /// Магазины маркетплейса.
    shops: Vec<Shop>,
});

fn check_len(field: &str, value: &str, max: usize) -> Result<(), String> {
    let n = value.chars().count();
    if n > max {
        return Err(format!("{field}: {n} символов, максимум {max}"));
    }
    Ok(())
}

impl_request!(Init => InitResponse, "v2/Init", {
    fn validate(&self) -> Result<(), String> {
        if self.amount == 0 {
            return Err("Amount должен быть больше нуля".into());
        }
        if self.order_id.is_empty() {
            return Err("OrderId пустой".into());
        }
        check_len("OrderId", &self.order_id, 50)?;
        if let Some(d) = &self.description {
            check_len("Description", d, 140)?;
        }
        if self.data.len() > 20 {
            return Err(format!("DATA: {} пар, максимум 20", self.data.len()));
        }
        for (k, v) in &self.data {
            if let Value::String(s) = v {
                check_len(&format!("DATA.{k}"), s, 100)?;
            }
        }
        if let Some(r) = &self.receipt {
            r.validate_for_amount(self.amount)?;
        }
        Ok(())
    }
});

response! {
    /// Ответ `Init`.
    pub struct InitResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Сумма в копейках.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub amount: Option<Amount>,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// Статус платежа (обычно `NEW`).
        pub status: Option<PaymentStatus>,
        /// Идентификатор платежа в Т-Банке.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
        /// Ссылка на платёжную форму.
        #[serde(rename = "PaymentURL", default)]
        pub payment_url: Option<String>,
    }
}

// ---------------------------------------------------------------- GetState

/// Получить статус платежа ([`v2/GetState`](https://developer.tbank.ru/eacq/api/get-state)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetState {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Вернуть телефон покупателя (СБП).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_phone: Option<bool>,
}

impl GetState {
    /// Статус платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            ip: None,
            get_phone: None,
        }
    }
}

setters!(GetState {
    /// IP-адрес покупателя.
    ip: String,
    /// Вернуть телефон покупателя.
    get_phone: bool,
});

impl_request!(GetState => GetStateResponse, "v2/GetState", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Ответ `GetState`.
    pub struct GetStateResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Сумма в копейках.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub amount: Option<Amount>,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// Статус платежа.
        pub status: PaymentStatus,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
        /// Идентификатор сохранённых реквизитов для `Charge`.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub rebill_id: Option<String>,
        /// Способ оплаты и детали (`Route`, `Source`, `CreditAmount`, …).
        #[serde(default)]
        pub params: Vec<Param>,
    }
}

impl GetStateResponse {
    /// Значение из `Params` по ключу, например `"Source"`.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.value.as_str())
    }
}

// ---------------------------------------------------------------- Confirm

/// Подтвердить списание при двухстадийной оплате
/// ([`v2/Confirm`](https://developer.tbank.ru/eacq/api/confirm)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Confirm {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Сумма списания в копейках (по умолчанию — сумма `Init`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    /// Чек (если сумма или состав отличаются от `Init`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    /// Распределение по магазинам.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shops: Option<Vec<Shop>>,
    /// Способ платежа (рассрочка, «Долями»).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<Route>,
    /// Источник платежа (рассрочка, «Долями»).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

impl Confirm {
    /// Подтвердить платёж `payment_id` на полную сумму.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            ip: None,
            amount: None,
            receipt: None,
            shops: None,
            route: None,
            source: None,
        }
    }

    /// Чек.
    #[must_use]
    pub fn receipt(mut self, receipt: impl Into<Receipt>) -> Self {
        self.receipt = Some(receipt.into());
        self
    }
}

setters!(Confirm {
    /// IP-адрес покупателя.
    ip: String,
    /// Сумма списания в копейках.
    amount: exact Amount,
    /// Магазины маркетплейса.
    shops: Vec<Shop>,
    /// Способ платежа.
    route: Route,
    /// Источник платежа.
    source: Source,
});

impl_request!(Confirm => ConfirmResponse, "v2/Confirm", {
    fn validate(&self) -> Result<(), String> {
        match &self.receipt {
            Some(r) => r.validate(),
            None => Ok(()),
        }
    }
});

response! {
    /// Ответ `Confirm`.
    pub struct ConfirmResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// Статус платежа (`CONFIRMED`).
        pub status: PaymentStatus,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
        /// Детали платежа.
        #[serde(default)]
        pub params: Vec<Param>,
    }
}

// ---------------------------------------------------------------- Cancel

/// Отменить платёж или вернуть деньги
/// ([`v2/Cancel`](https://developer.tbank.ru/eacq/api/cancel)).
///
/// Передайте [`external_request_id`](Self::external_request_id), чтобы
/// повтор запроса не создал второй возврат: тогда клиент сам повторяет
/// запрос при таймауте.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Cancel {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Сумма отмены в копейках (по умолчанию — полная).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    /// Чек возврата (обязателен при частичной отмене с онлайн-кассой).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    /// Распределение по магазинам.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shops: Option<Vec<Shop>>,
    /// Банк для возврата по СБП (из [`QrMembersList`](crate::QrMembersList)).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_member_id: Option<String>,
    /// Способ платежа (рассрочка, «Долями»).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<Route>,
    /// Источник платежа (рассрочка, «Долями»).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Ключ идемпотентности. Для «Долями» — UUID v4, для Alfa Pay — до 30 символов.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_request_id: Option<String>,
}

impl Cancel {
    /// Отменить платёж `payment_id` на полную сумму.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            ip: None,
            amount: None,
            receipt: None,
            shops: None,
            qr_member_id: None,
            route: None,
            source: None,
            external_request_id: None,
        }
    }

    /// Чек возврата.
    #[must_use]
    pub fn receipt(mut self, receipt: impl Into<Receipt>) -> Self {
        self.receipt = Some(receipt.into());
        self
    }
}

setters!(Cancel {
    /// IP-адрес покупателя.
    ip: String,
    /// Сумма отмены в копейках.
    amount: exact Amount,
    /// Магазины маркетплейса.
    shops: Vec<Shop>,
    /// Банк для возврата по СБП.
    qr_member_id: String,
    /// Способ платежа.
    route: Route,
    /// Источник платежа.
    source: Source,
    /// Ключ идемпотентности.
    external_request_id: String,
});

impl_request!(Cancel => CancelResponse, "v2/Cancel", {
    fn retry_safe(&self) -> bool {
        // Пустой ExternalRequestId банк обрабатывает без проверки дублей.
        self.external_request_id
            .as_deref()
            .is_some_and(|id| !id.trim().is_empty())
    }
    fn validate(&self) -> Result<(), String> {
        match &self.receipt {
            Some(r) => r.validate(),
            None => Ok(()),
        }
    }
});

response! {
    /// Ответ `Cancel`.
    pub struct CancelResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// Статус после отмены: `CANCELED`, `REVERSED`, `PARTIAL_REVERSED`,
        /// `REFUNDED`, `PARTIAL_REFUNDED`, `ASYNC_REFUNDING`, …
        pub status: PaymentStatus,
        /// Сумма до отмены, копейки.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub original_amount: Option<Amount>,
        /// Сумма после отмены, копейки.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub new_amount: Option<Amount>,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
        /// Ключ идемпотентности из запроса.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub external_request_id: Option<String>,
    }
}

// ---------------------------------------------------------------- Charge

/// Списать по сохранённым реквизитам
/// ([`v2/Charge`](https://developer.tbank.ru/eacq/api/charge)).
///
/// Сначала создайте платёж через [`Init`] (без `Recurrent`), затем вызовите
/// `Charge` с его `PaymentId` и `RebillId` родительского платежа.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Charge {
    /// Идентификатор нового платежа из `Init`.
    pub payment_id: String,
    /// `RebillId` родительского платежа.
    pub rebill_id: String,
    /// IP-адрес покупателя.
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Отправить покупателю письмо об оплате.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_email: Option<bool>,
    /// Email для письма (обязателен при `SendEmail=true`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_email: Option<String>,
}

impl Charge {
    /// Списание по платежу `payment_id` с реквизитами `rebill_id`.
    pub fn new(payment_id: impl Into<String>, rebill_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            rebill_id: rebill_id.into(),
            ip: None,
            send_email: None,
            info_email: None,
        }
    }
}

setters!(Charge {
    /// IP-адрес покупателя.
    ip: String,
    /// Отправить письмо об оплате.
    send_email: bool,
    /// Email для письма.
    info_email: String,
});

impl_request!(Charge => ChargeResponse, "v2/Charge", {
    fn validate(&self) -> Result<(), String> {
        if self.send_email == Some(true) && self.info_email.is_none() {
            return Err("при SendEmail=true нужен InfoEmail".into());
        }
        Ok(())
    }
});

response! {
    /// Ответ `Charge`.
    pub struct ChargeResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Сумма в копейках.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub amount: Option<Amount>,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// `CONFIRMED`, `AUTHORIZED` (двухстадийная) или `REJECTED`.
        pub status: PaymentStatus,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
    }
}

// ---------------------------------------------------------------- CheckOrder

/// Статус всех платежей заказа
/// ([`v2/CheckOrder`](https://developer.tbank.ru/eacq/api/check-order)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct CheckOrder {
    /// Идентификатор заказа.
    pub order_id: String,
}

impl CheckOrder {
    /// Платежи заказа `order_id`.
    pub fn new(order_id: impl Into<String>) -> Self {
        Self {
            order_id: order_id.into(),
        }
    }
}

impl_request!(CheckOrder => CheckOrderResponse, "v2/CheckOrder", {
    fn retry_safe(&self) -> bool { true }
});

response! {
    /// Ответ `CheckOrder`.
    pub struct CheckOrderResponse {
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// Платежи заказа.
        #[serde(default)]
        pub payments: Vec<OrderPayment>,
    }
}

/// Платёж в ответе `CheckOrder`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct OrderPayment {
    /// Идентификатор платежа.
    #[serde(deserialize_with = "crate::de::string")]
    pub payment_id: String,
    /// Сумма в копейках.
    #[serde(default, deserialize_with = "crate::de::opt_u64")]
    pub amount: Option<Amount>,
    /// Статус платежа.
    pub status: PaymentStatus,
    /// RRN операции.
    #[serde(rename = "RRN", default, deserialize_with = "crate::de::opt_string")]
    pub rrn: Option<String>,
    /// Успешность.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub success: bool,
    /// Код ошибки.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub error_code: String,
    /// Сообщение.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub message: Option<String>,
    /// Идентификатор платежа в СБП.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub sbp_payment_id: Option<String>,
    /// Хэш телефона покупателя (СБП).
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub sbp_customer_id: Option<String>,
    /// Идентификатор операции в ЦБ (цифровой рубль).
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub dr_payment_id: Option<String>,
    /// Дата исполнения распоряжения ЦБ (цифровой рубль).
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub dr_payment_settlement_date: Option<String>,
    /// Счёт цифрового рубля плательщика.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub participant_wallet_id: Option<String>,
    /// Поля, которых нет в этой версии библиотеки.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

// ---------------------------------------------------------------- SendClosingReceipt

/// Отправить закрывающий чек в кассу, например после предоплаты
/// ([`cashbox/SendClosingReceipt`](https://developer.tbank.ru/eacq/api/send-closing-receipt)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct SendClosingReceipt {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// Закрывающий чек.
    pub receipt: Receipt,
}

impl SendClosingReceipt {
    /// Закрывающий чек по платежу `payment_id`.
    pub fn new(payment_id: impl Into<String>, receipt: impl Into<Receipt>) -> Self {
        Self {
            payment_id: payment_id.into(),
            receipt: receipt.into(),
        }
    }
}

impl_request!(SendClosingReceipt => SendClosingReceiptResponse, "cashbox/SendClosingReceipt", {
    fn validate(&self) -> Result<(), String> {
        self.receipt.validate()
    }
});

response! {
    /// Ответ `SendClosingReceipt`.
    pub struct SendClosingReceiptResponse {}
}

// ---------------------------------------------------------------- getConfirmOperation

/// Запросить справку об операциях
/// ([`v2/getConfirmOperation`](https://developer.tbank.ru/eacq/api/get-confirm-operation)).
///
/// Справки придут на почту или на ваш сервис по `CallbackUrl`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct GetConfirmOperation {
    /// Идентификаторы платежей.
    pub payment_id_list: Vec<u64>,
    /// URL сервиса для получения справок (подключается через поддержку).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    /// До трёх адресов почты.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_list: Option<Vec<String>>,
}

impl GetConfirmOperation {
    /// Справки на почту.
    pub fn by_email(payment_ids: Vec<u64>, emails: Vec<String>) -> Self {
        Self {
            payment_id_list: payment_ids,
            callback_url: None,
            email_list: Some(emails),
        }
    }

    /// Справки на ваш сервис.
    pub fn by_callback(payment_ids: Vec<u64>, callback_url: impl Into<String>) -> Self {
        Self {
            payment_id_list: payment_ids,
            callback_url: Some(callback_url.into()),
            email_list: None,
        }
    }
}

impl_request!(GetConfirmOperation => GetConfirmOperationResponse, "v2/getConfirmOperation", {
    const SIGNING: Signing = Signing::TerminalKeyOnly;
    fn validate(&self) -> Result<(), String> {
        if self.payment_id_list.is_empty() {
            return Err("PaymentIdList пустой".into());
        }
        if self.email_list.as_ref().is_some_and(|e| e.len() > 3) {
            return Err("EmailList: максимум три адреса".into());
        }
        Ok(())
    }
});

response! {
    /// Ответ `getConfirmOperation`.
    pub struct GetConfirmOperationResponse {
        /// Результат по каждому платежу.
        #[serde(default)]
        pub payment_id_list: Vec<ConfirmOperationItem>,
    }
}

/// Результат запроса справки по одному платежу.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct ConfirmOperationItem {
    /// Идентификатор платежа.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub payment_id: String,
    /// Успешность.
    #[serde(default, deserialize_with = "crate::de::bool_lenient")]
    pub success: bool,
    /// Код ошибки.
    #[serde(default, deserialize_with = "crate::de::string")]
    pub error_code: String,
    /// Сервисное сообщение.
    #[serde(default, deserialize_with = "crate::de::opt_string")]
    pub message: Option<String>,
}
