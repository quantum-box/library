//! Linear GraphQL API client implementation.

use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use super::event_processor::LinearClient;
use super::payload::{
    Issue, IssueState, Label, Project, ProjectRef, Team, User,
};
use crate::sdk::OAuthTokenProvider;

const LINEAR_API_BASE: &str = "https://api.linear.app/graphql";
const USER_AGENT: &str = "inbound-sync/0.1.0";

/// Rate limiting configuration
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;
const MAX_BACKOFF_MS: u64 = 30000;

/// Linear GraphQL API client implementation.
#[derive(Debug)]
pub struct LinearApiClient {
    client: reqwest::Client,
    api_key: String,
}

impl LinearApiClient {
    /// Create a new Linear API client.
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Build request headers.
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            self.api_key.parse().expect("Invalid API key"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            USER_AGENT.parse().expect("Invalid user agent"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("Invalid content type"),
        );
        headers
    }

    /// Calculate exponential backoff delay.
    fn calculate_backoff(attempt: u32) -> Duration {
        let delay_ms = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
        Duration::from_millis(delay_ms.min(MAX_BACKOFF_MS))
    }

    /// Execute a GraphQL query with retry logic.
    async fn execute_query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> errors::Result<T> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let body = serde_json::json!({
                "query": query,
                "variables": variables
            });

            let response = self
                .client
                .post(LINEAR_API_BASE)
                .headers(self.build_headers())
                .json(&body)
                .send()
                .await
                .map_err(|e| {
                    errors::Error::internal_server_error(format!(
                        "HTTP request failed: {e}"
                    ))
                })?;

            let status = response.status();

            // Handle rate limiting (429)
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let backoff = Self::calculate_backoff(attempt);
                tracing::warn!(
                    attempt = attempt + 1,
                    backoff_ms = backoff.as_millis() as u64,
                    "Linear API rate limited, retrying with backoff"
                );
                sleep(backoff).await;
                continue;
            }

            // Handle server errors
            if status.is_server_error() {
                let backoff = Self::calculate_backoff(attempt);
                tracing::warn!(
                    attempt = attempt + 1,
                    status = %status,
                    backoff_ms = backoff.as_millis() as u64,
                    "Linear API server error, retrying with backoff"
                );
                last_error = Some(format!("Server error: {status}"));
                sleep(backoff).await;
                continue;
            }

            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(errors::Error::internal_server_error(format!(
                    "Linear API error ({status}): {error_text}"
                )));
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("unknown")
                .to_string();

            let response_text = response.text().await.map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to read Linear response: {e}"
                ))
            })?;
            let response_text =
                response_text.trim_start_matches('\u{feff}');

            let response_body: GraphQLResponse<serde_json::Value> =
                match serde_json::from_str(response_text) {
                    Ok(body) => body,
                    Err(err) => {
                        let sanitized = response_text
                            .chars()
                            .filter(|ch| {
                                !ch.is_control()
                                    || matches!(ch, '\n' | '\r' | '\t')
                            })
                            .collect::<String>();
                        if sanitized != response_text {
                            if let Ok(body) =
                                serde_json::from_str(&sanitized)
                            {
                                tracing::warn!(
                                    content_type = %content_type,
                                    "Sanitized Linear response control chars"
                                );
                                body
                            } else {
                                let preview = response_text
                                    .chars()
                                    .take(200)
                                    .collect::<String>();
                                tracing::error!(
                                    content_type = %content_type,
                                    response_preview = %preview,
                                    "Failed to parse Linear response"
                                );
                                return Err(errors::Error::internal_server_error(
                                    format!(
                                        "Failed to parse Linear response: {err}"
                                    ),
                                ));
                            }
                        } else {
                            let preview = response_text
                                .chars()
                                .take(200)
                                .collect::<String>();
                            tracing::error!(
                                content_type = %content_type,
                                response_preview = %preview,
                                "Failed to parse Linear response"
                            );
                            return Err(
                                errors::Error::internal_server_error(
                                    format!(
                                    "Failed to parse Linear response: {err}"
                                ),
                                ),
                            );
                        }
                    }
                };

            if !response_body.errors.is_empty() {
                return Err(errors::Error::internal_server_error(format!(
                    "Linear GraphQL errors: {:?}",
                    response_body.errors
                )));
            }

            let data = response_body.data.ok_or_else(|| {
                errors::Error::internal_server_error(
                    "No data in Linear response".to_string(),
                )
            })?;

            let parsed: T = serde_json::from_value(data).map_err(|e| {
                errors::Error::internal_server_error(format!(
                    "Failed to parse Linear data: {e}"
                ))
            })?;

            return Ok(parsed);
        }

        Err(errors::Error::internal_server_error(
            last_error
                .unwrap_or_else(|| "Max retries exceeded".to_string()),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

// Response types for GraphQL queries
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IssueQueryResponse {
    issue: LinearIssue,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectQueryResponse {
    project: LinearProject,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearIssue {
    id: String,
    identifier: String,
    title: String,
    description: Option<String>,
    priority: Option<i32>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
    state: Option<LinearState>,
    assignee: Option<LinearAssignee>,
    team: Option<LinearTeam>,
    project: Option<LinearProjectRef>,
    labels: Option<LinearLabelsConnection>,
}

#[derive(Debug, Deserialize)]
struct LinearState {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct LinearAssignee {
    id: String,
    name: String,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearTeam {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearProjectRef {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearLabelsConnection {
    nodes: Vec<LinearLabel>,
}

#[derive(Debug, Deserialize)]
struct LinearLabel {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinearProject {
    id: String,
    name: String,
    description: Option<String>,
    state: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
}

impl From<LinearIssue> for Issue {
    fn from(issue: LinearIssue) -> Self {
        Issue {
            id: issue.id,
            identifier: issue.identifier,
            title: issue.title,
            description: issue.description,
            priority: issue.priority,
            state: issue.state.map(|s| IssueState {
                id: s.id,
                name: s.name,
                color: None,
                state_type: None,
            }),
            assignee: issue.assignee.map(|a| User {
                id: a.id,
                name: a.name,
                email: a.email,
            }),
            creator: None,
            team: issue.team.map(|t| Team {
                id: t.id,
                name: t.name.unwrap_or_default(),
                key: String::new(),
            }),
            project: issue.project.map(|p| ProjectRef {
                id: p.id,
                name: p.name.unwrap_or_default(),
            }),
            cycle: None,
            labels: issue
                .labels
                .map(|l| {
                    l.nodes
                        .into_iter()
                        .map(|label| Label {
                            id: label.id,
                            name: label.name,
                            color: None,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            due_date: None,
            estimate: None,
            created_at: issue.created_at.unwrap_or_default(),
            updated_at: issue.updated_at.unwrap_or_default(),
            completed_at: None,
            canceled_at: None,
            url: None,
        }
    }
}

impl From<LinearProject> for Project {
    fn from(project: LinearProject) -> Self {
        Project {
            id: project.id,
            name: project.name,
            description: project.description,
            state: project.state,
            icon: None,
            color: None,
            lead: None,
            target_date: None,
            start_date: None,
            created_at: project.created_at.unwrap_or_default(),
            updated_at: project.updated_at.unwrap_or_default(),
            url: None,
        }
    }
}

const ISSUE_QUERY: &str = r#"
    query GetIssue($id: String!) {
        issue(id: $id) {
            id
            identifier
            title
            description
            priority
            createdAt
            updatedAt
            state {
                id
                name
            }
            assignee {
                id
                name
            }
            team {
                id
                name
            }
            project {
                id
                name
            }
            labels {
                nodes {
                    id
                    name
                }
            }
        }
    }
"#;

const PROJECT_QUERY: &str = r#"
    query GetProject($id: String!) {
        project(id: $id) {
            id
            name
            description
            state
            createdAt
            updatedAt
        }
    }
"#;

#[async_trait]
impl LinearClient for LinearApiClient {
    async fn get_issue(
        &self,
        _tenant_id: &value_object::TenantId,
        issue_id: &str,
    ) -> errors::Result<Issue> {
        let variables = serde_json::json!({
            "id": issue_id
        });

        let response: IssueQueryResponse =
            self.execute_query(ISSUE_QUERY, variables).await?;

        Ok(response.issue.into())
    }

    async fn get_project(
        &self,
        _tenant_id: &value_object::TenantId,
        project_id: &str,
    ) -> errors::Result<Project> {
        let variables = serde_json::json!({
            "id": project_id
        });

        let response: ProjectQueryResponse =
            self.execute_query(PROJECT_QUERY, variables).await?;

        Ok(response.project.into())
    }

    async fn list_issues(
        &self,
        _tenant_id: &value_object::TenantId,
        team_id: Option<&str>,
        project_id: Option<&str>,
    ) -> errors::Result<Vec<Issue>> {
        let base_fields = r#"
            id
            identifier
            title
            description
            priority
            state { id name }
            assignee { id name }
            creator { id name }
            team { id name key }
            project { id name }
            labels { nodes { id name color } }
            dueDate
            estimate
            createdAt
            updatedAt
            completedAt
            canceledAt
            url
        "#;

        let (query, variables) = match (team_id, project_id) {
            (Some(tid), Some(pid)) => (
                format!(
                    r#"
            query ListIssues($teamId: String!, $projectId: String!) {{
              issues(
                filter: {{
                  team: {{ id: {{ eq: $teamId }} }}
                  project: {{ id: {{ eq: $projectId }} }}
                }}
                first: 100
              ) {{
                nodes {{
                  {base_fields}
                }}
              }}
            }}
        "#
                ),
                serde_json::json!({ "teamId": tid, "projectId": pid }),
            ),
            (Some(tid), None) => (
                format!(
                    r#"
            query ListIssues($teamId: String!) {{
              issues(
                filter: {{
                  team: {{ id: {{ eq: $teamId }} }}
                }}
                first: 100
              ) {{
                nodes {{
                  {base_fields}
                }}
              }}
            }}
        "#
                ),
                serde_json::json!({ "teamId": tid }),
            ),
            (None, Some(pid)) => (
                format!(
                    r#"
            query ListIssues($projectId: String!) {{
              issues(
                filter: {{
                  project: {{ id: {{ eq: $projectId }} }}
                }}
                first: 100
              ) {{
                nodes {{
                  {base_fields}
                }}
              }}
            }}
        "#
                ),
                serde_json::json!({ "projectId": pid }),
            ),
            (None, None) => (
                format!(
                    r#"
            query ListIssues {{
              issues(first: 100) {{
                nodes {{
                  {base_fields}
                }}
              }}
            }}
        "#
                ),
                serde_json::json!({}),
            ),
        };

        #[derive(Deserialize)]
        struct Response {
            issues: Option<IssuesConnection>,
        }

        #[derive(Deserialize)]
        struct IssuesConnection {
            nodes: Vec<Option<IssueNode>>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IssueNode {
            id: String,
            identifier: String,
            title: String,
            description: Option<String>,
            priority: Option<f64>,
            state: Option<IssueState>,
            assignee: Option<User>,
            creator: Option<User>,
            team: Option<Team>,
            project: Option<ProjectRef>,
            labels: Option<LabelsConnection>,
            due_date: Option<String>,
            estimate: Option<f64>,
            created_at: String,
            updated_at: String,
            completed_at: Option<String>,
            canceled_at: Option<String>,
            url: Option<String>,
        }

        #[derive(Deserialize)]
        struct LabelsConnection {
            nodes: Vec<Label>,
        }

        let response: Response =
            self.execute_query(&query, variables).await?;

        let nodes = response
            .issues
            .map(|issues| issues.nodes)
            .unwrap_or_default();

        Ok(nodes
            .into_iter()
            .flatten()
            .map(|node| Issue {
                id: node.id,
                identifier: node.identifier,
                title: node.title,
                description: node.description,
                priority: node.priority.map(|p| p as i32),
                state: node.state,
                assignee: node.assignee,
                creator: node.creator,
                team: node.team,
                project: node.project,
                cycle: None,
                labels: node.labels.map(|l| l.nodes).unwrap_or_default(),
                due_date: node.due_date,
                estimate: node.estimate,
                created_at: node.created_at,
                updated_at: node.updated_at,
                completed_at: node.completed_at,
                canceled_at: node.canceled_at,
                url: node.url,
            })
            .collect())
    }

    async fn list_projects(
        &self,
        _tenant_id: &value_object::TenantId,
        team_id: Option<&str>,
    ) -> errors::Result<Vec<Project>> {
        let base_fields = r#"
            id
            name
            description
            state
            icon
            color
            lead { id name }
            targetDate
            startDate
            createdAt
            updatedAt
            url
        "#;

        #[derive(Deserialize)]
        struct ProjectsConnection {
            nodes: Vec<ProjectNode>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ProjectNode {
            id: String,
            name: String,
            description: Option<String>,
            state: Option<String>,
            icon: Option<String>,
            color: Option<String>,
            lead: Option<User>,
            target_date: Option<String>,
            start_date: Option<String>,
            created_at: String,
            updated_at: String,
            url: Option<String>,
        }

        let nodes = if let Some(tid) = team_id {
            let query = format!(
                r#"
            query ListProjects($teamId: String!) {{
              team(id: $teamId) {{
                projects(first: 100) {{
                  nodes {{
                    {base_fields}
                  }}
                }}
              }}
            }}
        "#
            );

            #[derive(Deserialize)]
            struct Response {
                team: Option<TeamProjects>,
            }

            #[derive(Deserialize)]
            struct TeamProjects {
                projects: ProjectsConnection,
            }

            let response: Response = self
                .execute_query(&query, serde_json::json!({ "teamId": tid }))
                .await?;

            response
                .team
                .map(|team| team.projects.nodes)
                .unwrap_or_default()
        } else {
            let query = format!(
                r#"
            query ListProjects {{
              projects(first: 100) {{
                nodes {{
                  {base_fields}
                }}
              }}
            }}
        "#
            );

            #[derive(Deserialize)]
            struct Response {
                projects: ProjectsConnection,
            }

            let response: Response =
                self.execute_query(&query, serde_json::json!({})).await?;

            response.projects.nodes
        };

        Ok(nodes
            .into_iter()
            .map(|node| Project {
                id: node.id,
                name: node.name,
                description: node.description,
                state: node.state,
                icon: node.icon,
                color: node.color,
                lead: node.lead,
                target_date: node.target_date,
                start_date: node.start_date,
                created_at: node.created_at,
                updated_at: node.updated_at,
                url: node.url,
            })
            .collect())
    }

    async fn list_teams(
        &self,
        _tenant_id: &value_object::TenantId,
    ) -> errors::Result<Vec<Team>> {
        let query = r#"
            query ListTeams {
              teams {
                nodes {
                  id
                  name
                  key
                }
              }
            }
        "#;

        #[derive(Deserialize)]
        struct Response {
            teams: TeamsConnection,
        }

        #[derive(Deserialize)]
        struct TeamsConnection {
            nodes: Vec<TeamNode>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TeamNode {
            id: String,
            name: String,
            key: String,
        }

        let response: Response =
            self.execute_query(query, serde_json::json!({})).await?;

        Ok(response
            .teams
            .nodes
            .into_iter()
            .map(|node| Team {
                id: node.id,
                name: node.name,
                key: node.key,
            })
            .collect())
    }
}

/// OAuth-integrated Linear API client.
///
/// This implementation fetches OAuth tokens via `OAuthTokenProvider` for
/// each request, allowing per-tenant Linear connections.
#[derive(Debug)]
pub struct OAuthLinearClient {
    token_provider: Arc<dyn OAuthTokenProvider>,
}

impl OAuthLinearClient {
    /// Create a new OAuth-integrated Linear client.
    pub fn new(token_provider: Arc<dyn OAuthTokenProvider>) -> Self {
        Self { token_provider }
    }

    async fn get_access_token(
        &self,
        tenant_id: &value_object::TenantId,
    ) -> errors::Result<String> {
        let token = self
            .token_provider
            .get_token(tenant_id, "linear")
            .await?
            .ok_or_else(|| {
                errors::Error::unauthorized(
                    "Linear is not connected for this tenant. Please complete OAuth authorization first.",
                )
            })?;

        if token.is_expired() {
            return Err(errors::Error::unauthorized(
                "Linear OAuth token has expired. Please reconnect.",
            ));
        }

        Ok(format!("Bearer {}", token.access_token))
    }
}

#[async_trait]
impl LinearClient for OAuthLinearClient {
    async fn get_issue(
        &self,
        tenant_id: &value_object::TenantId,
        issue_id: &str,
    ) -> errors::Result<Issue> {
        let access_token = self.get_access_token(tenant_id).await?;
        let client = LinearApiClient::new(access_token);
        client.get_issue(tenant_id, issue_id).await
    }

    async fn get_project(
        &self,
        tenant_id: &value_object::TenantId,
        project_id: &str,
    ) -> errors::Result<Project> {
        let access_token = self.get_access_token(tenant_id).await?;
        let client = LinearApiClient::new(access_token);
        client.get_project(tenant_id, project_id).await
    }

    async fn list_issues(
        &self,
        tenant_id: &value_object::TenantId,
        team_id: Option<&str>,
        project_id: Option<&str>,
    ) -> errors::Result<Vec<Issue>> {
        let access_token = self.get_access_token(tenant_id).await?;
        let client = LinearApiClient::new(access_token);
        client.list_issues(tenant_id, team_id, project_id).await
    }

    async fn list_projects(
        &self,
        tenant_id: &value_object::TenantId,
        team_id: Option<&str>,
    ) -> errors::Result<Vec<Project>> {
        let access_token = self.get_access_token(tenant_id).await?;
        let client = LinearApiClient::new(access_token);
        client.list_projects(tenant_id, team_id).await
    }

    async fn list_teams(
        &self,
        tenant_id: &value_object::TenantId,
    ) -> errors::Result<Vec<Team>> {
        let access_token = self.get_access_token(tenant_id).await?;
        let client = LinearApiClient::new(access_token);
        client.list_teams(tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = LinearApiClient::new("test_api_key".to_string());
        assert!(!client.api_key.is_empty());
    }

    #[test]
    fn test_backoff_calculation() {
        assert_eq!(
            LinearApiClient::calculate_backoff(0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            LinearApiClient::calculate_backoff(1),
            Duration::from_millis(2000)
        );
        assert_eq!(
            LinearApiClient::calculate_backoff(2),
            Duration::from_millis(4000)
        );
        // Should cap at MAX_BACKOFF_MS
        assert_eq!(
            LinearApiClient::calculate_backoff(10),
            Duration::from_millis(30000)
        );
    }
}
