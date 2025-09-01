#![forbid(unsafe_code)]

mod client;
mod token;
mod types;
mod webhook;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use crate::client::{TinkoffClient, TinkoffClientBuilder};
pub use crate::token::{make_token, top_fields_from_value};
pub use crate::types::*;
pub use crate::webhook::{verify_webhook_token, WebhookVerifyError};
