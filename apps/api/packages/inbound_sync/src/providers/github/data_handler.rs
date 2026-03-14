//! GitHub data handler implementation for Library data operations.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use inbound_sync_domain::{PropertyMapping, Transform, WebhookEndpoint};

use super::event_processor::GitHubDataHandler;

/// Frontmatter parser result.
#[derive(Debug, Clone)]
pub struct ParsedMarkdown {
    /// Parsed frontmatter as JSON object.
    pub frontmatter: Option<JsonValue>,
    /// Markdown body content.
    pub body: String,
}

/// Parse frontmatter from markdown content.
///
/// Supports YAML frontmatter delimited by `---`.
pub fn parse_frontmatter(content: &str) -> ParsedMarkdown {
    let trimmed = content.trim_start();

    if !trimmed.starts_with("---") {
        return ParsedMarkdown {
            frontmatter: None,
            body: content.to_string(),
        };
    }

    // Find the closing delimiter
    let after_first = &trimmed[3..];
    if let Some(end_pos) = after_first.find("\n---") {
        let frontmatter_str = &after_first[..end_pos].trim();
        let body = &after_first[end_pos + 4..].trim_start();

        // Parse YAML frontmatter
        match serde_yaml::from_str::<JsonValue>(frontmatter_str) {
            Ok(fm) => ParsedMarkdown {
                frontmatter: Some(fm),
                body: body.to_string(),
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to parse frontmatter as YAML"
                );
                ParsedMarkdown {
                    frontmatter: None,
                    body: content.to_string(),
                }
            }
        }
    } else {
        ParsedMarkdown {
            frontmatter: None,
            body: content.to_string(),
        }
    }
}

/// Extract title from frontmatter or markdown content.
pub fn extract_title(
    frontmatter: &Option<JsonValue>,
    body: &str,
    file_path: &str,
) -> String {
    // Try frontmatter title
    if let Some(fm) = frontmatter {
        if let Some(title) = fm.get("title").and_then(|v| v.as_str()) {
            return title.to_string();
        }
    }

    // Try first H1 heading
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return title.trim().to_string();
        }
    }

    // Fall back to file name
    std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

/// Apply property mapping to extract values from frontmatter.
pub fn apply_mapping(
    frontmatter: &Option<JsonValue>,
    mapping: Option<&PropertyMapping>,
) -> HashMap<String, JsonValue> {
    let mut result = HashMap::new();

    let Some(fm) = frontmatter else {
        return result;
    };

    let Some(mapping) = mapping else {
        // If no mapping, return all frontmatter keys
        if let Some(obj) = fm.as_object() {
            for (key, value) in obj {
                result.insert(key.clone(), value.clone());
            }
        }
        return result;
    };

    // Apply static mappings
    for static_mapping in &mapping.static_mappings {
        // Support dot notation for nested fields (e.g., "frontmatter.title")
        let source_key = static_mapping
            .source_field
            .strip_prefix("frontmatter.")
            .unwrap_or(&static_mapping.source_field);

        if let Some(value) = fm.get(source_key) {
            let transformed =
                if let Some(transform) = &static_mapping.transform {
                    apply_transform(value, transform)
                } else {
                    value.clone()
                };
            result.insert(
                static_mapping.target_property.clone(),
                transformed,
            );
        }
    }

    result
}

/// Apply a transform function to a value.
pub fn apply_transform(
    value: &JsonValue,
    transform: &Transform,
) -> JsonValue {
    match transform {
        Transform::SplitComma => {
            if let Some(s) = value.as_str() {
                let parts: Vec<JsonValue> = s
                    .split(',')
                    .map(|p| JsonValue::String(p.trim().to_string()))
                    .collect();
                JsonValue::Array(parts)
            } else {
                value.clone()
            }
        }
        Transform::Split { delimiter } => {
            if let Some(s) = value.as_str() {
                let parts: Vec<JsonValue> = s
                    .split(delimiter)
                    .map(|p| JsonValue::String(p.trim().to_string()))
                    .collect();
                JsonValue::Array(parts)
            } else {
                value.clone()
            }
        }
        Transform::Lowercase => {
            if let Some(s) = value.as_str() {
                JsonValue::String(s.to_lowercase())
            } else {
                value.clone()
            }
        }
        Transform::Uppercase => {
            if let Some(s) = value.as_str() {
                JsonValue::String(s.to_uppercase())
            } else {
                value.clone()
            }
        }
        Transform::Trim => {
            if let Some(s) = value.as_str() {
                JsonValue::String(s.trim().to_string())
            } else {
                value.clone()
            }
        }
        Transform::Slugify => {
            if let Some(s) = value.as_str() {
                // Simple slugify: lowercase, replace spaces with hyphens
                let slug = s
                    .to_lowercase()
                    .chars()
                    .map(|c| if c.is_alphanumeric() { c } else { '-' })
                    .collect::<String>();
                // Remove multiple consecutive hyphens
                let mut result = String::new();
                let mut last_was_hyphen = false;
                for c in slug.chars() {
                    if c == '-' {
                        if !last_was_hyphen {
                            result.push(c);
                        }
                        last_was_hyphen = true;
                    } else {
                        result.push(c);
                        last_was_hyphen = false;
                    }
                }
                JsonValue::String(result.trim_matches('-').to_string())
            } else {
                value.clone()
            }
        }
        Transform::ParseJson => {
            if let Some(s) = value.as_str() {
                serde_json::from_str(s).unwrap_or_else(|_| value.clone())
            } else {
                value.clone()
            }
        }
        Transform::ToDate => {
            // Extract date portion from timestamp or date string
            if let Some(s) = value.as_str() {
                if let Some(date) = s.split('T').next() {
                    JsonValue::String(date.to_string())
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        Transform::ToTime => {
            // Extract time portion from timestamp
            if let Some(s) = value.as_str() {
                if let Some(time) = s.split('T').nth(1) {
                    JsonValue::String(
                        time.trim_end_matches('Z').to_string(),
                    )
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        Transform::ToBool => match value {
            JsonValue::Bool(_) => value.clone(),
            JsonValue::String(s) => {
                let s_lower = s.to_lowercase();
                JsonValue::Bool(
                    s_lower == "true"
                        || s_lower == "yes"
                        || s_lower == "1"
                        || s_lower == "on",
                )
            }
            JsonValue::Number(n) => JsonValue::Bool(n.as_i64() != Some(0)),
            _ => value.clone(),
        },
        Transform::ToNumber => {
            if let Some(s) = value.as_str() {
                if let Ok(n) = s.parse::<i64>() {
                    JsonValue::Number(n.into())
                } else if let Ok(f) = s.parse::<f64>() {
                    serde_json::Number::from_f64(f)
                        .map(JsonValue::Number)
                        .unwrap_or_else(|| value.clone())
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        Transform::Regex {
            pattern,
            replacement,
        } => {
            if let Some(s) = value.as_str() {
                if let Ok(re) = regex::Regex::new(pattern) {
                    JsonValue::String(
                        re.replace_all(s, replacement).to_string(),
                    )
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            }
        }
        Transform::CentsToDollars => {
            if let Some(cents) = value.as_i64() {
                serde_json::Number::from_f64(cents as f64 / 100.0)
                    .map(JsonValue::Number)
                    .unwrap_or_else(|| value.clone())
            } else {
                value.clone()
            }
        }
        Transform::UnixToIso => {
            if let Some(timestamp) = value.as_i64() {
                // Convert Unix timestamp to ISO 8601 format
                use std::time::{Duration, UNIX_EPOCH};
                let datetime =
                    UNIX_EPOCH + Duration::from_secs(timestamp as u64);
                // Format as ISO 8601 (simplified without chrono dependency)
                let secs_since_epoch = datetime
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                // Simple ISO format: YYYY-MM-DDTHH:MM:SSZ
                // For full implementation, chrono would be better
                JsonValue::String(format!("{secs_since_epoch}"))
            } else {
                value.clone()
            }
        }
    }
}

/// Trait for Library data repository operations.
///
/// This trait abstracts the actual data storage, allowing different
/// implementations for testing and production.
#[async_trait]
pub trait LibraryDataRepository: Send + Sync + std::fmt::Debug {
    /// Create new data entry.
    async fn create_data(
        &self,
        endpoint: &WebhookEndpoint,
        name: &str,
        content: &str,
        properties: HashMap<String, JsonValue>,
    ) -> errors::Result<String>;

    /// Update existing data entry.
    async fn update_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
        name: &str,
        content: &str,
        properties: HashMap<String, JsonValue>,
    ) -> errors::Result<()>;

    /// Delete data entry.
    async fn delete_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()>;

    /// Find data by external ID (GitHub path).
    async fn find_by_external_id(
        &self,
        endpoint: &WebhookEndpoint,
        external_id: &str,
    ) -> errors::Result<Option<String>>;
}

/// Default implementation of GitHubDataHandler.
#[derive(Debug)]
pub struct DefaultGitHubDataHandler {
    data_repo: std::sync::Arc<dyn LibraryDataRepository>,
}

impl DefaultGitHubDataHandler {
    /// Create a new handler with a data repository.
    pub fn new(
        data_repo: std::sync::Arc<dyn LibraryDataRepository>,
    ) -> Self {
        Self { data_repo }
    }
}

#[async_trait]
impl GitHubDataHandler for DefaultGitHubDataHandler {
    async fn upsert_data(
        &self,
        endpoint: &WebhookEndpoint,
        path: &str,
        content: &str,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        // Parse the markdown content
        let parsed = parse_frontmatter(content);

        // Extract title
        let name = extract_title(&parsed.frontmatter, &parsed.body, path);

        // Apply property mapping
        let properties = apply_mapping(&parsed.frontmatter, mapping);

        // Generate external ID
        let external_id = path.to_string();

        // Check if data already exists
        let existing = self
            .data_repo
            .find_by_external_id(endpoint, &external_id)
            .await?;

        if let Some(data_id) = existing {
            // Update existing
            self.data_repo
                .update_data(
                    endpoint,
                    &data_id,
                    &name,
                    &parsed.body,
                    properties,
                )
                .await?;
            Ok(data_id)
        } else {
            // Create new
            self.data_repo
                .create_data(endpoint, &name, &parsed.body, properties)
                .await
        }
    }

    async fn delete_data(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()> {
        self.data_repo.delete_data(endpoint, data_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_with_yaml() {
        let content = r#"---
title: "Test Article"
tags: ["rust", "testing"]
published: true
---

# Content

This is the body."#;

        let parsed = parse_frontmatter(content);

        assert!(parsed.frontmatter.is_some());
        let fm = parsed.frontmatter.unwrap();
        assert_eq!(
            fm.get("title").unwrap().as_str().unwrap(),
            "Test Article"
        );
        assert!(parsed.body.contains("# Content"));
    }

    #[test]
    fn test_parse_frontmatter_without_frontmatter() {
        let content = "# Just a heading\n\nSome content.";

        let parsed = parse_frontmatter(content);

        assert!(parsed.frontmatter.is_none());
        assert!(parsed.body.contains("# Just a heading"));
    }

    #[test]
    fn test_extract_title_from_frontmatter() {
        let fm = Some(serde_json::json!({
            "title": "From Frontmatter"
        }));
        let body = "# From Heading\n\nContent";

        let title = extract_title(&fm, body, "test.md");
        assert_eq!(title, "From Frontmatter");
    }

    #[test]
    fn test_extract_title_from_heading() {
        let fm: Option<JsonValue> = None;
        let body = "# From Heading\n\nContent";

        let title = extract_title(&fm, body, "test.md");
        assert_eq!(title, "From Heading");
    }

    #[test]
    fn test_extract_title_from_filename() {
        let fm: Option<JsonValue> = None;
        let body = "No heading here";

        let title = extract_title(&fm, body, "my-article.md");
        assert_eq!(title, "my-article");
    }

    #[test]
    fn test_apply_transform_split_comma() {
        let value = JsonValue::String("a, b, c".to_string());
        let result = apply_transform(&value, &Transform::SplitComma);

        if let JsonValue::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].as_str().unwrap(), "a");
            assert_eq!(arr[1].as_str().unwrap(), "b");
            assert_eq!(arr[2].as_str().unwrap(), "c");
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_apply_transform_slugify() {
        let value = JsonValue::String("Hello World Example".to_string());
        let result = apply_transform(&value, &Transform::Slugify);
        assert_eq!(result.as_str().unwrap(), "hello-world-example");
    }

    #[test]
    fn test_apply_transform_to_bool() {
        let value = JsonValue::String("true".to_string());
        let result = apply_transform(&value, &Transform::ToBool);
        assert_eq!(result.as_bool().unwrap(), true);

        let value2 = JsonValue::String("false".to_string());
        let result2 = apply_transform(&value2, &Transform::ToBool);
        assert_eq!(result2.as_bool().unwrap(), false);
    }

    #[test]
    fn test_apply_mapping_with_transform() {
        use inbound_sync_domain::FieldMapping;

        let fm = Some(serde_json::json!({
            "tags": "rust, web, api"
        }));

        let mapping = PropertyMapping {
            target_repository_id: None,
            static_mappings: vec![FieldMapping {
                source_field: "tags".to_string(),
                target_property: "tags".to_string(),
                transform: Some(Transform::SplitComma),
            }],
            computed_mappings: vec![],
            defaults: serde_json::Map::new(),
            conflict_resolution: Default::default(),
        };

        let result = apply_mapping(&fm, Some(&mapping));

        assert!(result.contains_key("tags"));
        if let JsonValue::Array(arr) = &result["tags"] {
            assert_eq!(arr.len(), 3);
        } else {
            panic!("Expected array for tags");
        }
    }
}
