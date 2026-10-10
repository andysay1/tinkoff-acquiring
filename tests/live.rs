//! Проверки на настоящем API Т-Банка. По умолчанию пропускаются.
//!
//! ```sh
//! export TBANK_TERMINAL_KEY=...DEMO TBANK_PASSWORD=...
//! cargo test --all-features --test live -- --ignored --test-threads=1
//! ```
//!
//! Используйте DEMO-терминал: тесты создают и отменяют платежи, создают и
//! удаляют покупателей, проводят тестовую оплату по СБП (`SbpPayTest`).

use std::time::{SystemTime, UNIX_EPOCH};
use tinkoff_acquiring::*;

fn creds() -> Option<(String, String)> {
    match (
        std::env::var("TBANK_TERMINAL_KEY"),
        std::env::var("TBANK_PASSWORD"),
    ) {
        (Ok(k), Ok(p)) => Some((k, p)),
        _ => {
            eprintln!("TBANK_TERMINAL_KEY/TBANK_PASSWORD не заданы — пропуск");
            None
        }
    }
}

fn client() -> Option<Client> {
    let (k, p) = creds()?;
    Some(Client::new(k, p).unwrap())
}

fn unique(prefix: &str) -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{prefix}-{ms}")
}

fn receipt(amount: Amount) -> ReceiptFfd12 {
    ReceiptFfd12::new(
        Taxation::UsnIncome,
        vec![ItemFfd12::new("Тестовый товар", amount, 1.0, Tax::None)],
    )
    .email("test@example.com")
}

#[tokio::test]
#[ignore]
async fn payment_lifecycle() {
    let Some(c) = client() else { return };
    let order = unique("live");
    let p = c
        .init(
            &Init::new(10_000, &order)
                .description("live test")
                .receipt(receipt(10_000)),
        )
        .await
        .unwrap();
    assert_eq!(p.status, Some(PaymentStatus::New));
    assert!(p.payment_url.is_some());

    let s = c.get_state(&GetState::new(&p.payment_id)).await.unwrap();
    assert_eq!(s.status, PaymentStatus::New);
    assert_eq!(s.amount, Some(10_000));

    let o = c.check_order(&CheckOrder::new(&order)).await.unwrap();
    assert_eq!(o.payments.len(), 1);

    let x = c.cancel(&Cancel::new(&p.payment_id)).await.unwrap();
    assert_eq!(x.status, PaymentStatus::Canceled);
}

#[tokio::test]
#[ignore]
async fn recurrent_parent_payment_with_ffd105_receipt() {
    let Some(c) = client() else { return };
    let p = c
        .init(
            &Init::new(10_000, unique("rec"))
                .recurrent()
                .customer_key(unique("cust"))
                .operation_initiator_type(OperationInitiatorType::CitCredentialCapture)
                .two_stage()
                .receipt(
                    ReceiptFfd105::new(
                        Taxation::Osn,
                        vec![ItemFfd105::new("Товар", 10_000, 1.0, Tax::Vat22)],
                    )
                    .email("test@example.com"),
                ),
        )
        .await
        .unwrap();
    c.cancel(&Cancel::new(&p.payment_id)).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn customers_and_cards() {
    let Some(c) = client() else { return };
    let ck = unique("cust");
    c.add_customer(&AddCustomer::new(&ck).email("test@example.com"))
        .await
        .unwrap();
    let got = c.get_customer(&GetCustomer::new(&ck)).await.unwrap();
    assert_eq!(got.email.as_deref(), Some("test@example.com"));
    assert!(c
        .get_card_list(&GetCardList::new(&ck))
        .await
        .unwrap()
        .is_empty());
    let add = c.add_card(&AddCard::new(&ck)).await.unwrap();
    assert!(add.payment_url.is_some());
    let st = c
        .get_add_card_state(&GetAddCardState::new(&add.request_key))
        .await
        .unwrap();
    assert_eq!(st.status, AddCardStatus::New);
    c.remove_customer(&RemoveCustomer::new(&ck)).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn sbp_test_payment() {
    let Some(c) = client() else { return };
    let p = c.init(&Init::new(10_000, unique("sbp"))).await.unwrap();
    let qr = c.get_qr(&GetQr::new(&p.payment_id)).await.unwrap();
    assert!(qr.data.starts_with("https://qr.nspk.ru/"), "{}", qr.data);
    c.sbp_pay_test(&SbpPayTest::new(&p.payment_id))
        .await
        .unwrap();
    let s = c.get_state(&GetState::new(&p.payment_id)).await.unwrap();
    assert_eq!(s.status, PaymentStatus::Confirmed);
    assert_eq!(s.param("Source"), Some("qrsbp"));
    let banks = c
        .get_qr_bank_list(&GetQrBankList::new(
            Device::new(DeviceType::Mobile).os("iOS"),
        ))
        .await
        .unwrap();
    assert!(!banks.bank_list.is_empty());
}

#[tokio::test]
#[ignore]
async fn tpay_status_and_wrong_password() {
    let Some((k, _)) = creds() else { return };
    let Some(c) = client() else { return };
    c.tpay_status(&TPayStatus::new()).await.unwrap();
    // Подпись проверяется после поиска платежа, поэтому нужен настоящий PaymentId.
    let p = c.init(&Init::new(10_000, unique("bad"))).await.unwrap();
    let bad = Client::builder(k, "wrong-password").build().unwrap();
    let err = bad
        .get_state(&GetState::new(&p.payment_id))
        .await
        .unwrap_err();
    assert_eq!(err.error_code(), Some("204"), "{err}");
    c.cancel(&Cancel::new(&p.payment_id)).await.unwrap();
}

#[cfg(feature = "blocking")]
#[test]
#[ignore]
fn blocking_client() {
    let Some((k, p)) = creds() else { return };
    let c = blocking::Client::new(k, p).unwrap();
    let pay = c.init(&Init::new(10_000, unique("blk"))).unwrap();
    assert_eq!(
        c.get_state(&GetState::new(&pay.payment_id)).unwrap().status,
        PaymentStatus::New
    );
    c.cancel(&Cancel::new(&pay.payment_id)).unwrap();
}
