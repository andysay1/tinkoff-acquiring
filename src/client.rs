use crate::token::make_token;
use crate::types::*;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT},
    Client, ClientBuilder,
};
use std::collections::BTreeMap;
use std::time::Duration;
use tracing::instrument;

#[derive(Clone)]
pub struct TinkoffClient {
    http: Client,
    cfg: Config,
}

pub struct TinkoffClientBuilder {
    cfg: Config,
}
impl TinkoffClientBuilder {
    pub fn new(cfg: Config) -> Self { Self { cfg } }
    pub fn base(mut self, base: impl Into<String>) -> Self { self.cfg.base = base.into(); self }
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self { self.cfg.user_agent = Some(ua.into()); self }
    pub fn timeout_ms(mut self, ms: u64) -> Self { self.cfg.timeout_ms = ms; self }
    pub fn retries(mut self, max_retries: usize, initial_backoff_ms: u64) -> Self {
        self.cfg.max_retries = max_retries; self.cfg.initial_backoff_ms = initial_backoff_ms; self
    }
    pub fn idempotence_header(mut self, header_name: impl Into<String>) -> Self {
        self.cfg.idempotence_header = Some(header_name.into()); self
    }
    pub fn build(self) -> TinkoffClient {
        let mut headers = HeaderMap::new();
        if let Some(ua) = &self.cfg.user_agent {
            headers.insert(USER_AGENT, HeaderValue::from_str(ua).unwrap_or(HeaderValue::from_static("tinkoff-acq")));
        }
        let http = ClientBuilder::new()
            .default_headers(headers)
            .timeout(Duration::from_millis(self.cfg.timeout_ms))
            .build()
            .expect("reqwest client");
        TinkoffClient { http, cfg: self.cfg }
    }
}

impl TinkoffClient {
    pub fn new(cfg: Config) -> Self { TinkoffClientBuilder::new(cfg).build() }

    #[inline]
    fn url(&self, path: &str) -> String {
        format!("{}/v2/{}", self.cfg.base.trim_end_matches('/'), path)
    }

    async fn post_json<T: serde::Serialize, R: for<'de> serde::Deserialize<'de>>(
        &self,
        path: &str,
        body: &T,
        idempotence_key: Option<&str>,
    ) -> Result<R> {
        use backoff::future::retry;
        use backoff::ExponentialBackoff;

        let url = self.url(path);
        let mut req = self.http.post(url).json(body);
        if let (Some(hname), Some(key)) = (&self.cfg.idempotence_header, idempotence_key) {
            if let Ok(name) = HeaderName::from_bytes(hname.as_bytes()) {
                req = req.header(name, key);
            }
        }

        let backoff = ExponentialBackoff {
            initial_interval: Duration::from_millis(self.cfg.initial_backoff_ms),
            max_elapsed_time: Some(Duration::from_millis(self.cfg.initial_backoff_ms * 20)),
            max_interval: Duration::from_secs(3),
            ..ExponentialBackoff::default()
        };

        let fut = || async {
            let req_cloned = req
                .try_clone()
                .ok_or_else(|| backoff::Error::permanent(Error::Api {
                    code: None,
                    message: Some("failed to clone request".into()),
                }))?;

            let resp = req_cloned
                .send()
                .await
                .map_err(|e| backoff::Error::transient(Error::Http(e)))?;

            let status = resp.status();
            if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                // 5xx / 429 → transient API error (даём шанс ретраю)
                return Err(backoff::Error::transient(Error::Api {
                    code: None,
                    message: Some(format!("server {}", status)),
                }));
            }

            let resp = resp
                .error_for_status()
                .map_err(|e| backoff::Error::permanent(Error::Http(e)))?; // 4xx → permanent

            let out = resp
                .json::<R>()
                .await
                .map_err(|e| backoff::Error::permanent(Error::Http(e)))?;
            Ok(out)
        };

        match retry(backoff, fut).await {
            Ok(v) => Ok(v),
            Err(e) => Err(e), // уже наш Error
        }
    }

    /* ------------------- API ------------------- */

    #[instrument(skip(self, receipt, data))]
    pub async fn init(
        &self,
        amount: Cents,
        order_id: &str,
        description: Option<&str>,
        pay_type: Option<&str>,
        receipt: Option<crate::types::Receipt>,
        data: Option<serde_json::Value>,
        success_url: Option<&str>,
        fail_url: Option<&str>,
        recurrent: Option<&str>,
        idem_key: Option<&str>,
    ) -> Result<InitResponse> {
        let receipt_value = if let Some(r) = receipt { r.validate()?; Some(serde_json::to_value(r).unwrap()) } else { None };

        let mut top = BTreeMap::from([("Amount".into(), amount.to_string()), ("OrderId".into(), order_id.to_string())]);
        if let Some(d) = description { top.insert("Description".into(), d.to_string()); }
        if let Some(p) = pay_type     { top.insert("PayType".into(),     p.to_string()); }
        if let Some(s) = success_url  { top.insert("SuccessURL".into(),  s.to_string()); }
        if let Some(f) = fail_url     { top.insert("FailURL".into(),     f.to_string()); }
        if let Some(rc) = recurrent   { top.insert("Recurrent".into(),   rc.to_string()); }

        let req = InitRequest {
            terminal_key: &self.cfg.terminal_key,
            amount, order_id, description, pay_type,
            receipt: receipt_value, data, success_url, fail_url,
            recurrent: recurrent.map(|s| s.to_string()),
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };

        let out: InitResponse = self.post_json("Init", &req, idem_key).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn get_state(&self, payment_id: &str) -> Result<GetStateResponse> {
        let top = BTreeMap::from([("PaymentId".into(), payment_id.to_string())]);
        let req = GetStateRequest {
            terminal_key: &self.cfg.terminal_key,
            payment_id,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: GetStateResponse = self.post_json("GetState", &req, None).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn confirm(
        &self, payment_id: &str, amount: Option<Cents>, receipt: Option<crate::types::Receipt>, idem_key: Option<&str>
    ) -> Result<ConfirmResponse> {
        let receipt_value = if let Some(r) = receipt { r.validate()?; Some(serde_json::to_value(r).unwrap()) } else { None };
        let mut top = BTreeMap::from([("PaymentId".into(), payment_id.to_string())]);
        if let Some(a) = amount { top.insert("Amount".into(), a.to_string()); }
        let req = ConfirmRequest {
            terminal_key: &self.cfg.terminal_key,
            payment_id,
            amount,
            receipt: receipt_value,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: ConfirmResponse = self.post_json("Confirm", &req, idem_key).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn cancel(
        &self, payment_id: &str, amount: Option<Cents>, receipt: Option<crate::types::Receipt>, idem_key: Option<&str>
    ) -> Result<CancelResponse> {
        let receipt_value = if let Some(r) = receipt { r.validate()?; Some(serde_json::to_value(r).unwrap()) } else { None };
        let mut top = BTreeMap::from([("PaymentId".into(), payment_id.to_string())]);
        if let Some(a) = amount { top.insert("Amount".into(), a.to_string()); }
        let req = CancelRequest {
            terminal_key: &self.cfg.terminal_key,
            payment_id,
            amount,
            receipt: receipt_value,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: CancelResponse = self.post_json("Cancel", &req, idem_key).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn charge(&self, payment_id: &str) -> Result<ChargeResponse> {
        let top = BTreeMap::from([("PaymentId".into(), payment_id.to_string())]);
        let req = ChargeRequest {
            terminal_key: &self.cfg.terminal_key,
            payment_id,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: ChargeResponse = self.post_json("Charge", &req, None).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    /* ---- Cards ---- */
    pub async fn get_card_list(&self, customer_key: &str) -> Result<GetCardListResponse> {
        let top = BTreeMap::from([("CustomerKey".into(), customer_key.to_string())]);
        let req = GetCardListRequest {
            terminal_key: &self.cfg.terminal_key,
            customer_key,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: GetCardListResponse = self.post_json("GetCardList", &req, None).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn remove_card(&self, customer_key: &str, card_id: &str, idem_key: Option<&str>) -> Result<RemoveCardResponse> {
        let top = BTreeMap::from([
            ("CustomerKey".into(), customer_key.to_string()),
            ("CardId".into(), card_id.to_string()),
        ]);
        let req = RemoveCardRequest {
            terminal_key: &self.cfg.terminal_key,
            customer_key,
            card_id,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: RemoveCardResponse = self.post_json("RemoveCard", &req, idem_key).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    /* ---- Customers ---- */
    pub async fn add_customer(
        &self, customer_key: &str, email: Option<&str>, phone: Option<&str>, idem_key: Option<&str>
    ) -> Result<AddCustomerResponse> {
        let mut top = BTreeMap::from([("CustomerKey".into(), customer_key.to_string())]);
        if let Some(e) = email { top.insert("Email".into(), e.to_string()); }
        if let Some(p) = phone { top.insert("Phone".into(), p.to_string()); }
        let req = AddCustomerRequest {
            terminal_key: &self.cfg.terminal_key,
            customer_key, email, phone,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: AddCustomerResponse = self.post_json("AddCustomer", &req, idem_key).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn get_customer(&self, customer_key: &str) -> Result<GetCustomerResponse> {
        let top = BTreeMap::from([("CustomerKey".into(), customer_key.to_string())]);
        let req = GetCustomerRequest {
            terminal_key: &self.cfg.terminal_key,
            customer_key,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: GetCustomerResponse = self.post_json("GetCustomer", &req, None).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn remove_customer(&self, customer_key: &str, idem_key: Option<&str>) -> Result<RemoveCustomerResponse> {
        let top = BTreeMap::from([("CustomerKey".into(), customer_key.to_string())]);
        let req = RemoveCustomerRequest {
            terminal_key: &self.cfg.terminal_key,
            customer_key,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: RemoveCustomerResponse = self.post_json("RemoveCustomer", &req, idem_key).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    /* ---- BindCard / FinishAuthorize ---- */
    pub async fn bind_card(&self, customer_key: &str, idem_key: Option<&str>) -> Result<BindCardResponse> {
        let top = BTreeMap::from([("CustomerKey".into(), customer_key.to_string())]);
        let req = BindCardRequest {
            terminal_key: &self.cfg.terminal_key,
            customer_key,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: BindCardResponse = self.post_json("BindCard", &req, idem_key).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    pub async fn finish_authorize(&self, payment_id: &str, data: serde_json::Value) -> Result<FinishAuthorizeResponse> {
        let top = BTreeMap::from([("PaymentId".into(), payment_id.to_string())]);
        let req = FinishAuthorizeRequest {
            terminal_key: &self.cfg.terminal_key,
            payment_id,
            data,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: FinishAuthorizeResponse = self.post_json("FinishAuthorize", &req, None).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }

    /* ---- QR ---- */
    pub async fn get_qr(&self, payment_id: &str) -> Result<GetQrResponse> {
        let top = BTreeMap::from([("PaymentId".into(), payment_id.to_string())]);
        let req = GetQrRequest {
            terminal_key: &self.cfg.terminal_key,
            payment_id,
            token: make_token(&self.cfg.terminal_key, &self.cfg.password, top),
        };
        let out: GetQrResponse = self.post_json("GetQr", &req, None).await?;
        if out.success { Ok(out) } else { Err(Error::Api { code: out.error_code, message: out.message }) }
    }
}
