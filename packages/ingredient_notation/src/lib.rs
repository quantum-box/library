//! COM-860 / COM-861: shared validation rules for the ingredient catalog.
//!
//! These are pure functions shared by library-api, which validates drafts
//! when a release is published, and by the `library food import` command,
//! which turns the official food composition tables into drafts. Keeping
//! one copy means an import can never write something publishing would
//! read differently.

mod decimal;
mod draft_fields;
mod keys;
mod value_status;

pub use decimal::*;
pub use draft_fields::*;
pub use keys::*;
pub use value_status::*;
