//! Синхронный клиент (feature `blocking`).
//!
//! Методы те же, что у асинхронного [`crate::Client`]. Не создавайте и не
//! используйте его внутри async-рантайма — там нужен асинхронный клиент.
//!
//! ```no_run
//! use tinkoff_acquiring::{blocking, GetState};
//!
//! let client = blocking::Client::new("TerminalKey", "Password")?;
//! let state = client.get_state(&GetState::new("1234567890"))?;
//! println!("{}", state.status);
//! # Ok::<(), tinkoff_acquiring::Error>(())
//! ```

use crate::api::*;
use crate::client::{api_methods, ClientBuilder};
use crate::error::{Error, Result};
use crate::notification::{Notification, NotificationError, NotificationVerifier};
use crate::request::Request;
use crate::transport::{self, Body, Shared};
use serde_json::{Map, Value};
use std::fmt;
use std::sync::Arc;

/// Синхронный клиент API.
#[derive(Clone)]
pub struct Client {
    http: reqwest::blocking::Client,
    shared: Arc<Shared>,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.shared.fmt(f)
    }
}

impl Client {
    /// Клиент боевого API с настройками по умолчанию.
    pub fn new(terminal_key: impl Into<String>, password: impl Into<String>) -> Result<Self> {
        ClientBuilder::new(terminal_key, password).build_blocking()
    }

    /// Настройка клиента; завершите вызовом [`ClientBuilder::build_blocking`].
    pub fn builder(terminal_key: impl Into<String>, password: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(terminal_key, password)
    }

    pub(crate) fn from_parts(http: reqwest::blocking::Client, shared: Arc<Shared>) -> Self {
        Self { http, shared }
    }

    /// `TerminalKey` клиента.
    pub fn terminal_key(&self) -> &str {
        &self.shared.terminal_key
    }

    /// Базовый URL API.
    pub fn base_url(&self) -> &str {
        &self.shared.base_url
    }

    /// Проверка уведомлений с реквизитами этого клиента.
    pub fn notifications(&self) -> NotificationVerifier {
        NotificationVerifier::new(&self.shared.terminal_key, &self.shared.password)
    }

    /// Проверить подпись и `TerminalKey` уведомления и разобрать его.
    pub fn parse_notification(
        &self,
        body: &[u8],
    ) -> std::result::Result<Notification, NotificationError> {
        self.notifications().verify(body)
    }

    /// Выполнить любой запрос.
    pub fn execute<R: Request>(&self, req: &R) -> Result<R::Response> {
        let prepared = self.shared.prepare(req)?;
        self.send(prepared)
    }

    /// Вызвать метод, которого нет в библиотеке.
    pub fn execute_raw(&self, path: &str, body: Map<String, Value>) -> Result<Value> {
        let prepared = self.shared.prepare_raw(path, body)?;
        self.send(prepared)
    }

    fn send<T: serde::de::DeserializeOwned>(&self, p: transport::Prepared) -> Result<T> {
        let retry = self.shared.retry;
        let mut attempt = 0u32;
        loop {
            let rb = match &p.body {
                Body::Json(v) => self.http.post(&p.url).json(v),
                Body::Form(f) => self.http.post(&p.url).form(f),
                Body::Empty => self.http.get(&p.url),
            };
            tracing::debug!(method = p.name, attempt, "запрос к API Т-Банка");
            match rb.send() {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if attempt < retry.max_retries && transport::retry_status(status, p.retry_safe)
                    {
                        tracing::warn!(method = p.name, http = status, attempt, "повтор запроса");
                        std::thread::sleep(retry.delay(attempt));
                        attempt += 1;
                        continue;
                    }
                    let bytes = resp.bytes()?;
                    return transport::decode(p.name, status, &bytes);
                }
                Err(e) => {
                    if attempt < retry.max_retries && transport::retry_transport(&e, p.retry_safe) {
                        tracing::warn!(method = p.name, error = %e, attempt, "повтор запроса");
                        std::thread::sleep(retry.delay(attempt));
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::Transport(e));
                }
            }
        }
    }
}

macro_rules! blocking_methods {
    ($( $(#[$m:meta])* $name:ident($req:ty) -> $resp:ty; )*) => {
        impl Client {
            $(
                $(#[$m])*
                pub fn $name(&self, req: &$req) -> Result<$resp> {
                    self.execute(req)
                }
            )*
        }
    };
}
api_methods!(blocking_methods);
