//! Markdown composer for data export and sync
//!
//! This module provides functionality to compose Markdown documents
//! with YAML frontmatter from data and properties.

use serde_yaml::{Mapping, Value as YamlValue};
use std::collections::HashMap;

/// Check if property name is "content" (case insensitive)
fn is_content_property_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("content")
}

/// Check if property type is a body type (Markdown or Html)
fn is_body_property_type(
    typ: &database_manager::domain::PropertyType,
) -> bool {
    use database_manager::domain::PropertyType;
    matches!(typ, PropertyType::Markdown | PropertyType::Html)
}

/// Check if property name starts with ext_ prefix (extension fields)
fn is_ext_property(name: &str) -> bool {
    name.starts_with("ext_")
}

/// Convert property value to YAML value
fn property_value_to_yaml(
    value: &Option<database_manager::domain::PropertyDataValue>,
) -> YamlValue {
    use database_manager::domain::PropertyDataValue as V;
    match value {
        Some(V::String(s)) => YamlValue::String(s.clone()),
        Some(V::Integer(i)) => {
            serde_yaml::to_value(i).unwrap_or(YamlValue::Null)
        }
        Some(V::Html(s)) | Some(V::Markdown(s)) => {
            YamlValue::String(s.clone())
        }
        Some(V::Relation(db, ids)) => {
            let mut map = Mapping::new();
            map.insert(
                YamlValue::String("databaseId".into()),
                YamlValue::String(db.to_string()),
            );
            let list: Vec<YamlValue> = ids
                .iter()
                .map(|v| YamlValue::String(v.to_string()))
                .collect();
            map.insert(
                YamlValue::String("dataIds".into()),
                YamlValue::Sequence(list),
            );
            YamlValue::Mapping(map)
        }
        Some(V::Id(s)) => YamlValue::String(s.clone()),
        Some(V::Location(loc)) => YamlValue::String(loc.to_string()),
        Some(V::Select(s)) => YamlValue::String(s.to_string()),
        Some(V::MultiSelect(list)) => YamlValue::Sequence(
            list.iter()
                .map(|v| YamlValue::String(v.to_string()))
                .collect(),
        ),
        Some(V::Date(date)) => YamlValue::String(date.to_string()),
        Some(V::Image(url)) => YamlValue::String(url.clone()),
        None => YamlValue::Null,
    }
}

/// Convert JSON value to YAML value recursively
fn json_to_yaml(json: &serde_json::Value) -> YamlValue {
    match json {
        serde_json::Value::Null => YamlValue::Null,
        serde_json::Value::Bool(b) => YamlValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                YamlValue::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_yaml::to_value(f).unwrap_or(YamlValue::Null)
            } else {
                YamlValue::Null
            }
        }
        serde_json::Value::String(s) => YamlValue::String(s.clone()),
        serde_json::Value::Array(arr) => {
            YamlValue::Sequence(arr.iter().map(json_to_yaml).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = Mapping::new();
            for (k, v) in obj {
                map.insert(YamlValue::String(k.clone()), json_to_yaml(v));
            }
            YamlValue::Mapping(map)
        }
    }
}

/// Pick the body content from data properties
fn pick_body(
    data: &database_manager::domain::Data,
    properties: &[database_manager::domain::Property],
) -> String {
    let property_map: HashMap<_, _> =
        properties.iter().map(|p| (p.id(), p)).collect();

    // 1) content property name priority (markdown > html > string)
    for property_data in data.property_data() {
        let Some(property) = property_map.get(property_data.property_id())
        else {
            continue;
        };
        if !is_content_property_name(property.name().as_str()) {
            continue;
        }
        if let Some(value) = property_data.value() {
            use database_manager::domain::PropertyDataValue as V;
            return match value {
                V::Markdown(s) => s.clone(),
                V::Html(s) => s.clone(),
                V::String(s) => s.clone(),
                _ => value.string_value(),
            };
        }
    }

    // 2) Type priority (Markdown > Html)
    for property_data in data.property_data() {
        let Some(property) = property_map.get(property_data.property_id())
        else {
            continue;
        };
        if property.property_type().to_string() == "MARKDOWN" {
            if let Some(
                database_manager::domain::PropertyDataValue::Markdown(s),
            ) = property_data.value()
            {
                return s.clone();
            }
        }
    }
    for property_data in data.property_data() {
        let Some(property) = property_map.get(property_data.property_id())
        else {
            continue;
        };
        if property.property_type().to_string() == "HTML" {
            if let Some(
                database_manager::domain::PropertyDataValue::Html(s),
            ) = property_data.value()
            {
                return s.clone();
            }
        }
    }

    format!("# {}\n", data.name())
}

/// Build YAML frontmatter from data and properties
fn build_frontmatter(
    data: &database_manager::domain::Data,
    properties: &[database_manager::domain::Property],
) -> Mapping {
    let mut map = Mapping::new();
    map.insert(
        YamlValue::String("id".into()),
        YamlValue::String(data.id().to_string()),
    );
    map.insert(
        YamlValue::String("title".into()),
        YamlValue::String(data.name().to_string()),
    );

    let property_map: HashMap<_, _> =
        properties.iter().map(|p| (p.id(), p)).collect();

    for property_data in data.property_data() {
        let Some(property) = property_map.get(property_data.property_id())
        else {
            continue;
        };

        let name = property.name().as_str();
        if is_content_property_name(name) {
            continue;
        }
        if is_body_property_type(property.property_type()) {
            continue;
        }

        let key = YamlValue::String(name.to_string());

        // Handle ext_ prefixed properties specially - parse as JSON and
        // convert to YAML
        if is_ext_property(name) {
            if let Some(value) = property_data.value() {
                let value_str = value.string_value();
                if let Ok(json) =
                    serde_json::from_str::<serde_json::Value>(&value_str)
                {
                    // For ext_github, check sync_to_github flag
                    // If false or not present, skip including in frontmatter
                    if name == "ext_github" {
                        let sync_to_github = json
                            .get("sync_to_github")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if !sync_to_github {
                            continue;
                        }
                    }
                    map.insert(key, json_to_yaml(&json));
                    continue;
                }
            }
        }

        let value = property_value_to_yaml(property_data.value());
        map.insert(key, value);
    }

    map
}

/// Compose a Markdown document with YAML frontmatter from data and
/// properties.
///
/// This function is used both for the markdown export endpoint and for
/// syncing data to external providers like GitHub.
pub fn compose_markdown(
    data: &database_manager::domain::Data,
    properties: &[database_manager::domain::Property],
) -> String {
    let frontmatter = build_frontmatter(data, properties);
    let fm = serde_yaml::to_string(&frontmatter).unwrap_or_default();
    let body = pick_body(data, properties);
    format!("---\n{fm}---\n\n{body}\n")
}
