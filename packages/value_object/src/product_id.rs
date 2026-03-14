//! ProductId generation utilities.
//!
//! This module provides deterministic ProductId generation from model IDs,
//! eliminating the need for DB-based product registration.

use sha2::{Digest, Sha256};

/// Generates a deterministic ProductId from a model ID.
///
/// This function creates a stable, reproducible ProductId based on the model
/// ID, eliminating the need for DB-based product registration.
///
/// # Algorithm
///
/// 1. Compute SHA-256 hash of the model ID
/// 2. Take first 16 bytes of hash (128 bits)
/// 3. Encode as Crockford Base32 (lowercase)
/// 4. Prepend "pd_" prefix
///
/// # Examples
///
/// ```
/// use value_object::generate_product_id_for_model;
///
/// let product_id = generate_product_id_for_model("anthropic/claude-opus-4.5");
/// assert!(product_id.starts_with("pd_"));
/// assert_eq!(product_id.len(), 29); // "pd_" (3) + 26 chars
///
/// // Same input always produces same output
/// let product_id2 = generate_product_id_for_model("anthropic/claude-opus-4.5");
/// assert_eq!(product_id, product_id2);
///
/// // Different input produces different output
/// let product_id3 = generate_product_id_for_model("openai/gpt-5");
/// assert_ne!(product_id, product_id3);
/// ```
pub fn generate_product_id_for_model(model_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(model_id.as_bytes());
    let hash = hasher.finalize();

    // ULID is 26 characters in Crockford Base32
    // We use first 16 bytes of hash (128 bits) to generate 26 chars
    let encoded = crockford_encode(&hash[..16]);

    format!("pd_{encoded}")
}

/// Encodes bytes as Crockford Base32 (lowercase).
///
/// Crockford Base32 uses the alphabet: 0123456789abcdefghjkmnpqrstvwxyz
/// (excludes i, l, o, u to avoid confusion)
fn crockford_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

    let mut result = String::with_capacity(26);
    let mut bits: u64 = 0;
    let mut num_bits = 0;

    for &byte in bytes {
        bits = (bits << 8) | (byte as u64);
        num_bits += 8;

        while num_bits >= 5 {
            num_bits -= 5;
            let idx = ((bits >> num_bits) & 0x1F) as usize;
            result.push(ALPHABET[idx] as char);
        }
    }

    // Handle remaining bits
    if num_bits > 0 {
        let idx = ((bits << (5 - num_bits)) & 0x1F) as usize;
        result.push(ALPHABET[idx] as char);
    }

    // Pad to 26 characters (ULID length)
    while result.len() < 26 {
        result.push('0');
    }

    result.truncate(26);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_product_id_deterministic() {
        let id1 =
            generate_product_id_for_model("anthropic/claude-opus-4.5");
        let id2 =
            generate_product_id_for_model("anthropic/claude-opus-4.5");
        assert_eq!(id1, id2, "Same model ID should produce same ProductId");

        let id3 = generate_product_id_for_model("openai/gpt-5");
        assert_ne!(
            id1, id3,
            "Different model IDs should produce different ProductIds"
        );
    }

    #[test]
    fn test_generate_product_id_format() {
        let id = generate_product_id_for_model("anthropic/claude-opus-4.5");
        assert!(
            id.starts_with("pd_"),
            "ProductId should start with 'pd_' prefix"
        );
        assert_eq!(
            id.len(),
            29,
            "ProductId should be 29 chars (3 prefix + 26 ULID)"
        );
    }

    #[test]
    fn test_crockford_encode() {
        // Test that encoding produces valid Crockford Base32 characters
        let result = crockford_encode(&[0xFF; 16]);
        for c in result.chars() {
            assert!(
                "0123456789abcdefghjkmnpqrstvwxyz".contains(c),
                "Invalid character: {c}"
            );
        }
    }
}
