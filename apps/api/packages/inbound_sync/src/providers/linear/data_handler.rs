//! Linear data handler implementation for Library data operations.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

use inbound_sync_domain::{PropertyMapping, Transform, WebhookEndpoint};

use super::event_processor::LinearDataHandler;
use super::payload::{Issue, Project};
use crate::providers::github::LibraryDataRepository;

/// Default implementation of LinearDataHandler.
#[derive(Debug)]
pub struct DefaultLinearDataHandler {
    data_repo: Arc<dyn LibraryDataRepository>,
}

impl DefaultLinearDataHandler {
    /// Create a new handler with a data repository.
    pub fn new(data_repo: Arc<dyn LibraryDataRepository>) -> Self {
        Self { data_repo }
    }

    /// Map Linear issue properties to Library properties.
    fn map_issue_properties(
        issue: &Issue,
        mapping: Option<&PropertyMapping>,
    ) -> HashMap<String, JsonValue> {
        let mut properties = HashMap::new();

        // Default mappings if no custom mapping provided
        if mapping.is_none() {
            properties.insert(
                "identifier".to_string(),
                JsonValue::String(issue.identifier.clone()),
            );
            properties.insert(
                "title".to_string(),
                JsonValue::String(issue.title.clone()),
            );
            if let Some(desc) = &issue.description {
                properties.insert(
                    "description".to_string(),
                    JsonValue::String(desc.clone()),
                );
            }
            if let Some(state) = &issue.state {
                properties.insert(
                    "status".to_string(),
                    JsonValue::String(state.name.clone()),
                );
            }
            if let Some(priority) = issue.priority {
                properties.insert(
                    "priority".to_string(),
                    JsonValue::String(issue.priority_label().to_string()),
                );
                properties.insert(
                    "priority_value".to_string(),
                    JsonValue::Number(priority.into()),
                );
            }
            if let Some(assignee) = &issue.assignee {
                properties.insert(
                    "assignee".to_string(),
                    JsonValue::String(assignee.name.clone()),
                );
                if let Some(email) = &assignee.email {
                    properties.insert(
                        "assignee_email".to_string(),
                        JsonValue::String(email.clone()),
                    );
                }
            }
            if let Some(team) = &issue.team {
                properties.insert(
                    "team".to_string(),
                    JsonValue::String(team.name.clone()),
                );
            }
            if let Some(project) = &issue.project {
                properties.insert(
                    "project".to_string(),
                    JsonValue::String(project.name.clone()),
                );
            }
            if !issue.labels.is_empty() {
                let labels: Vec<JsonValue> = issue
                    .labels
                    .iter()
                    .map(|l| JsonValue::String(l.name.clone()))
                    .collect();
                properties
                    .insert("labels".to_string(), JsonValue::Array(labels));
            }
            if let Some(due_date) = &issue.due_date {
                properties.insert(
                    "due_date".to_string(),
                    JsonValue::String(due_date.clone()),
                );
            }
            if let Some(estimate) = issue.estimate {
                properties.insert(
                    "estimate".to_string(),
                    serde_json::Number::from_f64(estimate)
                        .map(JsonValue::Number)
                        .unwrap_or(JsonValue::Null),
                );
            }
            properties.insert(
                "created_at".to_string(),
                JsonValue::String(issue.created_at.clone()),
            );
            properties.insert(
                "updated_at".to_string(),
                JsonValue::String(issue.updated_at.clone()),
            );
            if let Some(url) = &issue.url {
                properties.insert(
                    "url".to_string(),
                    JsonValue::String(url.clone()),
                );
            }

            return properties;
        }

        // Apply custom mapping
        let mapping = mapping.unwrap();
        let issue_json =
            serde_json::to_value(issue).unwrap_or(JsonValue::Null);

        for field_mapping in &mapping.static_mappings {
            if let Some(value) =
                get_nested_value(&issue_json, &field_mapping.source_field)
            {
                let transformed =
                    if let Some(transform) = &field_mapping.transform {
                        apply_transform(&value, transform)
                    } else {
                        value
                    };
                properties.insert(
                    field_mapping.target_property.clone(),
                    transformed,
                );
            }
        }

        // Apply defaults
        for (key, value) in &mapping.defaults {
            if !properties.contains_key(key) {
                properties.insert(key.clone(), value.clone());
            }
        }

        properties
    }

    /// Map Linear project properties to Library properties.
    fn map_project_properties(
        project: &Project,
        mapping: Option<&PropertyMapping>,
    ) -> HashMap<String, JsonValue> {
        let mut properties = HashMap::new();

        // Default mappings
        if mapping.is_none() {
            properties.insert(
                "name".to_string(),
                JsonValue::String(project.name.clone()),
            );
            if let Some(desc) = &project.description {
                properties.insert(
                    "description".to_string(),
                    JsonValue::String(desc.clone()),
                );
            }
            if let Some(state) = &project.state {
                properties.insert(
                    "status".to_string(),
                    JsonValue::String(state.clone()),
                );
            }
            if let Some(lead) = &project.lead {
                properties.insert(
                    "lead".to_string(),
                    JsonValue::String(lead.name.clone()),
                );
            }
            if let Some(target_date) = &project.target_date {
                properties.insert(
                    "target_date".to_string(),
                    JsonValue::String(target_date.clone()),
                );
            }
            if let Some(start_date) = &project.start_date {
                properties.insert(
                    "start_date".to_string(),
                    JsonValue::String(start_date.clone()),
                );
            }
            properties.insert(
                "created_at".to_string(),
                JsonValue::String(project.created_at.clone()),
            );
            properties.insert(
                "updated_at".to_string(),
                JsonValue::String(project.updated_at.clone()),
            );
            if let Some(url) = &project.url {
                properties.insert(
                    "url".to_string(),
                    JsonValue::String(url.clone()),
                );
            }

            return properties;
        }

        // Apply custom mapping
        let mapping = mapping.unwrap();
        let project_json =
            serde_json::to_value(project).unwrap_or(JsonValue::Null);

        for field_mapping in &mapping.static_mappings {
            if let Some(value) =
                get_nested_value(&project_json, &field_mapping.source_field)
            {
                let transformed =
                    if let Some(transform) = &field_mapping.transform {
                        apply_transform(&value, transform)
                    } else {
                        value
                    };
                properties.insert(
                    field_mapping.target_property.clone(),
                    transformed,
                );
            }
        }

        // Apply defaults
        for (key, value) in &mapping.defaults {
            if !properties.contains_key(key) {
                properties.insert(key.clone(), value.clone());
            }
        }

        properties
    }
}

/// Get a nested value from JSON using dot notation.
fn get_nested_value(json: &JsonValue, path: &str) -> Option<JsonValue> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json.clone();

    for part in parts {
        match current {
            JsonValue::Object(map) => {
                current = map.get(part)?.clone();
            }
            _ => return None,
        }
    }

    Some(current)
}

/// Apply a transform function to a value.
fn apply_transform(value: &JsonValue, transform: &Transform) -> JsonValue {
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
        _ => value.clone(),
    }
}

#[async_trait]
impl LinearDataHandler for DefaultLinearDataHandler {
    async fn upsert_issue(
        &self,
        endpoint: &WebhookEndpoint,
        issue: &Issue,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let mut properties = Self::map_issue_properties(issue, mapping);

        // Add ext_linear property for Linear integration tracking
        let ext_linear = serde_json::json!({
            "issue_id": issue.id,
            "issue_url": issue.url,
            "identifier": issue.identifier,
            "sync_enabled": true,
            "last_synced_at": chrono::Utc::now().to_rfc3339(),
            "version_external": issue.updated_at,
        });
        properties.insert("ext_linear".to_string(), ext_linear);

        let content = issue.description.clone().unwrap_or_default();

        // Check for existing data
        let external_id = format!("linear:issue:{}", issue.id);
        let existing = self
            .data_repo
            .find_by_external_id(endpoint, &external_id)
            .await?;

        if let Some(data_id) = existing {
            self.data_repo
                .update_data(
                    endpoint,
                    &data_id,
                    &issue.title,
                    &content,
                    properties,
                )
                .await?;
            Ok(data_id)
        } else {
            self.data_repo
                .create_data(endpoint, &issue.title, &content, properties)
                .await
        }
    }

    async fn delete_issue(
        &self,
        endpoint: &WebhookEndpoint,
        data_id: &str,
    ) -> errors::Result<()> {
        self.data_repo.delete_data(endpoint, data_id).await
    }

    async fn upsert_project(
        &self,
        endpoint: &WebhookEndpoint,
        project: &Project,
        mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        let mut properties = Self::map_project_properties(project, mapping);

        // Add ext_linear property for Linear integration tracking
        let ext_linear = serde_json::json!({
            "project_id": project.id,
            "project_url": project.url,
            "sync_enabled": true,
            "last_synced_at": chrono::Utc::now().to_rfc3339(),
            "version_external": project.updated_at,
        });
        properties.insert("ext_linear".to_string(), ext_linear);

        let content = project.description.clone().unwrap_or_default();

        // Check for existing data
        let external_id = format!("linear:project:{}", project.id);
        let existing = self
            .data_repo
            .find_by_external_id(endpoint, &external_id)
            .await?;

        if let Some(data_id) = existing {
            self.data_repo
                .update_data(
                    endpoint,
                    &data_id,
                    &project.name,
                    &content,
                    properties,
                )
                .await?;
            Ok(data_id)
        } else {
            self.data_repo
                .create_data(endpoint, &project.name, &content, properties)
                .await
        }
    }

    async fn delete_project(
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
    use crate::providers::linear::payload::{IssueState, Label, User};

    fn create_test_issue() -> Issue {
        Issue {
            id: "issue-123".to_string(),
            identifier: "ENG-456".to_string(),
            title: "Fix bug".to_string(),
            description: Some("Description here".to_string()),
            priority: Some(2),
            state: Some(IssueState {
                id: "state-1".to_string(),
                name: "In Progress".to_string(),
                color: None,
                state_type: None,
            }),
            assignee: Some(User {
                id: "user-1".to_string(),
                name: "John Doe".to_string(),
                email: Some("john@example.com".to_string()),
            }),
            creator: None,
            team: None,
            project: None,
            cycle: None,
            labels: vec![
                Label {
                    id: "label-1".to_string(),
                    name: "bug".to_string(),
                    color: None,
                },
                Label {
                    id: "label-2".to_string(),
                    name: "urgent".to_string(),
                    color: None,
                },
            ],
            due_date: None,
            estimate: Some(3.0),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-02T00:00:00Z".to_string(),
            completed_at: None,
            canceled_at: None,
            url: Some("https://linear.app/team/ENG-456".to_string()),
        }
    }

    #[test]
    fn test_map_issue_properties_default() {
        let issue = create_test_issue();
        let properties =
            DefaultLinearDataHandler::map_issue_properties(&issue, None);

        assert_eq!(
            properties.get("identifier").unwrap().as_str().unwrap(),
            "ENG-456"
        );
        assert_eq!(
            properties.get("title").unwrap().as_str().unwrap(),
            "Fix bug"
        );
        assert_eq!(
            properties.get("status").unwrap().as_str().unwrap(),
            "In Progress"
        );
        assert_eq!(
            properties.get("priority").unwrap().as_str().unwrap(),
            "High"
        );
        assert_eq!(
            properties.get("assignee").unwrap().as_str().unwrap(),
            "John Doe"
        );

        // Check labels array
        let labels = properties.get("labels").unwrap().as_array().unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].as_str().unwrap(), "bug");
    }

    #[test]
    fn test_get_nested_value() {
        let json = serde_json::json!({
            "state": {
                "name": "In Progress",
                "type": "started"
            },
            "assignee": {
                "name": "John"
            }
        });

        assert_eq!(
            get_nested_value(&json, "state.name")
                .unwrap()
                .as_str()
                .unwrap(),
            "In Progress"
        );
        assert_eq!(
            get_nested_value(&json, "assignee.name")
                .unwrap()
                .as_str()
                .unwrap(),
            "John"
        );
        assert!(get_nested_value(&json, "nonexistent.field").is_none());
    }
}
