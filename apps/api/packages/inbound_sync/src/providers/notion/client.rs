//! Notion API client implementation.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::fmt::Debug;
use std::time::Duration;

const NOTION_API_VERSION: &str = "2022-06-28";
const NOTION_API_BASE: &str = "https://api.notion.com/v1";

/// Trait for Notion API operations.
#[async_trait]
pub trait NotionClient: Send + Sync + Debug {
    /// Get a page by ID.
    async fn get_page(
        &self,
        tenant_id: &value_object::TenantId,
        page_id: &str,
    ) -> errors::Result<serde_json::Value>;

    /// Get a database by ID.
    async fn get_database(
        &self,
        tenant_id: &value_object::TenantId,
        database_id: &str,
    ) -> errors::Result<serde_json::Value>;

    /// Query a database for pages.
    async fn query_database(
        &self,
        tenant_id: &value_object::TenantId,
        database_id: &str,
        filter: Option<serde_json::Value>,
        sorts: Option<Vec<serde_json::Value>>,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> errors::Result<NotionQueryResult>;

    /// Get page content (blocks).
    async fn get_page_content(
        &self,
        tenant_id: &value_object::TenantId,
        page_id: &str,
    ) -> errors::Result<Vec<serde_json::Value>>;

    /// Get page property value.
    async fn get_page_property(
        &self,
        tenant_id: &value_object::TenantId,
        page_id: &str,
        property_id: &str,
    ) -> errors::Result<serde_json::Value>;

    /// List all pages in a database (for Initial Sync).
    async fn list_database_pages(
        &self,
        tenant_id: &value_object::TenantId,
        database_id: &str,
    ) -> errors::Result<Vec<serde_json::Value>>;
}

/// Result of a database query.
#[derive(Debug, Clone, Deserialize)]
pub struct NotionQueryResult {
    pub results: Vec<serde_json::Value>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Notion API client implementation.
#[derive(Debug)]
pub struct NotionApiClient {
    client: Client,
    token: String,
}

impl NotionApiClient {
    /// Create a new Notion API client.
    pub fn new(token: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            token: token.into(),
        }
    }

    /// Build request headers.
    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.token)
                .parse()
                .expect("Invalid token"),
        );
        headers.insert(
            "Notion-Version",
            NOTION_API_VERSION.parse().expect("Invalid version"),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("Invalid content type"),
        );
        headers
    }

    /// Execute request with retry logic.
    async fn execute_with_retry<T: for<'de> Deserialize<'de>>(
        &self,
        request: reqwest::RequestBuilder,
    ) -> errors::Result<T> {
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 0..max_retries {
            let req = request.try_clone().ok_or_else(|| {
                errors::Error::internal_server_error(
                    "Failed to clone request",
                )
            })?;

            match req.send().await {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        return response.json::<T>().await.map_err(|e| {
                            errors::Error::internal_server_error(format!(
                                "Failed to parse response: {e}"
                            ))
                        });
                    }

                    // Handle rate limiting
                    if status.as_u16() == 429 {
                        let retry_after = response
                            .headers()
                            .get("Retry-After")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(1);

                        tracing::warn!(
                            attempt = attempt + 1,
                            retry_after,
                            "Rate limited, retrying"
                        );
                        tokio::time::sleep(Duration::from_secs(
                            retry_after,
                        ))
                        .await;
                        continue;
                    }

                    let error_body =
                        response.text().await.unwrap_or_default();
                    last_error = Some(
                        errors::Error::internal_server_error(format!(
                            "Notion API error ({status}): {error_body}"
                        )),
                    );

                    // Retry on server errors
                    if status.is_server_error() && attempt < max_retries - 1
                    {
                        let delay = Duration::from_millis(
                            500 * (attempt as u64 + 1),
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }

                    break;
                }
                Err(e) => {
                    last_error =
                        Some(errors::Error::internal_server_error(
                            format!("Request failed: {e}"),
                        ));

                    if attempt < max_retries - 1 {
                        let delay = Duration::from_millis(
                            500 * (attempt as u64 + 1),
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            errors::Error::internal_server_error("Unknown error")
        }))
    }
}

#[async_trait]
impl NotionClient for NotionApiClient {
    async fn get_page(
        &self,
        _tenant_id: &value_object::TenantId,
        page_id: &str,
    ) -> errors::Result<serde_json::Value> {
        let url = format!("{NOTION_API_BASE}/pages/{page_id}");

        let request = self.client.get(&url).headers(self.headers());

        self.execute_with_retry(request).await
    }

    async fn get_database(
        &self,
        _tenant_id: &value_object::TenantId,
        database_id: &str,
    ) -> errors::Result<serde_json::Value> {
        let url = format!("{NOTION_API_BASE}/databases/{database_id}");

        let request = self.client.get(&url).headers(self.headers());

        self.execute_with_retry(request).await
    }

    async fn query_database(
        &self,
        _tenant_id: &value_object::TenantId,
        database_id: &str,
        filter: Option<serde_json::Value>,
        sorts: Option<Vec<serde_json::Value>>,
        start_cursor: Option<String>,
        page_size: Option<u32>,
    ) -> errors::Result<NotionQueryResult> {
        let url =
            format!("{NOTION_API_BASE}/databases/{database_id}/query");

        let mut body = serde_json::Map::new();
        if let Some(f) = filter {
            body.insert("filter".to_string(), f);
        }
        if let Some(s) = sorts {
            body.insert("sorts".to_string(), serde_json::Value::Array(s));
        }
        if let Some(sc) = start_cursor {
            body.insert(
                "start_cursor".to_string(),
                serde_json::Value::String(sc),
            );
        }
        if let Some(ps) = page_size {
            body.insert(
                "page_size".to_string(),
                serde_json::Value::Number(ps.into()),
            );
        }

        let request = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&serde_json::Value::Object(body));

        self.execute_with_retry(request).await
    }

    async fn get_page_content(
        &self,
        _tenant_id: &value_object::TenantId,
        page_id: &str,
    ) -> errors::Result<Vec<serde_json::Value>> {
        let url = format!("{NOTION_API_BASE}/blocks/{page_id}/children");

        let request = self.client.get(&url).headers(self.headers());

        #[derive(Deserialize)]
        struct BlocksResponse {
            results: Vec<serde_json::Value>,
        }

        let response: BlocksResponse =
            self.execute_with_retry(request).await?;
        Ok(response.results)
    }

    async fn get_page_property(
        &self,
        _tenant_id: &value_object::TenantId,
        page_id: &str,
        property_id: &str,
    ) -> errors::Result<serde_json::Value> {
        let url = format!(
            "{NOTION_API_BASE}/pages/{page_id}/properties/{property_id}"
        );

        let request = self.client.get(&url).headers(self.headers());

        self.execute_with_retry(request).await
    }

    async fn list_database_pages(
        &self,
        _tenant_id: &value_object::TenantId,
        database_id: &str,
    ) -> errors::Result<Vec<serde_json::Value>> {
        let mut all_pages = Vec::new();
        let mut has_more = true;
        let mut cursor: Option<String> = None;

        while has_more {
            let result = self
                .query_database(
                    _tenant_id,
                    database_id,
                    None,
                    None,
                    cursor.clone(),
                    Some(100),
                )
                .await?;

            all_pages.extend(result.results);
            has_more = result.has_more;
            cursor = result.next_cursor;

            if has_more && cursor.is_none() {
                break;
            }
        }

        Ok(all_pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = NotionApiClient::new("test-token");
        assert!(!client.token.is_empty());
    }
}
