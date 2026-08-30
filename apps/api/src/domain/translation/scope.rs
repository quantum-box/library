//! What a translation row is attached to, and how fresh it is.

use errors::Error;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

/// The kind of thing a translation row translates.
///
/// The variants are ordered by the tier they belong to: the first three
/// are schema-level (translated once, applying to every record), the
/// last two are record-level.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TranslationScope {
    /// Tier 1 -- the display name of a database.
    Database,
    /// Tier 1 -- the name of a property definition, i.e. a column
    /// heading.
    PropertyDef,
    /// Tier 1 -- one option label of a `Select` or `MultiSelect`.
    SelectOption,
    /// Tier 2 -- a record's `name`, which is what a docs listing shows.
    RecordName,
    /// Tier 2 and 3 -- one property value of one record.
    PropertyValue,
}

impl TranslationScope {
    /// The stored representation, matching the `scope` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Database => "DATABASE",
            Self::PropertyDef => "PROPERTY_DEF",
            Self::SelectOption => "SELECT_OPTION",
            Self::RecordName => "RECORD_NAME",
            Self::PropertyValue => "PROPERTY_VALUE",
        }
    }
}

impl FromStr for TranslationScope {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DATABASE" => Ok(Self::Database),
            "PROPERTY_DEF" => Ok(Self::PropertyDef),
            "SELECT_OPTION" => Ok(Self::SelectOption),
            "RECORD_NAME" => Ok(Self::RecordName),
            "PROPERTY_VALUE" => Ok(Self::PropertyValue),
            other => Err(Error::type_error(format!(
                "Unknown translation scope `{other}`"
            ))),
        }
    }
}

impl Display for TranslationScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Freshness of a cached translation relative to its source text.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TranslationStatus {
    /// The source hash matches; the translation can be served as-is.
    Fresh,
    /// The source changed after this translation was produced.
    ///
    /// A public reader is still served the stale text: someone who
    /// cannot read the source language is better off with a slightly
    /// out-of-date translation than with the original.
    Stale,
    /// Queued but not yet produced.
    Pending,
    /// Translation was attempted and failed; the reader falls back to
    /// the source.
    Failed,
}

impl TranslationStatus {
    /// The stored representation, matching the `status` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fresh => "FRESH",
            Self::Stale => "STALE",
            Self::Pending => "PENDING",
            Self::Failed => "FAILED",
        }
    }

    /// Whether a row in this state carries text that can be shown.
    pub fn has_text(&self) -> bool {
        matches!(self, Self::Fresh | Self::Stale)
    }
}

impl FromStr for TranslationStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FRESH" => Ok(Self::Fresh),
            "STALE" => Ok(Self::Stale),
            "PENDING" => Ok(Self::Pending),
            "FAILED" => Ok(Self::Failed),
            other => Err(Error::type_error(format!(
                "Unknown translation status `{other}`"
            ))),
        }
    }
}

impl Display for TranslationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_round_trips_through_its_stored_form() {
        for scope in [
            TranslationScope::Database,
            TranslationScope::PropertyDef,
            TranslationScope::SelectOption,
            TranslationScope::RecordName,
            TranslationScope::PropertyValue,
        ] {
            let stored = scope.as_str();
            assert_eq!(stored.parse::<TranslationScope>().unwrap(), scope);
        }
    }

    #[test]
    fn status_round_trips_through_its_stored_form() {
        for status in [
            TranslationStatus::Fresh,
            TranslationStatus::Stale,
            TranslationStatus::Pending,
            TranslationStatus::Failed,
        ] {
            let stored = status.as_str();
            assert_eq!(
                stored.parse::<TranslationStatus>().unwrap(),
                status
            );
        }
    }

    #[test]
    fn unknown_stored_values_are_rejected_rather_than_defaulted() {
        assert!("WHATEVER".parse::<TranslationScope>().is_err());
        assert!("whatever".parse::<TranslationStatus>().is_err());
        // Lowercase is not the stored form and must not be accepted.
        assert!("database".parse::<TranslationScope>().is_err());
    }

    #[test]
    fn only_produced_translations_carry_text() {
        assert!(TranslationStatus::Fresh.has_text());
        assert!(TranslationStatus::Stale.has_text());
        assert!(!TranslationStatus::Pending.has_text());
        assert!(!TranslationStatus::Failed.has_text());
    }
}
