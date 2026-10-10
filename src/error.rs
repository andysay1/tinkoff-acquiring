use serde_json::Value;
use std::fmt;

/// Результат вызова API.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Ошибка клиента.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// API ответило `Success: false` или HTTP-ошибкой с телом в формате API.
    #[error("{0}")]
    Api(Box<ApiError>),
    /// Сетевая ошибка, таймаут, ошибка TLS.
    #[error("транспорт: {}", with_causes(.0))]
    Transport(#[from] reqwest::Error),
    /// HTTP-ошибка без понятного тела.
    #[error("HTTP {status}: {body}")]
    Http {
        /// Код ответа.
        status: u16,
        /// Начало тела ответа.
        body: String,
    },
    /// Ответ не удалось разобрать.
    #[error("не удалось разобрать ответ {method}: {source}; тело: {body}")]
    Decode {
        /// Метод API.
        method: String,
        /// Причина.
        #[source]
        source: serde_json::Error,
        /// Начало тела ответа.
        body: String,
    },
    /// Запрос не прошёл локальную проверку и не был отправлен.
    #[error("некорректный запрос: {0}")]
    Validation(String),
    /// Ошибка настройки клиента.
    #[error("настройка клиента: {0}")]
    Config(String),
}

/// Текст ошибки вместе с цепочкой причин: reqwest прячет главное
/// (например, «certificate verify failed») в `source()`.
fn with_causes(e: &reqwest::Error) -> String {
    let mut out = e.to_string();
    let mut cur = std::error::Error::source(e);
    while let Some(c) = cur {
        let s = c.to_string();
        if !out.contains(&s) {
            out.push_str(": ");
            out.push_str(&s);
        }
        cur = c.source();
    }
    out
}

impl Error {
    /// Ошибка API, если это она.
    pub fn api(&self) -> Option<&ApiError> {
        match self {
            Error::Api(e) => Some(e.as_ref()),
            _ => None,
        }
    }

    /// Код ошибки API (`ErrorCode`), если это ошибка API.
    pub fn error_code(&self) -> Option<&str> {
        self.api().map(|e| e.code.as_str())
    }

    /// Можно ли безопасно повторить запрос (сеть, 5xx, 429).
    ///
    /// Для операций со списанием денег повторяйте только после проверки
    /// статуса через `GetState`/`CheckOrder`.
    pub fn is_transient(&self) -> bool {
        match self {
            Error::Transport(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            Error::Http { status, .. } => *status == 429 || *status >= 500,
            Error::Api(e) => e.http_status.is_some_and(|s| s == 429 || s >= 500),
            _ => false,
        }
    }
}

/// Ошибка, которую вернуло API Т-Банка.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ApiError {
    /// Метод API.
    pub method: String,
    /// `ErrorCode` ([справочник кодов](https://developer.tbank.ru/eacq/intro/errors)).
    pub code: String,
    /// `Message` — краткое описание.
    pub message: Option<String>,
    /// `Details` — подробности.
    pub details: Option<String>,
    /// HTTP-статус, если он не 200.
    pub http_status: Option<u16>,
    /// Полное тело ответа: в нём остаются `Status`, `PaymentId` и другие
    /// поля, например при `REJECTED` в `Charge`.
    pub raw: Value,
}

impl ApiError {
    /// Поле `Status` из тела ответа, если есть.
    pub fn status(&self) -> Option<&str> {
        self.raw.get("Status").and_then(Value::as_str)
    }

    /// Поле `PaymentId` из тела ответа, если есть.
    pub fn payment_id(&self) -> Option<String> {
        match self.raw.get("PaymentId")? {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ошибка API {}", self.method, self.code)?;
        if let Some(m) = &self.message {
            write!(f, ": {m}")?;
        }
        if let Some(d) = &self.details {
            if Some(d) != self.message.as_ref() {
                write!(f, " ({d})")?;
            }
        }
        if let Some(s) = self.http_status {
            write!(f, " [HTTP {s}]")?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}
