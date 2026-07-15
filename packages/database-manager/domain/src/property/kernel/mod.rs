//! Versioned, compile-time Property Type Kernel.
//!
//! The existing [`PropertyType`](crate::PropertyType) and
//! [`PropertyDataValue`](crate::PropertyDataValue) enums remain the public
//! compatibility facade. New persistence and adapter code should resolve a
//! [`PropertyTypeRef`] through [`BUILTIN_PROPERTY_TYPE_REGISTRY`] and keep the
//! type and value-encoding versions in their envelopes.

mod capabilities;
mod conversion;
mod handler;
mod legacy;
mod registry;
mod types;

pub use capabilities::*;
pub use conversion::*;
pub use handler::*;
pub use legacy::*;
pub use registry::*;
pub use types::*;

#[cfg(test)]
mod tests;
