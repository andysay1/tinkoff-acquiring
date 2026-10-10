//! Перечисления API. У каждого есть вариант `Other(String)` для значений,
//! которые Т-Банк добавит позже.

string_enum! {
    /// Статус платежа ([справочник статусов](https://developer.tbank.ru/eacq/intro/developer/operation-statuses)).
    pub enum PaymentStatus {
        /// Платёж создан методом `Init`.
        New = "NEW",
        /// Платёжная форма открыта покупателем.
        FormShowed = "FORM_SHOWED" | "FORM SHOWED",
        /// Покупатель превысил число попыток открытия формы.
        AttemptsExpired = "ATTEMPTS_EXPIRED",
        /// Проверка платёжных данных покупателя.
        Preauthorizing = "PREAUTHORIZING",
        /// Платёж обрабатывается банком и платёжной системой.
        Authorizing = "AUTHORIZING",
        /// Идёт проверка 3DS.
        ThreeDsChecking = "3DS_CHECKING",
        /// Проверка 3DS пройдена.
        ThreeDsChecked = "3DS_CHECKED",
        /// Платёж завершился ошибкой или не прошёл 3DS.
        AuthFail = "AUTH_FAIL",
        /// Деньги заблокированы на счёте покупателя (двухстадийная оплата).
        Authorized = "AUTHORIZED",
        /// Статус авторизации неизвестен (таймаут/сетевая ошибка), идёт проверка.
        PayChecking = "PAY_CHECKING",
        /// Подтверждение обрабатывается.
        Confirming = "CONFIRMING",
        /// Статус подтверждения неизвестен (таймаут/сетевая ошибка), идёт проверка.
        ConfirmChecking = "CONFIRM_CHECKING",
        /// Деньги списаны.
        Confirmed = "CONFIRMED",
        /// Идёт отмена авторизованного платежа.
        Reversing = "REVERSING",
        /// Частичная отмена авторизованного платежа.
        PartialReversed = "PARTIAL_REVERSED",
        /// Полная отмена авторизованного платежа.
        Reversed = "REVERSED",
        /// Идёт возврат подтверждённого платежа.
        Refunding = "REFUNDING",
        /// Идёт возврат по QR (СБП).
        AsyncRefunding = "ASYNC_REFUNDING",
        /// Частичный возврат подтверждённого платежа.
        PartialRefunded = "PARTIAL_REFUNDED",
        /// Полный возврат подтверждённого платежа.
        Refunded = "REFUNDED",
        /// Платёж отклонён.
        Rejected = "REJECTED",
        /// Мерчант отменил платёж.
        Canceled = "CANCELED",
        /// Истёк срок жизни ссылки (`RedirectDueDate`) или срок прохождения 3DS.
        DeadlineExpired = "DEADLINE_EXPIRED",
        /// Обработка (СБП, цифровой рубль).
        Processing = "PROCESSING",
        /// Проверка.
        Checking = "CHECKING",
        /// Проверено.
        Checked = "CHECKED",
        /// Завершается.
        Completing = "COMPLETING",
        /// Завершён.
        Completed = "COMPLETED",
        /// Статус неизвестен.
        Unknown = "UNKNOWN",
    }
}

string_enum! {
    /// Статус привязки карты (`AddCard`, `AttachCard`, `GetAddCardState`).
    pub enum AddCardStatus {
        /// Новая сессия привязки.
        New = "NEW",
        /// Показана форма привязки.
        FormShowed = "FORM_SHOWED",
        /// Идёт проверка 3DS.
        ThreeDsChecking = "3DS_CHECKING",
        /// Проверка 3DS пройдена.
        ThreeDsChecked = "3DS_CHECKED",
        /// Привязка обрабатывается.
        Authorizing = "AUTHORIZING",
        /// Проверочное списание заблокировано.
        Authorized = "AUTHORIZED",
        /// Карта привязана.
        Completed = "COMPLETED",
        /// Привязка отклонена.
        Rejected = "REJECTED",
    }
}

string_enum! {
    /// Статус привязки счёта СБП для автоплатежей.
    pub enum AccountQrStatus {
        /// Получен запрос на привязку.
        New = "NEW",
        /// QR сформирован и отправлен мерчанту.
        Processing = "PROCESSING",
        /// Счёт привязан.
        Active = "ACTIVE",
        /// Привязка отклонена.
        Inactive = "INACTIVE",
    }
}

string_enum! {
    /// Статус сохранённой карты.
    pub enum CardStatus {
        /// Активная.
        Active = "A",
        /// Удалённая.
        Deleted = "D",
    }
}

string_enum! {
    /// Тип проведения платежа.
    pub enum PayType {
        /// Одностадийная оплата.
        OneStage = "O",
        /// Двухстадийная оплата (холдирование + `Confirm`).
        TwoStage = "T",
    }
}

string_enum! {
    /// Язык платёжной формы.
    pub enum Language {
        /// Русский.
        Ru = "ru",
        /// Английский.
        En = "en",
    }
}

string_enum! {
    /// Признак инициатора операции (`DATA.OperationInitiatorType`).
    ///
    /// Обязателен при создании родительского платежа с сохранением карты.
    pub enum OperationInitiatorType {
        /// Обычный платёж.
        Regular = "0",
        /// CIT CC — покупатель платит и сохраняет реквизиты.
        CitCredentialCapture = "1",
        /// CIT COF — покупатель платит сохранёнными реквизитами.
        CitCredentialOnFile = "2",
        /// MIT COF Recurring — регулярное списание (подписка).
        MitRecurring = "R",
        /// MIT COF Installment — рассрочка.
        MitInstallment = "I",
        /// MIT COF Delayed-Charge — отложенное списание.
        MitDelayedCharge = "D",
        /// MIT COF No-Show — штраф за неявку.
        MitNoShow = "N",
    }
}

string_enum! {
    /// Проверка карты при привязке (`AddCard`).
    pub enum CheckType {
        /// Сохранить без проверки.
        No = "NO",
        /// Проверочное блокирование суммы.
        Hold = "HOLD",
        /// Проверка 3DS.
        ThreeDs = "3DS",
        /// 3DS + блокирование.
        ThreeDsHold = "3DSHOLD",
    }
}

string_enum! {
    /// Формат данных QR в ответе.
    pub enum QrDataType {
        /// Только payload (ссылка для QR).
        Payload = "PAYLOAD",
        /// SVG-изображение QR.
        Image = "IMAGE",
    }
}

string_enum! {
    /// Способ оплаты по QR.
    pub enum QrPaymentMethod {
        /// Система быстрых платежей.
        Sbp = "SBP",
        /// Цифровой рубль.
        DigitalRuble = "DR",
    }
}

string_enum! {
    /// Тип устройства покупателя.
    pub enum DeviceType {
        /// Десктоп.
        Desktop = "desktop",
        /// Мобильное устройство.
        Mobile = "mobile",
    }
}

string_enum! {
    /// Сценарий для списка банков СБП.
    pub enum QrScenario {
        /// Оплата.
        Payment = "qr",
        /// Привязка счёта.
        Subscription = "sub",
    }
}

string_enum! {
    /// Способ платежа в `Confirm`/`Cancel` (рассрочка, «Долями»).
    pub enum Route {
        /// Рассрочка.
        Tcb = "TCB",
        /// «Долями».
        Bnpl = "BNPL",
    }
}

string_enum! {
    /// Источник платежа в `Confirm`/`Cancel` (рассрочка, «Долями»).
    pub enum Source {
        /// Рассрочка.
        Installment = "installment",
        /// «Долями».
        Bnpl = "BNPL",
    }
}

string_enum! {
    /// Канал устройства для 3DS.
    pub enum DeviceChannel {
        /// Мобильное приложение (APP).
        App = "01",
        /// Браузер (BRW).
        Browser = "02",
    }
}

string_enum! {
    /// Тип уведомления, если это не уведомление о платеже.
    pub enum NotificationType {
        /// Привязка карты.
        LinkCard = "LINKCARD",
        /// Привязка счёта СБП.
        LinkAccount = "LINKACCOUNT",
    }
}

/// Тип сохранённой карты.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CardType {
    /// Карта списания.
    Charge,
    /// Карта пополнения.
    TopUp,
    /// Карта пополнения и списания.
    ChargeAndTopUp,
    /// Значение, которого нет в этой версии библиотеки.
    Other(u64),
}

impl CardType {
    fn code(self) -> u64 {
        match self {
            CardType::Charge => 0,
            CardType::TopUp => 1,
            CardType::ChargeAndTopUp => 2,
            CardType::Other(n) => n,
        }
    }
}

impl serde::Serialize for CardType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(self.code())
    }
}

impl<'de> serde::Deserialize<'de> for CardType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match crate::de::u64_lenient(d)? {
            0 => CardType::Charge,
            1 => CardType::TopUp,
            2 => CardType::ChargeAndTopUp,
            n => CardType::Other(n),
        })
    }
}
