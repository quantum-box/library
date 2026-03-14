//! Linear webhook payload types.
//!
//! Linear sends webhooks for various events including:
//! - Issue: create, update, remove
//! - Comment: create, update, remove
//! - Project: create, update, remove
//! - Cycle: create, update, remove
//!
//! Reference: https://developers.linear.app/docs/graphql/webhooks

use serde::{Deserialize, Serialize};

/// Linear webhook event wrapper.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinearWebhookEvent {
    /// Action type: create, update, remove
    pub action: String,
    /// Actor who triggered the event
    #[serde(default)]
    pub actor: Option<Actor>,
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Event data - can be Issue, Comment, Project, or Cycle
    #[serde(default)]
    pub data: serde_json::Value,
    /// Event type: Issue, Comment, Project, Cycle
    #[serde(rename = "type")]
    pub event_type: String,
    /// URL to the resource
    #[serde(default)]
    pub url: Option<String>,
    /// Organization ID
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    /// Webhook ID
    #[serde(rename = "webhookId")]
    pub webhook_id: String,
    /// Webhook timestamp
    #[serde(rename = "webhookTimestamp")]
    #[serde(default)]
    pub webhook_timestamp: Option<i64>,
}

impl LinearWebhookEvent {
    /// Parse the data field as an Issue.
    pub fn as_issue(&self) -> Option<Issue> {
        if self.event_type == "Issue" {
            serde_json::from_value(self.data.clone()).ok()
        } else {
            None
        }
    }

    /// Parse the data field as a Comment.
    pub fn as_comment(&self) -> Option<Comment> {
        if self.event_type == "Comment" {
            serde_json::from_value(self.data.clone()).ok()
        } else {
            None
        }
    }

    /// Parse the data field as a Project.
    pub fn as_project(&self) -> Option<Project> {
        if self.event_type == "Project" {
            serde_json::from_value(self.data.clone()).ok()
        } else {
            None
        }
    }

    /// Parse the data field as a Cycle.
    pub fn as_cycle(&self) -> Option<Cycle> {
        if self.event_type == "Cycle" {
            serde_json::from_value(self.data.clone()).ok()
        } else {
            None
        }
    }
}

/// Actor who triggered the event.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Actor {
    /// Actor ID
    pub id: String,
    /// Actor name
    #[serde(default)]
    pub name: Option<String>,
    /// Actor email
    #[serde(default)]
    pub email: Option<String>,
    /// Actor type (user, application, etc.)
    #[serde(rename = "type")]
    #[serde(default)]
    pub actor_type: Option<String>,
}

/// Linear Issue.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Issue {
    /// Issue ID (UUID)
    pub id: String,
    /// Issue identifier (e.g., "ENG-123")
    pub identifier: String,
    /// Issue title
    pub title: String,
    /// Issue description (Markdown)
    #[serde(default)]
    pub description: Option<String>,
    /// Issue priority (0 = no priority, 1 = urgent, 2 = high, 3 = normal, 4 = low)
    #[serde(default)]
    pub priority: Option<i32>,
    /// Issue state
    #[serde(default)]
    pub state: Option<IssueState>,
    /// Assignee
    #[serde(default)]
    pub assignee: Option<User>,
    /// Creator
    #[serde(default)]
    pub creator: Option<User>,
    /// Team
    #[serde(default)]
    pub team: Option<Team>,
    /// Project
    #[serde(default)]
    pub project: Option<ProjectRef>,
    /// Cycle
    #[serde(default)]
    pub cycle: Option<CycleRef>,
    /// Labels
    #[serde(default)]
    pub labels: Vec<Label>,
    /// Due date (ISO 8601)
    #[serde(rename = "dueDate")]
    #[serde(default)]
    pub due_date: Option<String>,
    /// Estimate (points)
    #[serde(default)]
    pub estimate: Option<f64>,
    /// Created at timestamp
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Updated at timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// Completed at timestamp
    #[serde(rename = "completedAt")]
    #[serde(default)]
    pub completed_at: Option<String>,
    /// Canceled at timestamp
    #[serde(rename = "canceledAt")]
    #[serde(default)]
    pub canceled_at: Option<String>,
    /// URL to the issue
    #[serde(default)]
    pub url: Option<String>,
}

impl Issue {
    /// Get the priority label.
    pub fn priority_label(&self) -> &str {
        match self.priority {
            Some(0) => "No Priority",
            Some(1) => "Urgent",
            Some(2) => "High",
            Some(3) => "Normal",
            Some(4) => "Low",
            _ => "Unknown",
        }
    }

    /// Check if the issue is completed.
    pub fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }

    /// Check if the issue is canceled.
    pub fn is_canceled(&self) -> bool {
        self.canceled_at.is_some()
    }
}

/// Issue state.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssueState {
    /// State ID
    pub id: String,
    /// State name
    pub name: String,
    /// State color
    #[serde(default)]
    pub color: Option<String>,
    /// State type (backlog, unstarted, started, completed, canceled)
    #[serde(rename = "type")]
    #[serde(default)]
    pub state_type: Option<String>,
}

/// User reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// User name
    pub name: String,
    /// User email
    #[serde(default)]
    pub email: Option<String>,
}

/// Team reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Team {
    /// Team ID
    pub id: String,
    /// Team name
    pub name: String,
    /// Team key (e.g., "ENG")
    pub key: String,
}

/// Project reference (minimal).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProjectRef {
    /// Project ID
    pub id: String,
    /// Project name
    pub name: String,
}

/// Cycle reference (minimal).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CycleRef {
    /// Cycle ID
    pub id: String,
    /// Cycle number
    #[serde(default)]
    pub number: Option<i32>,
    /// Cycle name
    #[serde(default)]
    pub name: Option<String>,
}

/// Label.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    /// Label ID
    pub id: String,
    /// Label name
    pub name: String,
    /// Label color
    #[serde(default)]
    pub color: Option<String>,
}

/// Comment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Comment {
    /// Comment ID
    pub id: String,
    /// Comment body (Markdown)
    pub body: String,
    /// Issue the comment belongs to
    #[serde(default)]
    pub issue: Option<IssueRef>,
    /// Comment author
    #[serde(default)]
    pub user: Option<User>,
    /// Created at timestamp
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Updated at timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

/// Issue reference (minimal).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IssueRef {
    /// Issue ID
    pub id: String,
    /// Issue identifier
    #[serde(default)]
    pub identifier: Option<String>,
}

/// Project.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Project {
    /// Project ID
    pub id: String,
    /// Project name
    pub name: String,
    /// Project description
    #[serde(default)]
    pub description: Option<String>,
    /// Project state (planned, started, paused, completed, canceled)
    #[serde(default)]
    pub state: Option<String>,
    /// Project icon
    #[serde(default)]
    pub icon: Option<String>,
    /// Project color
    #[serde(default)]
    pub color: Option<String>,
    /// Project lead
    #[serde(default)]
    pub lead: Option<User>,
    /// Target date
    #[serde(rename = "targetDate")]
    #[serde(default)]
    pub target_date: Option<String>,
    /// Start date
    #[serde(rename = "startDate")]
    #[serde(default)]
    pub start_date: Option<String>,
    /// Created at timestamp
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Updated at timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    /// URL to the project
    #[serde(default)]
    pub url: Option<String>,
}

/// Cycle.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cycle {
    /// Cycle ID
    pub id: String,
    /// Cycle number
    pub number: i32,
    /// Cycle name
    #[serde(default)]
    pub name: Option<String>,
    /// Cycle description
    #[serde(default)]
    pub description: Option<String>,
    /// Start date
    #[serde(rename = "startsAt")]
    pub starts_at: String,
    /// End date
    #[serde(rename = "endsAt")]
    pub ends_at: String,
    /// Team
    #[serde(default)]
    pub team: Option<Team>,
    /// Created at timestamp
    #[serde(rename = "createdAt")]
    pub created_at: String,
    /// Updated at timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_issue_event() {
        let json = r#"{
            "action": "create",
            "createdAt": "2024-01-15T10:00:00.000Z",
            "data": {
                "id": "issue-uuid",
                "identifier": "ENG-123",
                "title": "Fix bug",
                "priority": 2,
                "createdAt": "2024-01-15T10:00:00.000Z",
                "updatedAt": "2024-01-15T10:00:00.000Z"
            },
            "type": "Issue",
            "organizationId": "org-uuid",
            "webhookId": "webhook-uuid"
        }"#;

        let event: LinearWebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.action, "create");
        assert_eq!(event.event_type, "Issue");

        let issue = event.as_issue().unwrap();
        assert_eq!(issue.identifier, "ENG-123");
        assert_eq!(issue.title, "Fix bug");
        assert_eq!(issue.priority, Some(2));
        assert_eq!(issue.priority_label(), "High");
    }

    #[test]
    fn test_issue_priority_labels() {
        let make_issue = |priority: Option<i32>| Issue {
            id: "id".to_string(),
            identifier: "ENG-1".to_string(),
            title: "Test".to_string(),
            description: None,
            priority,
            state: None,
            assignee: None,
            creator: None,
            team: None,
            project: None,
            cycle: None,
            labels: vec![],
            due_date: None,
            estimate: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
            completed_at: None,
            canceled_at: None,
            url: None,
        };

        assert_eq!(make_issue(Some(0)).priority_label(), "No Priority");
        assert_eq!(make_issue(Some(1)).priority_label(), "Urgent");
        assert_eq!(make_issue(Some(2)).priority_label(), "High");
        assert_eq!(make_issue(Some(3)).priority_label(), "Normal");
        assert_eq!(make_issue(Some(4)).priority_label(), "Low");
        assert_eq!(make_issue(None).priority_label(), "Unknown");
    }
}
