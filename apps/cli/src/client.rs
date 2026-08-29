//! Thin HTTP layer over the Library REST API and MCP endpoint.
//!
//! Everything the CLI sends and receives is `serde_json::Value`. The API
//! already owns the schema, and reproducing it here would only give the
//! CLI a second definition to keep in step.

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};

use crate::config::ResolvedConfig;

pub struct LibraryClient {
    http: reqwest::Client,
    config: ResolvedConfig,
}

impl LibraryClient {
    pub fn new(config: ResolvedConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("library-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to build the HTTP client")?;

        Ok(Self { http, config })
    }

    pub fn api_base_url(&self) -> &str {
        &self.config.api_base_url
    }

    pub fn has_api_key(&self) -> bool {
        self.config.api_key.is_some()
    }

    pub fn api_key(&self) -> Option<&str> {
        self.config.api_key.as_deref()
    }

    /// Send a request and return the decoded JSON body, or `Value::Null`
    /// for the endpoints that answer `204 No Content`.
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<Value> {
        let url = format!("{}{path}", self.config.api_base_url);
        let mut request = self.http.request(method.clone(), &url);

        if !query.is_empty() {
            request = request.query(query);
        }
        if let Some(api_key) = &self.config.api_key {
            request = request.bearer_auth(api_key);
        }
        if let Some(operator_id) = &self.config.operator_id {
            request = request.header("x-operator-id", operator_id);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.with_context(|| {
            format!("{method} {url} failed to reach the Library API")
        })?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            bail!(describe_failure(method, &url, status, &text));
        }
        if status == StatusCode::NO_CONTENT || text.trim().is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_str(&text).with_context(|| {
            format!("{method} {url} returned a body that is not JSON")
        })
    }

    pub async fn get(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<Value> {
        self.request(Method::GET, path, query, None).await
    }

    /// Fetch an endpoint that answers with something other than JSON —
    /// today, the Markdown rendering of a document.
    pub async fn get_text(&self, path: &str) -> Result<String> {
        let url = format!("{}{path}", self.config.api_base_url);
        let mut request = self.http.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.bearer_auth(api_key);
        }
        if let Some(operator_id) = &self.config.operator_id {
            request = request.header("x-operator-id", operator_id);
        }

        let response = request.send().await.with_context(|| {
            format!("GET {url} failed to reach the Library API")
        })?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!(describe_failure(Method::GET, &url, status, &text));
        }

        Ok(text)
    }

    pub async fn post(&self, path: &str, body: Value) -> Result<Value> {
        self.request(Method::POST, path, &[], Some(body)).await
    }

    pub async fn put(&self, path: &str, body: Value) -> Result<Value> {
        self.request(Method::PUT, path, &[], Some(body)).await
    }

    pub async fn delete(&self, path: &str) -> Result<Value> {
        self.request(Method::DELETE, path, &[], None).await
    }

    /// Call the MCP endpoint over its plain JSON-RPC transport. The CLI
    /// uses this to let an operator exercise the very tools an agent
    /// sees, without standing up an MCP client first.
    pub async fn mcp_rpc(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value> {
        let mut request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
        });
        if let Some(params) = params {
            request["params"] = params;
        }

        let response = self.post("/mcp", request).await?;
        if let Some(error) = response.get("error") {
            bail!("MCP error: {error}");
        }
        response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("MCP response carried no result"))
    }
}

/// Turn a failed response into a message that says what to do about it.
/// The API answers errors as JSON, so the useful part is usually buried
/// one field deep rather than in the status line.
fn describe_failure(
    method: Method,
    url: &str,
    status: StatusCode,
    body: &str,
) -> String {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            ["message", "error", "detail"]
                .iter()
                .find_map(|field| {
                    value
                        .get(*field)
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .or_else(|| Some(value.to_string()))
        })
        .unwrap_or_else(|| body.trim().to_string());

    let hint = match status {
        StatusCode::UNAUTHORIZED => Some(
            "run `library auth login --api-key pk_…`, or set \
             LIBRARY_API_KEY",
        ),
        StatusCode::FORBIDDEN => Some(
            "the key is valid but not allowed to do this in that \
             organization",
        ),
        StatusCode::NOT_FOUND => Some(
            "check the org/repo slugs, and whether the key can see a \
             private repository",
        ),
        _ => None,
    };

    let mut message = format!("{method} {url} failed with {status}");
    if !detail.is_empty() {
        message.push_str(&format!(": {detail}"));
    }
    if let Some(hint) = hint {
        message.push_str(&format!("\nhint: {hint}"));
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_json_error_body_is_unwrapped_into_the_message() {
        let message = describe_failure(
            Method::GET,
            "https://api.example.com/v1beta/repos/acme/docs",
            StatusCode::NOT_FOUND,
            r#"{"message":"repo is not found"}"#,
        );

        assert!(message.contains("repo is not found"));
        assert!(message.contains("hint:"));
    }

    #[test]
    fn a_non_json_error_body_is_reported_as_is() {
        let message = describe_failure(
            Method::POST,
            "https://api.example.com/v1beta/orgs",
            StatusCode::BAD_GATEWAY,
            "upstream timed out",
        );

        assert!(message.contains("upstream timed out"));
        // No hint exists for 502, and inventing one would be noise.
        assert!(!message.contains("hint:"));
    }

    #[test]
    fn an_unauthorized_response_says_how_to_authenticate() {
        let message = describe_failure(
            Method::GET,
            "https://api.example.com/v1beta/orgs/acme",
            StatusCode::UNAUTHORIZED,
            "",
        );

        assert!(message.contains("library auth login"));
    }
}
