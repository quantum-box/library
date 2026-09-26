//! COM-860 / COM-861: the value rules of the common ingredient catalog.
//!
//! These are pure functions shared by library-api, which validates drafts
//! when a release is published, and by the `library food import` command,
//! which turns the official food composition tables into drafts. Keeping
//! one copy means an import can never write something publishing would
//! read differently.

mod decimal;
mod keys;
mod value_status;

pub use decimal::*;
pub use keys::*;
pub use value_status::*;
