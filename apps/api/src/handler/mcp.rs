use std::sync::Arc;

use axum::{
    extract::Extension,
    http::{
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tachyon_sdk::auth::ExecutorAction;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::{
    LibraryExecutor, LibraryExecutorKind,
};
use crate::sdk_auth::SdkAuthApp;
use crate::usecase::markdown_composer::compose_markdown;
use crate::usecase::{
    AddDataInputData, LibraryOrg, PropertyDataInputData,
    PropertyDataValueInputData, SearchDataInputData, ViewDataInputData,
    ViewDataListInputData, ViewOrgInputData,
};

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const MCP_AUTH_SCOPE_READ: &str = "library:read";
const MCP_AUTH_SCOPE_WRITE: &str = "library:write";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct ListDataArgs {
    org: String,
    repo: String,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SearchDataArgs {
    org: String,
    repo: String,
    query: String,
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GetDataArgs {
    org: String,
    repo: String,
    data_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateDataArgs {
    org: String,
    repo: String,
    name: String,
    #[serde(default)]
    property_data: Vec<CreateDataPropertyArgs>,
}

#[derive(Debug, Deserialize)]
struct CreateDataPropertyArgs {
    property_id: String,
    value: String,
    #[serde(default)]
    value_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct McpDataSummary {
    id: String,
    title: String,
}

#[derive(Debug, Serialize)]
struct McpData {
    id: String,
    title: String,
    markdown: String,
}

#[axum::debug_handler]
pub async fn mcp_handler(
    headers: HeaderMap,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Extension(sdk): Extension<Arc<SdkAuthApp>>,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    if should_challenge(&headers, &request) {
        return auth_challenge_response();
    }

    let org_hint = request_org_hint(&request);
    let auth = resolve_auth_context(
        &headers,
        sdk,
        library_app.clone(),
        org_hint.as_deref(),
    )
    .await;
    if mcp_auth_required() && !auth.is_authenticated() {
        return auth_challenge_response();
    }
    Json(handle_rpc(library_app, auth, request).await).into_response()
}

async fn handle_rpc(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    request: JsonRpcRequest,
) -> Value {
    let id = request.id.clone().unwrap_or(Value::Null);
    let result = match request.method.as_str() {
        "initialize" => Ok(initialize_result()),
        "notifications/initialized" => Ok(json!({})),
        "tools/list" => Ok(tools_list_result(auth.can_use_write_tools())),
        "tools/call" => call_tool(library_app, auth, request.params).await,
        _ => Err(json_rpc_error(
            -32601,
            format!("Method not found: {}", request.method),
        )),
    };

    match result {
        Ok(result) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": error,
        }),
    }
}

async fn call_tool(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    params: Option<Value>,
) -> Result<Value, Value> {
    let params: ToolCallParams = serde_json::from_value(
        params.ok_or_else(|| json_rpc_error(-32602, "Missing params"))?,
    )
    .map_err(|err| json_rpc_error(-32602, err.to_string()))?;

    match params.name.as_str() {
        "list_data" => {
            let args: ListDataArgs = parse_tool_args(params.arguments)?;
            let output = list_data(library_app, args).await?;
            Ok(tool_text_result(output))
        }
        "search_data" => {
            let args: SearchDataArgs = parse_tool_args(params.arguments)?;
            let output = search_data(library_app, args).await?;
            Ok(tool_text_result(output))
        }
        "get_data" => {
            let args: GetDataArgs = parse_tool_args(params.arguments)?;
            let output = get_data(library_app, args).await?;
            Ok(tool_text_result(output))
        }
        "create_data" => {
            let args: CreateDataArgs = parse_tool_args(params.arguments)?;
            let output = create_data(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        name => {
            Err(json_rpc_error(-32602, format!("Unknown tool: {name}")))
        }
    }
}

async fn list_data(
    library_app: Arc<LibraryApp>,
    args: ListDataArgs,
) -> Result<Value, Value> {
    let executor = anonymous_executor();
    let library_org = LibraryOrg::with_org(args.org.clone());
    let input = ViewDataListInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
        page: Some(args.page.unwrap_or(1)),
        page_size: Some(args.page_size.unwrap_or(20)),
    };

    let (data_list, _properties, paginator) = library_app
        .view_data_list
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;

    let data_list = data_list
        .iter()
        .map(|data| McpDataSummary {
            id: data.id().to_string(),
            title: data.name().to_string(),
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "data": data_list,
        "paginator": paginator,
    }))
}

async fn search_data(
    library_app: Arc<LibraryApp>,
    args: SearchDataArgs,
) -> Result<Value, Value> {
    let executor = anonymous_executor();
    let library_org = LibraryOrg::with_org(args.org.clone());
    let input = SearchDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: &args.org,
        repo_username: &args.repo,
        name: &args.query,
        page: Some(args.page.unwrap_or(1)),
        page_size: Some(args.page_size.unwrap_or(20)),
    };

    let (data_list, _properties, paginator) = library_app
        .search_data
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;

    let data_list = data_list
        .iter()
        .map(|data| McpDataSummary {
            id: data.id().to_string(),
            title: data.name().to_string(),
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "data": data_list,
        "paginator": paginator,
    }))
}

async fn get_data(
    library_app: Arc<LibraryApp>,
    args: GetDataArgs,
) -> Result<Value, Value> {
    let executor = anonymous_executor();
    let library_org = LibraryOrg::with_org(args.org.clone());
    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
        data_id: args.data_id,
    };

    let (data, properties) = library_app
        .view_data
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;
    let data = McpData {
        id: data.id().to_string(),
        title: data.name().to_string(),
        markdown: compose_markdown(&data, &properties),
    };

    Ok(json!({ "data": data }))
}

async fn create_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: CreateDataArgs,
) -> Result<Value, Value> {
    let executor = auth.executor.ok_or_else(|| {
        json_rpc_error(-32001, "Authentication required for create_data")
    })?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let property_data = args
        .property_data
        .into_iter()
        .map(|property| PropertyDataInputData {
            property_id: property.property_id,
            value: match property.value_type.as_deref() {
                Some("integer") => {
                    PropertyDataValueInputData::Integer(property.value)
                }
                Some("html") => {
                    PropertyDataValueInputData::Html(property.value)
                }
                Some("markdown") => {
                    PropertyDataValueInputData::Markdown(property.value)
                }
                Some("select") => {
                    PropertyDataValueInputData::Select(property.value)
                }
                Some("date") => {
                    PropertyDataValueInputData::Date(property.value)
                }
                Some("image") => {
                    PropertyDataValueInputData::Image(property.value)
                }
                _ => PropertyDataValueInputData::String(property.value),
            },
        })
        .collect::<Vec<_>>();

    let input = AddDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        actor: executor.get_id(),
        org_username: &args.org,
        repo_username: &args.repo,
        data_name: &args.name,
        property_data,
    };

    let (data, properties) = library_app
        .save_data
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({
        "data": {
            "id": data.id().to_string(),
            "title": data.name().to_string(),
            "property_count": properties.len()
        }
    }))
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": MCP_PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "library-mcp",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn tools_list_result(is_authenticated: bool) -> Value {
    let mut tools = vec![
        json!({
            "name": "list_data",
            "description": "List data records in a public Library repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "repo": { "type": "string" },
                    "page": { "type": "integer", "minimum": 1 },
                    "page_size": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["org", "repo"]
            }
        }),
        json!({
            "name": "search_data",
            "description": "Search data records by name or indexed content in a public Library repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "repo": { "type": "string" },
                    "query": { "type": "string" },
                    "page": { "type": "integer", "minimum": 1 },
                    "page_size": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": ["org", "repo", "query"]
            }
        }),
        json!({
            "name": "get_data",
            "description": "Get one public Library data record as composed Markdown.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "repo": { "type": "string" },
                    "data_id": { "type": "string" }
                },
                "required": ["org", "repo", "data_id"]
            }
        }),
    ];

    if is_authenticated {
        tools.push(json!({
            "name": "create_data",
            "description": "Create a Library data record. Requires authentication and repository write permission.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "repo": { "type": "string" },
                    "name": { "type": "string" },
                    "property_data": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "property_id": { "type": "string" },
                                "value": { "type": "string" },
                                "value_type": {
                                    "type": "string",
                                    "enum": [
                                        "string",
                                        "integer",
                                        "html",
                                        "markdown",
                                        "select",
                                        "date",
                                        "image"
                                    ]
                                }
                            },
                            "required": ["property_id", "value"]
                        }
                    }
                },
                "required": ["org", "repo", "name"]
            }
        }));
    }

    json!({ "tools": tools })
}

fn parse_tool_args<T>(arguments: Value) -> Result<T, Value>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(arguments)
        .map_err(|err| json_rpc_error(-32602, err.to_string()))
}

fn tool_text_result(value: Value) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&value)
                    .unwrap_or_else(|_| value.to_string())
            }
        ]
    })
}

fn tool_execution_error(err: errors::Error) -> Value {
    json_rpc_error(-32000, err.to_string())
}

fn json_rpc_error(code: i64, message: impl Into<String>) -> Value {
    json!({
        "code": code,
        "message": message.into(),
    })
}

fn anonymous_executor() -> LibraryExecutor {
    LibraryExecutor {
        inner: LibraryExecutorKind::None,
        original_token: None,
    }
}

#[derive(Debug, Clone)]
struct McpAuthContext {
    executor: Option<LibraryExecutor>,
    accepted_credentials: bool,
    write_tools_available: bool,
}

impl McpAuthContext {
    fn anonymous() -> Self {
        Self {
            executor: None,
            accepted_credentials: false,
            write_tools_available: false,
        }
    }

    fn authenticated(executor: LibraryExecutor) -> Self {
        Self {
            executor: Some(executor),
            accepted_credentials: true,
            write_tools_available: true,
        }
    }

    fn accepted_without_executor(write_tools_available: bool) -> Self {
        Self {
            executor: None,
            accepted_credentials: true,
            write_tools_available,
        }
    }

    fn is_authenticated(&self) -> bool {
        self.accepted_credentials
    }

    fn can_use_write_tools(&self) -> bool {
        self.executor
            .as_ref()
            .is_some_and(|executor| !executor.is_none())
            || self.write_tools_available
    }
}

async fn resolve_auth_context(
    headers: &HeaderMap,
    sdk: Arc<SdkAuthApp>,
    library_app: Arc<LibraryApp>,
    org_username: Option<&str>,
) -> McpAuthContext {
    let Some(token) = bearer_token(headers) else {
        return McpAuthContext::anonymous();
    };

    let environment = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".into());
    if token == "dummy-token"
        && (environment == "development" || environment == "test")
    {
        return McpAuthContext::accepted_without_executor(false);
    }

    if token.starts_with("pk_") {
        if let Some(org_username) = org_username {
            if let Ok(org) = library_app
                .view_org
                .execute(&ViewOrgInputData {
                    executor: &tachyon_sdk::auth::Executor::SystemUser,
                    multi_tenancy: &LibraryOrg::with_org(
                        org_username.to_string(),
                    ),
                    organization_username: org_username.to_string(),
                })
                .await
            {
                if let Ok(service_account) =
                    sdk.verify_api_key(org.organization.id(), &token).await
                {
                    return McpAuthContext::authenticated(
                        LibraryExecutor {
                            inner: LibraryExecutorKind::ServiceAccount(
                                Box::new(service_account),
                            ),
                            original_token: Some(token),
                        },
                    );
                }
            }
        }
        return McpAuthContext::anonymous();
    }

    match sdk.verify_token(&token).await {
        Ok(user) => McpAuthContext::authenticated(LibraryExecutor {
            inner: LibraryExecutorKind::User(Box::new(user)),
            original_token: Some(token),
        }),
        Err(error) => {
            tracing::warn!("MCP bearer token verification failed: {error}");
            McpAuthContext::anonymous()
        }
    }
}

async fn authenticated_library_org(
    library_app: &LibraryApp,
    org_username: &str,
) -> errors::Result<LibraryOrg> {
    let temp_library_org = LibraryOrg::with_org(org_username.to_string());
    let org = library_app
        .view_org
        .execute(&ViewOrgInputData {
            executor: &tachyon_sdk::auth::Executor::SystemUser,
            multi_tenancy: &temp_library_org,
            organization_username: org_username.to_string(),
        })
        .await?;
    Ok(LibraryOrg::with_org_and_operator(
        org_username.to_string(),
        org.organization.id().clone(),
    ))
}

fn should_challenge(headers: &HeaderMap, request: &JsonRpcRequest) -> bool {
    if bearer_token(headers).is_some() {
        return false;
    }

    if mcp_auth_required() {
        return true;
    }

    request.method == "tools/call"
        && request
            .params
            .as_ref()
            .and_then(|params| {
                serde_json::from_value::<ToolCallParams>(params.clone())
                    .ok()
            })
            .is_some_and(|params| requires_auth_tool(&params.name))
}

fn requires_auth_tool(name: &str) -> bool {
    matches!(name, "create_data")
}

fn request_org_hint(request: &JsonRpcRequest) -> Option<String> {
    if request.method != "tools/call" {
        return None;
    }
    request
        .params
        .as_ref()
        .and_then(|params| {
            serde_json::from_value::<ToolCallParams>(params.clone()).ok()
        })
        .and_then(|params| {
            params
                .arguments
                .get("org")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn mcp_auth_required() -> bool {
    std::env::var("MCP_AUTH_REQUIRED")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE"))
        .unwrap_or(false)
}

fn mcp_resource_url() -> String {
    std::env::var("MCP_RESOURCE_URL").unwrap_or_else(|_| {
        let base_url = std::env::var("LIBRARY_API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:50053".to_string());
        format!("{}/mcp", base_url.trim_end_matches('/'))
    })
}

fn mcp_resource_metadata_url() -> String {
    std::env::var("MCP_RESOURCE_METADATA_URL").unwrap_or_else(|_| {
        let base_url = std::env::var("LIBRARY_API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:50053".to_string());
        format!(
            "{}/.well-known/oauth-protected-resource",
            base_url.trim_end_matches('/')
        )
    })
}

fn auth_challenge_response() -> Response {
    let challenge = format!(
        "Bearer resource_metadata=\"{}\"",
        mcp_resource_metadata_url()
    );
    (
        StatusCode::UNAUTHORIZED,
        [(WWW_AUTHENTICATE, HeaderValue::from_str(&challenge).unwrap())],
        "Authentication required",
    )
        .into_response()
}

pub async fn protected_resource_metadata() -> Json<Value> {
    let authorization_servers = std::env::var("MCP_AUTHORIZATION_SERVERS")
        .or_else(|_| std::env::var("MCP_AUTHORIZATION_SERVER"))
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    Json(json!({
        "resource": mcp_resource_url(),
        "authorization_servers": authorization_servers,
        "scopes_supported": [
            MCP_AUTH_SCOPE_READ,
            MCP_AUTH_SCOPE_WRITE
        ],
        "bearer_methods_supported": ["header"],
        "resource_name": "Library MCP"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_includes_library_data_tools() {
        let result = tools_list_result(false);
        let tools = result["tools"].as_array().expect("tools list");
        let names = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"list_data"));
        assert!(names.contains(&"search_data"));
        assert!(names.contains(&"get_data"));
        assert!(!names.contains(&"create_data"));
    }

    #[test]
    fn authenticated_tools_list_includes_write_tool() {
        let result = tools_list_result(true);
        let tools = result["tools"].as_array().expect("tools list");
        let names = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"create_data"));
    }

    #[test]
    fn initialize_advertises_tools_capability() {
        let result = initialize_result();

        assert_eq!(result["serverInfo"]["name"], "library-mcp");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tool_text_result_serializes_pretty_json() {
        let result = tool_text_result(json!({ "ok": true }));

        assert_eq!(result["content"][0]["type"], "text");
        assert!(result["content"][0]["text"]
            .as_str()
            .expect("text")
            .contains("\"ok\": true"));
    }
}
