//! Which values may be translated, derived from the property type.
//!
//! The type system already knows most of the answer. Translating an
//! `Id` breaks the reference it names, translating a `Date` or an
//! `Integer` corrupts a value that was never prose, and translating an
//! `Image` URL points it at nothing. Those are excluded mechanically.
//!
//! What the type cannot know is that a particular `String` column holds
//! part numbers, internal codes or person names. That judgement belongs
//! to whoever designed the schema, so the default computed here is an
//! opening position that a stored override replaces.

use database_manager::domain::PropertyType;

/// Whether a record's value for this property is prose by default.
///
/// `Select` and `MultiSelect` are deliberately excluded: their values
/// are references to option labels, and the labels are translated once
/// at the schema level instead of once per record.
// Consumed by Tier 2, which translates record values; Tier 1 only
// needs the schema-label rules above. The decision table it encodes is
// safety-relevant enough to keep tested rather than delete and rewrite.
#[allow(dead_code)]
pub fn record_value_is_translatable_by_default(
    property_type: &PropertyType,
) -> bool {
    match property_type {
        PropertyType::String
        | PropertyType::Markdown
        | PropertyType::RichText
        | PropertyType::Html => true,

        PropertyType::Integer
        | PropertyType::Date
        | PropertyType::Image
        | PropertyType::Id(_)
        | PropertyType::Location(_)
        | PropertyType::Relation(_)
        | PropertyType::Select(_)
        | PropertyType::MultiSelect(_) => false,
    }
}

/// Whether this property carries option labels that are translated at
/// the schema level.
///
/// Translating these is the cheapest work in the whole pipeline: a
/// handful of labels per property, translated once, changing how every
/// record reads.
pub fn has_translatable_schema_labels(
    property_type: &PropertyType,
) -> bool {
    matches!(
        property_type,
        PropertyType::Select(_) | PropertyType::MultiSelect(_)
    )
}

/// Whether a property whose name starts with this prefix should be left
/// alone by default.
///
/// `ext_` properties are mirrored from an external system by the sync
/// providers. Translating them would make the copy disagree with its
/// source on the next round trip.
pub fn is_externally_owned_property(property_name: &str) -> bool {
    property_name.starts_with("ext_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use database_manager::domain::{
        TypeId, TypeLocation, TypeMultiSelect, TypeRelation, TypeSelect,
    };

    #[test]
    fn prose_types_are_translatable() {
        for property_type in [
            PropertyType::String,
            PropertyType::Markdown,
            PropertyType::RichText,
            PropertyType::Html,
        ] {
            assert!(
                record_value_is_translatable_by_default(&property_type),
                "{property_type:?} should be translatable"
            );
        }
    }

    #[test]
    fn types_that_would_break_when_translated_are_excluded() {
        assert!(!record_value_is_translatable_by_default(
            &PropertyType::Integer
        ));
        assert!(!record_value_is_translatable_by_default(
            &PropertyType::Date
        ));
        assert!(!record_value_is_translatable_by_default(
            &PropertyType::Image
        ));
        assert!(!record_value_is_translatable_by_default(
            &PropertyType::Id(TypeId::default())
        ));
        assert!(!record_value_is_translatable_by_default(
            &PropertyType::Location(TypeLocation::default())
        ));
        assert!(!record_value_is_translatable_by_default(
            &PropertyType::Relation(TypeRelation::default())
        ));
    }

    #[test]
    fn select_values_are_not_translated_per_record() {
        let select = PropertyType::Select(TypeSelect::default());
        let multi = PropertyType::MultiSelect(TypeMultiSelect::default());

        // The value is a reference to a label...
        assert!(!record_value_is_translatable_by_default(&select));
        assert!(!record_value_is_translatable_by_default(&multi));

        // ...and the label itself is translated once, at the schema
        // level.
        assert!(has_translatable_schema_labels(&select));
        assert!(has_translatable_schema_labels(&multi));
    }

    #[test]
    fn prose_types_carry_no_schema_labels() {
        assert!(!has_translatable_schema_labels(&PropertyType::String));
        assert!(!has_translatable_schema_labels(&PropertyType::Markdown));
    }

    #[test]
    fn externally_synced_properties_are_left_alone() {
        assert!(is_externally_owned_property("ext_github_issue_number"));
        assert!(!is_externally_owned_property("summary"));
        assert!(!is_externally_owned_property("extension"));
    }
}
