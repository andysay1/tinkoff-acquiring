//! Чеки 54-ФЗ: ФФД 1.05 и ФФД 1.2.
//!
//! Версию ФФД определяет онлайн-касса, подключённая к терминалу. Для ФФД 1.2
//! используйте [`ReceiptFfd12`] и [`ItemFfd12`], для ФФД 1.05 — [`ReceiptFfd105`]
//! и [`ItemFfd105`]. Оба типа приводятся к [`Receipt`] через `From`.
//!
//! Часовой пояс места расчёта (тег 1011, обязателен с 01.09.2025 для чеков с
//! маркированными товарами) Т-Банк подставляет сам — по умолчанию Москва;
//! сменить его можно только через поддержку банка.

use crate::Amount;
use serde::{Deserialize, Serialize};

string_enum! {
    /// Система налогообложения (тег 1055).
    pub enum Taxation {
        /// Общая.
        Osn = "osn",
        /// УСН «доходы» (АУСН налоговая определит по ИНН сама).
        UsnIncome = "usn_income",
        /// УСН «доходы минус расходы».
        UsnIncomeOutcome = "usn_income_outcome",
        /// ЕСХН.
        Esn = "esn",
        /// Патент.
        Patent = "patent",
    }
}

string_enum! {
    /// Ставка НДС (тег 1199).
    ///
    /// С 01.01.2026 основная ставка — 22 %. Ставки 5 % и 7 % — для плательщиков
    /// УСН, перешедших на уплату НДС.
    pub enum Tax {
        /// Без НДС.
        None = "none",
        /// НДС 0 %.
        Vat0 = "vat0",
        /// НДС 5 %.
        Vat5 = "vat5",
        /// НДС 7 %.
        Vat7 = "vat7",
        /// НДС 10 %.
        Vat10 = "vat10",
        /// НДС 22 % (с 01.01.2026).
        Vat22 = "vat22",
        /// НДС по расчётной ставке 5/105.
        Vat105 = "vat105",
        /// НДС по расчётной ставке 7/107.
        Vat107 = "vat107",
        /// НДС по расчётной ставке 10/110.
        Vat110 = "vat110",
        /// НДС по расчётной ставке 22/122 (с 01.01.2026).
        Vat122 = "vat122",
        /// НДС 20 %. Действовал до 31.12.2025, в актуальной спецификации
        /// Т-Банка его нет. Оставлен для чеков возврата по продажам 2025 года.
        Vat20 = "vat20",
        /// НДС по расчётной ставке 20/120. См. [`Tax::Vat20`].
        Vat120 = "vat120",
    }
}

string_enum! {
    /// Признак способа расчёта (тег 1214).
    pub enum PaymentMethod {
        /// Предоплата 100 %.
        FullPrepayment = "full_prepayment",
        /// Частичная предоплата.
        Prepayment = "prepayment",
        /// Аванс.
        Advance = "advance",
        /// Полный расчёт.
        FullPayment = "full_payment",
        /// Частичный расчёт и кредит.
        PartialPayment = "partial_payment",
        /// Передача в кредит.
        Credit = "credit",
        /// Оплата кредита.
        CreditPayment = "credit_payment",
    }
}

string_enum! {
    /// Признак предмета расчёта (тег 1212).
    pub enum PaymentObject {
        /// Товар.
        Commodity = "commodity",
        /// Подакцизный товар.
        Excise = "excise",
        /// Работа.
        Job = "job",
        /// Услуга.
        Service = "service",
        /// Ставка азартной игры.
        GamblingBet = "gambling_bet",
        /// Выигрыш азартной игры.
        GamblingPrize = "gambling_prize",
        /// Лотерейный билет.
        Lottery = "lottery",
        /// Выигрыш лотереи.
        LotteryPrize = "lottery_prize",
        /// Результаты интеллектуальной деятельности.
        IntellectualActivity = "intellectual_activity",
        /// Платёж.
        Payment = "payment",
        /// Агентское вознаграждение.
        AgentCommission = "agent_commission",
        /// Составной предмет расчёта (ФФД 1.05).
        Composite = "composite",
        /// Выплата (ФФД 1.2).
        Contribution = "contribution",
        /// Имущественное право (ФФД 1.2).
        PropertyRights = "property_rights",
        /// Внереализационный доход (ФФД 1.2).
        Unrealization = "unrealization",
        /// Суммы расходов, уменьшающих налог (ФФД 1.2).
        TaxReduction = "tax_reduction",
        /// Торговый сбор (ФФД 1.2).
        TradeFee = "trade_fee",
        /// Курортный сбор (ФФД 1.2).
        ResortTax = "resort_tax",
        /// Залог (ФФД 1.2).
        Pledge = "pledge",
        /// Расход, уменьшающий доход (ФФД 1.2).
        IncomeDecrease = "income_decrease",
        /// Взносы на ОПС ИП без выплат (ФФД 1.2).
        IePensionInsuranceWithoutPayments = "ie_pension_insurance_without_payments",
        /// Взносы на ОПС ИП с выплатами (ФФД 1.2).
        IePensionInsuranceWithPayments = "ie_pension_insurance_with_payments",
        /// Взносы на ОМС ИП без выплат (ФФД 1.2).
        IeMedicalInsuranceWithoutPayments = "ie_medical_insurance_without_payments",
        /// Взносы на ОМС ИП с выплатами (ФФД 1.2).
        IeMedicalInsuranceWithPayments = "ie_medical_insurance_with_payments",
        /// Взносы на социальное страхование (ФФД 1.2).
        SocialInsurance = "social_insurance",
        /// Платёж казино (ФФД 1.2).
        CasinoChips = "casino_chips",
        /// Выдача денежных средств банковским платёжным агентом (ФФД 1.2).
        AgentPayment = "agent_payment",
        /// Подакцизный товар без кода маркировки (ФФД 1.2).
        ExcisableGoodsWithoutMarkingCode = "excisable_goods_without_marking_code",
        /// Подакцизный товар с кодом маркировки (ФФД 1.2).
        ExcisableGoodsWithMarkingCode = "excisable_goods_with_marking_code",
        /// Товар без кода маркировки (ФФД 1.2).
        GoodsWithoutMarkingCode = "goods_without_marking_code",
        /// Товар с кодом маркировки (ФФД 1.2).
        GoodsWithMarkingCode = "goods_with_marking_code",
        /// Иной предмет расчёта.
        Another = "another",
    }
}

string_enum! {
    /// Признак агента (тег 1222).
    pub enum AgentSign {
        /// Банковский платёжный агент.
        BankPayingAgent = "bank_paying_agent",
        /// Банковский платёжный субагент.
        BankPayingSubagent = "bank_paying_subagent",
        /// Платёжный агент.
        PayingAgent = "paying_agent",
        /// Платёжный субагент.
        PayingSubagent = "paying_subagent",
        /// Поверенный.
        Attorney = "attorney",
        /// Комиссионер.
        CommissionAgent = "commission_agent",
        /// Другой тип агента.
        Another = "another",
    }
}

string_enum! {
    /// Тип кода маркировки.
    pub enum MarkCodeType {
        /// Формат не идентифицирован.
        Unknown = "UNKNOWN",
        /// EAN-8.
        Ean8 = "EAN8",
        /// EAN-13.
        Ean13 = "EAN13",
        /// ITF-14.
        Itf14 = "ITF14",
        /// GS1 на немаркируемом товаре.
        Gs10 = "GS10",
        /// GS1 на маркируемом товаре.
        Gs1m = "GS1M",
        /// Короткий код маркировки.
        Short = "SHORT",
        /// Контрольно-идентификационный знак мехового изделия.
        Fur = "FUR",
        /// ЕГАИС-2.0.
        Egais20 = "EGAIS20",
        /// ЕГАИС-3.0.
        Egais30 = "EGAIS30",
        /// Код маркировки в том виде, как его прочитал сканер.
        RawCode = "RAWCODE",
    }
}

/// Стоимость позиции в копейках: `round(price × quantity)`.
/// Количество передаётся с точностью до 6 знаков, поэтому считаем в
/// миллионных долях: `100 × 0.285` даёт 29, а не 28 (как в `f64`).
fn line_amount(price: Amount, quantity: f64) -> Amount {
    let micro = (quantity * 1_000_000.0).round() as i128;
    ((price as i128 * micro + 500_000) / 1_000_000).max(0) as Amount
}

/// Чек. Версия ФФД задаётся настройками онлайн-кассы терминала.
///
/// При разборе JSON версия выбирается по полям, которые есть только в
/// ФФД 1.2 (`MeasurementUnit`, `MarkCode`, `ClientInfo`, …), и ошибка разбора
/// не маскируется откатом на ФФД 1.05.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
// Чек создаётся один раз на запрос, разница в размере вариантов не важна.
#[allow(clippy::large_enum_variant)]
pub enum Receipt {
    /// ФФД 1.2.
    Ffd12(ReceiptFfd12),
    /// ФФД 1.05.
    Ffd105(ReceiptFfd105),
}

impl Receipt {
    /// Сумма `Amount` всех позиций в копейках.
    pub fn total(&self) -> Amount {
        match self {
            Receipt::Ffd12(r) => r.items.iter().map(|i| i.amount).sum(),
            Receipt::Ffd105(r) => r.items.iter().map(|i| i.amount).sum(),
        }
    }

    /// Проверки, которые Т-Банк всё равно сделает на своей стороне,
    /// но с понятным сообщением об ошибке.
    pub fn validate(&self) -> Result<(), String> {
        let (count, email, phone) = match self {
            Receipt::Ffd12(r) => (r.items.len(), &r.email, &r.phone),
            Receipt::Ffd105(r) => (r.items.len(), &r.email, &r.phone),
        };
        if count == 0 {
            return Err("в чеке нет позиций".into());
        }
        if count > 100 {
            return Err(format!("в чеке {count} позиций, максимум 100"));
        }
        if email.is_none() && phone.is_none() {
            return Err("в чеке нужен Email или Phone покупателя".into());
        }
        let lines: Vec<(&str, Amount, f64, Amount)> = match self {
            Receipt::Ffd12(r) => r
                .items
                .iter()
                .map(|i| (i.name.as_str(), i.price, i.quantity, i.amount))
                .collect(),
            Receipt::Ffd105(r) => r
                .items
                .iter()
                .map(|i| (i.name.as_str(), i.price, i.quantity, i.amount))
                .collect(),
        };
        for (name, price, quantity, amount) in lines {
            if quantity <= 0.0 {
                return Err(format!(
                    "позиция «{name}»: количество должно быть больше нуля"
                ));
            }
            // Меньше Price × Quantity можно (скидка), больше — нет.
            if amount > line_amount(price, quantity) {
                return Err(format!(
                    "позиция «{name}»: Amount {amount} больше Price {price} × Quantity {quantity}"
                ));
            }
        }
        Ok(())
    }

    /// Разбивка оплаты по видам, если задана.
    pub fn payments(&self) -> Option<&Payments> {
        match self {
            Receipt::Ffd12(r) => r.payments.as_ref(),
            Receipt::Ffd105(r) => r.payments.as_ref(),
        }
    }

    /// Проверка чека против суммы платежа. Без `Payments` сумма позиций
    /// должна равняться `amount`; с `Payments` — `Electronic` равен `amount`,
    /// а сумма всех видов оплаты равна сумме позиций.
    pub fn validate_for_amount(&self, amount: Amount) -> Result<(), String> {
        self.validate()?;
        match self.payments() {
            None if self.total() != amount => Err(format!(
                "сумма позиций чека {} ≠ Amount {amount}",
                self.total()
            )),
            None => Ok(()),
            Some(p) if p.electronic != amount => Err(format!(
                "Payments.Electronic {} ≠ Amount {amount}",
                p.electronic
            )),
            Some(p) if p.total() != self.total() => Err(format!(
                "сумма Payments {} ≠ сумме позиций чека {}",
                p.total(),
                self.total()
            )),
            Some(_) => Ok(()),
        }
    }
}

const FFD12_RECEIPT_KEYS: &[&str] = &[
    "ClientInfo",
    "Customer",
    "CustomerInn",
    "OperatingCheckProps",
    "SectoralCheckProps",
    "AdditionalCheckProps",
];
const FFD12_ITEM_KEYS: &[&str] = &[
    "MeasurementUnit",
    "MarkCode",
    "MarkQuantity",
    "MarkProcessingMode",
    "SectoralItemProps",
    "UserData",
    "Excise",
    "CountryCode",
    "DeclarationNumber",
];

impl<'de> Deserialize<'de> for Receipt {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let v = serde_json::Value::deserialize(d)?;
        let has =
            |obj: &serde_json::Value, keys: &[&str]| keys.iter().any(|k| obj.get(*k).is_some());
        let is_ffd12 = has(&v, FFD12_RECEIPT_KEYS)
            || v.get("Items")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| items.iter().any(|i| has(i, FFD12_ITEM_KEYS)));
        if is_ffd12 {
            serde_json::from_value(v)
                .map(Receipt::Ffd12)
                .map_err(D::Error::custom)
        } else {
            serde_json::from_value(v)
                .map(Receipt::Ffd105)
                .map_err(D::Error::custom)
        }
    }
}

impl From<ReceiptFfd12> for Receipt {
    fn from(r: ReceiptFfd12) -> Self {
        Receipt::Ffd12(r)
    }
}

impl From<ReceiptFfd105> for Receipt {
    fn from(r: ReceiptFfd105) -> Self {
        Receipt::Ffd105(r)
    }
}

/// Чек ФФД 1.2.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct ReceiptFfd12 {
    /// Позиции чека (не больше 100).
    pub items: Vec<ItemFfd12>,
    /// Система налогообложения.
    pub taxation: Taxation,
    /// Email покупателя (нужен Email или Phone).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Телефон покупателя в формате `+{Ц}` (нужен Email или Phone).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Данные покупателя (теги 1243–1246, 1254).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,
    /// Покупатель — организация или ИП (тег 1227).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    /// ИНН покупателя (тег 1228).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_inn: Option<String>,
    /// Разбивка оплаты по видам. Без неё вся сумма идёт как безналичная.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payments: Option<Payments>,
    /// Операционный реквизит чека (тег 1270).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_check_props: Option<OperatingCheckProps>,
    /// Отраслевые реквизиты чека (тег 1261).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::de::opt_vec_or_one"
    )]
    pub sectoral_check_props: Option<Vec<SectoralProps>>,
    /// Дополнительный реквизит пользователя (тег 1084).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_user_prop: Option<AddUserProp>,
    /// Дополнительный реквизит чека, БСО (тег 1192).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_check_props: Option<String>,
}

impl ReceiptFfd12 {
    /// Новый чек ФФД 1.2.
    pub fn new(taxation: Taxation, items: Vec<ItemFfd12>) -> Self {
        Self {
            items,
            taxation,
            email: None,
            phone: None,
            client_info: None,
            customer: None,
            customer_inn: None,
            payments: None,
            operating_check_props: None,
            sectoral_check_props: None,
            add_user_prop: None,
            additional_check_props: None,
        }
    }
}

setters!(ReceiptFfd12 {
    /// Email покупателя.
    email: String,
    /// Телефон покупателя (`+79991234567`).
    phone: String,
    /// Данные покупателя.
    client_info: ClientInfo,
    /// Покупатель — организация или ИП.
    customer: String,
    /// ИНН покупателя.
    customer_inn: String,
    /// Разбивка оплаты по видам.
    payments: Payments,
    /// Операционный реквизит чека.
    operating_check_props: OperatingCheckProps,
    /// Отраслевые реквизиты чека.
    sectoral_check_props: Vec<SectoralProps>,
    /// Дополнительный реквизит пользователя.
    add_user_prop: AddUserProp,
    /// Дополнительный реквизит чека (БСО).
    additional_check_props: String,
});

/// Позиция чека ФФД 1.2.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct ItemFfd12 {
    /// Наименование (тег 1030), до 128 символов.
    pub name: String,
    /// Цена за единицу в копейках (тег 1079).
    pub price: Amount,
    /// Количество или вес (тег 1023).
    pub quantity: f64,
    /// Стоимость позиции в копейках (тег 1043) = Price × Quantity.
    pub amount: Amount,
    /// Ставка НДС (тег 1199).
    pub tax: Tax,
    /// Способ расчёта (тег 1214).
    pub payment_method: PaymentMethod,
    /// Предмет расчёта (тег 1212).
    pub payment_object: PaymentObject,
    /// Единица измерения (тег 2108): `шт`, `кг`, `л`, `м`, `ч`, …
    pub measurement_unit: String,
    /// Дополнительный реквизит предмета расчёта (тег 1191).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<String>,
    /// Сумма акциза в рублях с копейками (тег 1229).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::de::opt_f64"
    )]
    pub excise: Option<f64>,
    /// Код страны происхождения, 3 цифры (тег 1230).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    /// Номер таможенной декларации (тег 1231).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declaration_number: Option<String>,
    /// Режим обработки кода маркировки (тег 2102), всегда `"0"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark_processing_mode: Option<String>,
    /// Код маркировки.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark_code: Option<MarkCode>,
    /// Дробное количество маркированного товара (теги 1293, 1294).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark_quantity: Option<MarkQuantity>,
    /// Отраслевые реквизиты предмета расчёта.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::de::opt_vec_or_one"
    )]
    pub sectoral_item_props: Option<Vec<SectoralProps>>,
    /// Данные агента.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_data: Option<AgentData>,
    /// Данные поставщика (обязательны вместе с `AgentData`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_info: Option<SupplierInfo>,
}

impl ItemFfd12 {
    /// Позиция с полным расчётом за товар в штуках. `Amount` считается как
    /// `round(price × quantity)`; способ/предмет расчёта и единицу измерения
    /// можно поменять сеттерами.
    pub fn new(name: impl Into<String>, price: Amount, quantity: f64, tax: Tax) -> Self {
        Self {
            name: name.into(),
            price,
            quantity,
            amount: line_amount(price, quantity),
            tax,
            payment_method: PaymentMethod::FullPayment,
            payment_object: PaymentObject::Commodity,
            measurement_unit: "шт".into(),
            user_data: None,
            excise: None,
            country_code: None,
            declaration_number: None,
            mark_processing_mode: None,
            mark_code: None,
            mark_quantity: None,
            sectoral_item_props: None,
            agent_data: None,
            supplier_info: None,
        }
    }

    /// Способ расчёта.
    #[must_use]
    pub fn payment_method(mut self, v: PaymentMethod) -> Self {
        self.payment_method = v;
        self
    }

    /// Предмет расчёта.
    #[must_use]
    pub fn payment_object(mut self, v: PaymentObject) -> Self {
        self.payment_object = v;
        self
    }

    /// Единица измерения.
    #[must_use]
    pub fn measurement_unit(mut self, v: impl Into<String>) -> Self {
        self.measurement_unit = v.into();
        self
    }

    /// Задать `Amount` явно, например при скидке на позицию. Больше
    /// `Price × Quantity` быть не может.
    #[must_use]
    pub fn amount(mut self, v: Amount) -> Self {
        self.amount = v;
        self
    }

    /// Код маркировки; заодно ставит режим обработки `"0"`.
    #[must_use]
    pub fn mark_code(mut self, kind: MarkCodeType, value: impl Into<String>) -> Self {
        self.mark_code = Some(MarkCode {
            mark_code_type: kind,
            value: value.into(),
        });
        self.mark_processing_mode.get_or_insert_with(|| "0".into());
        self
    }
}

setters!(ItemFfd12 {
    /// Дополнительный реквизит предмета расчёта.
    user_data: String,
    /// Сумма акциза в рублях.
    excise: f64,
    /// Код страны происхождения.
    country_code: String,
    /// Номер таможенной декларации.
    declaration_number: String,
    /// Режим обработки кода маркировки.
    mark_processing_mode: String,
    /// Дробное количество маркированного товара.
    mark_quantity: MarkQuantity,
    /// Отраслевые реквизиты предмета расчёта.
    sectoral_item_props: Vec<SectoralProps>,
    /// Данные агента.
    agent_data: AgentData,
    /// Данные поставщика.
    supplier_info: SupplierInfo,
});

/// Чек ФФД 1.05.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct ReceiptFfd105 {
    /// Позиции чека (не больше 100).
    pub items: Vec<ItemFfd105>,
    /// Система налогообложения.
    pub taxation: Taxation,
    /// Email покупателя (нужен Email или Phone).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Телефон покупателя (нужен Email или Phone).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Разбивка оплаты по видам.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payments: Option<Payments>,
    /// Дополнительный реквизит пользователя.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_user_prop: Option<AddUserProp>,
    /// Дополнительный реквизит чека (БСО). В ФФД 1.05 имя поля с маленькой буквы.
    #[serde(
        rename = "additionalCheckProps",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_check_props: Option<String>,
}

impl ReceiptFfd105 {
    /// Новый чек ФФД 1.05.
    pub fn new(taxation: Taxation, items: Vec<ItemFfd105>) -> Self {
        Self {
            items,
            taxation,
            email: None,
            phone: None,
            payments: None,
            add_user_prop: None,
            additional_check_props: None,
        }
    }
}

setters!(ReceiptFfd105 {
    /// Email покупателя.
    email: String,
    /// Телефон покупателя.
    phone: String,
    /// Разбивка оплаты по видам.
    payments: Payments,
    /// Дополнительный реквизит пользователя.
    add_user_prop: AddUserProp,
    /// Дополнительный реквизит чека (БСО).
    additional_check_props: String,
});

/// Позиция чека ФФД 1.05.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct ItemFfd105 {
    /// Наименование (тег 1030).
    pub name: String,
    /// Цена в копейках (тег 1078).
    pub price: Amount,
    /// Количество или вес (тег 1023).
    pub quantity: f64,
    /// Стоимость позиции в копейках (тег 1043).
    pub amount: Amount,
    /// Ставка НДС (тег 1199).
    pub tax: Tax,
    /// Способ расчёта (по умолчанию у банка — `full_payment`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<PaymentMethod>,
    /// Предмет расчёта (по умолчанию у банка — `commodity`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_object: Option<PaymentObject>,
    /// Штрихкод (тег 1162).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ean13: Option<String>,
    /// Код магазина (маркетплейсы).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shop_code: Option<String>,
    /// Данные агента.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_data: Option<AgentData>,
    /// Данные поставщика.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_info: Option<SupplierInfo>,
}

impl ItemFfd105 {
    /// Позиция; `Amount` = `round(price × quantity)`.
    pub fn new(name: impl Into<String>, price: Amount, quantity: f64, tax: Tax) -> Self {
        Self {
            name: name.into(),
            price,
            quantity,
            amount: line_amount(price, quantity),
            tax,
            payment_method: None,
            payment_object: None,
            ean13: None,
            shop_code: None,
            agent_data: None,
            supplier_info: None,
        }
    }

    /// Задать `Amount` явно.
    #[must_use]
    pub fn amount(mut self, v: Amount) -> Self {
        self.amount = v;
        self
    }
}

setters!(ItemFfd105 {
    /// Способ расчёта.
    payment_method: PaymentMethod,
    /// Предмет расчёта.
    payment_object: PaymentObject,
    /// Штрихкод.
    ean13: String,
    /// Код магазина.
    shop_code: String,
    /// Данные агента.
    agent_data: AgentData,
    /// Данные поставщика.
    supplier_info: SupplierInfo,
});

/// Данные покупателя в чеке ФФД 1.2.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct ClientInfo {
    /// Дата рождения `ДД.ММ.ГГГГ` (тег 1243).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<String>,
    /// Код страны гражданства по ОКСМ (тег 1244).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citizenship: Option<String>,
    /// Код вида документа: `21` — паспорт РФ и т. д. (тег 1245).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_code: Option<String>,
    /// Реквизиты документа (тег 1246).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_data: Option<String>,
    /// Адрес покупателя (тег 1254).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

setters!(ClientInfo {
    /// Дата рождения `ДД.ММ.ГГГГ`.
    birthdate: String,
    /// Код страны гражданства.
    citizenship: String,
    /// Код вида документа.
    document_code: String,
    /// Реквизиты документа.
    document_data: String,
    /// Адрес покупателя.
    address: String,
});

/// Разбивка оплаты по видам (суммы в копейках).
///
/// `Electronic` должно совпадать с `Amount` платежа, а сумма всех видов —
/// с суммой позиций чека.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct Payments {
    /// Безналичными (тег 1081).
    pub electronic: Amount,
    /// Наличными (тег 1031).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash: Option<Amount>,
    /// Предоплатой (аванс) (тег 1215).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance_payment: Option<Amount>,
    /// Постоплатой (кредит) (тег 1216).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit: Option<Amount>,
    /// Иной формой оплаты (тег 1217).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provision: Option<Amount>,
}

impl Payments {
    /// Сумма всех видов оплаты.
    pub fn total(&self) -> Amount {
        self.electronic
            + self.cash.unwrap_or(0)
            + self.advance_payment.unwrap_or(0)
            + self.credit.unwrap_or(0)
            + self.provision.unwrap_or(0)
    }

    /// Вся сумма безналичными.
    pub fn electronic(amount: Amount) -> Self {
        Self {
            electronic: amount,
            ..Self::default()
        }
    }
}

setters!(Payments {
    /// Наличными.
    cash: exact Amount,
    /// Предоплатой (аванс).
    advance_payment: exact Amount,
    /// Постоплатой (кредит).
    credit: exact Amount,
    /// Иной формой оплаты.
    provision: exact Amount,
});

/// Данные агента (теги 1222, 1044, 1073–1075, 1026, 1005, 1016).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AgentData {
    /// Признак агента.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_sign: Option<AgentSign>,
    /// Наименование операции.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    /// Телефоны платёжного агента.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phones: Option<Vec<String>>,
    /// Телефоны оператора по приёму платежей.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver_phones: Option<Vec<String>>,
    /// Телефоны оператора перевода.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_phones: Option<Vec<String>>,
    /// Наименование оператора перевода.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_name: Option<String>,
    /// Адрес оператора перевода.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_address: Option<String>,
    /// ИНН оператора перевода.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_inn: Option<String>,
}

setters!(AgentData {
    /// Признак агента.
    agent_sign: AgentSign,
    /// Наименование операции.
    operation_name: String,
    /// Телефоны платёжного агента.
    phones: Vec<String>,
    /// Телефоны оператора по приёму платежей.
    receiver_phones: Vec<String>,
    /// Телефоны оператора перевода.
    transfer_phones: Vec<String>,
    /// Наименование оператора перевода.
    operator_name: String,
    /// Адрес оператора перевода.
    operator_address: String,
    /// ИНН оператора перевода.
    operator_inn: String,
});

/// Данные поставщика (теги 1171, 1225, 1226).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct SupplierInfo {
    /// Телефоны поставщика.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phones: Option<Vec<String>>,
    /// Наименование поставщика.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// ИНН поставщика.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inn: Option<String>,
}

setters!(SupplierInfo {
    /// Телефоны поставщика.
    phones: Vec<String>,
    /// Наименование поставщика.
    name: String,
    /// ИНН поставщика.
    inn: String,
});

/// Код маркировки.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct MarkCode {
    /// Тип кода.
    pub mark_code_type: MarkCodeType,
    /// Значение кода.
    pub value: String,
}

impl MarkCode {
    /// Новый код маркировки.
    pub fn new(kind: MarkCodeType, value: impl Into<String>) -> Self {
        Self {
            mark_code_type: kind,
            value: value.into(),
        }
    }
}

/// Дробное количество маркированного товара.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct MarkQuantity {
    /// Числитель (тег 1293).
    pub numerator: u64,
    /// Знаменатель (тег 1294).
    pub denominator: u64,
}

impl MarkQuantity {
    /// `numerator / denominator` упаковки.
    pub fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

/// Отраслевой реквизит чека или предмета расчёта (теги 1261–1265).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct SectoralProps {
    /// Идентификатор ФОИВ, `001`–`072`.
    pub federal_id: String,
    /// Дата нормативного акта `ДД.ММ.ГГГГ`.
    pub date: String,
    /// Номер нормативного акта.
    pub number: String,
    /// Значение реквизита.
    pub value: String,
}

impl SectoralProps {
    /// Новый отраслевой реквизит.
    pub fn new(
        federal_id: impl Into<String>,
        date: impl Into<String>,
        number: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            federal_id: federal_id.into(),
            date: date.into(),
            number: number.into(),
            value: value.into(),
        }
    }
}

/// Операционный реквизит чека (тег 1270).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct OperatingCheckProps {
    /// Идентификатор операции.
    pub name: String,
    /// Данные операции.
    pub value: String,
    /// Дата и время `dd.mm.yyyy HH:MM:SS`.
    pub timestamp: String,
}

impl OperatingCheckProps {
    /// Новый операционный реквизит.
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            timestamp: timestamp.into(),
        }
    }
}

/// Дополнительный реквизит пользователя (теги 1085, 1086).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[non_exhaustive]
pub struct AddUserProp {
    /// Наименование.
    pub name: String,
    /// Значение.
    pub value: String,
}

impl AddUserProp {
    /// Новый реквизит.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}
