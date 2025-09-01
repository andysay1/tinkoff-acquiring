use tinkoff_acquiring::{make_token, top_fields_from_value};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn token_is_stable_for_sorted_fields() {
    let mut top = BTreeMap::new();
    top.insert("Amount".to_string(), "100".to_string());
    top.insert("OrderId".to_string(), "A-1".to_string());
    let t1 = make_token("TKEY", "PWD", top.clone());
    let t2 = make_token("TKEY", "PWD", top);
    assert_eq!(t1, t2);
}

#[test]
fn top_fields_excludes_nested_and_token() {
    let v = json!({
        "Amount": "100",
        "OrderId": "A-1",
        "Token": "zzz",
        "DATA": {"x":1}
    });
    let top = top_fields_from_value(&v);
    assert!(top.get("Token").is_none());
    assert!(top.get("DATA").is_none());
    assert_eq!(top.get("Amount").unwrap(), "100");
    assert_eq!(top.get("OrderId").unwrap(), "A-1");
}
