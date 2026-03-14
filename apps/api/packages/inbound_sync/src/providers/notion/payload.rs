//! Notion webhook payload types.

use serde::{Deserialize, Serialize};

/// Notion webhook event payload.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionWebhookEvent {
    /// Event type (e.g., "page.created", "page.updated", "page.deleted")
    #[serde(rename = "type")]
    pub event_type: String,
    /// Workspace ID where the event occurred
    pub workspace_id: Option<String>,
    /// Integration ID that received the event
    pub integration_id: Option<String>,
    /// Event data containing the affected object
    pub data: NotionEventData,
    /// Timestamp when the event occurred
    pub timestamp: Option<String>,
}

impl NotionWebhookEvent {
    /// Get the action from the event type.
    pub fn action(&self) -> NotionAction {
        if self.event_type.ends_with(".created") {
            NotionAction::Created
        } else if self.event_type.ends_with(".updated") {
            NotionAction::Updated
        } else if self.event_type.ends_with(".deleted") {
            NotionAction::Deleted
        } else if self.event_type.ends_with(".archived") {
            NotionAction::Archived
        } else if self.event_type.ends_with(".unarchived") {
            NotionAction::Unarchived
        } else {
            NotionAction::Unknown
        }
    }

    /// Get the object type from the event type.
    pub fn object_type(&self) -> NotionObjectType {
        if self.event_type.starts_with("page.") {
            NotionObjectType::Page
        } else if self.event_type.starts_with("database.") {
            NotionObjectType::Database
        } else if self.event_type.starts_with("block.") {
            NotionObjectType::Block
        } else if self.event_type.starts_with("comment.") {
            NotionObjectType::Comment
        } else {
            NotionObjectType::Unknown
        }
    }
}

/// Event data containing the affected object.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionEventData {
    /// The Notion object (page, database, block, etc.)
    pub object: NotionObject,
    /// Parent information for pages in databases
    #[serde(default)]
    pub parent: Option<NotionParent>,
}

/// Notion object (page, database, block).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionObject {
    /// Object type ("page", "database", "block")
    pub object: String,
    /// Unique identifier
    pub id: String,
    /// Creation timestamp
    pub created_time: Option<String>,
    /// Last edited timestamp
    pub last_edited_time: Option<String>,
    /// Created by user
    pub created_by: Option<NotionUser>,
    /// Last edited by user
    pub last_edited_by: Option<NotionUser>,
    /// Whether the object is archived
    #[serde(default)]
    pub archived: bool,
    /// Whether the object is in trash
    #[serde(default)]
    pub in_trash: bool,
    /// Page/Database icon
    pub icon: Option<NotionIcon>,
    /// Page/Database cover
    pub cover: Option<NotionCover>,
    /// Properties (for pages and databases)
    #[serde(default)]
    pub properties: serde_json::Value,
    /// Title (for databases)
    pub title: Option<Vec<NotionRichText>>,
    /// Description (for databases)
    pub description: Option<Vec<NotionRichText>>,
    /// URL to the page/database
    pub url: Option<String>,
    /// Public URL if shared
    pub public_url: Option<String>,
}

impl NotionObject {
    /// Get the object title as plain text.
    pub fn title_plain_text(&self) -> Option<String> {
        // For databases, use the title field
        if let Some(title) = &self.title {
            return Some(
                title
                    .iter()
                    .map(|t| t.plain_text.as_str())
                    .collect::<Vec<_>>()
                    .join(""),
            );
        }

        // For pages, try to get title from properties
        if let Some(props) = self.properties.as_object() {
            // Look for common title property names
            for key in ["title", "Title", "Name", "name"] {
                if let Some(prop) = props.get(key) {
                    if let Some(title_array) = prop.get("title") {
                        if let Some(arr) = title_array.as_array() {
                            return Some(
                                arr.iter()
                                    .filter_map(|t| {
                                        t.get("plain_text")
                                            .and_then(|v| v.as_str())
                                    })
                                    .collect::<Vec<_>>()
                                    .join(""),
                            );
                        }
                    }
                }
            }
        }

        None
    }
}

/// Parent reference for pages.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotionParent {
    /// Page is in a database
    DatabaseId { database_id: String },
    /// Page is nested under another page
    PageId { page_id: String },
    /// Page is at workspace root
    Workspace { workspace: bool },
    /// Block parent
    BlockId { block_id: String },
}

impl NotionParent {
    /// Get the database ID if this is a database parent.
    pub fn database_id(&self) -> Option<&str> {
        match self {
            NotionParent::DatabaseId { database_id } => Some(database_id),
            _ => None,
        }
    }
}

/// Notion user reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionUser {
    /// User object type
    pub object: Option<String>,
    /// User ID
    pub id: String,
    /// User name
    pub name: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// User type ("person" or "bot")
    #[serde(rename = "type")]
    pub user_type: Option<String>,
}

/// Rich text element.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionRichText {
    /// Text type
    #[serde(rename = "type")]
    pub text_type: String,
    /// Plain text content
    pub plain_text: String,
    /// Href if this is a link
    pub href: Option<String>,
    /// Annotations (bold, italic, etc.)
    #[serde(default)]
    pub annotations: NotionAnnotations,
}

/// Text annotations.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NotionAnnotations {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub strikethrough: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub code: bool,
    #[serde(default)]
    pub color: String,
}

/// Page/Database icon.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotionIcon {
    Emoji { emoji: String },
    External { external: NotionFile },
    File { file: NotionFile },
}

/// Page/Database cover.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotionCover {
    External { external: NotionFile },
    File { file: NotionFile },
}

/// File reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionFile {
    pub url: String,
    pub expiry_time: Option<String>,
}

/// Notion action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotionAction {
    Created,
    Updated,
    Deleted,
    Archived,
    Unarchived,
    Unknown,
}

impl NotionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotionAction::Created => "created",
            NotionAction::Updated => "updated",
            NotionAction::Deleted => "deleted",
            NotionAction::Archived => "archived",
            NotionAction::Unarchived => "unarchived",
            NotionAction::Unknown => "unknown",
        }
    }
}

/// Notion object type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotionObjectType {
    Page,
    Database,
    Block,
    Comment,
    Unknown,
}

impl NotionObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotionObjectType::Page => "page",
            NotionObjectType::Database => "database",
            NotionObjectType::Block => "block",
            NotionObjectType::Comment => "comment",
            NotionObjectType::Unknown => "unknown",
        }
    }
}

/// Property value types for Notion pages.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotionPropertyValue {
    Title {
        title: Vec<NotionRichText>,
    },
    RichText {
        rich_text: Vec<NotionRichText>,
    },
    Number {
        number: Option<f64>,
    },
    Select {
        select: Option<NotionSelectOption>,
    },
    MultiSelect {
        multi_select: Vec<NotionSelectOption>,
    },
    Date {
        date: Option<NotionDate>,
    },
    People {
        people: Vec<NotionUser>,
    },
    Files {
        files: Vec<NotionFileObject>,
    },
    Checkbox {
        checkbox: bool,
    },
    Url {
        url: Option<String>,
    },
    Email {
        email: Option<String>,
    },
    PhoneNumber {
        phone_number: Option<String>,
    },
    Formula {
        formula: serde_json::Value,
    },
    Relation {
        relation: Vec<NotionRelation>,
    },
    Rollup {
        rollup: serde_json::Value,
    },
    CreatedTime {
        created_time: String,
    },
    CreatedBy {
        created_by: NotionUser,
    },
    LastEditedTime {
        last_edited_time: String,
    },
    LastEditedBy {
        last_edited_by: NotionUser,
    },
    Status {
        status: Option<NotionStatusOption>,
    },
    #[serde(other)]
    Unknown,
}

/// Select option.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionSelectOption {
    pub id: Option<String>,
    pub name: String,
    pub color: Option<String>,
}

/// Status option.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionStatusOption {
    pub id: Option<String>,
    pub name: String,
    pub color: Option<String>,
}

/// Date value.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionDate {
    pub start: String,
    pub end: Option<String>,
    pub time_zone: Option<String>,
}

/// File object in properties.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionFileObject {
    pub name: String,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    pub file: Option<NotionFile>,
    pub external: Option<NotionFile>,
}

/// Relation reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotionRelation {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_parsing() {
        let event = NotionWebhookEvent {
            event_type: "page.created".to_string(),
            workspace_id: None,
            integration_id: None,
            data: NotionEventData {
                object: NotionObject {
                    object: "page".to_string(),
                    id: "test-id".to_string(),
                    created_time: None,
                    last_edited_time: None,
                    created_by: None,
                    last_edited_by: None,
                    archived: false,
                    in_trash: false,
                    icon: None,
                    cover: None,
                    properties: serde_json::Value::Object(
                        Default::default(),
                    ),
                    title: None,
                    description: None,
                    url: None,
                    public_url: None,
                },
                parent: None,
            },
            timestamp: None,
        };

        assert_eq!(event.action(), NotionAction::Created);
        assert_eq!(event.object_type(), NotionObjectType::Page);
    }
}
