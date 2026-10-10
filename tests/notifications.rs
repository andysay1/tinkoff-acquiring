//! Проверка уведомлений на примерах из документации Т-Банка.

use serde_json::{json, Value};
use tinkoff_acquiring::*;

const TK: &str = "1234567890DEMO";
const PASSWORD: &str = "11111111111";

/// Подписать уведомление так, как это делает банк.
fn signed(mut v: Value) -> Vec<u8> {
    let token = token::sign(PASSWORD, v.as_object().unwrap());
    v["Token"] = token.into();
    serde_json::to_vec(&v).unwrap()
}

fn verifier() -> NotificationVerifier {
    NotificationVerifier::new(TK, PASSWORD)
}

#[test]
fn docs_token_example_from_raw_body() {
    // Пример со страницы «Уведомления»: ожидаемый токен из документации.
    let body = br#"{"TerminalKey":"1234567890DEMO","OrderId":"000000","Success":true,"Status":"AUTHORIZED","PaymentId":"0000000","ErrorCode":"0","Amount":"1111","CardId":"000000","Pan":"200000******0000","ExpDate":"1111","RebillId":"000000","Token":"1c0964277d0213349243065a0d5b838b8e90d2d25f740d0f2767836e710e80c8"}"#;
    let n = verifier().verify(body).unwrap();
    let Notification::Payment(p) = n else {
        panic!("ожидалось уведомление о платеже")
    };
    assert_eq!(p.status, PaymentStatus::Authorized);
    assert_eq!(p.rebill_id.as_deref(), Some("000000"));
    assert_eq!(p.amount, Some(1111));
}

#[test]
fn card_payment_with_numeric_ids() {
    let body = signed(json!({
        "TerminalKey": TK, "OrderId": "7000001", "Success": true, "Status": "CONFIRMED",
        "PaymentId": 123456789, "ErrorCode": "0", "Amount": 10000, "CardId": 6000003,
        "Pan": "200000******0003", "ExpDate": "0424"
    }));
    match verifier().verify(&body).unwrap() {
        Notification::Payment(p) => {
            assert_eq!(p.payment_id, "123456789");
            assert_eq!(p.card_id.as_deref(), Some("6000003"));
            assert_eq!(p.status, PaymentStatus::Confirmed);
            assert!(p.success);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn sbp_payment_with_data_object() {
    let body = signed(json!({
        "TerminalKey": TK, "OrderId": "0-1", "Success": true, "Status": "AUTHORIZED",
        "PaymentId": 800000004, "ErrorCode": "0", "Amount": 5000, "Pan": "+7(98*)***-**-*4",
        "Data": {"Source": "qrsbp", "interBankOperation": false, "REDIRECT": false}
    }));
    let Notification::Payment(p) = verifier().verify(&body).unwrap() else {
        panic!()
    };
    assert_eq!(p.data["Source"], "qrsbp");
}

#[test]
fn rejected_recurring_charge() {
    let body = signed(json!({
        "TerminalKey": TK, "OrderId": "1", "Success": false, "Status": "REJECTED",
        "PaymentId": 900000007, "ErrorCode": "5060", "Amount": 100,
        "Data": {"OperationInitiatorType": "I", "QR": "true"}
    }));
    let Notification::Payment(p) = verifier().verify(&body).unwrap() else {
        panic!()
    };
    assert!(!p.success);
    assert_eq!(p.error_code, "5060");
}

#[test]
fn card_binding() {
    let body = signed(json!({
        "TerminalKey": TK, "CustomerKey": "8-9", "RequestKey": "3-5", "Success": true,
        "Status": "COMPLETED", "PaymentId": 123456789, "ErrorCode": "0", "CardId": 600000005,
        "Pan": "200000******0001", "ExpDate": "0424", "NotificationType": "LINKCARD",
        "RebillId": "100000002"
    }));
    let Notification::CardBinding(c) = verifier().verify(&body).unwrap() else {
        panic!()
    };
    assert_eq!(c.status, AddCardStatus::Completed);
    assert_eq!(c.rebill_id.as_deref(), Some("100000002"));
    assert_eq!(c.card_id.as_deref(), Some("600000005"));
}

#[test]
fn sbp_account_binding_with_string_success() {
    let body = signed(json!({
        "TerminalKey": TK, "RequestKey": "16", "Status": "ACTIVE", "Success": "true",
        "ErrorCode": "0", "Message": "Плательщик подтвердил привязку счета",
        "AccountToken": "abc", "BankMemberId": "100000000001", "BankMemberName": "bankname",
        "NotificationType": "LINKACCOUNT"
    }));
    let Notification::AccountBinding(a) = verifier().verify(&body).unwrap() else {
        panic!()
    };
    assert!(a.success);
    assert_eq!(a.status, AccountQrStatus::Active);
    assert_eq!(a.account_token.as_deref(), Some("abc"));
}

#[test]
fn fiscalization_with_nested_receipt() {
    let body = signed(json!({
        "TerminalKey": TK, "OrderId": "21050", "Success": true, "Status": "RECEIPT",
        "PaymentId": 13660, "ErrorCode": "0", "Amount": 100000, "FiscalNumber": 0,
        "ShiftNumber": 0, "ReceiptDatetime": "string", "FnNumber": "string",
        "FiscalDocumentNumber": 0, "FiscalDocumentAttribute": 0, "Type": "Income",
        "Receipt": {"Taxation": "osn", "Items": [], "Payments": [{"Electronic": 100000}]}
    }));
    let Notification::Fiscalization(f) = verifier().verify(&body).unwrap() else {
        panic!()
    };
    assert_eq!(f.status, "RECEIPT");
    assert_eq!(f.kind.as_deref(), Some("Income"));
    assert!(f.receipt.is_some());
}

#[test]
fn unknown_status_does_not_break_parsing() {
    let body = signed(json!({
        "TerminalKey": TK, "OrderId": "1", "Success": true, "Status": "SOMETHING_NEW",
        "PaymentId": 1, "ErrorCode": "0"
    }));
    let Notification::Payment(p) = verifier().verify(&body).unwrap() else {
        panic!()
    };
    assert_eq!(p.status, PaymentStatus::Other("SOMETHING_NEW".into()));
}

#[test]
fn wrong_password_is_rejected() {
    let body =
        signed(json!({"TerminalKey": TK, "OrderId": "1", "Status": "CONFIRMED", "PaymentId": 1}));
    let err = NotificationVerifier::new(TK, "other")
        .verify(&body)
        .unwrap_err();
    assert!(matches!(err, NotificationError::InvalidToken));
}

#[test]
fn tampered_amount_is_rejected() {
    let body = signed(
        json!({"TerminalKey": TK, "OrderId": "1", "Status": "CONFIRMED", "PaymentId": 1, "Amount": 100}),
    );
    let tampered = String::from_utf8(body)
        .unwrap()
        .replace("\"Amount\":100", "\"Amount\":1");
    let err = verifier().verify(tampered.as_bytes()).unwrap_err();
    assert!(matches!(err, NotificationError::InvalidToken));
}

#[test]
fn other_terminal_is_rejected() {
    let mut v =
        json!({"TerminalKey": "OTHER", "OrderId": "1", "Status": "CONFIRMED", "PaymentId": 1});
    v["Token"] = token::sign(PASSWORD, v.as_object().unwrap()).into();
    let err = verifier()
        .verify(&serde_json::to_vec(&v).unwrap())
        .unwrap_err();
    assert!(matches!(err, NotificationError::TerminalMismatch(t) if t == "OTHER"));
}

#[test]
fn missing_token() {
    let err = verifier()
        .verify(br#"{"TerminalKey":"1234567890DEMO"}"#)
        .unwrap_err();
    assert!(matches!(err, NotificationError::MissingToken));
}

#[test]
fn verify_value_matches_verify() {
    let body = signed(
        json!({"TerminalKey": TK, "OrderId": "1", "Success": true, "Status": "CONFIRMED", "PaymentId": 5, "Amount": 700}),
    );
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert!(verifier().verify_value(&v).is_ok());
}

#[test]
fn null_fields_are_ignored_in_token() {
    // «Если значение параметра — null, оно не будет учитываться».
    let mut v = json!({"TerminalKey": TK, "OrderId": "1", "Status": "CONFIRMED", "PaymentId": 1});
    v["Token"] = token::sign(PASSWORD, v.as_object().unwrap()).into();
    v["RebillId"] = Value::Null;
    assert!(verifier().verify(&serde_json::to_vec(&v).unwrap()).is_ok());
}
