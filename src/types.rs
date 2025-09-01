use serde::{Deserialize, Serialize};

pub type Cents = i64;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api error {code:?}: {message:?}")]
    Api { code: Option<String>, message: Option<String> },
    #[error("token verification failed")]
    Token,
    #[error("invalid receipt: {0}")]
    Receipt(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Config {
    pub terminal_key: String,
    pub password: String,
    /// e.g. "https://securepay.tinkoff.ru"
    pub base: String,
    pub user_agent: Option<String>,
    /// default request timeout (ms)
    pub timeout_ms: u64,
    /// retries
    pub max_retries: usize,
    pub initial_backoff_ms: u64,
    /// заголовок для идемпотентности
    pub idempotence_header: Option<String>,
}
impl Config {
    pub fn new(terminal_key: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            terminal_key: terminal_key.into(),
            password: password.into(),
            base: "https://securepay.tinkoff.ru".into(),
            user_agent: Some(format!("tinkoff-acquiring/{}", env!("CARGO_PKG_VERSION"))),
            timeout_ms: 10_000,
            max_retries: 3,
            initial_backoff_ms: 250,
            idempotence_header: Some("Idempotence-Key".into()),
        }
    }
}

/* ---------- Общие enum'ы ---------- */

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    New,
    ThreeDsChecking,
    ThreeDsChecked,
    Authorized,
    Confirmed,
    Rejected,
    Canceled,
    Refund,
    PartiallyRefunded,
    DeadlineExpired,
    AuthorizationDeclined,
    Reversed,
    FormShowed,
    Unknown,
}
impl Default for Status { fn default() -> Self { Status::Unknown } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ErrorCodeEnum {
    Known(KnownError),
    Unknown(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KnownError {
    #[serde(alias="0")] Ok,
    #[serde(alias="7")] InvalidToken,
    #[serde(alias="8")] PaymentNotFound,
    #[serde(alias="9999")] Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureReason {
    InsufficientFunds,
    ExpiredCard,
    IncorrectCvv,
    BankDecline,
    SuspectedFraud,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Taxation { Osn, UsnIncome, UsnIncomeOutcome, Patent, Envd, Esn }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Tax { None, Vat0, Vat10, Vat20, Vat110, Vat120 }

/* ---------- Типизированный чек (упрощённый) ---------- */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    #[serde(rename="Email", skip_serializing_if="Option::is_none")]
    pub email: Option<String>,
    #[serde(rename="Phone", skip_serializing_if="Option::is_none")]
    pub phone: Option<String>,
    #[serde(rename="Taxation")]
    pub taxation: Taxation,
    #[serde(rename="Items")]
    pub items: Vec<Item>,
    #[serde(skip)]
    pub currency: Option<String>,
}
impl Receipt {
    pub fn validate(&self) -> Result<()> {
        if self.items.is_empty() { return Err(Error::Receipt("empty items")); }
        for it in &self.items {
            // допускаем дробное количество (округление до копейки)
            let expected = ((it.price as f64) * it.quantity).round() as i64;
            if expected != it.amount {
                return Err(Error::Receipt("item amount != price * quantity (rounded)"));
            }
            if it.price <= 0 || it.amount <= 0 {
                return Err(Error::Receipt("non-positive price/amount"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename="Name")]
    pub name: String,
    #[serde(rename="Price")]
    pub price: Cents,
    #[serde(rename="Quantity")]
    pub quantity: f64,
    #[serde(rename="Amount")]
    pub amount: Cents,
    #[serde(rename="Tax")]
    pub tax: Tax,
}

/* ---------- DTO ---------- */

#[derive(Debug, Serialize)]
pub struct InitRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="Amount")]      pub amount: Cents,
    #[serde(rename="OrderId")]     pub order_id: &'a str,
    #[serde(rename="Description", skip_serializing_if="Option::is_none")] pub description: Option<&'a str>,
    #[serde(rename="PayType",     skip_serializing_if="Option::is_none")] pub pay_type: Option<&'a str>,
    #[serde(rename="Receipt",     skip_serializing_if="Option::is_none")] pub receipt: Option<serde_json::Value>,
    #[serde(rename="DATA",        skip_serializing_if="Option::is_none")] pub data: Option<serde_json::Value>,
    #[serde(rename="SuccessURL",  skip_serializing_if="Option::is_none")] pub success_url: Option<&'a str>,
    #[serde(rename="FailURL",     skip_serializing_if="Option::is_none")] pub fail_url: Option<&'a str>,
    #[serde(rename="Recurrent",   skip_serializing_if="Option::is_none")] pub recurrent: Option<String>,
    #[serde(rename="Token")]      pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct InitResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="PaymentId")] pub payment_id: Option<String>,
    #[serde(rename="PaymentURL")]pub payment_url: Option<String>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetStateRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="PaymentId")]   pub payment_id: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize, Default)]
pub struct GetStateResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="Status")]    pub status: Option<Status>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
    #[serde(rename="FailureReason")] pub failure_reason: Option<FailureReason>,
}

#[derive(Debug, Serialize)]
pub struct ConfirmRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="PaymentId")]   pub payment_id: &'a str,
    #[serde(rename="Amount", skip_serializing_if="Option::is_none")]  pub amount: Option<Cents>,
    #[serde(rename="Receipt", skip_serializing_if="Option::is_none")] pub receipt: Option<serde_json::Value>,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct ConfirmResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="Status")]    pub status: Option<Status>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CancelRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="PaymentId")]   pub payment_id: &'a str,
    #[serde(rename="Amount", skip_serializing_if="Option::is_none")]  pub amount: Option<Cents>,
    #[serde(rename="Receipt", skip_serializing_if="Option::is_none")] pub receipt: Option<serde_json::Value>,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct CancelResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="Status")]    pub status: Option<Status>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}

/* ---- charge ---- */
#[derive(Debug, Serialize)]
pub struct ChargeRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="PaymentId")]   pub payment_id: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct ChargeResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="Status")]    pub status: Option<Status>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}

/* ---- cards ---- */
#[derive(Debug, Serialize)]
pub struct GetCardListRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="CustomerKey")] pub customer_key: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct GetCardListResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="Cards")]     pub cards: Option<Vec<Card>>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Card {
    #[serde(rename="Pan")]     pub pan: Option<String>,
    #[serde(rename="CardId")]  pub card_id: Option<String>,
    #[serde(rename="Status")]  pub status: Option<String>,
    #[serde(rename="RebillId")]pub rebill_id: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct RemoveCardRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="CustomerKey")] pub customer_key: &'a str,
    #[serde(rename="CardId")]     pub card_id: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct RemoveCardResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}

/* ---- customers ---- */
#[derive(Debug, Serialize)]
pub struct AddCustomerRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="CustomerKey")] pub customer_key: &'a str,
    #[serde(rename="Email", skip_serializing_if="Option::is_none")] pub email: Option<&'a str>,
    #[serde(rename="Phone", skip_serializing_if="Option::is_none")] pub phone: Option<&'a str>,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct AddCustomerResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct GetCustomerRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="CustomerKey")] pub customer_key: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct GetCustomerResponse {
    #[serde(rename="Success")]     pub success: bool,
    #[serde(rename="CustomerKey")] pub customer_key: Option<String>,
    #[serde(rename="Email")]       pub email: Option<String>,
    #[serde(rename="Phone")]       pub phone: Option<String>,
    #[serde(rename="ErrorCode")]   pub error_code: Option<String>,
    #[serde(rename="Message")]     pub message: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct RemoveCustomerRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="CustomerKey")] pub customer_key: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct RemoveCustomerResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}

/* ---- bind/finish authorize ---- */
#[derive(Debug, Serialize)]
pub struct BindCardRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="CustomerKey")] pub customer_key: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct BindCardResponse {
    #[serde(rename="Success")]     pub success: bool,
    #[serde(rename="RequestKey")]  pub request_key: Option<String>,
    #[serde(rename="ErrorCode")]   pub error_code: Option<String>,
    #[serde(rename="Message")]     pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FinishAuthorizeRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="PaymentId")]   pub payment_id: &'a str,
    #[serde(rename="DATA")]        pub data: serde_json::Value,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct FinishAuthorizeResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="Status")]    pub status: Option<Status>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}

/* ---- QR ---- */
#[derive(Debug, Serialize)]
pub struct GetQrRequest<'a> {
    #[serde(rename="TerminalKey")] pub terminal_key: &'a str,
    #[serde(rename="PaymentId")]   pub payment_id: &'a str,
    #[serde(rename="Token")]       pub token: String,
}
#[derive(Debug, Deserialize)]
pub struct GetQrResponse {
    #[serde(rename="Success")]   pub success: bool,
    #[serde(rename="Data")]      pub data: Option<String>,
    #[serde(rename="ErrorCode")] pub error_code: Option<String>,
    #[serde(rename="Message")]   pub message: Option<String>,
}
