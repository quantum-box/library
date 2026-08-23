//! Rich text documents.
//!
//! The stored value is the editor's own block tree as JSON, which is
//! lossless. Markdown and HTML are *views* of it, produced on demand.
//!
//! The block shape happens to be BlockNote's today. That is deliberately not
//! part of the type contract: the property kernel versions the value
//! encoding separately from the type, so a different representation becomes
//! encoding v2 rather than a new property type.

use serde_json::Value;

mod from_markdown;
mod inline;
mod to_markdown;

pub use from_markdown::from_markdown;
pub use inline::plain_text;
pub use to_markdown::to_markdown;

/// The block array of a document.
///
/// Accepts either a bare array or an object wrapping one under `blocks`, so
/// that a client which adds an envelope does not break every read path.
pub(crate) fn blocks_of(document: &Value) -> &[Value] {
    if let Some(blocks) = document.as_array() {
        return blocks;
    }
    document
        .get("blocks")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
