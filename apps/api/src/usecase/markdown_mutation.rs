//! Markdown reader for data writes
//!
//! The inverse of [`super::markdown_composer`]: it reads back the
//! `---\n<yaml>\n---\n\n<body>` document that `compose_markdown` emits, so a
//! caller can fetch a record as Markdown, edit it, and write it back without
//! assembling typed property values by hand.
//!
//! Frontmatter keys name properties; the body goes to the record's body
//! property. A key that names no property is dropped with a warning rather
//! than failing the write, because a document written against an older
//! schema should still apply as far as it can.

use database_manager::domain::{Property, PropertyType};
use serde_yaml::{Mapping, Value as YamlValue};
use value_object::Location;

use super::{PropertyDataInputData, PropertyDataValueInputData};

/// Keys [`super::markdown_composer`] writes from the record itself rather
/// than from a property. They are dropped silently unless the repository
/// really has a property of that name.
const RESERVED_KEYS: [&str; 3] = ["id", "title", "url"];

/// A frontmatter key that was read but not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownWarning {
    pub key: String,
    pub reason: String,
}

impl MarkdownWarning {
    fn new(key: &str, reason: impl Into<String>) -> Self {
        Self {
            key: key.to_string(),
            reason: reason.into(),
        }
    }
}

/// What a Markdown document asks a write to do.
#[derive(Debug, Clone, Default)]
pub struct MarkdownMutation {
    /// The `title` key, which names the record rather than a property.
    pub title: Option<String>,
    pub property_data: Vec<PropertyDataInputData>,
    pub warnings: Vec<MarkdownWarning>,
}

/// A document split into its frontmatter mapping and its body.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MarkdownDocument {
    pub frontmatter: Mapping,
    pub body: String,
}

/// Split a document into YAML frontmatter and body.
///
/// A document without a leading `---` fence is all body. So is one whose
/// fence never closes, or whose frontmatter is not a YAML mapping: the
/// caller asked to write text, and guessing at half-parsed metadata would
/// silently drop part of it.
pub fn split_markdown_document(document: &str) -> MarkdownDocument {
    let trimmed = document.trim_start_matches(['\u{feff}', ' ', '\t']);
    let Some(rest) = trimmed
        .strip_prefix("---\n")
        .or_else(|| trimmed.strip_prefix("---\r\n"))
    else {
        return MarkdownDocument {
            frontmatter: Mapping::new(),
            body: document.to_string(),
        };
    };

    let Some((frontmatter, body)) = split_at_closing_fence(rest) else {
        return MarkdownDocument {
            frontmatter: Mapping::new(),
            body: document.to_string(),
        };
    };

    match serde_yaml::from_str::<YamlValue>(frontmatter) {
        Ok(YamlValue::Mapping(mapping)) => MarkdownDocument {
            frontmatter: mapping,
            body: body.trim_start_matches('\n').to_string(),
        },
        // An empty frontmatter block parses as null, which is a document
        // with no metadata rather than a malformed one.
        Ok(YamlValue::Null) => MarkdownDocument {
            frontmatter: Mapping::new(),
            body: body.trim_start_matches('\n').to_string(),
        },
        _ => MarkdownDocument {
            frontmatter: Mapping::new(),
            body: document.to_string(),
        },
    }
}

/// Find the `---` line that closes the frontmatter, returning the YAML
/// before it and the body after it.
fn split_at_closing_fence(rest: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']).trim_end() == "---" {
            return Some((&rest[..offset], &rest[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

/// Read a Markdown document against a repository's properties.
///
/// Errors only on a value the target property cannot hold -- the property
/// exists and the caller asked for it, so writing part of the value or
/// skipping it would be worse than refusing the call.
pub fn read_markdown_mutation(
    document: &str,
    properties: &[Property],
) -> errors::Result<MarkdownMutation> {
    let MarkdownDocument { frontmatter, body } =
        split_markdown_document(document);
    // The composer ends the document with a newline after the body, so a
    // body kept verbatim would grow one on every round trip.
    let body = body.trim_end_matches(['\n', '\r']);
    let mut mutation = MarkdownMutation::default();
    let body_target = body_property(properties);

    for (key, value) in &frontmatter {
        let Some(key) = key.as_str() else {
            mutation.warnings.push(MarkdownWarning::new(
                "<non-string key>",
                "frontmatter keys must be strings",
            ));
            continue;
        };

        if key == "title" {
            mutation.title = scalar_text(value);
        }

        let Some(property) = find_property(properties, key) else {
            if !RESERVED_KEYS.contains(&key) {
                mutation.warnings.push(MarkdownWarning::new(
                    key,
                    "no property with this name; value skipped",
                ));
            }
            continue;
        };

        // The body owns its property. Letting frontmatter write it too
        // would make which one wins depend on key order.
        if body_target.is_some_and(|target| target.id() == property.id()) {
            mutation.warnings.push(MarkdownWarning::new(
                key,
                "holds the document body; value taken from the body instead",
            ));
            continue;
        }

        let value = input_value(property, value).map_err(|reason| {
            errors::Error::invalid(format!(
                "frontmatter key '{key}': {reason}"
            ))
        })?;
        mutation.property_data.push(PropertyDataInputData {
            property_id: property.id().to_string(),
            value,
        });
    }

    // An empty body leaves the body property alone. A caller editing only
    // frontmatter should not have to carry the body along to keep it.
    if !body.trim().is_empty() {
        match body_target {
            Some(property) => {
                let value = body_value(property, body);
                mutation.property_data.push(PropertyDataInputData {
                    property_id: property.id().to_string(),
                    value,
                });
            }
            None => mutation.warnings.push(MarkdownWarning::new(
                "<body>",
                "repository has no body property; body skipped",
            )),
        }
    }

    Ok(mutation)
}

/// Pick the property the body belongs to.
///
/// Mirrors [`super::markdown_composer`]'s read-side order so a document
/// round-trips into the property it came out of: a property named `content`
/// first, then by type.
fn body_property(properties: &[Property]) -> Option<&Property> {
    properties
        .iter()
        .find(|property| {
            property.name().eq_ignore_ascii_case("content")
                && is_body_type(property.property_type())
        })
        .or_else(|| {
            properties.iter().find(|property| {
                matches!(property.property_type(), PropertyType::RichText)
            })
        })
        .or_else(|| {
            properties.iter().find(|property| {
                matches!(property.property_type(), PropertyType::Markdown)
            })
        })
        .or_else(|| {
            properties.iter().find(|property| {
                matches!(property.property_type(), PropertyType::Html)
            })
        })
}

fn is_body_type(property_type: &PropertyType) -> bool {
    matches!(
        property_type,
        PropertyType::RichText
            | PropertyType::Markdown
            | PropertyType::Html
            | PropertyType::String
    )
}

/// Resolve a frontmatter key to a property, by name and then by id.
///
/// The composer writes names, so an exact name wins; a case-insensitive
/// match follows, because YAML keys are routinely re-cased by hand.
fn find_property<'a>(
    properties: &'a [Property],
    key: &str,
) -> Option<&'a Property> {
    properties
        .iter()
        .find(|property| property.name() == key)
        .or_else(|| {
            properties
                .iter()
                .find(|property| property.name().eq_ignore_ascii_case(key))
        })
        .or_else(|| properties.iter().find(|property| property.id() == key))
}

/// Carry the body into the variant its property accepts.
///
/// A rich text property stores a block document, so Markdown is converted
/// here rather than at the value adapter, which rejects non-JSON.
fn body_value(
    property: &Property,
    body: &str,
) -> PropertyDataValueInputData {
    match property.property_type() {
        PropertyType::RichText => PropertyDataValueInputData::RichText(
            database_manager::domain::rich_text::from_markdown(body)
                .to_string(),
        ),
        PropertyType::Html => {
            PropertyDataValueInputData::Html(body.to_string())
        }
        PropertyType::Markdown => {
            PropertyDataValueInputData::Markdown(body.to_string())
        }
        _ => PropertyDataValueInputData::String(body.to_string()),
    }
}

/// Convert one frontmatter value into the input variant its property's type
/// accepts. The value adapter matches type against variant exactly, so the
/// mapping has to be complete.
fn input_value(
    property: &Property,
    value: &YamlValue,
) -> Result<PropertyDataValueInputData, String> {
    let text = || scalar_text(value).unwrap_or_default();
    let value = match property.property_type() {
        // A String property is where the composer puts a JSON envelope such
        // as `ext_github`, which it expands into a YAML mapping. Re-encoding
        // it keeps the round trip from blanking the envelope.
        PropertyType::String => {
            PropertyDataValueInputData::String(text_or_json(value)?)
        }
        PropertyType::Id(_) => PropertyDataValueInputData::String(text()),
        PropertyType::Integer => {
            PropertyDataValueInputData::Integer(text())
        }
        PropertyType::Html => PropertyDataValueInputData::Html(text()),
        PropertyType::Markdown => {
            PropertyDataValueInputData::Markdown(text())
        }
        PropertyType::RichText => PropertyDataValueInputData::RichText(
            database_manager::domain::rich_text::from_markdown(&text())
                .to_string(),
        ),
        PropertyType::Date => PropertyDataValueInputData::Date(text()),
        PropertyType::Image => PropertyDataValueInputData::Image(text()),
        PropertyType::Select(select) => PropertyDataValueInputData::Select(
            select_option_id(select.items(), &text()),
        ),
        PropertyType::MultiSelect(select) => {
            PropertyDataValueInputData::MultiSelect(
                string_list(value)?
                    .iter()
                    .map(|item| select_option_id(select.items(), item))
                    .collect(),
            )
        }
        PropertyType::Relation(_) => {
            PropertyDataValueInputData::Relation(relation_ids(value)?)
        }
        PropertyType::Boolean => {
            PropertyDataValueInputData::Boolean(boolean(value)?)
        }
        PropertyType::Location(_) => {
            PropertyDataValueInputData::Location(location(value)?)
        }
    };
    Ok(value)
}

/// Read a scalar as text, or a mapping or list as the JSON text it was
/// stored as before the composer expanded it.
fn text_or_json(value: &YamlValue) -> Result<String, String> {
    match value {
        YamlValue::Mapping(_) | YamlValue::Sequence(_) => {
            serde_json::to_string(value).map_err(|error| {
                format!("structured value is not JSON-encodable: {error}")
            })
        }
        scalar => Ok(scalar_text(scalar).unwrap_or_default()),
    }
}

/// Resolve a select option written by key or label back to its id.
///
/// The composer writes ids, so a round-trip needs no lookup; a person
/// editing the document writes the option they see. Text that matches
/// nothing is passed through, so the value adapter reports the bad option
/// rather than this function guessing.
fn select_option_id(
    items: &[database_manager::domain::SelectItem],
    text: &str,
) -> String {
    if items.iter().any(|item| item.id() == text) {
        return text.to_string();
    }
    items
        .iter()
        .find(|item| {
            item.key().to_string().eq_ignore_ascii_case(text)
                || item.name().to_string().eq_ignore_ascii_case(text)
        })
        .map_or_else(|| text.to_string(), |item| item.id().to_string())
}

/// Render a YAML scalar as the text the value adapter expects. A null is an
/// empty string, which every type reads as "clear this value".
fn scalar_text(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(text) => Some(text.clone()),
        YamlValue::Number(number) => Some(number.to_string()),
        YamlValue::Bool(flag) => Some(flag.to_string()),
        YamlValue::Null => Some(String::new()),
        _ => None,
    }
}

/// Read a list of ids, accepting the single scalar a hand-written document
/// tends to carry.
fn string_list(value: &YamlValue) -> Result<Vec<String>, String> {
    match value {
        YamlValue::Null => Ok(vec![]),
        YamlValue::Sequence(items) => items
            .iter()
            .map(|item| {
                scalar_text(item)
                    .ok_or_else(|| "list items must be scalars".to_string())
            })
            .collect(),
        scalar => match scalar_text(scalar) {
            Some(text) if text.is_empty() => Ok(vec![]),
            Some(text) => Ok(vec![text]),
            None => Err("expected a list of ids".to_string()),
        },
    }
}

/// Read relation targets, including the `{ databaseId, dataIds }` mapping
/// the composer writes.
fn relation_ids(value: &YamlValue) -> Result<Vec<String>, String> {
    if let YamlValue::Mapping(mapping) = value {
        let ids = mapping
            .get(YamlValue::String("dataIds".into()))
            .ok_or_else(|| "relation mapping needs dataIds".to_string())?;
        return string_list(ids);
    }
    string_list(value)
}

fn boolean(value: &YamlValue) -> Result<bool, String> {
    match value {
        YamlValue::Bool(flag) => Ok(*flag),
        YamlValue::Null => Ok(false),
        other => match scalar_text(other).unwrap_or_default().trim() {
            "true" | "1" | "yes" | "on" => Ok(true),
            "false" | "0" | "no" | "off" | "" => Ok(false),
            _ => Err("expected true or false".to_string()),
        },
    }
}

/// Read a location as the `latitude,longitude` text the composer writes, or
/// as a mapping of the two fields.
fn location(value: &YamlValue) -> Result<Location, String> {
    if let YamlValue::Mapping(mapping) = value {
        let coordinate = |key: &str| -> Result<f64, String> {
            mapping
                .get(YamlValue::String(key.into()))
                .and_then(|value| match value {
                    YamlValue::Number(number) => number.as_f64(),
                    YamlValue::String(text) => text.trim().parse().ok(),
                    _ => None,
                })
                .ok_or_else(|| format!("location needs a numeric {key}"))
        };
        return Location::new(
            coordinate("latitude")?,
            coordinate("longitude")?,
        )
        .map_err(|error| error.to_string());
    }

    scalar_text(value)
        .ok_or_else(|| "expected 'latitude,longitude'".to_string())?
        .parse::<Location>()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::markdown_composer::compose_markdown;
    use crate::usecase::property_value_adapter::property_value_command;
    use chrono::Utc;
    use database_manager::domain::{
        Data, DataId, DatabaseId, PropertyData, PropertyId, SelectItem,
        SelectItemId, TypeMultiSelect, TypeRelation, TypeSelect,
    };
    use value_object::TenantId;

    struct Fixture {
        tenant_id: TenantId,
        database_id: DatabaseId,
        properties: Vec<Property>,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                tenant_id: TenantId::default(),
                database_id: DatabaseId::default(),
                properties: vec![],
            }
        }

        fn property(
            &mut self,
            name: &str,
            property_type: PropertyType,
        ) -> Property {
            let property = Property::new(
                &PropertyId::default(),
                &self.tenant_id,
                &self.database_id,
                name,
                &property_type,
                false,
                self.properties.len() as u32,
            );
            self.properties.push(property.clone());
            property
        }

        fn data(
            &self,
            name: &str,
            property_data: Vec<PropertyData>,
        ) -> Data {
            Data::new(
                &DataId::default(),
                &self.tenant_id,
                &self.database_id,
                name,
                property_data,
                Utc::now(),
                Utc::now(),
            )
            .expect("fixture data should be valid")
        }
    }

    fn value_for<'a>(
        mutation: &'a MarkdownMutation,
        property: &Property,
    ) -> Option<&'a PropertyDataValueInputData> {
        mutation
            .property_data
            .iter()
            .find(|entry| entry.property_id == property.id().to_string())
            .map(|entry| &entry.value)
    }

    #[test]
    fn frontmatter_names_properties_and_the_body_fills_the_body_property() {
        let mut fixture = Fixture::new();
        let slug = fixture.property("slug", PropertyType::String);
        let content = fixture.property("content", PropertyType::Markdown);

        let mutation = read_markdown_mutation(
            "---\ntitle: Release note\nslug: v1-shipped\n---\n\n# Body\n\nHello\n",
            &fixture.properties,
        )
        .expect("document should read");

        assert_eq!(mutation.title.as_deref(), Some("Release note"));
        assert!(mutation.warnings.is_empty());
        assert!(
            matches!(value_for(&mutation, &slug), Some(PropertyDataValueInputData::String(text)) if text == "v1-shipped")
        );
        assert!(
            matches!(value_for(&mutation, &content), Some(PropertyDataValueInputData::Markdown(body)) if body == "# Body\n\nHello")
        );
    }

    #[test]
    fn an_unknown_key_is_skipped_with_a_warning_and_the_rest_applies() {
        let mut fixture = Fixture::new();
        let slug = fixture.property("slug", PropertyType::String);

        let mutation = read_markdown_mutation(
            "---\nslug: v1\nreviewer: someone\n---\n\nBody\n",
            &fixture.properties,
        )
        .expect("an unknown key must not fail the write");

        assert!(value_for(&mutation, &slug).is_some());
        assert_eq!(mutation.warnings.len(), 2);
        assert_eq!(mutation.warnings[0].key, "reviewer");
        // No body property in this repository, so the body is reported too.
        assert_eq!(mutation.warnings[1].key, "<body>");
    }

    #[test]
    fn reserved_keys_are_dropped_without_a_warning() {
        let fixture = Fixture::new();

        let mutation = read_markdown_mutation(
            "---\nid: data_01example\ntitle: Note\nurl: https://example.test/note\n---\n",
            &fixture.properties,
        )
        .expect("document should read");

        assert!(mutation.property_data.is_empty());
        assert!(mutation.warnings.is_empty());
        assert_eq!(mutation.title.as_deref(), Some("Note"));
    }

    #[test]
    fn an_empty_body_leaves_the_body_property_alone() {
        let mut fixture = Fixture::new();
        let content = fixture.property("content", PropertyType::RichText);

        let mutation = read_markdown_mutation(
            "---\ntitle: Note\n---\n\n   \n",
            &fixture.properties,
        )
        .expect("document should read");

        assert!(value_for(&mutation, &content).is_none());
        assert!(mutation.warnings.is_empty());
    }

    #[test]
    fn a_rich_text_body_is_converted_from_markdown() {
        let mut fixture = Fixture::new();
        let content = fixture.property("content", PropertyType::RichText);

        let mutation = read_markdown_mutation(
            "---\ntitle: Note\n---\n\n# Heading\n",
            &fixture.properties,
        )
        .expect("document should read");

        let Some(PropertyDataValueInputData::RichText(document)) =
            value_for(&mutation, &content)
        else {
            panic!("rich text body must become a block document");
        };
        assert!(document.starts_with('['), "{document}");
        assert!(document.contains("heading"), "{document}");
    }

    /// Only one rich text property can hold the body, so a second one is
    /// written from frontmatter -- still as Markdown, not as a block
    /// document the caller had to build.
    #[test]
    fn a_rich_text_property_in_frontmatter_also_takes_markdown() {
        let mut fixture = Fixture::new();
        let content = fixture.property("content", PropertyType::RichText);
        let summary = fixture.property("summary", PropertyType::RichText);

        let mutation = read_markdown_mutation(
            "---\nsummary: |\n  # Summary\n\n  One line.\n---\n\n# Body\n",
            &fixture.properties,
        )
        .expect("document should read");

        let Some(PropertyDataValueInputData::RichText(document)) =
            value_for(&mutation, &summary)
        else {
            panic!("a rich text property must take Markdown text");
        };
        assert!(document.contains("heading"), "{document}");
        assert!(document.contains("Summary"), "{document}");
        assert!(matches!(
            value_for(&mutation, &content),
            Some(PropertyDataValueInputData::RichText(_))
        ));
        assert!(mutation.warnings.is_empty(), "{:?}", mutation.warnings);
    }

    #[test]
    fn frontmatter_may_not_write_the_property_the_body_owns() {
        let mut fixture = Fixture::new();
        let content = fixture.property("content", PropertyType::Markdown);

        let mutation = read_markdown_mutation(
            "---\ncontent: from frontmatter\n---\n\nfrom body\n",
            &fixture.properties,
        )
        .expect("document should read");

        assert!(
            matches!(value_for(&mutation, &content), Some(PropertyDataValueInputData::Markdown(body)) if body == "from body")
        );
        assert_eq!(mutation.warnings.len(), 1);
        assert_eq!(mutation.warnings[0].key, "content");
    }

    #[test]
    fn a_select_option_written_by_key_resolves_to_its_id() {
        let mut fixture = Fixture::new();
        let option = SelectItemId::default();
        let status = fixture.property(
            "status",
            PropertyType::Select(TypeSelect::new(vec![SelectItem::new(
                option.clone(),
                "shipped".parse().unwrap(),
                "Shipped".parse().unwrap(),
            )])),
        );

        let mutation = read_markdown_mutation(
            "---\nstatus: shipped\n---\n",
            &fixture.properties,
        )
        .expect("document should read");

        assert!(
            matches!(value_for(&mutation, &status), Some(PropertyDataValueInputData::Select(id)) if id == &option.to_string())
        );
    }

    #[test]
    fn a_value_the_property_cannot_hold_fails_the_write() {
        let mut fixture = Fixture::new();
        fixture
            .property("spot", PropertyType::Location(Default::default()));

        let error = read_markdown_mutation(
            "---\nspot: nowhere\n---\n",
            &fixture.properties,
        )
        .expect_err("a bad value must not be written as something else");

        assert!(error.to_string().contains("spot"), "{error}");
    }

    #[test]
    fn a_document_without_frontmatter_is_all_body() {
        let mut fixture = Fixture::new();
        let content = fixture.property("content", PropertyType::Markdown);

        let mutation = read_markdown_mutation(
            "# Heading\n\n---\n",
            &fixture.properties,
        )
        .expect("document should read");

        assert!(mutation.title.is_none());
        assert!(
            matches!(value_for(&mutation, &content), Some(PropertyDataValueInputData::Markdown(body)) if body == "# Heading\n\n---")
        );
    }

    /// `ext_github` is stored as JSON text and composed as a YAML mapping.
    /// Reading it back as a scalar would blank the sync envelope.
    #[test]
    fn a_json_envelope_survives_the_round_trip_as_json() {
        let mut fixture = Fixture::new();
        let envelope = serde_json::json!({
            "repo": "quantum-box/library",
            "path": "docs/note.md",
            "sync_to_github": true
        });
        let ext = fixture.property("ext_github", PropertyType::String);
        let content = fixture.property("content", PropertyType::Markdown);
        let data = fixture.data(
            "Note",
            vec![
                PropertyData::new(&ext, envelope.to_string()).unwrap(),
                PropertyData::new(&content, "Body".to_string()).unwrap(),
            ],
        );

        let document = compose_markdown(&data, &fixture.properties);
        let mutation =
            read_markdown_mutation(&document, &fixture.properties)
                .expect("a composed document must read back");

        let Some(PropertyDataValueInputData::String(text)) =
            value_for(&mutation, &ext)
        else {
            panic!("the envelope must be written back as text");
        };
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(text).unwrap(),
            envelope
        );
    }

    #[test]
    fn a_composed_document_reads_back_into_the_values_it_came_from() {
        let mut fixture = Fixture::new();
        let target_database = DatabaseId::default();
        let option = SelectItemId::default();
        let tag = SelectItemId::default();
        let related = DataId::default();
        let item = |id: &SelectItemId| {
            SelectItem::new(
                id.clone(),
                "option".parse().unwrap(),
                "Option".parse().unwrap(),
            )
        };

        let typed = vec![
            (fixture.property("slug", PropertyType::String), "v1-shipped"),
            (fixture.property("count", PropertyType::Integer), "3"),
            (fixture.property("done", PropertyType::Boolean), "true"),
            (fixture.property("due", PropertyType::Date), "2026-09-07"),
            (
                fixture.property("cover", PropertyType::Image),
                "https://example.test/cover.png",
            ),
            (
                fixture.property(
                    "spot",
                    PropertyType::Location(Default::default()),
                ),
                "35.1,139.2",
            ),
            (
                fixture.property(
                    "status",
                    PropertyType::Select(TypeSelect::new(vec![item(
                        &option,
                    )])),
                ),
                option.as_str(),
            ),
            (
                fixture.property(
                    "tags",
                    PropertyType::MultiSelect(TypeMultiSelect::new(vec![
                        item(&tag),
                    ])),
                ),
                tag.as_str(),
            ),
            (
                fixture.property(
                    "parent",
                    PropertyType::Relation(TypeRelation::new(
                        target_database.clone(),
                    )),
                ),
                related.as_str(),
            ),
            (
                fixture.property("content", PropertyType::Markdown),
                "# Body\n\nHello Library",
            ),
        ];

        let property_data = typed
            .iter()
            .map(|(property, text)| {
                PropertyData::new(property, (*text).to_string())
                    .expect("fixture value should be valid")
            })
            .collect::<Vec<_>>();
        let data = fixture.data("Release note", property_data.clone());

        let document = compose_markdown(&data, &fixture.properties);
        let mutation =
            read_markdown_mutation(&document, &fixture.properties)
                .expect("a composed document must read back");

        assert_eq!(mutation.title.as_deref(), Some("Release note"));
        assert!(mutation.warnings.is_empty(), "{:?}", mutation.warnings);
        for (property, _) in &typed {
            let input =
                value_for(&mutation, property).unwrap_or_else(|| {
                    panic!("{} was dropped", property.name())
                });
            let command = property_value_command(property, input)
                .unwrap_or_else(|error| {
                    panic!("{}: {error}", property.name())
                });
            let restored = PropertyData::from_command(property, command)
                .expect("restored value should be valid");
            let original = property_data
                .iter()
                .find(|entry| entry.property_id() == property.id())
                .expect("fixture value");
            assert_eq!(
                restored.value(),
                original.value(),
                "{}",
                property.name()
            );
        }
    }
}
