//! Identifier and code rules of catalog drafts.

/// Stable identifiers: letters, digits, `.`, `_` and `-`, at most 64.
///
/// Used for `ingredient_key`, `nutrient_key` and `source_food_code`. Food
/// numbers keep their leading zeros, so they are validated as text.
pub fn validate_key(field: &str, raw: &str) -> errors::Result<String> {
    let s = raw.trim();
    let valid = !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    if !valid {
        return Err(errors::Error::invalid(format!(
            "{field} must be 1-64 characters of [A-Za-z0-9._-]: {raw:?}"
        )));
    }
    Ok(s.to_string())
}

/// Cooking or processing state code, e.g. `raw`, `boiled`, `dried`.
/// Blank means "no state".
pub fn validate_cooking_state(
    raw: Option<&str>,
) -> errors::Result<Option<String>> {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let valid = s.len() <= 32
        && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    if !valid {
        return Err(errors::Error::invalid(format!(
            "cooking_state must be 1-32 characters of [a-z_]: {s:?}"
        )));
    }
    Ok(Some(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_keep_leading_zeros_and_reject_spaces() {
        assert_eq!(validate_key("k", " 01001 ").unwrap(), "01001");
        assert_eq!(validate_key("k", "PROT-").unwrap(), "PROT-");
        assert!(validate_key("k", "").is_err());
        assert!(validate_key("k", "VITK A").is_err());
        assert!(validate_key("k", &"a".repeat(65)).is_err());
    }

    #[test]
    fn cooking_state_is_lowercase_snake() {
        assert_eq!(
            validate_cooking_state(Some("stir_fried")).unwrap(),
            Some("stir_fried".to_string())
        );
        assert_eq!(validate_cooking_state(Some(" ")).unwrap(), None);
        assert!(validate_cooking_state(Some("Raw")).is_err());
        assert!(validate_cooking_state(Some("ゆで")).is_err());
    }
}
