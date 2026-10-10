/// Строковое перечисление API с запасным вариантом `Other(String)`.
///
/// Неизвестные значения не ломают разбор ответа: Т-Банк регулярно добавляет
/// новые статусы и коды, и они попадают в `Other`.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $value:literal $(| $alias:literal)*
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )*
            /// Значение, которого нет в этой версии библиотеки.
            Other(String),
        }

        impl $name {
            /// Значение в том виде, в каком его передаёт API.
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$variant => $value, )*
                    Self::Other(s) => s.as_str(),
                }
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                match s {
                    $( $value $(| $alias)* => Self::$variant, )*
                    other => Self::Other(other.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self::from(s.as_str())
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = ::std::convert::Infallible;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self::from(s))
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                crate::de::string(d).map(Self::from)
            }
        }
    };
}

/// Методы-сеттеры для необязательных полей запроса: `.field(value)`.
///
/// `field: exact T` — сеттер принимает ровно `T` (для сумм, чтобы
/// `.amount(5000)` работало без суффикса типа).
macro_rules! setters {
    ($ty:ident { $($body:tt)* }) => {
        impl $ty {
            setters!(@items $($body)*);
        }
    };
    (@items) => {};
    (@items $(#[$m:meta])* $field:ident : exact $t:ty, $($rest:tt)*) => {
        $(#[$m])*
        #[must_use]
        pub fn $field(mut self, value: $t) -> Self {
            self.$field = Some(value);
            self
        }
        setters!(@items $($rest)*);
    };
    (@items $(#[$m:meta])* $field:ident : $t:ty, $($rest:tt)*) => {
        $(#[$m])*
        #[must_use]
        pub fn $field(mut self, value: impl Into<$t>) -> Self {
            self.$field = Some(value.into());
            self
        }
        setters!(@items $($rest)*);
    };
}

/// Структура ответа API: добавляет общие поля `Success`, `ErrorCode`,
/// `Message`, `Details` и сохраняет неизвестные поля в `extra`.
macro_rules! response {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$fmeta:meta])*
                pub $field:ident : $ty:ty,
            )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, ::serde::Serialize, ::serde::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        #[non_exhaustive]
        pub struct $name {
            $( $(#[$fmeta])* pub $field: $ty, )*
            /// Успешность запроса. В `Ok(..)` всегда `true`: ответы с `false`
            /// превращаются в [`Error::Api`](crate::Error::Api).
            #[serde(default, deserialize_with = "crate::de::bool_lenient")]
            pub success: bool,
            /// Код ошибки, `"0"` при успехе.
            #[serde(default, deserialize_with = "crate::de::string")]
            pub error_code: String,
            /// Краткое описание ошибки или сервисное сообщение.
            #[serde(default, deserialize_with = "crate::de::opt_string")]
            pub message: Option<String>,
            /// Подробное описание ошибки.
            #[serde(default, deserialize_with = "crate::de::opt_string")]
            pub details: Option<String>,
            /// Поля ответа, которых нет в этой версии библиотеки.
            #[serde(flatten)]
            pub extra: ::serde_json::Map<String, ::serde_json::Value>,
        }
    };
}
