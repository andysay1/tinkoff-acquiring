//! Регрессии по итогам ревью 0.3.0.

use serde_json::json;
use std::time::Duration;
use tinkoff_acquiring::*;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn item(price: Amount, qty: f64) -> ItemFfd12 {
    ItemFfd12::new("Товар", price, qty, Tax::Vat22)
}

#[test]
fn mixed_payments_are_checked_against_electronic() {
    // Позиции на 1000 ₽, из них 300 ₽ — ранее внесённый аванс, картой — 700 ₽.
    let receipt = ReceiptFfd12::new(Taxation::Osn, vec![item(100_000, 1.0)])
        .email("a@b.ru")
        .payments(Payments::electronic(70_000).advance_payment(30_000));
    assert!(Request::validate(&Init::new(70_000, "o").receipt(receipt.clone())).is_ok());
    assert!(Request::validate(&Init::new(100_000, "o").receipt(receipt)).is_err());

    let wrong_total = ReceiptFfd12::new(Taxation::Osn, vec![item(100_000, 1.0)])
        .email("a@b.ru")
        .payments(Payments::electronic(70_000));
    assert!(Request::validate(&Init::new(70_000, "o").receipt(wrong_total)).is_err());
}

#[test]
fn discounted_line_amount_is_allowed_but_not_above_price() {
    let ok =
        ReceiptFfd12::new(Taxation::Osn, vec![item(10_000, 1.0).amount(9_000)]).email("a@b.ru");
    assert!(Request::validate(&Init::new(9_000, "o").receipt(ok)).is_ok());
    let too_much =
        ReceiptFfd12::new(Taxation::Osn, vec![item(10_000, 1.0).amount(10_001)]).email("a@b.ru");
    assert!(Request::validate(&Init::new(10_001, "o").receipt(too_much)).is_err());
}

#[test]
fn fractional_quantity_rounds_like_decimal() {
    // 100 × 0.285 в f64 = 28.4999…, а в копейках должно быть 29.
    assert_eq!(item(100, 0.285).amount, 29);
    assert_eq!(item(19_990, 1.5).amount, 29_985);
    assert_eq!(ItemFfd105::new("x", 333, 3.0, Tax::None).amount, 999);
}

#[tokio::test]
async fn cancel_with_empty_external_request_id_is_not_retried() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/Cancel"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let client = Client::builder("T", "P")
        .base_url(server.uri())
        .retry(RetryPolicy::new(
            3,
            Duration::from_millis(1),
            Duration::from_millis(2),
        ))
        .build()
        .unwrap();
    let _ = client
        .cancel(&Cancel::new("1").external_request_id("  "))
        .await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[test]
fn receipt_version_is_chosen_by_ffd12_fields() {
    // Пример из спецификации: Excise строкой, SectoralItemProps объектом.
    let ffd12: Receipt = serde_json::from_value(json!({
        "Taxation": "osn", "Email": "a@b.ru",
        "Items": [{
            "Name": "Товар", "Price": 10000, "Quantity": 1, "Amount": 10000, "Tax": "vat22",
            "PaymentMethod": "full_payment", "PaymentObject": "goods_with_marking_code",
            "MeasurementUnit": "шт", "Excise": "12.2",
            "SectoralItemProps": {"FederalId": "001", "Date": "21.11.2020", "Number": "1", "Value": "v"}
        }]
    }))
    .unwrap();
    let Receipt::Ffd12(r) = &ffd12 else {
        panic!("ожидался ФФД 1.2: {ffd12:?}")
    };
    assert_eq!(r.items[0].excise, Some(12.2));
    assert_eq!(r.items[0].sectoral_item_props.as_ref().unwrap().len(), 1);
    // Повторная сериализация не теряет поля ФФД 1.2.
    let back = serde_json::to_value(&ffd12).unwrap();
    assert_eq!(back["Items"][0]["MeasurementUnit"], "шт");

    let ffd105: Receipt = serde_json::from_value(json!({
        "Taxation": "usn_income", "Phone": "+7",
        "Items": [{"Name": "x", "Price": 1, "Quantity": 1, "Amount": 1, "Tax": "none"}]
    }))
    .unwrap();
    assert!(matches!(ffd105, Receipt::Ffd105(_)));

    // Битый ФФД 1.2 даёт ошибку, а не тихий откат на ФФД 1.05.
    let broken = serde_json::from_value::<Receipt>(json!({
        "Taxation": "osn", "Email": "a@b.ru",
        "Items": [{"Name": "x", "Price": 1, "Quantity": 1, "Amount": 1, "Tax": "none", "MeasurementUnit": "шт"}]
    }));
    assert!(broken.is_err());
}

fn signed(v: serde_json::Value) -> Vec<u8> {
    let mut v = v;
    v["Token"] = token::sign("pw", v.as_object().unwrap()).into();
    serde_json::to_vec(&v).unwrap()
}

#[test]
fn notification_without_terminal_key_is_rejected() {
    let body = signed(json!({"OrderId": "1", "Status": "CONFIRMED", "PaymentId": 1}));
    let err = NotificationVerifier::new("T", "pw")
        .verify(&body)
        .unwrap_err();
    assert!(matches!(err, NotificationError::MissingTerminalKey));
}

#[test]
fn notification_with_null_data_is_accepted() {
    let body = signed(
        json!({"TerminalKey": "T", "OrderId": "1", "Status": "CONFIRMED", "PaymentId": 1, "Data": null}),
    );
    let Notification::Payment(p) = NotificationVerifier::new("T", "pw").verify(&body).unwrap()
    else {
        panic!()
    };
    assert!(p.data.is_empty());
}

#[test]
fn get_path_segments_are_escaped() {
    assert_eq!(
        TPayLink::new("1/../2").version("2.0").path("T").as_ref(),
        "v2/TinkoffPay/transactions/1%2F..%2F2/versions/2.0/link"
    );
    assert_eq!(
        TPayStatus::new().path("A B").as_ref(),
        "v2/TinkoffPay/terminals/A%20B/status"
    );
}

#[tokio::test]
async fn decode_errors_do_not_print_rebill_id() {
    let server = MockServer::start().await;
    // Ответ, который не подходит под тип (нет Status), с RebillId внутри.
    Mock::given(path("/v2/GetState"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": true, "ErrorCode": "0", "PaymentId": "1", "RebillId": "SECRET-REBILL"
        })))
        .mount(&server)
        .await;
    let client = Client::builder("T", "P")
        .base_url(server.uri())
        .build()
        .unwrap();
    let err = client.get_state(&GetState::new("1")).await.unwrap_err();
    assert!(matches!(err, Error::Decode { .. }), "{err}");
    let text = err.to_string();
    assert!(!text.contains("SECRET-REBILL"), "{text}");
    assert!(text.contains("***"), "{text}");
}
