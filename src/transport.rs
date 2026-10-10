//! Общая часть async- и blocking-клиентов: подготовка, подпись и разбор.

use crate::error::{ApiError, Error, Result};
use crate::request::{Encoding, Request, Signing};
use crate::token;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use std::fmt;
use std::time::Duration;

/// Боевой адрес API (по спецификации Т-Банка).
pub const PRODUCTION_URL: &str = "https://securepay.tinkoff.ru";
/// Тестовый адрес API.
pub const TEST_URL: &str = "https://rest-api-test.tinkoff.ru";

/// Политика повторов.
///
/// Запрос, который не дошёл до сервера (ошибка соединения), повторяется
/// всегда. Таймауты, 5xx и 429 повторяются только для запросов без побочных
/// эффектов (`GetState`, `CheckOrder`, `GetCardList`, …) и для `Cancel` с
/// `ExternalRequestId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetryPolicy {
    /// Сколько раз повторять (0 — без повторов).
    pub max_retries: u32,
    /// Пауза перед первым повтором; дальше удваивается.
    pub initial_backoff: Duration,
    /// Максимальная пауза.
    pub max_backoff: Duration,
}

impl RetryPolicy {
    /// Без повторов.
    pub const fn none() -> Self {
        Self {
            max_retries: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        }
    }

    /// `max_retries` повторов с паузой от `initial_backoff` до `max_backoff`.
    pub const fn new(max_retries: u32, initial_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            max_retries,
            initial_backoff,
            max_backoff,
        }
    }

    pub(crate) fn delay(&self, attempt: u32) -> Duration {
        let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
        self.initial_backoff
            .checked_mul(factor)
            .unwrap_or(self.max_backoff)
            .min(self.max_backoff)
    }
}

impl Default for RetryPolicy {
    /// Два повтора: через 300 мс и 600 мс (максимум 3 с).
    fn default() -> Self {
        Self::new(2, Duration::from_millis(300), Duration::from_secs(3))
    }
}

/// Реквизиты терминала и общие настройки.
#[derive(Clone)]
pub(crate) struct Shared {
    pub terminal_key: String,
    pub password: String,
    pub base_url: String,
    pub retry: RetryPolicy,
}

impl fmt::Debug for Shared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("terminal_key", &self.terminal_key)
            .field("password", &"***")
            .field("base_url", &self.base_url)
            .field("retry", &self.retry)
            .finish()
    }
}

pub(crate) enum Body {
    Json(Map<String, Value>),
    Form(Vec<(String, String)>),
    Empty,
}

pub(crate) struct Prepared {
    pub name: &'static str,
    pub url: String,
    pub body: Body,
    pub retry_safe: bool,
}

impl Shared {
    pub(crate) fn prepare<R: Request>(&self, req: &R) -> Result<Prepared> {
        req.validate()
            .map_err(|e| Error::Validation(format!("{}: {e}", R::PATH)))?;
        let url = format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            req.path(&self.terminal_key)
        );
        let body = match R::ENCODING {
            Encoding::Get => Body::Empty,
            Encoding::Json | Encoding::Form => {
                let obj = self.signed_body(R::SIGNING, req)?;
                if R::ENCODING == Encoding::Form {
                    Body::Form(to_form(&obj))
                } else {
                    Body::Json(obj)
                }
            }
        };
        Ok(Prepared {
            name: R::PATH,
            url,
            body,
            retry_safe: req.retry_safe(),
        })
    }

    fn signed_body<T: serde::Serialize>(
        &self,
        signing: Signing,
        req: &T,
    ) -> Result<Map<String, Value>> {
        let mut obj = match serde_json::to_value(req)
            .map_err(|e| Error::Validation(format!("сериализация запроса: {e}")))?
        {
            Value::Object(m) => m,
            Value::Null => Map::new(),
            _ => {
                return Err(Error::Validation(
                    "тело запроса должно быть объектом".into(),
                ))
            }
        };
        match signing {
            Signing::Body => {
                obj.insert("TerminalKey".into(), self.terminal_key.clone().into());
                obj.remove("Token");
                let t = token::sign(&self.password, &obj);
                obj.insert("Token".into(), t.into());
            }
            Signing::TerminalKeyOnly => {
                obj.insert("TerminalKey".into(), self.terminal_key.clone().into());
                let t = token::sign_pairs(
                    &self.password,
                    [("TerminalKey", self.terminal_key.as_str())],
                );
                obj.insert("Token".into(), t.into());
            }
            Signing::None => {}
        }
        Ok(obj)
    }

    pub(crate) fn prepare_raw(&self, path: &str, body: Map<String, Value>) -> Result<Prepared> {
        Ok(Prepared {
            name: "raw",
            url: format!(
                "{}/{}",
                self.base_url.trim_end_matches('/'),
                path.trim_start_matches('/')
            ),
            body: Body::Json(self.signed_body(Signing::Body, &body)?),
            retry_safe: false,
        })
    }
}

fn to_form(obj: &Map<String, Value>) -> Vec<(String, String)> {
    obj.iter()
        .filter_map(|(k, v)| {
            let s = match v {
                Value::Null => return None,
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            Some((k.clone(), s))
        })
        .collect()
}

/// Поля, которые не стоит печатать в текстах ошибок и логах: с ними и
/// реквизитами терминала можно списывать деньги.
const SENSITIVE: &[&str] = &["RebillId", "AccountToken", "Token", "CardData"];

fn mask(v: &mut Value) {
    match v {
        Value::Object(m) => {
            for (k, x) in m.iter_mut() {
                if SENSITIVE.contains(&k.as_str()) && !x.is_null() {
                    *x = Value::String("***".into());
                } else {
                    mask(x);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(mask),
        _ => {}
    }
}

fn snippet(bytes: &[u8]) -> String {
    let masked = serde_json::from_slice::<Value>(bytes).ok().map(|mut v| {
        mask(&mut v);
        v.to_string()
    });
    let s = match &masked {
        Some(m) => std::borrow::Cow::Borrowed(m.as_str()),
        None => String::from_utf8_lossy(bytes),
    };
    let mut out: String = s.chars().take(500).collect();
    if s.chars().count() > 500 {
        out.push('…');
    }
    out
}

fn text(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Null => None,
        Value::String(s) if s.is_empty() => None,
        Value::String(s) => Some(s.clone()),
        Value::Array(a) => Some(
            a.iter()
                .map(|x| {
                    x.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| x.to_string())
                })
                .collect::<Vec<_>>()
                .join("; "),
        ),
        other => Some(other.to_string()),
    }
}

fn is_false(v: &Value) -> bool {
    match v {
        Value::Bool(b) => !b,
        Value::String(s) => s.eq_ignore_ascii_case("false"),
        _ => false,
    }
}

/// Разбор ответа: `Success=false` и HTTP-ошибки → [`Error::Api`]/[`Error::Http`].
pub(crate) fn decode<T: DeserializeOwned>(name: &str, status: u16, bytes: &[u8]) -> Result<T> {
    let ok_status = (200..300).contains(&status);
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(source) if ok_status => {
            return Err(Error::Decode {
                method: name.to_owned(),
                source,
                body: snippet(bytes),
            })
        }
        Err(_) => {
            return Err(Error::Http {
                status,
                body: snippet(bytes),
            })
        }
    };
    if let Some(obj) = value.as_object() {
        let failed = obj.get("Success").is_some_and(is_false) || !ok_status;
        if failed {
            let code = text(obj.get("ErrorCode")).unwrap_or_else(|| status.to_string());
            let message = text(obj.get("Message"));
            let details = text(obj.get("Details")).or_else(|| text(obj.get("Causes")));
            tracing::debug!(method = name, http = status, code = %code, "ошибка API");
            return Err(Error::Api(Box::new(ApiError {
                method: name.to_owned(),
                code,
                message,
                details,
                http_status: (status != 200).then_some(status),
                raw: value,
            })));
        }
    } else if !ok_status {
        return Err(Error::Http {
            status,
            body: snippet(bytes),
        });
    }
    serde_json::from_value(value).map_err(|source| Error::Decode {
        method: name.to_owned(),
        source,
        body: snippet(bytes),
    })
}

/// Стоит ли повторить запрос после ошибки отправки.
pub(crate) fn retry_transport(e: &reqwest::Error, retry_safe: bool) -> bool {
    e.is_connect() || (retry_safe && (e.is_timeout() || e.is_request()))
}

/// Стоит ли повторить запрос после HTTP-ответа.
pub(crate) fn retry_status(status: u16, retry_safe: bool) -> bool {
    retry_safe && (status == 429 || status >= 500)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        let p = RetryPolicy::new(5, Duration::from_millis(100), Duration::from_millis(350));
        assert_eq!(p.delay(0), Duration::from_millis(100));
        assert_eq!(p.delay(1), Duration::from_millis(200));
        assert_eq!(p.delay(2), Duration::from_millis(350));
        assert_eq!(p.delay(40), Duration::from_millis(350));
    }

    #[test]
    fn success_false_is_api_error() {
        let body = br#"{"Success":false,"ErrorCode":"9","Message":"Bad","Details":"More","Status":"REJECTED","PaymentId":123}"#;
        let err = decode::<Value>("v2/Charge", 200, body).unwrap_err();
        let api = err.api().unwrap();
        assert_eq!(api.code, "9");
        assert_eq!(api.status(), Some("REJECTED"));
        assert_eq!(api.payment_id().as_deref(), Some("123"));
        assert_eq!(api.http_status, None);
    }

    #[test]
    fn http_500_with_causes() {
        let body =
            br#"{"Success":false,"ErrorCode":"500","Message":"Internal","Causes":["a","b"]}"#;
        let err = decode::<Value>("v2/Init", 500, body).unwrap_err();
        assert!(err.is_transient());
        assert_eq!(err.api().unwrap().details.as_deref(), Some("a; b"));
    }

    #[test]
    fn non_json_error() {
        let err = decode::<Value>("v2/Init", 502, b"<html>Bad gateway</html>").unwrap_err();
        assert!(matches!(err, Error::Http { status: 502, .. }));
    }
}
