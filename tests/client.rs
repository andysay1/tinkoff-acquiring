//! Поведение клиента на мок-сервере: подпись, ошибки, повторы, форматы.

use serde_json::{json, Value};
use std::time::Duration;
use tinkoff_acquiring::*;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, Request as WireRequest, ResponseTemplate};

const TK: &str = "TestTerminal";
const PASSWORD: &str = "secret";

async fn client(server: &MockServer) -> Client {
    Client::builder(TK, PASSWORD)
        .base_url(server.uri())
        .retry(RetryPolicy::new(
            2,
            Duration::from_millis(1),
            Duration::from_millis(5),
        ))
        .build()
        .unwrap()
}

fn json_body(req: &WireRequest) -> serde_json::Map<String, Value> {
    serde_json::from_slice::<Value>(&req.body)
        .unwrap()
        .as_object()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn init_is_signed_like_the_bank_expects() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v2/Init"))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": true, "ErrorCode": "0", "TerminalKey": TK, "Status": "NEW",
            "PaymentId": "3093639567", "OrderId": "21090", "Amount": 140000,
            "PaymentURL": "https://pay.tbank.ru/new/fU1ppgqa"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let receipt = ReceiptFfd12::new(
        Taxation::Osn,
        vec![ItemFfd12::new("Товар", 140_000, 1.0, Tax::Vat22)],
    )
    .email("a@b.ru");
    let resp = client(&server)
        .await
        .init(
            &Init::new(140_000, "21090")
                .description("Заказ")
                .receipt(receipt)
                .data("Phone", "+7"),
        )
        .await
        .unwrap();
    assert_eq!(resp.payment_id, "3093639567");
    assert_eq!(resp.status, Some(PaymentStatus::New));
    assert_eq!(
        resp.payment_url.as_deref(),
        Some("https://pay.tbank.ru/new/fU1ppgqa")
    );

    let req = &server.received_requests().await.unwrap()[0];
    let body = json_body(req);
    assert_eq!(body["TerminalKey"], TK);
    assert_eq!(body["Amount"], 140_000);
    assert_eq!(body["Receipt"]["Items"][0]["Tax"], "vat22");
    assert_eq!(body["Receipt"]["Items"][0]["MeasurementUnit"], "шт");
    assert_eq!(body["DATA"]["Phone"], "+7");
    // Токен по корневым полям, без DATA и Receipt.
    let expected = token::sign_pairs(
        PASSWORD,
        [
            ("TerminalKey", TK),
            ("Amount", "140000"),
            ("OrderId", "21090"),
            ("Description", "Заказ"),
        ],
    );
    assert_eq!(body["Token"], expected);
}

#[tokio::test]
async fn success_false_becomes_api_error_with_raw_body() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/Charge"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": false, "ErrorCode": "10", "Message": "Метод Charge заблокирован",
            "Status": "REJECTED", "PaymentId": 42
        })))
        .mount(&server)
        .await;
    let err = client(&server)
        .await
        .charge(&Charge::new("42", "777"))
        .await
        .unwrap_err();
    let api = err.api().expect("ошибка API");
    assert_eq!(api.code, "10");
    assert_eq!(api.status(), Some("REJECTED"));
    assert_eq!(api.payment_id().as_deref(), Some("42"));
    assert_eq!(err.error_code(), Some("10"));

    let body = json_body(&server.received_requests().await.unwrap()[0]);
    assert_eq!(body["RebillId"], "777");
}

#[tokio::test]
async fn read_only_methods_are_retried_on_5xx() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/GetState"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(path("/v2/GetState"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": true, "ErrorCode": "0", "Status": "CONFIRMED", "PaymentId": "1",
            "OrderId": "o", "Amount": 100, "TerminalKey": TK
        })))
        .mount(&server)
        .await;
    let state = client(&server)
        .await
        .get_state(&GetState::new("1"))
        .await
        .unwrap();
    assert_eq!(state.status, PaymentStatus::Confirmed);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn money_moving_methods_are_not_retried_on_5xx() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/Init"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "Success": false, "ErrorCode": "500", "Message": "Internal", "Causes": ["timeout"]
        })))
        .mount(&server)
        .await;
    let err = client(&server)
        .await
        .init(&Init::new(100, "o"))
        .await
        .unwrap_err();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert!(err.is_transient());
    assert_eq!(err.api().unwrap().http_status, Some(500));
}

#[tokio::test]
async fn cancel_with_external_request_id_is_retried() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/Cancel"))
        .respond_with(ResponseTemplate::new(502))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(path("/v2/Cancel"))
        .and(body_string_contains("ExternalRequestId"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": true, "ErrorCode": "0", "Status": "REFUNDED", "PaymentId": "1",
            "OrderId": "o", "OriginalAmount": 100, "NewAmount": 0, "TerminalKey": TK
        })))
        .mount(&server)
        .await;
    let resp = client(&server)
        .await
        .cancel(&Cancel::new("1").external_request_id("refund-1"))
        .await
        .unwrap();
    assert_eq!(resp.status, PaymentStatus::Refunded);
    assert_eq!(resp.new_amount, Some(0));
}

#[tokio::test]
async fn card_list_is_an_array() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/GetCardList"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"CardId": "881900", "Pan": "518223******0036", "Status": "A", "RebillId": "6155312073", "CardType": 0, "ExpDate": "1122"},
            {"CardId": 881901, "Pan": "430000******0777", "Status": "D", "CardType": 2}
        ])))
        .mount(&server)
        .await;
    let cards = client(&server)
        .await
        .get_card_list(&GetCardList::new("c"))
        .await
        .unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].rebill_id.as_deref(), Some("6155312073"));
    assert_eq!(cards[1].card_id, "881901");
    assert_eq!(cards[1].card_type, Some(CardType::ChargeAndTopUp));
}

#[tokio::test]
async fn submit_3ds_uses_form_encoding() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/Submit3DSAuthorizationV2"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": true, "ErrorCode": "0", "Status": "CONFIRMED", "PaymentId": "1",
            "OrderId": "o", "TerminalKey": TK
        })))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .await
        .submit_3ds_authorization_v2(&Submit3dsAuthorizationV2::new("1").cres("Y3Jlcw"))
        .await
        .unwrap();
    let req = &server.received_requests().await.unwrap()[0];
    let form: Vec<(String, String)> = serde_urlencoded_pairs(&req.body);
    let get = |k: &str| form.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    assert_eq!(get("PaymentId").as_deref(), Some("1"));
    assert_eq!(get("cres").as_deref(), Some("Y3Jlcw"));
    let expected = token::sign_pairs(
        PASSWORD,
        [("PaymentId", "1"), ("TerminalKey", TK), ("cres", "Y3Jlcw")],
    );
    assert_eq!(get("Token"), Some(expected));
}

fn serde_urlencoded_pairs(body: &[u8]) -> Vec<(String, String)> {
    std::str::from_utf8(body)
        .unwrap()
        .split('&')
        .map(|kv| {
            let (k, v) = kv.split_once('=').unwrap();
            (decode(k), decode(v))
        })
        .collect()
}

fn decode(s: &str) -> String {
    let bytes = s.replace('+', " ").into_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            out.push(
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap(), 16).unwrap(),
            );
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

#[tokio::test]
async fn tpay_status_is_an_unsigned_get() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v2/TinkoffPay/terminals/{TK}/status")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": true, "ErrorCode": "0", "Params": {"Allowed": true, "Version": "2.0"}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let r = client(&server)
        .await
        .tpay_status(&TPayStatus::new())
        .await
        .unwrap();
    assert!(r.params.allowed);
    assert_eq!(r.params.version.as_deref(), Some("2.0"));
}

#[tokio::test]
async fn confirm_operation_token_uses_terminal_key_only() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/getConfirmOperation"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "Success": true, "ErrorCode": "0",
            "PaymentIdList": [{"PaymentId": 1, "Success": true, "ErrorCode": "0", "Message": "OK"}]
        })))
        .mount(&server)
        .await;
    let r = client(&server)
        .await
        .get_confirm_operation(&GetConfirmOperation::by_callback(
            vec![1, 2],
            "https://e/cb",
        ))
        .await
        .unwrap();
    assert_eq!(r.payment_id_list[0].payment_id, "1");
    let body = json_body(&server.received_requests().await.unwrap()[0]);
    assert_eq!(
        body["Token"],
        token::sign_pairs(PASSWORD, [("TerminalKey", TK)])
    );
    assert_eq!(body["PaymentIdList"], json!([1, 2]));
}

#[tokio::test]
async fn invalid_request_is_not_sent() {
    let server = MockServer::start().await;
    let receipt = ReceiptFfd12::new(
        Taxation::Osn,
        vec![ItemFfd12::new("Товар", 100, 1.0, Tax::Vat22)],
    )
    .email("a@b.ru");
    let err = client(&server)
        .await
        .init(&Init::new(200, "o").receipt(receipt))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "{err}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn execute_raw_signs_unknown_methods() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/NewMethod"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"Success": true, "ErrorCode": "0", "X": 1})),
        )
        .mount(&server)
        .await;
    let mut body = serde_json::Map::new();
    body.insert("PaymentId".into(), "5".into());
    let v = client(&server)
        .await
        .execute_raw("v2/NewMethod", body)
        .await
        .unwrap();
    assert_eq!(v["X"], 1);
    let sent = json_body(&server.received_requests().await.unwrap()[0]);
    assert_eq!(
        sent["Token"],
        token::sign_pairs(PASSWORD, [("PaymentId", "5"), ("TerminalKey", TK)])
    );
}

#[tokio::test]
async fn non_json_gateway_error() {
    let server = MockServer::start().await;
    Mock::given(path("/v2/Init"))
        .respond_with(ResponseTemplate::new(502).set_body_string("<html>Bad Gateway</html>"))
        .mount(&server)
        .await;
    let err = client(&server)
        .await
        .init(&Init::new(100, "o"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Http { status: 502, .. }), "{err}");
}

#[test]
fn debug_output_hides_password() {
    let c = Client::new(TK, "super-secret-password").unwrap();
    let s = format!("{c:?}");
    assert!(s.contains(TK));
    assert!(!s.contains("super-secret-password"));
}
