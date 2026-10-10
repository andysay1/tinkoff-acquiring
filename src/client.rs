use crate::api::*;
use crate::error::{Error, Result};
use crate::notification::{Notification, NotificationError, NotificationVerifier};
use crate::request::Request;
use crate::transport::{self, Body, RetryPolicy, Shared, PRODUCTION_URL, TEST_URL};
use serde_json::{Map, Value};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

/// PEM корневого сертификата НУЦ Минцифры (Russian Trusted Root CA).
///
/// SHA-256: `D2:6D:2D:02:31:B7:C3:9F:92:CC:73:85:12:BA:54:10:35:19:E4:40:5D:68:B5:BD:70:3E:97:88:CA:8E:CF:31`,
/// действителен до 27.02.2032. Пригодится, если вы передаёте свой
/// `reqwest::Client` через [`ClientBuilder::http_client`].
pub const RUSSIAN_TRUSTED_ROOT_CA_PEM: &[u8] =
    include_bytes!("../certs/russian_trusted_root_ca.pem");

/// Список всех методов API: имя метода клиента, тип запроса, тип ответа.
macro_rules! api_methods {
    ($mac:ident) => {
        $mac! {
            /// [`Init`] — инициировать платёж.
            init(Init) -> InitResponse;
            /// [`GetState`] — статус платежа.
            get_state(GetState) -> GetStateResponse;
            /// [`Confirm`] — подтвердить списание (двухстадийная оплата).
            confirm(Confirm) -> ConfirmResponse;
            /// [`Cancel`] — отменить платёж или вернуть деньги.
            cancel(Cancel) -> CancelResponse;
            /// [`Charge`] — списать по сохранённым реквизитам.
            charge(Charge) -> ChargeResponse;
            /// [`CheckOrder`] — статусы всех платежей заказа.
            check_order(CheckOrder) -> CheckOrderResponse;
            /// [`SendClosingReceipt`] — закрывающий чек.
            send_closing_receipt(SendClosingReceipt) -> SendClosingReceiptResponse;
            /// [`GetConfirmOperation`] — справки по операциям.
            get_confirm_operation(GetConfirmOperation) -> GetConfirmOperationResponse;
            /// [`AddCustomer`] — зарегистрировать покупателя.
            add_customer(AddCustomer) -> CustomerResponse;
            /// [`GetCustomer`] — данные покупателя.
            get_customer(GetCustomer) -> CustomerResponse;
            /// [`RemoveCustomer`] — удалить покупателя.
            remove_customer(RemoveCustomer) -> CustomerResponse;
            /// [`AddCard`] — начать привязку карты.
            add_card(AddCard) -> AddCardResponse;
            /// [`AttachCard`] — привязать карту со своей формы.
            attach_card(AttachCard) -> AttachCardResponse;
            /// [`GetAddCardState`] — статус привязки карты.
            get_add_card_state(GetAddCardState) -> GetAddCardStateResponse;
            /// [`GetCardList`] — карты покупателя.
            get_card_list(GetCardList) -> Vec<Card>;
            /// [`RemoveCard`] — удалить карту.
            remove_card(RemoveCard) -> RemoveCardResponse;
            /// [`Check3dsVersion`] — версия 3DS карты.
            check_3ds_version(Check3dsVersion) -> Check3dsVersionResponse;
            /// [`FinishAuthorize`] — оплата со своей формы.
            finish_authorize(FinishAuthorize) -> FinishAuthorizeResponse;
            /// [`Submit3dsAuthorization`] — завершить 3DS v1.
            submit_3ds_authorization(Submit3dsAuthorization) -> Submit3dsResponse;
            /// [`Submit3dsAuthorizationV2`] — завершить 3DS v2.
            submit_3ds_authorization_v2(Submit3dsAuthorizationV2) -> Submit3dsResponse;
            /// [`GetQr`] — QR/ссылка СБП или цифрового рубля.
            get_qr(GetQr) -> GetQrResponse;
            /// [`QrMembersList`] — банки для возврата по СБП.
            qr_members_list(QrMembersList) -> QrMembersListResponse;
            /// [`GetQrState`] — статус возврата по СБП.
            get_qr_state(GetQrState) -> GetQrStateResponse;
            /// [`GetQrBankList`] — банки СБП для выбора.
            get_qr_bank_list(GetQrBankList) -> GetQrBankListResponse;
            /// [`SbpPayTest`] — тестовая оплата по СБП.
            sbp_pay_test(SbpPayTest) -> SbpPayTestResponse;
            /// [`AddAccountQr`] — привязать счёт СБП.
            add_account_qr(AddAccountQr) -> AddAccountQrResponse;
            /// [`GetAddAccountQrState`] — статус привязки счёта.
            get_add_account_qr_state(GetAddAccountQrState) -> AccountQr;
            /// [`GetAccountQrList`] — привязанные счета.
            get_account_qr_list(GetAccountQrList) -> GetAccountQrListResponse;
            /// [`ChargeQr`] — автоплатёж по счёту СБП.
            charge_qr(ChargeQr) -> ChargeQrResponse;
            /// [`TPayStatus`] — доступность T-Pay.
            tpay_status(TPayStatus) -> TPayStatusResponse;
            /// [`TPayLink`] — ссылка T-Pay.
            tpay_link(TPayLink) -> LinkResponse;
            /// [`SberPayLinkGet`] — заказ и ссылка SberPay.
            sberpay_link_get(SberPayLinkGet) -> LinkResponse;
            /// [`SberPayLink`] — ссылка SberPay (GET).
            sberpay_link(SberPayLink) -> LinkResponse;
            /// [`MirPayDeepLink`] — DeepLink Mir Pay.
            mirpay_deep_link(MirPayDeepLink) -> MirPayDeepLinkResponse;
            /// [`AlfaPayLink`] — ссылка Alfa Pay.
            alfapay_link(AlfaPayLink) -> LinkResponse;
        }
    };
}
#[cfg(feature = "blocking")]
pub(crate) use api_methods;

/// Асинхронный клиент API. Дёшево клонируется (`Arc` внутри).
///
/// ```no_run
/// # async fn run() -> tinkoff_acquiring::Result<()> {
/// use tinkoff_acquiring::{Client, Init};
///
/// let client = Client::new("TerminalKey", "Password")?;
/// let resp = client.init(&Init::new(10_000, "order-1").description("Тест")).await?;
/// println!("{:?}", resp.payment_url);
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
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
        Self::builder(terminal_key, password).build()
    }

    /// Настройка клиента.
    pub fn builder(terminal_key: impl Into<String>, password: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(terminal_key, password)
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
    /// Ответьте Т-Банку `200 OK` с телом [`NOTIFICATION_OK`](crate::NOTIFICATION_OK).
    pub fn parse_notification(
        &self,
        body: &[u8],
    ) -> std::result::Result<Notification, NotificationError> {
        self.notifications().verify(body)
    }

    /// Выполнить любой запрос (в том числе свой тип с [`Request`]).
    pub async fn execute<R: Request>(&self, req: &R) -> Result<R::Response> {
        let prepared = self.shared.prepare(req)?;
        self.send(prepared).await
    }

    /// Вызвать метод, которого нет в библиотеке: тело подписывается и
    /// дополняется `TerminalKey`, ответ возвращается как JSON.
    pub async fn execute_raw(&self, path: &str, body: Map<String, Value>) -> Result<Value> {
        let prepared = self.shared.prepare_raw(path, body)?;
        self.send(prepared).await
    }

    async fn send<T: serde::de::DeserializeOwned>(&self, p: transport::Prepared) -> Result<T> {
        let retry = self.shared.retry;
        let mut attempt = 0u32;
        loop {
            let rb = match &p.body {
                Body::Json(v) => self.http.post(&p.url).json(v),
                Body::Form(f) => self.http.post(&p.url).form(f),
                Body::Empty => self.http.get(&p.url),
            };
            tracing::debug!(method = p.name, attempt, "запрос к API Т-Банка");
            match rb.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if attempt < retry.max_retries && transport::retry_status(status, p.retry_safe)
                    {
                        tracing::warn!(method = p.name, http = status, attempt, "повтор запроса");
                        tokio::time::sleep(retry.delay(attempt)).await;
                        attempt += 1;
                        continue;
                    }
                    let bytes = resp.bytes().await?;
                    return transport::decode(p.name, status, &bytes);
                }
                Err(e) => {
                    if attempt < retry.max_retries && transport::retry_transport(&e, p.retry_safe) {
                        tracing::warn!(method = p.name, error = %e, attempt, "повтор запроса");
                        tokio::time::sleep(retry.delay(attempt)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::Transport(e));
                }
            }
        }
    }
}

macro_rules! async_methods {
    ($( $(#[$m:meta])* $name:ident($req:ty) -> $resp:ty; )*) => {
        impl Client {
            $(
                $(#[$m])*
                pub async fn $name(&self, req: &$req) -> Result<$resp> {
                    self.execute(req).await
                }
            )*
        }
    };
}
api_methods!(async_methods);

/// Окружение API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Environment {
    /// `https://securepay.tinkoff.ru`. DEMO-терминалы тоже работают здесь.
    Production,
    /// `https://rest-api-test.tinkoff.ru` (доступ по согласованию с банком).
    Test,
}

/// Настройка [`Client`] (и `blocking::Client`).
pub struct ClientBuilder {
    terminal_key: String,
    password: String,
    base_url: String,
    timeout: Duration,
    connect_timeout: Duration,
    user_agent: String,
    retry: RetryPolicy,
    #[cfg(any(feature = "rustls", feature = "native-tls"))]
    roots: Vec<reqwest::Certificate>,
    #[cfg(feature = "russian-trusted-ca")]
    russian_trusted_ca: bool,
    http: Option<reqwest::Client>,
    #[cfg(feature = "blocking")]
    blocking_http: Option<reqwest::blocking::Client>,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("terminal_key", &self.terminal_key)
            .field("password", &"***")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("retry", &self.retry)
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    /// Настройки по умолчанию: боевой URL, таймаут 30 с, два повтора.
    pub fn new(terminal_key: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            terminal_key: terminal_key.into(),
            password: password.into(),
            base_url: PRODUCTION_URL.into(),
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            user_agent: concat!("tinkoff-acquiring-rs/", env!("CARGO_PKG_VERSION")).into(),
            retry: RetryPolicy::default(),
            #[cfg(any(feature = "rustls", feature = "native-tls"))]
            roots: Vec::new(),
            #[cfg(feature = "russian-trusted-ca")]
            russian_trusted_ca: true,
            http: None,
            #[cfg(feature = "blocking")]
            blocking_http: None,
        }
    }

    /// Окружение (боевое по умолчанию).
    #[must_use]
    pub fn environment(mut self, env: Environment) -> Self {
        self.base_url = match env {
            Environment::Production => PRODUCTION_URL,
            Environment::Test => TEST_URL,
        }
        .into();
        self
    }

    /// Свой базовый URL, например `https://securepay.tbank.ru` или адрес
    /// мок-сервера в тестах.
    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Таймаут всего запроса (по умолчанию 30 с).
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Таймаут установки соединения (по умолчанию 10 с).
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Заголовок `User-Agent`.
    #[must_use]
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    /// Политика повторов.
    #[must_use]
    pub fn retry(mut self, policy: RetryPolicy) -> Self {
        self.retry = policy;
        self
    }

    /// Отключить повторы.
    #[must_use]
    pub fn no_retries(self) -> Self {
        self.retry(RetryPolicy::none())
    }

    /// Дополнительный доверенный корневой сертификат.
    #[cfg(any(feature = "rustls", feature = "native-tls"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "rustls", feature = "native-tls"))))]
    #[must_use]
    pub fn add_root_certificate(mut self, cert: reqwest::Certificate) -> Self {
        self.roots.push(cert);
        self
    }

    /// Доверять сертификату Минцифры (включено по умолчанию). Сертификат
    /// добавляется только в HTTP-клиент этой библиотеки, системное
    /// хранилище не меняется.
    #[cfg(feature = "russian-trusted-ca")]
    #[cfg_attr(docsrs, doc(cfg(feature = "russian-trusted-ca")))]
    #[must_use]
    pub fn russian_trusted_ca(mut self, enabled: bool) -> Self {
        self.russian_trusted_ca = enabled;
        self
    }

    /// Свой `reqwest::Client`. Таймауты, `User-Agent` и сертификаты из этого
    /// builder к нему не применяются.
    #[must_use]
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Свой `reqwest::blocking::Client` для [`build_blocking`](Self::build_blocking).
    #[cfg(feature = "blocking")]
    #[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
    #[must_use]
    pub fn blocking_http_client(mut self, client: reqwest::blocking::Client) -> Self {
        self.blocking_http = Some(client);
        self
    }

    fn shared(&self) -> Result<Arc<Shared>> {
        if self.terminal_key.is_empty() {
            return Err(Error::Config("пустой TerminalKey".into()));
        }
        if self.password.is_empty() {
            return Err(Error::Config("пустой пароль терминала".into()));
        }
        Ok(Arc::new(Shared {
            terminal_key: self.terminal_key.clone(),
            password: self.password.clone(),
            base_url: self.base_url.trim_end_matches('/').to_owned(),
            retry: self.retry,
        }))
    }

    #[cfg(any(feature = "rustls", feature = "native-tls"))]
    fn root_certificates(&self) -> Result<Vec<reqwest::Certificate>> {
        #[allow(unused_mut)]
        let mut roots = self.roots.clone();
        #[cfg(feature = "russian-trusted-ca")]
        if self.russian_trusted_ca {
            roots.push(
                reqwest::Certificate::from_pem(RUSSIAN_TRUSTED_ROOT_CA_PEM)
                    .map_err(|e| Error::Config(format!("сертификат Минцифры: {e}")))?,
            );
        }
        Ok(roots)
    }

    /// Собрать асинхронный клиент.
    pub fn build(self) -> Result<Client> {
        let shared = self.shared()?;
        let http = match &self.http {
            Some(c) => c.clone(),
            None => {
                #[allow(unused_mut)]
                let mut b = reqwest::Client::builder()
                    .timeout(self.timeout)
                    .connect_timeout(self.connect_timeout)
                    .user_agent(self.user_agent.clone());
                #[cfg(feature = "rustls")]
                {
                    b = b.tls_backend_rustls();
                }
                #[cfg(all(feature = "native-tls", not(feature = "rustls")))]
                {
                    b = b.tls_backend_native();
                }
                #[cfg(any(feature = "rustls", feature = "native-tls"))]
                {
                    let roots = self.root_certificates()?;
                    if !roots.is_empty() {
                        b = b.tls_certs_merge(roots);
                    }
                }
                b.build()
                    .map_err(|e| Error::Config(format!("HTTP-клиент: {e}")))?
            }
        };
        Ok(Client { http, shared })
    }

    /// Собрать синхронный клиент. Не вызывайте внутри async-рантайма.
    #[cfg(feature = "blocking")]
    #[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
    pub fn build_blocking(self) -> Result<crate::blocking::Client> {
        let shared = self.shared()?;
        let http = match &self.blocking_http {
            Some(c) => c.clone(),
            None => {
                #[allow(unused_mut)]
                let mut b = reqwest::blocking::Client::builder()
                    .timeout(self.timeout)
                    .connect_timeout(self.connect_timeout)
                    .user_agent(self.user_agent.clone());
                #[cfg(feature = "rustls")]
                {
                    b = b.tls_backend_rustls();
                }
                #[cfg(all(feature = "native-tls", not(feature = "rustls")))]
                {
                    b = b.tls_backend_native();
                }
                #[cfg(any(feature = "rustls", feature = "native-tls"))]
                {
                    let roots = self.root_certificates()?;
                    if !roots.is_empty() {
                        b = b.tls_certs_merge(roots);
                    }
                }
                b.build()
                    .map_err(|e| Error::Config(format!("HTTP-клиент: {e}")))?
            }
        };
        Ok(crate::blocking::Client::from_parts(http, shared))
    }
}
