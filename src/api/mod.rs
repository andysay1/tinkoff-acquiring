//! Типы запросов и ответов, по разделам API.

pub mod customers;
pub mod payments;
pub mod sbp;
pub mod three_ds;
pub mod wallets;

pub use customers::*;
pub use payments::*;
pub use sbp::*;
pub use three_ds::*;
pub use wallets::*;
