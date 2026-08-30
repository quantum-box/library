//! Translation of user-entered data.
//!
//! Library stores a record in the language its author wrote it in, and
//! that value stays the single source of truth. Everything in this
//! module supports a derived, read-only translation cache layered over
//! it: the original write path is untouched, and the whole cache can be
//! discarded and rebuilt from the source at any time.
//!
//! A cached row is keyed by the hash of the source text, which is what
//! makes staleness free to detect -- no hooks on the editing path, no
//! change feed, just a hash that either matches or does not.

mod detect;
mod language_tag;
mod negotiation;
mod repository;
mod scope;
mod translatable;
mod translator;

pub use detect::*;
pub use language_tag::*;
pub use negotiation::*;
pub use repository::*;
pub use scope::*;
pub use translatable::*;
pub use translator::*;

use sha2::{Digest, Sha256};

/// Hashes source text into the value stored in `translations.source_hash`.
///
/// This is the cache key for the entire feature: a translation is
/// current exactly when the hash of the text it was made from still
/// matches the hash of the text as it stands now.
pub fn source_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashing_is_stable_and_fits_the_storage_column() {
        let hash = source_hash("これは本文です");
        assert_eq!(hash.len(), 64, "the column is CHAR(64)");
        assert_eq!(hash, source_hash("これは本文です"));
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn any_edit_changes_the_hash() {
        let before = source_hash("これは本文です");
        let after = source_hash("これは本文です。");
        assert_ne!(before, after);
    }

    #[test]
    fn whitespace_is_significant() {
        // Normalizing here would let an edit slip past the staleness
        // check, so the raw text is hashed as-is.
        assert_ne!(source_hash("a b"), source_hash("a  b"));
    }
}
