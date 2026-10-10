# Changelog

## 0.3.0 — 2026-10

Полная переработка под актуальную спецификацию Т-Банка (OpenAPI v1.36). API
несовместим с 0.2.

### Изменения у Т-Банка, которые теперь поддержаны

- **Сертификаты НУЦ Минцифры.** API Т-Банка переходит на Russian Trusted CA; без него
  TLS-рукопожатие падает. Feature `russian-trusted-ca` (по умолчанию) добавляет
  корневой сертификат в HTTP-клиент библиотеки.
- **НДС 22 % с 01.01.2026**: `Tax::Vat22`, `Tax::Vat122`, а также `Vat5`, `Vat7`,
  `Vat105`, `Vat107` для УСН. `Vat20`/`Vat120` оставлены для чеков возврата по
  продажам 2025 года.
- Чеки **ФФД 1.2** (`ReceiptFfd12`, `ItemFfd12`): единицы измерения, маркировка,
  отраслевые и операционные реквизиты, данные покупателя.
- `OperationInitiatorType` (признак инициатора операции) для рекуррентных платежей.
- Новые статусы: `PAY_CHECKING`, `CONFIRM_CHECKING`, `ATTEMPTS_EXPIRED`,
  `ASYNC_REFUNDING` и др.
- Новые методы: `AddCard`/`AttachCard`/`GetAddCardState`, `Check3dsVersion`,
  `Submit3DSAuthorization(V2)`, вся линейка СБП и автоплатежей по СБП, цифровой
  рубль, T-Pay, SberPay, Mir Pay, Alfa Pay, `CheckOrder`, `SendClosingReceipt`,
  `getConfirmOperation`.
- Идемпотентность возвратов через `ExternalRequestId`.

### Исправленные ошибки 0.2

- `Status` не разбирал 15 из 24 статусов (`3DS_CHECKING`, `REFUNDED`,
  `PARTIAL_REFUNDED`, …): `get_state` падал на любом возврате. Теперь все
  статусы спецификации плюс `Other(String)` для будущих.
- `Taxation::UsnIncome` сериализовался как `"usnincome"` вместо `"usn_income"`.
- `charge` не передавал обязательный `RebillId`; `init` не умел `CustomerKey` —
  рекуррентные платежи не работали.
- `bind_card` вызывал несуществующий метод `BindCard`.
- `get_card_list` ожидал объект, а API возвращает массив.
- Заголовок `Idempotence-Key` не поддерживается API — убран; повторы больше не
  применяются к `Init`/`Charge`/`Confirm` при таймаутах и 5xx.
- `blocking`-клиент был пустой заглушкой — теперь в нём все методы.
- Настройка `max_retries` игнорировалась.
- Зависимость `backoff` (не поддерживается, RUSTSEC-2025-0012) удалена.

### Переход с 0.2

| 0.2 | 0.3 |
|---|---|
| `TinkoffClient::new(Config::new(k, p))` | `Client::new(k, p)?` |
| `client.init(amount, order, Some(desc), …10 аргументов)` | `client.init(&Init::new(amount, order).description(desc))` |
| `Receipt { items: vec![Item { … }], … }` | `ReceiptFfd12::new(taxation, vec![ItemFfd12::new(name, price, qty, tax)])` |
| `Tax::Vat20` | `Tax::Vat22` (с 2026 года) |
| `client.charge(payment_id)` | `client.charge(&Charge::new(payment_id, rebill_id))` |
| `client.bind_card(customer_key, _)` | `client.add_card(&AddCard::new(customer_key))` |
| `verify_webhook_token(k, p, &json)` | `NotificationVerifier::new(k, p).verify(&body_bytes)` |
| `make_token(k, p, fields)` | `token::sign(p, &json_object)` / `token::sign_pairs(p, pairs)` |
| `Error::Api { code, message }` | `Error::Api(ApiError { code, message, details, raw, .. })` |
