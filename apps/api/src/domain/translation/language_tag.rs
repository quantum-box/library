//! # LanguageTag
//!
//! A normalized BCP-47 language tag, restricted to the subset the
//! translation pipeline needs: a primary subtag, an optional script and
//! an optional region. Extensions and private-use tags are rejected
//! rather than passed through, because the tag becomes part of a cache
//! key, an ETag and an `hreflang` attribute -- three places where two
//! spellings of the same language are a bug.

use errors::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Display, str::FromStr};

/// A normalized BCP-47 language tag such as `ja`, `en` or `zh-Hans`.
///
/// Parsing is case-insensitive and the result is canonicalized:
/// lowercase primary subtag, title-case script, uppercase region. Two
/// tags naming the same language therefore compare equal and hash the
/// same, whichever way a caller spelled them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LanguageTag {
    value: String,
}

impl LanguageTag {
    /// Parses and normalizes a language tag.
    ///
    /// # Errors
    ///
    /// Returns a type error when the tag is empty, longer than the
    /// storage column, or not of the form
    /// `primary[-Script][-REGION]`.
    pub fn new(tag: impl Into<String>) -> Result<Self, Error> {
        let raw = tag.into();
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return Err(Error::type_error("Language tag cannot be empty"));
        }

        // The storage column is VARCHAR(16); refuse anything that would
        // be silently truncated on write.
        if trimmed.len() > 16 {
            return Err(Error::type_error(
                "Language tag is too long. Must be 16 characters or less",
            ));
        }

        let mut parts = trimmed.split('-');

        let primary = parts.next().unwrap_or_default();
        if !is_primary_subtag(primary) {
            return Err(Error::type_error(format!(
                "Invalid language tag `{trimmed}`: the primary subtag \
                 must be 2 or 3 letters"
            )));
        }
        let mut value = primary.to_ascii_lowercase();

        let mut next = parts.next();

        if let Some(candidate) = next {
            if is_script_subtag(candidate) {
                value.push('-');
                value.push_str(&title_case(candidate));
                next = parts.next();
            }
        }

        if let Some(candidate) = next {
            if is_region_subtag(candidate) {
                value.push('-');
                value.push_str(&candidate.to_ascii_uppercase());
                next = parts.next();
            } else {
                return Err(Error::type_error(format!(
                    "Invalid language tag `{trimmed}`: `{candidate}` is \
                     neither a script nor a region subtag"
                )));
            }
        }

        if next.is_some() {
            return Err(Error::type_error(format!(
                "Invalid language tag `{trimmed}`: only \
                 `primary[-Script][-REGION]` is supported"
            )));
        }

        Ok(Self { value })
    }

    /// The normalized tag, suitable for `hreflang` and `Content-Language`.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// The primary subtag alone (`zh` for `zh-Hans`).
    ///
    /// Used when matching a reader's `Accept-Language` against the
    /// published languages: someone asking for `en-GB` should be served
    /// the `en` translation rather than the source.
    pub fn primary(&self) -> &str {
        self.value.split('-').next().unwrap_or(&self.value)
    }

    /// Whether two tags name the same language ignoring script and
    /// region.
    pub fn matches_primary(&self, other: &Self) -> bool {
        self.primary() == other.primary()
    }
}

fn is_primary_subtag(candidate: &str) -> bool {
    matches!(candidate.len(), 2 | 3)
        && candidate.chars().all(|c| c.is_ascii_alphabetic())
}

fn is_script_subtag(candidate: &str) -> bool {
    candidate.len() == 4
        && candidate.chars().all(|c| c.is_ascii_alphabetic())
}

fn is_region_subtag(candidate: &str) -> bool {
    (candidate.len() == 2
        && candidate.chars().all(|c| c.is_ascii_alphabetic()))
        || (candidate.len() == 3
            && candidate.chars().all(|c| c.is_ascii_digit()))
}

fn title_case(candidate: &str) -> String {
    let mut chars = candidate.chars();
    match chars.next() {
        Some(first) => {
            let mut cased = first.to_ascii_uppercase().to_string();
            cased.push_str(&chars.as_str().to_ascii_lowercase());
            cased
        }
        None => String::new(),
    }
}

impl FromStr for LanguageTag {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Display for LanguageTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Serialize for LanguageTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for LanguageTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        LanguageTag::new(raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bare_primary_subtag() {
        assert_eq!(LanguageTag::new("ja").unwrap().as_str(), "ja");
        assert_eq!(LanguageTag::new("en").unwrap().as_str(), "en");
    }

    #[test]
    fn normalizes_case_so_one_language_has_one_cache_key() {
        assert_eq!(LanguageTag::new("JA").unwrap().as_str(), "ja");
        assert_eq!(
            LanguageTag::new("zh-hans").unwrap().as_str(),
            "zh-Hans"
        );
        assert_eq!(LanguageTag::new("en-gb").unwrap().as_str(), "en-GB");
        assert_eq!(
            LanguageTag::new("ZH-HANS-CN").unwrap().as_str(),
            "zh-Hans-CN"
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(LanguageTag::new("  en  ").unwrap().as_str(), "en");
    }

    #[test]
    fn rejects_input_that_is_not_a_language() {
        for invalid in ["", "   ", "e", "english!", "en_GB", "en-", "-en"] {
            assert!(
                LanguageTag::new(invalid).is_err(),
                "`{invalid}` must be rejected"
            );
        }
    }

    #[test]
    fn rejects_a_tag_longer_than_the_storage_column() {
        assert!(LanguageTag::new("en-Latn-GB-oxendict").is_err());
    }

    #[test]
    fn rejects_trailing_subtags_beyond_script_and_region() {
        assert!(LanguageTag::new("en-Latn-GB-x").is_err());
    }

    #[test]
    fn exposes_the_primary_subtag_for_fallback_matching() {
        let tag = LanguageTag::new("zh-Hans-CN").unwrap();
        assert_eq!(tag.primary(), "zh");

        let requested = LanguageTag::new("en-GB").unwrap();
        let published = LanguageTag::new("en").unwrap();
        assert!(requested.matches_primary(&published));

        let japanese = LanguageTag::new("ja").unwrap();
        assert!(!requested.matches_primary(&japanese));
    }

    #[test]
    fn round_trips_through_serde() {
        let tag = LanguageTag::new("zh-Hant").unwrap();
        let json = serde_json::to_string(&tag).unwrap();
        assert_eq!(json, "\"zh-Hant\"");
        let parsed: LanguageTag = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, tag);
    }
}
