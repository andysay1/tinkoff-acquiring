//! Сверка типов со спецификацией Т-Банка (openapi.yaml v1.36).
//!
//! `fixtures/openapi_examples.json` — выжимка из спецификации: для каждого
//! метода список полей запроса и примеры успешных ответов.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use tinkoff_acquiring::*;

fn spec() -> Value {
    serde_json::from_str(include_str!("fixtures/openapi_examples.json")).unwrap()
}

fn endpoint<'a>(spec: &'a Value, path: &str) -> &'a Value {
    spec.get(path)
        .unwrap_or_else(|| panic!("в спецификации нет {path}"))
}

fn parses<T: DeserializeOwned + std::fmt::Debug>(path: &str) {
    let spec = spec();
    let examples = endpoint(&spec, path)["response_examples"]
        .as_array()
        .unwrap();
    assert!(!examples.is_empty(), "{path}: нет примеров ответа");
    for ex in examples {
        if let Err(e) = serde_json::from_value::<T>(ex.clone()) {
            panic!("{path}: пример ответа не разобрался: {e}\n{ex:#}");
        }
    }
}

#[test]
fn response_examples_parse() {
    parses::<InitResponse>("/v2/Init");
    parses::<Check3dsVersionResponse>("/v2/Check3dsVersion");
    parses::<FinishAuthorizeResponse>("/v2/FinishAuthorize");
    parses::<ConfirmResponse>("/v2/Confirm");
    parses::<CancelResponse>("/v2/Cancel");
    parses::<ChargeResponse>("/v2/Charge");
    parses::<GetStateResponse>("/v2/GetState");
    parses::<CustomerResponse>("/v2/AddCustomer");
    parses::<CustomerResponse>("/v2/GetCustomer");
    parses::<CustomerResponse>("/v2/RemoveCustomer");
    parses::<AddCardResponse>("/v2/AddCard");
    parses::<AttachCardResponse>("/v2/AttachCard");
    parses::<AcsResult>("/v2/AcsUrl");
    parses::<Submit3dsResponse>("/v2/Submit3DSAuthorization");
    parses::<Submit3dsResponse>("/v2/Submit3DSAuthorizationV2");
    parses::<GetAddCardStateResponse>("/v2/GetAddCardState");
    // По схеме и по живому API ответ — массив, а пример в спецификации —
    // одиночная карта.
    parses::<Card>("/v2/GetCardList");
    parses::<RemoveCardResponse>("/v2/RemoveCard");
    parses::<GetQrResponse>("/v2/GetQr");
    parses::<TPayStatusResponse>("/v2/TinkoffPay/terminals/{TerminalKey}/status");
    parses::<LinkResponse>("/v2/TinkoffPay/transactions/{paymentId}/versions/{version}/link");
    parses::<LinkResponse>("/v2/SberPay/link/get");
    parses::<LinkResponse>("/v2/SberPay/transactions/{paymentId}/link");
    parses::<QrMembersListResponse>("/v2/QrMembersList");
    parses::<AddAccountQrResponse>("/v2/AddAccountQr");
    parses::<AccountQr>("/v2/GetAddAccountQrState");
    parses::<GetAccountQrListResponse>("/v2/GetAccountQrList");
    parses::<ChargeQrResponse>("/v2/ChargeQr");
    parses::<SbpPayTestResponse>("/v2/SbpPayTest");
    parses::<GetQrStateResponse>("/v2/GetQrState");
    parses::<GetQrBankListResponse>("/v2/GetQrBankList");
    parses::<CheckOrderResponse>("/v2/CheckOrder");
    parses::<SendClosingReceiptResponse>("/cashbox/SendClosingReceipt");
    parses::<GetConfirmOperationResponse>("/v2/getConfirmOperation");
}

#[test]
fn spec_examples_keep_values() {
    let spec = spec();
    let ex = &endpoint(&spec, "/v2/GetState")["response_examples"][0];
    let r: GetStateResponse = serde_json::from_value(ex.clone()).unwrap();
    assert_eq!(r.payment_id, ex["PaymentId"].as_str().unwrap());
    assert_eq!(r.status.as_str(), ex["Status"].as_str().unwrap());

    let ex = &endpoint(&spec, "/v2/GetCardList")["response_examples"][0];
    let cards: Vec<Card> = serde_json::from_value(Value::Array(vec![ex.clone()])).unwrap();
    assert_eq!(cards[0].card_id, ex["CardId"].as_str().unwrap());
    assert_eq!(cards[0].status.as_str(), ex["Status"].as_str().unwrap());
}

/// Ключи запроса (без `TerminalKey`/`Token`, их добавляет клиент) должны
/// быть в спецификации метода, а путь — совпадать с путём спецификации.
fn fields_in_spec<R: Request + Serialize>(req: &R, extra_allowed: &[&str]) {
    let spec = spec();
    let path = format!("/{}", R::PATH);
    let allowed: BTreeSet<String> = endpoint(&spec, &path)["request_props"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .chain(extra_allowed.iter().map(|s| s.to_string()))
        .collect();
    let body = serde_json::to_value(req).unwrap();
    for key in body.as_object().unwrap().keys() {
        assert!(
            allowed.contains(key),
            "{path}: поля {key} нет в спецификации; есть {allowed:?}"
        );
    }
    assert!(req.validate().is_ok(), "{path}: {:?}", req.validate());
}

fn receipt12() -> ReceiptFfd12 {
    ReceiptFfd12::new(
        Taxation::Osn,
        vec![ItemFfd12::new("Товар", 10_000, 1.0, Tax::Vat22)
            .user_data("u")
            .country_code("643")
            .mark_code(MarkCodeType::Gs1m, "010460")
            .mark_quantity(MarkQuantity::new(1, 2))
            .agent_data(AgentData::default().agent_sign(AgentSign::Another))
            .supplier_info(SupplierInfo::default().name("ООО").inn("7710140679"))],
    )
    .email("a@b.ru")
    .phone("+79990000000")
    .client_info(ClientInfo::default().document_code("21"))
    .customer("ООО Покупатель")
    .customer_inn("7710140679")
    .payments(Payments::electronic(10_000))
    .operating_check_props(OperatingCheckProps::new("0", "v", "01.01.2026 10:00:00"))
    .sectoral_check_props(vec![SectoralProps::new("001", "01.01.2026", "1", "v")])
    .add_user_prop(AddUserProp::new("n", "v"))
    .additional_check_props("x")
}

#[test]
fn request_fields_exist_in_spec() {
    fields_in_spec(
        &Init::new(10_000, "o")
            .description("d")
            .customer_key("c")
            .recurrent()
            .two_stage()
            .language(Language::Ru)
            .notification_url("https://e/n")
            .success_url("https://e/s")
            .fail_url("https://e/f")
            .redirect_due_date("2026-12-31T23:59:59+03:00")
            .operation_initiator_type(OperationInitiatorType::CitCredentialCapture)
            .receipt(receipt12())
            .shops(vec![Shop::new("s", 10_000).name("n").fee("1")]),
        &[],
    );
    fields_in_spec(&GetState::new("1").ip("1.1.1.1").get_phone(true), &[]);
    fields_in_spec(
        &Confirm::new("1")
            .ip("1.1.1.1")
            .amount(10_000)
            .receipt(receipt12())
            .shops(vec![Shop::new("s", 1)])
            .route(Route::Tcb)
            .source(Source::Installment),
        &[],
    );
    fields_in_spec(
        &Cancel::new("1")
            .ip("1.1.1.1")
            .amount(10_000)
            .receipt(
                ReceiptFfd105::new(
                    Taxation::Osn,
                    vec![ItemFfd105::new("Товар", 10_000, 1.0, Tax::Vat22)],
                )
                .email("a@b.ru"),
            )
            .shops(vec![Shop::new("s", 1)])
            .qr_member_id("m")
            .route(Route::Bnpl)
            .source(Source::Bnpl)
            .external_request_id("x"),
        &[],
    );
    fields_in_spec(
        &Charge::new("1", "2")
            .ip("1.1.1.1")
            .send_email(true)
            .info_email("a@b.ru"),
        &[],
    );
    fields_in_spec(&CheckOrder::new("o"), &[]);
    fields_in_spec(&SendClosingReceipt::new("1", receipt12()), &[]);
    fields_in_spec(
        &GetConfirmOperation::by_email(vec![1], vec!["a@b.ru".into()]),
        &[],
    );
    fields_in_spec(&GetConfirmOperation::by_callback(vec![1], "https://e"), &[]);
    fields_in_spec(
        &AddCustomer::new("c").ip("1").email("a@b.ru").phone("+7"),
        &[],
    );
    fields_in_spec(&GetCustomer::new("c").ip("1"), &[]);
    fields_in_spec(&RemoveCustomer::new("c").ip("1"), &[]);
    fields_in_spec(
        &AddCard::new("c")
            .check_type(CheckType::ThreeDsHold)
            .ip("1")
            .resident_state(true),
        &[],
    );
    let browser = ThreeDsBrowserData::new("Y", "ru", "-180", "800", "600", "https://e/cres")
        .color_depth("24")
        .java_enabled(false);
    fields_in_spec(&AttachCard::new("rk", "enc").data(browser.clone()), &[]);
    fields_in_spec(&GetAddCardState::new("rk"), &[]);
    fields_in_spec(&GetCardList::new("c").saved_card(true).ip("1"), &[]);
    fields_in_spec(&RemoveCard::new("c", "1").ip("1"), &[]);
    fields_in_spec(&Check3dsVersion::new("1", "enc"), &[]);
    fields_in_spec(
        &FinishAuthorize::new("1", "enc")
            .amount(1)
            .ip("1")
            .send_email(true)
            .info_email("a@b.ru")
            .source("cards")
            .route("ACQ")
            .device_channel(DeviceChannel::Browser)
            .data(browser),
        &[],
    );
    fields_in_spec(
        &Submit3dsAuthorization::new("md", "pares").payment_id("1"),
        &[],
    );
    // `cres` в спецификации нет, но API его требует (ошибка 110 «Empty cres»).
    fields_in_spec(&Submit3dsAuthorizationV2::new("1").cres("c"), &["cres"]);
    fields_in_spec(
        &GetQr::new("1")
            .data_type(QrDataType::Image)
            .bank_id("b")
            .payment_method(QrPaymentMethod::DigitalRuble),
        &[],
    );
    fields_in_spec(&QrMembersList::new("1"), &[]);
    fields_in_spec(&GetQrState::new("1"), &[]);
    fields_in_spec(
        &GetQrBankList::new(Device::new(DeviceType::Mobile).os("iOS"))
            .scenario_type(QrScenario::Subscription)
            .payment_method(QrPaymentMethod::Sbp),
        &[],
    );
    fields_in_spec(
        &SbpPayTest::new("1")
            .is_deadline_expired(false)
            .is_rejected(true),
        &[],
    );
    fields_in_spec(
        &AddAccountQr::new("d")
            .data_type(QrDataType::Payload)
            .bank_id("b")
            .data("k", "v")
            .redirect_due_date("2026-12-31T23:59:59+03:00"),
        &[],
    );
    fields_in_spec(&GetAddAccountQrState::new("rk"), &[]);
    fields_in_spec(&GetAccountQrList::new(), &[]);
    fields_in_spec(
        &ChargeQr::new("1", "t")
            .ip("1")
            .send_email(true)
            .info_email("a@b.ru")
            .bank_member_id("m"),
        &[],
    );
    fields_in_spec(&SberPayLinkGet::new("1").phone("+7"), &[]);
    fields_in_spec(&MirPayDeepLink::new("1"), &[]);
    fields_in_spec(&AlfaPayLink::new("1"), &[]);
}

#[test]
fn get_paths_match_spec() {
    let spec = spec();
    for p in [TPayStatus::PATH, TPayLink::PATH, SberPayLink::PATH] {
        assert!(spec.get(format!("/{p}")).is_some(), "{p}");
    }
    assert_eq!(
        TPayStatus::new().path("TK").as_ref(),
        "v2/TinkoffPay/terminals/TK/status"
    );
    assert_eq!(
        TPayLink::new("42").path("TK").as_ref(),
        "v2/TinkoffPay/transactions/42/versions/2.0/link"
    );
    assert_eq!(
        SberPayLink::new("42").path("TK").as_ref(),
        "v2/SberPay/transactions/42/link"
    );
}

#[test]
fn every_spec_method_is_covered() {
    let covered: BTreeSet<&str> = [
        Init::PATH,
        Check3dsVersion::PATH,
        FinishAuthorize::PATH,
        Confirm::PATH,
        Cancel::PATH,
        Charge::PATH,
        GetState::PATH,
        AddCustomer::PATH,
        GetCustomer::PATH,
        RemoveCustomer::PATH,
        AddCard::PATH,
        AttachCard::PATH,
        Submit3dsAuthorization::PATH,
        Submit3dsAuthorizationV2::PATH,
        GetAddCardState::PATH,
        GetCardList::PATH,
        RemoveCard::PATH,
        GetQr::PATH,
        TPayStatus::PATH,
        TPayLink::PATH,
        SberPayLinkGet::PATH,
        SberPayLink::PATH,
        MirPayDeepLink::PATH,
        AlfaPayLink::PATH,
        QrMembersList::PATH,
        AddAccountQr::PATH,
        GetAddAccountQrState::PATH,
        GetAccountQrList::PATH,
        ChargeQr::PATH,
        SbpPayTest::PATH,
        GetQrState::PATH,
        GetQrBankList::PATH,
        CheckOrder::PATH,
        SendClosingReceipt::PATH,
        GetConfirmOperation::PATH,
    ]
    .into_iter()
    .collect();
    // Эти два шага выполняет браузер покупателя на стороне ACS; для них
    // есть помощники `three_ds_method_data` и `three_ds_creq`.
    let browser_side = ["v2/ThreeDSMethod", "v2/AcsUrl"];
    let spec = spec();
    for path in spec.as_object().unwrap().keys() {
        let p = path.trim_start_matches('/');
        assert!(
            covered.contains(p) || browser_side.contains(&p),
            "метод {path} из спецификации не покрыт"
        );
    }
}
