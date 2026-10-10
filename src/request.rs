use serde::de::DeserializeOwned;
use serde::Serialize;
use std::borrow::Cow;

/// Как отправляется запрос.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Encoding {
    /// `POST` с JSON-телом.
    Json,
    /// `POST` с телом `application/x-www-form-urlencoded`.
    Form,
    /// `GET` без тела и без подписи.
    Get,
}

/// Как формируется `Token`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Signing {
    /// Все корневые параметры тела + `Password` (обычное правило).
    Body,
    /// Только `TerminalKey` + `Password` (метод `getConfirmOperation`).
    TerminalKeyOnly,
    /// Без подписи и без `TerminalKey` в теле.
    None,
}

/// Метод API. Реализован для всех типов запросов библиотеки; можно
/// реализовать и для своего типа, чтобы вызвать метод, которого здесь нет:
/// клиент сам добавит `TerminalKey` и `Token`.
///
/// ```
/// use serde::{Deserialize, Serialize};
/// use tinkoff_acquiring::Request;
///
/// #[derive(Serialize)]
/// #[serde(rename_all = "PascalCase")]
/// struct NewMethod { payment_id: String }
///
/// #[derive(Deserialize)]
/// struct NewMethodResponse { #[serde(rename = "Status")] status: String }
///
/// impl Request for NewMethod {
///     type Response = NewMethodResponse;
///     const PATH: &'static str = "v2/NewMethod";
/// }
/// ```
pub trait Request: Serialize {
    /// Тип успешного ответа.
    type Response: DeserializeOwned;

    /// Путь относительно базового URL, например `v2/Init`.
    const PATH: &'static str;

    /// Формат запроса.
    const ENCODING: Encoding = Encoding::Json;

    /// Правило подписи.
    const SIGNING: Signing = Signing::Body;

    /// Путь с подставленными параметрами (для `GET`-методов).
    fn path(&self, _terminal_key: &str) -> Cow<'static, str> {
        Cow::Borrowed(Self::PATH)
    }

    /// Можно ли автоматически повторять запрос при таймауте и 5xx.
    ///
    /// Ошибку соединения (запрос не ушёл) клиент повторяет всегда.
    fn retry_safe(&self) -> bool {
        false
    }

    /// Локальная проверка перед отправкой.
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Экранирование сегмента пути (RFC 3986, всё кроме unreserved).
pub(crate) fn path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

macro_rules! impl_request {
    ($req:ty => $resp:ty, $path:literal $(, { $($extra:tt)* })?) => {
        impl crate::request::Request for $req {
            type Response = $resp;
            const PATH: &'static str = $path;
            $($($extra)*)?
        }
    };
}
