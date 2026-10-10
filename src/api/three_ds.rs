//! Своя платёжная форма (PCI DSS) и 3-D Secure: `Check3dsVersion`,
//! `FinishAuthorize`, `Submit3DSAuthorization`, `Submit3DSAuthorizationV2`,
//! а также помощники для шагов, которые выполняет браузер покупателя
//! (3DS Method и CReq).

use crate::enums::{DeviceChannel, PaymentStatus};
use crate::request::Encoding;
use crate::Amount;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Данные карты для полей `CardData` (до шифрования).
///
/// API принимает строку `PAN=…;ExpDate=MMYY;CardHolder=…;CVV=…`,
/// зашифрованную открытым ключом терминала (X509 RSA 2048) и закодированную
/// в Base64. Ключ выдаётся в личном кабинете (раздел «Магазины»).
/// Шифрование оставлено вызывающему коду: [`to_plaintext`](Self::to_plaintext)
/// даёт строку, которую нужно зашифровать.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct CardData {
    /// Номер карты.
    pub pan: String,
    /// Срок действия `MMYY`.
    pub exp_date: String,
    /// Имя держателя.
    pub card_holder: Option<String>,
    /// CVV/CVC.
    pub cvv: Option<String>,
    /// Electronic Commerce Indicator.
    pub eci: Option<String>,
    /// CAVV (оплата токеном).
    pub cavv: Option<String>,
}

impl CardData {
    /// Карта `pan` со сроком `exp_date` (`MMYY`).
    pub fn new(pan: impl Into<String>, exp_date: impl Into<String>) -> Self {
        Self {
            pan: pan.into(),
            exp_date: exp_date.into(),
            ..Self::default()
        }
    }

    /// Строка `ключ=значение;…` для шифрования.
    pub fn to_plaintext(&self) -> String {
        let mut parts = vec![
            format!("PAN={}", self.pan),
            format!("ExpDate={}", self.exp_date),
        ];
        let optional = [
            ("CardHolder", &self.card_holder),
            ("CVV", &self.cvv),
            ("ECI", &self.eci),
            ("CAVV", &self.cavv),
        ];
        for (k, v) in optional {
            if let Some(v) = v {
                parts.push(format!("{k}={v}"));
            }
        }
        parts.join(";")
    }
}

setters!(CardData {
    /// Имя держателя.
    card_holder: String,
    /// CVV/CVC.
    cvv: String,
    /// ECI.
    eci: String,
    /// CAVV.
    cavv: String,
});

impl std::fmt::Debug for CardData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let digits: Vec<char> = self.pan.chars().collect();
        let masked = if digits.len() > 10 {
            let head: String = digits[..6].iter().collect();
            let tail: String = digits[digits.len() - 4..].iter().collect();
            format!("{head}******{tail}")
        } else {
            "***".into()
        };
        f.debug_struct("CardData")
            .field("pan", &masked)
            .field("exp_date", &"**")
            .finish_non_exhaustive()
    }
}

/// Данные браузера для 3DS v2 (`deviceChannel=02`), передаются в `DATA`
/// методов [`FinishAuthorize`] и [`AttachCard`](crate::AttachCard).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ThreeDsBrowserData {
    /// Результат 3DS Method: `Y` — выполнен, `N` — не выполнен, `U` — не требовался.
    pub three_ds_comp_ind: String,
    /// `navigator.language`.
    pub language: String,
    /// `new Date().getTimezoneOffset()`.
    pub timezone: String,
    /// `screen.height`.
    pub screen_height: String,
    /// `screen.width`.
    pub screen_width: String,
    /// URL, на который придёт CRes после Challenge Flow.
    pub cres_callback_url: String,
    /// `screen.colorDepth`.
    pub color_depth: Option<String>,
    /// Поддержка Java браузером.
    pub java_enabled: Option<bool>,
}

impl ThreeDsBrowserData {
    /// Данные браузера: результат 3DS Method (`Y`/`N`/`U`),
    /// `navigator.language`, `getTimezoneOffset()`, `screen.height`,
    /// `screen.width` и URL для CRes.
    pub fn new(
        three_ds_comp_ind: impl Into<String>,
        language: impl Into<String>,
        timezone: impl Into<String>,
        screen_height: impl Into<String>,
        screen_width: impl Into<String>,
        cres_callback_url: impl Into<String>,
    ) -> Self {
        Self {
            three_ds_comp_ind: three_ds_comp_ind.into(),
            language: language.into(),
            timezone: timezone.into(),
            screen_height: screen_height.into(),
            screen_width: screen_width.into(),
            cres_callback_url: cres_callback_url.into(),
            color_depth: None,
            java_enabled: None,
        }
    }

    /// Объект `DATA` с данными браузера.
    pub fn into_data(self) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("threeDSCompInd".into(), self.three_ds_comp_ind.into());
        m.insert("language".into(), self.language.into());
        m.insert("timezone".into(), self.timezone.into());
        m.insert("screen_height".into(), self.screen_height.into());
        m.insert("screen_width".into(), self.screen_width.into());
        m.insert("cresCallbackUrl".into(), self.cres_callback_url.into());
        if let Some(v) = self.color_depth {
            m.insert("colorDepth".into(), v.into());
        }
        if let Some(v) = self.java_enabled {
            m.insert("javaEnabled".into(), v.into());
        }
        m
    }
}

setters!(ThreeDsBrowserData {
    /// `screen.colorDepth`.
    color_depth: String,
    /// Поддержка Java.
    java_enabled: bool,
});

impl From<ThreeDsBrowserData> for Map<String, Value> {
    fn from(d: ThreeDsBrowserData) -> Self {
        d.into_data()
    }
}

/// Значение поля `threeDSMethodData` для скрытой формы, которую браузер
/// отправляет на `ThreeDSMethodURL` из ответа [`Check3dsVersion`].
pub fn three_ds_method_data(three_ds_server_trans_id: &str, notification_url: &str) -> String {
    let payload = json!({
        "threeDSServerTransID": three_ds_server_trans_id,
        "threeDSMethodNotificationURL": notification_url,
    });
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string())
}

/// Значение поля `creq` для формы, которую браузер отправляет на `ACSUrl`
/// (Challenge Flow 3DS v2). `challenge_window_size`: `01`–`05`.
pub fn three_ds_creq(
    three_ds_server_trans_id: &str,
    acs_trans_id: &str,
    message_version: &str,
    challenge_window_size: &str,
) -> String {
    let payload = json!({
        "threeDSServerTransID": three_ds_server_trans_id,
        "acsTransID": acs_trans_id,
        "challengeWindowSize": challenge_window_size,
        "messageType": "CReq",
        "messageVersion": message_version,
    });
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string())
}

// ---------------------------------------------------------------- Check3dsVersion

/// Проверить версию 3DS карты
/// ([`v2/Check3dsVersion`](https://developer.tbank.ru/eacq/api/check-3-ds-version)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Check3dsVersion {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// Зашифрованные данные карты.
    pub card_data: String,
}

impl Check3dsVersion {
    /// Проверка версии 3DS для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>, card_data: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            card_data: card_data.into(),
        }
    }
}

impl_request!(Check3dsVersion => Check3dsVersionResponse, "v2/Check3dsVersion");

response! {
    /// Ответ `Check3dsVersion`.
    pub struct Check3dsVersionResponse {
        /// Версия протокола: `1.0.0`, `2.1.0`, …
        #[serde(default)]
        pub version: String,
        /// Идентификатор 3DS Server (для 3DS v2).
        #[serde(rename = "TdsServerTransID", default)]
        pub tds_server_trans_id: Option<String>,
        /// URL для 3DS Method (для 3DS v2).
        #[serde(rename = "ThreeDSMethodURL", default)]
        pub three_ds_method_url: Option<String>,
        /// Платёжная система карты.
        #[serde(default)]
        pub payment_system: Option<String>,
    }
}

// ---------------------------------------------------------------- FinishAuthorize

/// Подтвердить платёж данными карты со своей формы
/// ([`v2/FinishAuthorize`](https://developer.tbank.ru/eacq/api/finish-authorize)).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct FinishAuthorize {
    /// Идентификатор платежа.
    pub payment_id: String,
    /// Зашифрованные данные карты (не нужны при `EncryptedPaymentData`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_data: Option<String>,
    /// Платёжный токен Apple Pay / Google Pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_payment_data: Option<String>,
    /// Сумма в копейках.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<Amount>,
    /// IP-адрес покупателя (IPv6 — в полном формате).
    #[serde(rename = "IP", skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    /// Отправить письмо об оплате.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_email: Option<bool>,
    /// Email для письма.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_email: Option<String>,
    /// Источник платежа (`cards`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Способ платежа (`ACQ`, …), обязателен для Apple Pay / Google Pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    /// Канал устройства.
    #[serde(rename = "deviceChannel", skip_serializing_if = "Option::is_none")]
    pub device_channel: Option<DeviceChannel>,
    /// `DATA`, для 3DS v2 — [`ThreeDsBrowserData`].
    #[serde(rename = "DATA", skip_serializing_if = "Map::is_empty")]
    pub data: Map<String, Value>,
}

impl FinishAuthorize {
    /// Оплата платежа `payment_id` зашифрованной картой `card_data`.
    pub fn new(payment_id: impl Into<String>, card_data: impl Into<String>) -> Self {
        Self {
            card_data: Some(card_data.into()),
            ..Self::empty(payment_id.into())
        }
    }

    /// Оплата платёжным токеном Apple Pay / Google Pay.
    pub fn with_encrypted_payment_data(
        payment_id: impl Into<String>,
        encrypted_payment_data: impl Into<String>,
        route: impl Into<String>,
    ) -> Self {
        Self {
            encrypted_payment_data: Some(encrypted_payment_data.into()),
            route: Some(route.into()),
            ..Self::empty(payment_id.into())
        }
    }

    fn empty(payment_id: String) -> Self {
        Self {
            payment_id,
            card_data: None,
            encrypted_payment_data: None,
            amount: None,
            ip: None,
            send_email: None,
            info_email: None,
            source: None,
            route: None,
            device_channel: None,
            data: Map::new(),
        }
    }

    /// Параметры `DATA`.
    #[must_use]
    pub fn data(mut self, data: impl Into<Map<String, Value>>) -> Self {
        self.data = data.into();
        self
    }
}

setters!(FinishAuthorize {
    /// Сумма в копейках.
    amount: exact Amount,
    /// IP-адрес покупателя.
    ip: String,
    /// Отправить письмо об оплате.
    send_email: bool,
    /// Email для письма.
    info_email: String,
    /// Источник платежа.
    source: String,
    /// Способ платежа.
    route: String,
    /// Канал устройства.
    device_channel: DeviceChannel,
});

impl_request!(FinishAuthorize => FinishAuthorizeResponse, "v2/FinishAuthorize", {
    fn validate(&self) -> Result<(), String> {
        if self.card_data.is_none() && self.encrypted_payment_data.is_none() {
            return Err("нужен CardData или EncryptedPaymentData".into());
        }
        if self.send_email == Some(true) && self.info_email.is_none() {
            return Err("при SendEmail=true нужен InfoEmail".into());
        }
        Ok(())
    }
});

response! {
    /// Ответ `FinishAuthorize`. При `Status = 3DS_CHECKING` заполнены поля для
    /// перехода на ACS (`ACSUrl` и `MD`/`PaReq` для v1 или
    /// `TdsServerTransId`/`AcsTransId` для v2).
    pub struct FinishAuthorizeResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Сумма в копейках.
        #[serde(default, deserialize_with = "crate::de::opt_u64")]
        pub amount: Option<Amount>,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// `CONFIRMED`, `AUTHORIZED`, `3DS_CHECKING` или `REJECTED`.
        pub status: PaymentStatus,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
        /// Идентификатор реквизитов для `Charge`.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub rebill_id: Option<String>,
        /// Идентификатор карты.
        #[serde(default, deserialize_with = "crate::de::opt_string")]
        pub card_id: Option<String>,
        /// Адрес ACS.
        #[serde(rename = "ACSUrl", default)]
        pub acs_url: Option<String>,
        /// `MD` (3DS v1).
        #[serde(rename = "MD", default)]
        pub md: Option<String>,
        /// `PaReq` (3DS v1).
        #[serde(default)]
        pub pa_req: Option<String>,
        /// Идентификатор 3DS Server (3DS v2).
        #[serde(default)]
        pub tds_server_trans_id: Option<String>,
        /// Идентификатор ACS (3DS v2).
        #[serde(default)]
        pub acs_trans_id: Option<String>,
        /// Тип интерфейса ACS (3DS v2 APP).
        #[serde(default)]
        pub acs_interface: Option<String>,
        /// Шаблон интерфейса ACS (3DS v2 APP).
        #[serde(default)]
        pub acs_ui_template: Option<String>,
        /// JWS-объект ACS (3DS v2 APP).
        #[serde(default)]
        pub acs_signed_content: Option<String>,
        /// Идентификатор EMVCo (3DS v2 APP).
        #[serde(default)]
        pub acs_reference_number: Option<String>,
        /// Идентификатор 3DS SDK (3DS v2 APP).
        #[serde(rename = "SdkTransID", default)]
        pub sdk_trans_id: Option<String>,
    }
}

// ---------------------------------------------------------------- Submit3DSAuthorization

/// Подтвердить прохождение 3DS v1.0 после ответа ACS на `TermUrl`
/// ([`v2/Submit3DSAuthorization`](https://developer.tbank.ru/eacq/api/submit-3-ds-authorization)).
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Submit3dsAuthorization {
    /// `MD` из ответа ACS.
    #[serde(rename = "MD")]
    pub md: String,
    /// `PaRes` из ответа ACS.
    #[serde(rename = "PaRes")]
    pub pa_res: String,
    /// Идентификатор платежа.
    #[serde(rename = "PaymentId", skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<String>,
}

impl Submit3dsAuthorization {
    /// Результат 3DS v1.
    pub fn new(md: impl Into<String>, pa_res: impl Into<String>) -> Self {
        Self {
            md: md.into(),
            pa_res: pa_res.into(),
            payment_id: None,
        }
    }
}

setters!(Submit3dsAuthorization {
    /// Идентификатор платежа.
    payment_id: String,
});

impl_request!(Submit3dsAuthorization => Submit3dsResponse, "v2/Submit3DSAuthorization", {
    const ENCODING: Encoding = Encoding::Form;
});

/// Подтвердить прохождение 3DS v2.1 после получения CRes на `cresCallbackUrl`
/// ([`v2/Submit3DSAuthorizationV2`](https://developer.tbank.ru/eacq/api/submit-3-ds-authorization-v-2)).
///
/// В спецификации поля `cres` нет, но API без него отвечает ошибкой 110
/// «Empty cres»: передайте значение, которое ACS прислал на `cresCallbackUrl`
/// (см. [`AcsResult`]).
#[derive(Clone, Debug, Serialize)]
#[non_exhaustive]
pub struct Submit3dsAuthorizationV2 {
    /// Идентификатор платежа.
    #[serde(rename = "PaymentId")]
    pub payment_id: String,
    /// `cres` от ACS (Base64).
    #[serde(rename = "cres", skip_serializing_if = "Option::is_none")]
    pub cres: Option<String>,
}

impl Submit3dsAuthorizationV2 {
    /// Подтверждение 3DS v2 для платежа `payment_id`.
    pub fn new(payment_id: impl Into<String>) -> Self {
        Self {
            payment_id: payment_id.into(),
            cres: None,
        }
    }
}

setters!(Submit3dsAuthorizationV2 {
    /// `cres` от ACS.
    cres: String,
});

impl_request!(Submit3dsAuthorizationV2 => Submit3dsResponse, "v2/Submit3DSAuthorizationV2", {
    const ENCODING: Encoding = Encoding::Form;
});

response! {
    /// Ответ `Submit3DSAuthorization` и `Submit3DSAuthorizationV2`.
    pub struct Submit3dsResponse {
        /// Идентификатор терминала.
        #[serde(default)]
        pub terminal_key: String,
        /// Идентификатор заказа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub order_id: String,
        /// `CONFIRMED`, `AUTHORIZED` или `REJECTED`.
        pub status: PaymentStatus,
        /// Идентификатор платежа.
        #[serde(default, deserialize_with = "crate::de::string")]
        pub payment_id: String,
    }
}

/// Ответ ACS на `TermUrl` / `cresCallbackUrl`, присланный браузером.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AcsResult {
    /// `MD` (3DS v1).
    #[serde(rename = "MD", default)]
    pub md: Option<String>,
    /// `PaRes` (3DS v1).
    #[serde(rename = "PaRes", default)]
    pub pa_res: Option<String>,
    /// `cres` (3DS v2), Base64.
    #[serde(default)]
    pub cres: Option<String>,
}

impl AcsResult {
    /// Расшифрованный `cres` (JSON) для 3DS v2.
    pub fn cres_json(&self) -> Option<Value> {
        let raw = self.cres.as_deref()?.trim();
        let engines = [
            base64::engine::general_purpose::URL_SAFE_NO_PAD,
            base64::engine::general_purpose::STANDARD_NO_PAD,
        ];
        let trimmed = raw.trim_end_matches('=');
        engines
            .iter()
            .find_map(|e| e.decode(trimmed).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}
