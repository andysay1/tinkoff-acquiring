use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Формирование токена: берём ТОЛЬКО корневые поля (примитивы), добавляем
/// TerminalKey и Password, сортируем по ключу, конкатенируем значения, SHA-256 hex.
pub fn make_token(
    terminal_key: &str,
    password: &str,
    mut top_fields: BTreeMap<String, String>,
) -> String {
    top_fields.insert("TerminalKey".into(), terminal_key.to_string());
    top_fields.insert("Password".into(), password.to_string());
    let mut s = String::new();
    for (_, v) in top_fields.iter() {
        s.push_str(v);
    }
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Собираем только корневые примитивы (исключая Token и вложенные объекты/массивы).
pub fn top_fields_from_value(v: &Value) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(obj) = v.as_object() {
        for (k, v) in obj {
            if k == "Token" { continue; }
            if v.is_object() || v.is_array() { continue; }
            let s = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
            out.insert(k.clone(), s);
        }
    }
    out
}
