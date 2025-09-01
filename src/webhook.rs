use crate::token::{make_token, top_fields_from_value};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum WebhookVerifyError {
    #[error("no token in payload")]
    NoToken,
    #[error("token mismatch")]
    Mismatch,
}

/// Валидация webhook: считаем токен по корневым простым полям (без Token и без вложенных).
pub fn verify_webhook_token(
    terminal_key: &str,
    password: &str,
    body: &Value,
) -> std::result::Result<(), WebhookVerifyError> {
    let got = body
        .get("Token")
        .and_then(|v| v.as_str())
        .ok_or(WebhookVerifyError::NoToken)?;
    let top = top_fields_from_value(body);
    let expected = make_token(terminal_key, password, top);
    if got == expected { Ok(()) } else { Err(WebhookVerifyError::Mismatch) }
}
