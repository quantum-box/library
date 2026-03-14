//! Property mapping configuration for transforming external data to Library properties.

use serde::{Deserialize, Serialize};

use crate::ConflictResolutionStrategy;

/// Configuration for conflict resolution behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionConfig {
    /// Strategy for resolving conflicts when data is modified in both places.
    #[serde(default)]
    pub strategy: ConflictResolutionStrategy,

    /// Whether to create a backup version before overwriting.
    #[serde(default)]
    pub create_backup: bool,

    /// Whether to send notifications on conflict detection.
    #[serde(default)]
    pub notify_on_conflict: bool,
}

impl Default for ConflictResolutionConfig {
    fn default() -> Self {
        Self {
            strategy: ConflictResolutionStrategy::LastWriteWins,
            create_backup: false,
            notify_on_conflict: false,
        }
    }
}

/// Property mapping configuration.
///
/// Defines how external data fields map to Library properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyMapping {
    /// Target Library repository ID
    #[serde(default)]
    pub target_repository_id: Option<String>,

    /// Static field mappings (source_field -> target_property)
    #[serde(default)]
    pub static_mappings: Vec<FieldMapping>,

    /// Computed/dynamic mappings
    #[serde(default)]
    pub computed_mappings: Vec<ComputedMapping>,

    /// Default values for unmapped properties
    #[serde(default)]
    pub defaults: serde_json::Map<String, serde_json::Value>,

    /// Conflict resolution configuration
    #[serde(default)]
    pub conflict_resolution: ConflictResolutionConfig,
}

impl Default for PropertyMapping {
    fn default() -> Self {
        Self {
            target_repository_id: None,
            static_mappings: Vec::new(),
            computed_mappings: Vec::new(),
            defaults: serde_json::Map::new(),
            conflict_resolution: ConflictResolutionConfig::default(),
        }
    }
}

/// A single field mapping from source to target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    /// Source field path (e.g., "frontmatter.title", "properties.Name")
    pub source_field: String,

    /// Target property name in Library
    pub target_property: String,

    /// Optional transformation to apply
    #[serde(default)]
    pub transform: Option<Transform>,
}

/// A computed mapping that derives a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedMapping {
    /// Target property name in Library
    pub target_property: String,

    /// Expression to compute the value
    pub expression: String,
}

/// Transformation functions for field values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transform {
    /// Split string by delimiter into array
    SplitComma,
    /// Split string by custom delimiter
    Split { delimiter: String },
    /// Convert to lowercase
    Lowercase,
    /// Convert to uppercase
    Uppercase,
    /// Slugify string (for URLs)
    Slugify,
    /// Parse as JSON
    ParseJson,
    /// Extract date from timestamp
    ToDate,
    /// Extract time from timestamp
    ToTime,
    /// Convert boolean string to boolean
    ToBool,
    /// Convert numeric string to number
    ToNumber,
    /// Trim whitespace
    Trim,
    /// Custom regex replacement
    Regex {
        pattern: String,
        replacement: String,
    },
    /// Convert cents to dollars (divide by 100)
    CentsToDollars,
    /// Convert Unix timestamp to ISO date string
    UnixToIso,
}

impl PropertyMapping {
    /// Create a new empty property mapping.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a static field mapping.
    pub fn add_mapping(
        mut self,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        self.static_mappings.push(FieldMapping {
            source_field: source.into(),
            target_property: target.into(),
            transform: None,
        });
        self
    }

    /// Add a static field mapping with transformation.
    pub fn add_mapping_with_transform(
        mut self,
        source: impl Into<String>,
        target: impl Into<String>,
        transform: Transform,
    ) -> Self {
        self.static_mappings.push(FieldMapping {
            source_field: source.into(),
            target_property: target.into(),
            transform: Some(transform),
        });
        self
    }

    /// Add a computed mapping.
    pub fn add_computed(
        mut self,
        target: impl Into<String>,
        expression: impl Into<String>,
    ) -> Self {
        self.computed_mappings.push(ComputedMapping {
            target_property: target.into(),
            expression: expression.into(),
        });
        self
    }

    /// Add a default value.
    pub fn add_default(
        mut self,
        property: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.defaults.insert(property.into(), value);
        self
    }

    /// Set target repository ID.
    pub fn with_target_repository(
        mut self,
        repository_id: impl Into<String>,
    ) -> Self {
        self.target_repository_id = Some(repository_id.into());
        self
    }
}

/// Result of applying property mapping to external data.
#[derive(Debug, Clone, Default)]
pub struct MappedData {
    /// Mapped properties
    pub properties: serde_json::Map<String, serde_json::Value>,
    /// Content/body (if any)
    pub content: Option<String>,
    /// External ID
    pub external_id: Option<String>,
    /// External version/ETag
    pub external_version: Option<String>,
}

impl MappedData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_property(
        mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn with_external_id(mut self, id: impl Into<String>) -> Self {
        self.external_id = Some(id.into());
        self
    }

    pub fn with_external_version(
        mut self,
        version: impl Into<String>,
    ) -> Self {
        self.external_version = Some(version.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_mapping_builder() {
        let mapping = PropertyMapping::new()
            .with_target_repository("repo_123")
            .add_mapping("frontmatter.title", "title")
            .add_mapping_with_transform(
                "frontmatter.tags",
                "tags",
                Transform::SplitComma,
            )
            .add_computed("slug", "slugify(source.title)")
            .add_default("status", serde_json::json!("draft"));

        assert_eq!(
            mapping.target_repository_id,
            Some("repo_123".to_string())
        );
        assert_eq!(mapping.static_mappings.len(), 2);
        assert_eq!(mapping.computed_mappings.len(), 1);
        assert_eq!(mapping.defaults.len(), 1);
    }

    #[test]
    fn test_mapped_data() {
        let data = MappedData::new()
            .with_property("title", serde_json::json!("Hello World"))
            .with_content("# Hello\n\nWorld")
            .with_external_id("gh_123")
            .with_external_version("abc123");

        assert_eq!(data.properties.len(), 1);
        assert!(data.content.is_some());
        assert_eq!(data.external_id, Some("gh_123".to_string()));
    }
}
