use std::cmp::Ordering;
use std::collections::HashSet;

use serde_json::json;
use value_object::Location;

use crate::{
    DataId, DatabaseId, PropertyDataValue, PropertyType, SelectItem,
    SelectItemId, TypeId, TypeLocation, TypeMultiSelect, TypeRelation,
    TypeSelect,
};

use super::*;

const RELATION_DATABASE: &str = "db_01hmp05xtq6fs5mmk8fg125cy7";
const RELATION_DATA: &str = "data_01hmp06dkf89pzjp75p1p6gfw7";

struct ContractFixture {
    expected_key: &'static str,
    expected_legacy_key: &'static str,
    expected_storage: StorageClass,
    expected_indexes: IndexCapabilities,
    expected_references: Vec<PropertyReference>,
    config: PropertyConfig,
    value: PropertyDataValue,
}

fn option(id: &str, key: &str, name: &str) -> SelectItem {
    SelectItem::new(
        id.parse().expect("valid SelectItemId"),
        key.parse().expect("valid option key"),
        name.parse().expect("valid option name"),
    )
}

fn fixtures() -> Vec<ContractFixture> {
    let database_id: DatabaseId = RELATION_DATABASE
        .parse()
        .expect("valid relation DatabaseId");
    let data_id: DataId =
        RELATION_DATA.parse().expect("valid relation DataId");
    let red = option("op_red", "red", "Red");
    let blue = option("op_blue", "blue", "Blue");

    vec![
        ContractFixture {
            expected_key: "string",
            expected_legacy_key: "STRING",
            expected_storage: StorageClass::Text,
            expected_indexes: IndexCapabilities {
                exact: true,
                range: false,
                full_text: true,
                sortable: true,
                unique: true,
                multi_value: false,
            },
            expected_references: vec![],
            config: PropertyConfig::String,
            value: PropertyDataValue::String("hello".to_string()),
        },
        ContractFixture {
            expected_key: "integer",
            expected_legacy_key: "INTEGER",
            expected_storage: StorageClass::Integer,
            expected_indexes: IndexCapabilities::scalar(
                true, false, true, true,
            ),
            expected_references: vec![],
            config: PropertyConfig::Integer,
            value: PropertyDataValue::Integer(42),
        },
        ContractFixture {
            expected_key: "html",
            expected_legacy_key: "HTML",
            expected_storage: StorageClass::Text,
            expected_indexes: IndexCapabilities::content(),
            expected_references: vec![],
            config: PropertyConfig::Html,
            value: PropertyDataValue::Html("<p>Hello</p>".to_string()),
        },
        ContractFixture {
            expected_key: "markdown",
            expected_legacy_key: "MARKDOWN",
            expected_storage: StorageClass::Text,
            expected_indexes: IndexCapabilities::content(),
            expected_references: vec![],
            config: PropertyConfig::Markdown,
            value: PropertyDataValue::Markdown("# Hello".to_string()),
        },
        ContractFixture {
            expected_key: "relation",
            expected_legacy_key: "RELATION",
            expected_storage: StorageClass::MultiReference,
            expected_indexes: IndexCapabilities::multi_reference(),
            expected_references: vec![PropertyReference::Data {
                database_id: database_id.clone(),
                data_id: data_id.clone(),
            }],
            config: PropertyConfig::Relation(TypeRelation::new(
                database_id.clone(),
            )),
            value: PropertyDataValue::Relation(database_id, vec![data_id]),
        },
        ContractFixture {
            expected_key: "select",
            expected_legacy_key: "SELECT",
            expected_storage: StorageClass::Reference,
            expected_indexes: IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: true,
                unique: true,
                multi_value: false,
            },
            expected_references: vec![PropertyReference::SelectOption(
                red.id().clone(),
            )],
            config: PropertyConfig::Select(TypeSelect::new(vec![
                red.clone(),
                blue.clone(),
            ])),
            value: PropertyDataValue::Select(red.id().clone()),
        },
        ContractFixture {
            expected_key: "multi_select",
            expected_legacy_key: "MULTI_SELECT",
            expected_storage: StorageClass::MultiReference,
            expected_indexes: IndexCapabilities::multi_reference(),
            expected_references: vec![
                PropertyReference::SelectOption(red.id().clone()),
                PropertyReference::SelectOption(blue.id().clone()),
            ],
            config: PropertyConfig::MultiSelect(TypeMultiSelect::new(
                vec![red.clone(), blue.clone()],
            )),
            value: PropertyDataValue::MultiSelect(vec![
                red.id().clone(),
                blue.id().clone(),
            ]),
        },
        ContractFixture {
            expected_key: "id",
            expected_legacy_key: "ID",
            expected_storage: StorageClass::Text,
            expected_indexes: IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: true,
                unique: true,
                multi_value: false,
            },
            expected_references: vec![],
            config: PropertyConfig::Id(TypeId::new(true)),
            value: PropertyDataValue::Id(
                "data_01hmp06dkf89pzjp75p1p6gfw7".to_string(),
            ),
        },
        ContractFixture {
            expected_key: "location",
            expected_legacy_key: "LOCATION",
            expected_storage: StorageClass::Location,
            expected_indexes: IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: false,
                unique: false,
                multi_value: false,
            },
            expected_references: vec![],
            config: PropertyConfig::Location(TypeLocation::new(
                Some(35.681_236_2),
                Some(139.764_936_1),
            )),
            value: PropertyDataValue::Location(
                Location::new(35.681_236_2, 139.764_936_1)
                    .expect("valid location"),
            ),
        },
        ContractFixture {
            expected_key: "date",
            expected_legacy_key: "DATE",
            expected_storage: StorageClass::Date,
            expected_indexes: IndexCapabilities::scalar(
                true, false, true, true,
            ),
            expected_references: vec![],
            config: PropertyConfig::Date,
            value: PropertyDataValue::Date("2024-02-29".to_string()),
        },
        ContractFixture {
            expected_key: "image",
            expected_legacy_key: "IMAGE",
            expected_storage: StorageClass::Text,
            expected_indexes: IndexCapabilities {
                exact: true,
                range: false,
                full_text: false,
                sortable: false,
                unique: false,
                multi_value: false,
            },
            expected_references: vec![PropertyReference::ExternalUrl(
                "https://example.com/image.png".to_string(),
            )],
            config: PropertyConfig::Image,
            value: PropertyDataValue::Image(
                "https://example.com/image.png".to_string(),
            ),
        },
        ContractFixture {
            expected_key: "rich_text",
            expected_legacy_key: "RICH_TEXT",
            expected_storage: StorageClass::Text,
            // Nothing is indexable: see the handler for why a full-text
            // index over the stored document is worse than none.
            expected_indexes: IndexCapabilities {
                exact: false,
                range: false,
                full_text: false,
                sortable: false,
                unique: false,
                multi_value: false,
            },
            expected_references: vec![],
            config: PropertyConfig::RichText,
            value: PropertyDataValue::RichText(json!([
                {
                    "id": "block-1",
                    "type": "paragraph",
                    "props": {},
                    "content": [
                        { "type": "text", "text": "Hello", "styles": {} }
                    ],
                    "children": [],
                },
                // The empty paragraph this property type exists for.
                {
                    "id": "block-2",
                    "type": "paragraph",
                    "props": {},
                    "content": [],
                    "children": [],
                },
            ])),
        },
    ]
}

#[test]
fn all_builtin_v1_types_satisfy_the_same_contract() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 12);

    for fixture in fixtures {
        let handler = fixture.config.handler();
        let type_ref = handler.type_ref();
        assert_eq!(type_ref.key.as_str(), fixture.expected_key);
        assert_eq!(type_ref.version, PROPERTY_TYPE_VERSION_V1);
        assert_eq!(
            handler.value_encoding_version(),
            VALUE_ENCODING_VERSION_V1
        );
        assert_eq!(
            handler.conversion_policy(&type_ref),
            ConversionPolicy::Identity
        );

        handler
            .validate_config(&fixture.config)
            .expect("fixture config must be valid");
        handler
            .validate_value(&fixture.config, &fixture.value)
            .expect("fixture value must be valid");

        let encoded_config = handler
            .encode_config(&fixture.config)
            .expect("config must encode");
        let decoded_config = handler
            .decode_config(encoded_config.clone())
            .expect("config must decode");
        assert_eq!(
            handler
                .encode_config(&decoded_config)
                .expect("decoded config must re-encode"),
            encoded_config
        );
        let facade = PropertyType::from_meta(
            fixture.expected_legacy_key,
            encoded_config.clone(),
        )
        .expect("legacy facade must decode every built-in type");
        assert_eq!(facade.canonical_type_ref(), type_ref);
        assert_eq!(
            facade.get_meta().expect("legacy facade config must encode"),
            encoded_config
        );

        let encoded_value = handler
            .encode_value(&fixture.config, &fixture.value)
            .expect("value must encode");
        assert_eq!(
            fixture
                .value
                .encode_canonical(&facade)
                .expect("compatibility value facade must encode"),
            encoded_value
        );
        let canonical_value = fixture
            .value
            .canonical_value(&facade)
            .expect("compatibility value facade must create an envelope");
        assert_eq!(canonical_value.type_ref(), &type_ref);
        assert_eq!(
            canonical_value.encoding_version(),
            VALUE_ENCODING_VERSION_V1
        );
        let resolved_config = ResolvedPropertyConfig::Known(decoded_config);
        let decoded_value = BUILTIN_PROPERTY_TYPE_REGISTRY
            .decode_value(
                type_ref,
                VALUE_ENCODING_VERSION_V1,
                &resolved_config,
                encoded_value,
            )
            .expect("value must decode");
        decoded_value
            .ensure_writable()
            .expect("a registry-decoded built-in value must be writable");
        let PropertyValue::Known(decoded_value) = decoded_value else {
            panic!("a built-in v1 value must not become opaque");
        };
        assert_eq!(decoded_value.value(), &fixture.value);
        assert_eq!(
            handler
                .compare(
                    &fixture.config,
                    decoded_value.value(),
                    &fixture.value,
                )
                .expect("values must compare"),
            Ordering::Equal
        );
        let references = handler
            .extract_references(&fixture.config, &fixture.value)
            .expect("reference extraction must be defined for every type");
        assert_eq!(references, fixture.expected_references);
        assert_eq!(handler.storage_class(), fixture.expected_storage);
        assert_eq!(handler.index_capabilities(), fixture.expected_indexes);
    }
}

#[test]
fn compile_time_registry_has_exactly_one_handler_per_builtin_type() {
    let handlers = BUILTIN_PROPERTY_TYPE_REGISTRY.handlers();
    assert_eq!(handlers.len(), PropertyKind::ALL.len());

    let mut identities = HashSet::new();
    let mut kinds = HashSet::new();
    for handler in handlers {
        let type_ref = handler.type_ref();
        assert!(identities.insert((
            type_ref.key.as_str().to_string(),
            type_ref.version.get(),
        )));
        assert!(kinds.insert(handler.kind()));
    }
    assert_eq!(kinds.len(), PropertyKind::ALL.len());
}

#[test]
fn unknown_type_and_encoding_are_lossless_and_read_only() {
    let unknown_ref = PropertyTypeRef::new(
        PropertyTypeKey::new("future_type").expect("canonical key"),
        PropertyTypeVersion::new(7).expect("non-zero type version"),
    );
    let raw_config = json!({"nested": {"must": ["stay", "intact"]}});
    let config = BUILTIN_PROPERTY_TYPE_REGISTRY
        .decode_config(unknown_ref.clone(), raw_config.clone())
        .expect("unknown config must be preserved");
    let ResolvedPropertyConfig::Opaque(opaque_config) = &config else {
        panic!("unknown config must stay opaque");
    };
    assert_eq!(opaque_config.raw_config, raw_config);
    let config_round_trip: OpaquePropertyConfig = serde_json::from_value(
        serde_json::to_value(opaque_config)
            .expect("opaque config must serialize"),
    )
    .expect("opaque config must deserialize");
    assert_eq!(&config_round_trip, opaque_config);
    let error = opaque_config
        .ensure_writable()
        .expect_err("unknown config must be read-only");
    assert!(error.to_string().contains("future_type@7"));

    let raw_value = json!({"future": [1, true, null]});
    let value = BUILTIN_PROPERTY_TYPE_REGISTRY
        .decode_value(
            unknown_ref,
            ValueEncodingVersion::new(3)
                .expect("non-zero encoding version"),
            &config,
            raw_value.clone(),
        )
        .expect("unknown value must be preserved");
    let PropertyValue::Opaque(opaque_value) = value else {
        panic!("unknown value must stay opaque");
    };
    assert_eq!(opaque_value.raw_value, raw_value);
    let value_round_trip: OpaquePropertyValue = serde_json::from_value(
        serde_json::to_value(&opaque_value)
            .expect("opaque value must serialize"),
    )
    .expect("opaque value must deserialize");
    assert_eq!(value_round_trip, opaque_value);
    let error = opaque_value
        .ensure_writable()
        .expect_err("unknown value must be read-only");
    assert!(error.to_string().contains("future_type@7"));

    let future_string_ref = PropertyTypeRef::new(
        PropertyTypeKey::new("string").expect("canonical key"),
        PropertyTypeVersion::new(2).expect("non-zero type version"),
    );
    let future_config = json!({"future_config": true});
    let config = BUILTIN_PROPERTY_TYPE_REGISTRY
        .decode_config(future_string_ref, future_config.clone())
        .expect("future type version must be preserved");
    let ResolvedPropertyConfig::Opaque(config) = config else {
        panic!("future type version must stay opaque");
    };
    assert_eq!(config.raw_config, future_config);
    let error = config
        .ensure_writable()
        .expect_err("future type version must be read-only");
    assert!(error.to_string().contains("string@2"));

    let known_config =
        ResolvedPropertyConfig::Known(PropertyConfig::String);
    let unknown_encoding = json!({"not": "a v1 string"});
    let value = BUILTIN_PROPERTY_TYPE_REGISTRY
        .decode_value(
            PropertyKind::String.type_ref(),
            ValueEncodingVersion::new(2)
                .expect("non-zero encoding version"),
            &known_config,
            unknown_encoding.clone(),
        )
        .expect("future encoding must be preserved");
    let PropertyValue::Opaque(value) = value else {
        panic!("future encoding must stay opaque");
    };
    assert_eq!(value.raw_value, unknown_encoding);
    let error = value
        .ensure_writable()
        .expect_err("future encoding must be read-only");
    assert!(error.to_string().contains("string@1 encoding v2"));
}

#[test]
fn canonical_keys_reject_legacy_or_display_spelling() {
    assert!(PropertyTypeKey::new("multi_select").is_ok());
    assert!(PropertyTypeKey::new("MULTI_SELECT").is_err());
    assert!(PropertyTypeKey::new("multi-select").is_err());
    assert!(PropertyTypeKey::new("multi__select").is_err());
    assert!(PropertyTypeKey::new("multi_select_").is_err());
    assert!(PropertyTypeKey::new("").is_err());
    assert!(
        PropertyTypeKey::new("a".repeat(MAX_PROPERTY_TYPE_KEY_BYTES))
            .is_ok()
    );
    assert!(
        PropertyTypeKey::new("a".repeat(MAX_PROPERTY_TYPE_KEY_BYTES + 1))
            .is_err()
    );
    assert!(
        serde_json::from_value::<PropertyTypeKey>(json!("INVALID"))
            .is_err()
    );
    assert!(
        serde_json::from_value::<PropertyTypeVersion>(json!(0)).is_err()
    );
    assert!(
        serde_json::from_value::<ValueEncodingVersion>(json!(0)).is_err()
    );
}

#[test]
fn legacy_uppercase_and_relation_csv_are_isolated_compatibility_readers() {
    let relation_meta = json!({"database_id": RELATION_DATABASE});
    let property_type = PropertyType::from_meta("RELATION", relation_meta)
        .expect("legacy type");
    let legacy_csv = format!("{RELATION_DATABASE},{RELATION_DATA}");
    let value =
        PropertyDataValue::from_storage(&legacy_csv, &property_type)
            .expect("legacy Relation CSV");
    assert_eq!(value.string_value(), legacy_csv);

    assert!(PropertyType::from_meta("relation", json!(null)).is_err());
    assert!(PropertyTypeKey::new("RELATION").is_err());
}

#[test]
fn legacy_invalid_config_is_readable_but_not_canonically_writable() {
    let first = option("op_same", "first", "First");
    let duplicate = option("op_same", "second", "Second");
    let legacy_select_meta = serde_json::to_value(TypeSelect::new(vec![
        first.clone(),
        duplicate,
    ]))
    .expect("legacy Select metadata must serialize");

    let legacy_select =
        PropertyType::from_meta("SELECT", legacy_select_meta).expect(
            "historically accepted Select metadata must remain readable",
        );
    let PropertyType::Select(select) = &legacy_select else {
        panic!("legacy Select must retain its facade variant");
    };
    assert_eq!(select.items().len(), 2);
    legacy_select
        .get_meta()
        .expect_err("canonical writes must reject duplicate Select ids");
    PropertyDataValue::Select(first.id().clone())
        .canonical_value(&legacy_select)
        .expect_err(
            "canonical normalization must reject the invalid config",
        );

    let legacy_location = PropertyType::from_meta(
        "LOCATION",
        json!({
            "default_latitude": 91.0,
            "default_longitude": 0.0,
        }),
    )
    .expect("legacy Location metadata is restored without new validation");
    let PropertyType::Location(location) = &legacy_location else {
        panic!("legacy Location must retain its facade variant");
    };
    assert_eq!(*location.default_latitude(), Some(91.0));
    legacy_location.get_meta().expect_err(
        "canonical writes must reject invalid Location defaults",
    );
}

#[test]
fn forged_known_envelopes_cannot_bypass_the_writable_guard() {
    let unknown_ref = PropertyTypeRef::new(
        PropertyTypeKey::new("future_type").expect("canonical key"),
        PropertyTypeVersion::new(7).expect("non-zero type version"),
    );
    let forged_unknown =
        super::registry::forge_known_property_value_for_test(
            unknown_ref,
            VALUE_ENCODING_VERSION_V1,
            PropertyDataValue::String("must stay read-only".to_string()),
        );
    let error = PropertyValue::Known(forged_unknown)
        .ensure_writable()
        .expect_err(
            "an unknown envelope cannot become writable by being forged",
        );
    assert!(error.to_string().contains("future_type@7"));

    let forged_mismatch =
        super::registry::forge_known_property_value_for_test(
            PropertyKind::String.type_ref(),
            VALUE_ENCODING_VERSION_V1,
            PropertyDataValue::Integer(42),
        );
    let error = PropertyValue::Known(forged_mismatch)
        .ensure_writable()
        .expect_err("a payload variant must match its known envelope");
    assert!(
        error
            .to_string()
            .contains("does not match envelope string@1")
    );
}

#[test]
fn conversion_is_never_implicit_across_distinct_types() {
    let string = PropertyConfig::String.handler();
    assert_eq!(
        string.conversion_policy(&PropertyKind::Integer.type_ref()),
        ConversionPolicy::ExplicitValidated
    );
    assert!(
        !string
            .conversion_policy(&PropertyKind::Integer.type_ref())
            .is_implicit()
    );

    let relation = PropertyConfig::Relation(TypeRelation::new(
        RELATION_DATABASE.parse().expect("valid DatabaseId"),
    ));
    assert_eq!(
        relation
            .handler()
            .conversion_policy(&PropertyKind::Integer.type_ref()),
        ConversionPolicy::Forbidden
    );
}

#[test]
fn representative_builtin_values_have_deterministic_ordering() {
    assert_ordered(
        PropertyConfig::String,
        PropertyDataValue::String("alpha".to_string()),
        PropertyDataValue::String("beta".to_string()),
    );
    assert_ordered(
        PropertyConfig::Integer,
        PropertyDataValue::Integer(41),
        PropertyDataValue::Integer(42),
    );
    assert_ordered(
        PropertyConfig::Date,
        PropertyDataValue::Date("2024-02-28".to_string()),
        PropertyDataValue::Date("2024-02-29".to_string()),
    );

    let first = option("op_first", "first", "First");
    let second = option("op_second", "second", "Second");
    let config = PropertyConfig::Select(TypeSelect::new(vec![
        first.clone(),
        second.clone(),
    ]));
    assert_ordered(
        config,
        PropertyDataValue::Select(first.id().clone()),
        PropertyDataValue::Select(second.id().clone()),
    );
}

fn assert_ordered(
    config: PropertyConfig,
    lesser: PropertyDataValue,
    greater: PropertyDataValue,
) {
    let handler = config.handler();
    assert_eq!(
        handler
            .compare(&config, &lesser, &greater)
            .expect("less-than comparison"),
        Ordering::Less
    );
    assert_eq!(
        handler
            .compare(&config, &greater, &lesser)
            .expect("greater-than comparison"),
        Ordering::Greater
    );
}

#[test]
fn select_config_and_value_validation_reject_ambiguous_ids() {
    let first = option("op_same", "first", "First");
    let duplicate = option("op_same", "second", "Second");
    let config = PropertyConfig::Select(TypeSelect::new(vec![
        first.clone(),
        duplicate,
    ]));
    assert!(config.handler().validate_config(&config).is_err());

    let valid_config = PropertyConfig::Select(TypeSelect::new(vec![first]));
    let missing: SelectItemId =
        "op_missing".parse().expect("valid option id");
    assert!(
        valid_config
            .handler()
            .validate_value(
                &valid_config,
                &PropertyDataValue::Select(missing),
            )
            .is_err()
    );
}
