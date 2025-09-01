#[cfg(feature = "blocking")]
pub mod blocking {
    use crate::types::*;
    use backoff::{retry, ExponentialBackoff};
    use reqwest::blocking::{Client, ClientBuilder};
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue, USER_AGENT};
    use std::time::Duration;

    #[derive(Clone)]
    pub struct TinkoffBlocking {
        http: Client,
        cfg: Config,
    }

    impl TinkoffBlocking {
        pub fn new(cfg: Config) -> Self {
            let mut headers = HeaderMap::new();
            if let Some(ua) = &cfg.user_agent {
                headers.insert(
                    USER_AGENT,
                    HeaderValue::from_str(ua).unwrap_or(HeaderValue::from_static("tinkoff-acq")),
                );
            }
            let http = ClientBuilder::new()
                .default_headers(headers)
                .timeout(Duration::from_millis(cfg.timeout_ms))
                .build()
                .expect("reqwest blocking client");
            Self { http, cfg }
        }

        #[inline]
        fn url(&self, path: &str) -> String {
            format!("{}/v2/{}", self.cfg.base.trim_end_matches('/'), path)
        }

        fn post_json<T: serde::Serialize, R: for<'de> serde::Deserialize<'de>>(
            &self,
            path: &str,
            body: &T,
            idempotence_key: Option<&str>,
        ) -> Result<R> {
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

            let op = || {
              let req_cloned = req
                    .try_clone()
                    .ok_or_else(|| backoff::Error::permanent(Error::Api {
                        code: None,
                        message: Some("failed to clone request".into()),
                    }))?;

                let resp = req_cloned
                    .send()
                    .map_err(|e| backoff::Error::transient(Error::Http(e)))?;


                let status = resp.status();
                if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    return Err(backoff::Error::transient(Error::Api {
                        code: None,
                        message: Some(format!("server {}", status)),
                    }));
                }

                let resp = resp
                    .error_for_status()
                    .map_err(|e| backoff::Error::permanent(Error::Http(e)))?;

                let out = resp
                    .json::<R>()
                    .map_err(|e| backoff::Error::permanent(Error::Http(e)))?;
                Ok(out)
            };

            match retry(backoff, op) {
                Ok(v) => Ok(v),
                Err(backoff::Error::Permanent(e)) => Err(e),
                Err(backoff::Error::Transient { err: e, .. }) => Err(e),
            }
        }

        // При необходимости — добавить sync-методы init/get_state/... поверх post_json(...)
    }
}
