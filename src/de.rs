//! Терпимые к типам десериализаторы.
//!
//! API Т-Банка не всегда последовательно в типах: `PaymentId` приходит то
//! строкой, то числом, `Success` в уведомлениях бывает строкой `"true"`,
//! суммы — строкой. Эти функции принимают все встречающиеся варианты.

use serde::de::{self, Deserializer, Visitor};
use std::fmt;

struct StringVisitor;

impl<'de> Visitor<'de> for StringVisitor {
    type Value = Option<String>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("строку, число или bool")
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Some(v.to_owned()))
    }
    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Some(v))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Some(v.to_string()))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(v.to_string()))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Some(v.to_string()))
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Some(v.to_string()))
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(StringVisitor)
    }
}

/// Строка из строки/числа/bool; `null` → пустая строка.
pub(crate) fn string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(d.deserialize_any(StringVisitor)?.unwrap_or_default())
}

/// Как [`string`], но `null` → `None`.
pub(crate) fn opt_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    d.deserialize_any(StringVisitor)
}

struct BoolVisitor;

impl Visitor<'_> for BoolVisitor {
    type Value = Option<bool>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bool или строку \"true\"/\"false\"")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Some(v))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match v.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "y" => Ok(Some(true)),
            "false" | "0" | "n" | "" => Ok(Some(false)),
            _ => Err(E::invalid_value(de::Unexpected::Str(v), &self)),
        }
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Some(v != 0))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(v != 0))
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
}

/// bool из bool/строки/числа; `null` → `false`.
pub(crate) fn bool_lenient<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(d.deserialize_any(BoolVisitor)?.unwrap_or(false))
}

struct U64Visitor;

impl Visitor<'_> for U64Visitor {
    type Value = Option<u64>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("неотрицательное целое число или строку с ним")
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Some(v))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        u64::try_from(v)
            .map(Some)
            .map_err(|_| E::invalid_value(de::Unexpected::Signed(v), &self))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        if v >= 0.0 && v.fract() == 0.0 && v <= u64::MAX as f64 {
            Ok(Some(v as u64))
        } else {
            Err(E::invalid_value(de::Unexpected::Float(v), &self))
        }
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let t = v.trim();
        if t.is_empty() {
            return Ok(None);
        }
        t.parse::<u64>()
            .map(Some)
            .map_err(|_| E::invalid_value(de::Unexpected::Str(v), &self))
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }
}

/// Целое из числа или строки; `null`/пустая строка → `None`.
pub(crate) fn opt_u64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    d.deserialize_any(U64Visitor)
}

/// Целое из числа или строки; `null` → `0`.
pub(crate) fn u64_lenient<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    Ok(d.deserialize_any(U64Visitor)?.unwrap_or(0))
}

/// Дробное число из числа или строки (`"12.2"`); `null` → `None`.
pub(crate) fn opt_f64<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    use serde::Deserialize;
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Number(n) => Ok(n.as_f64()),
        serde_json::Value::String(s) if s.trim().is_empty() => Ok(None),
        serde_json::Value::String(s) => s
            .trim()
            .replace(',', ".")
            .parse()
            .map(Some)
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(&s), &"число")),
        other => Err(de::Error::invalid_type(
            de::Unexpected::Other(&other.to_string()),
            &"число",
        )),
    }
}

/// Массив, одиночный объект (→ массив из одного) или `null`.
pub(crate) fn opt_vec_or_one<'de, D, T>(d: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    use serde::Deserialize;
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Null => Ok(None),
        v @ serde_json::Value::Array(_) => serde_json::from_value(v)
            .map(Some)
            .map_err(de::Error::custom),
        v => serde_json::from_value(v)
            .map(|one| Some(vec![one]))
            .map_err(de::Error::custom),
    }
}

/// JSON-объект или `null` (→ пустой объект).
pub(crate) fn map_or_null<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<serde_json::Map<String, serde_json::Value>, D::Error> {
    use serde::Deserialize;
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::Null => Ok(serde_json::Map::new()),
        serde_json::Value::Object(m) => Ok(m),
        other => Err(de::Error::invalid_type(
            de::Unexpected::Other(&other.to_string()),
            &"объект",
        )),
    }
}
