//! Подпись запросов и уведомлений
//! ([документация](https://developer.tbank.ru/eacq/intro/developer/token)).
//!
//! Берутся только параметры корневого объекта (вложенные объекты и массивы —
//! `DATA`, `Receipt`, `Shops` — не участвуют), добавляется `Password`, пары
//! сортируются по ключу, значения склеиваются и хэшируются SHA-256.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Строковое значение корневого параметра для подписи, или `None`, если
/// параметр в подписи не участвует (`null`, объект, массив).
pub(crate) fn token_value(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Null | Value::Object(_) | Value::Array(_) => None,
    }
}

/// Подпись из готовых пар ключ–значение (без `Password`, без `Token`).
pub fn sign_pairs<K, V, I>(password: &str, pairs: I) -> String
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    let mut map: BTreeMap<String, String> = pairs
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .filter(|(k, _)| k != "Token")
        .collect();
    map.insert("Password".into(), password.to_owned());
    let mut hasher = Sha256::new();
    for v in map.values() {
        hasher.update(v.as_bytes());
    }
    hex(&hasher.finalize())
}

/// Подпись JSON-объекта запроса или уведомления. Поле `Token`, если оно есть,
/// игнорируется.
pub fn sign(password: &str, body: &Map<String, Value>) -> String {
    sign_pairs(
        password,
        body.iter()
            .filter_map(|(k, v)| token_value(v).map(|s| (k.clone(), s))),
    )
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

/// Сравнение за время, не зависящее от содержимого.
pub(crate) fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Пример со страницы «Токен» документации Т-Банка.
    #[test]
    fn docs_init_example() {
        let body = json!({
            "TerminalKey": "MerchantTerminalKey",
            "Amount": 19200,
            "OrderId": "00000",
            "Description": "Подарочная карта на 1000 рублей",
            "DATA": {"Phone": "+71234567890", "Email": "a@test.com"},
            "Receipt": {"Email": "a@test.ru", "Taxation": "osn", "Items": []}
        });
        assert_eq!(
            sign("11111111111111", body.as_object().unwrap()),
            "72dd466f8ace0a37a1f740ce5fb78101712bc0665d91a8108c7c8a0ccd426db2"
        );
    }

    /// Пример со страницы «Уведомления»: bool даёт `"true"`, число — свою запись.
    #[test]
    fn docs_notification_example() {
        let body = json!({
            "TerminalKey": "1234567890DEMO",
            "OrderId": "000000",
            "Success": true,
            "Status": "AUTHORIZED",
            "PaymentId": "0000000",
            "ErrorCode": "0",
            "Amount": "1111",
            "CardId": "000000",
            "Pan": "200000******0000",
            "ExpDate": "1111",
            "RebillId": "000000",
            "Token": "ignored"
        });
        assert_eq!(
            sign("11111111111", body.as_object().unwrap()),
            "1c0964277d0213349243065a0d5b838b8e90d2d25f740d0f2767836e710e80c8"
        );
    }

    #[test]
    fn nulls_are_skipped() {
        let a = json!({"A": "1", "B": null});
        let b = json!({"A": "1"});
        assert_eq!(
            sign("p", a.as_object().unwrap()),
            sign("p", b.as_object().unwrap())
        );
    }
}
