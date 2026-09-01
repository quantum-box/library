use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Extension, Form, Query},
    http::{
        header::{AUTHORIZATION, LOCATION, WWW_AUTHENTICATE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{Html, IntoResponse, Response},
    Json,
};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tachyon_sdk::auth::{ExecutorAction, OperatorId};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::{
    CallerAuthApp, LibraryExecutor, LibraryExecutorKind,
};
use crate::sdk_auth::SdkAuthApp;
use crate::usecase::markdown_composer::compose_markdown;
use crate::usecase::{
    AddDataInputData, AddPropertyInputData, CreateOrganizationInputData,
    CreateRepoInputData, CreateSourceInputData, DeleteDataInputData,
    DeletePropertyInputData, DeleteRepoInputData, DeleteSourceInputData,
    FindSourcesInputData, GetPropertiesInputData, GetSourceInputData,
    LibraryOrg, PropertyDataInputData, PropertyDataValueInputData,
    SearchDataInputData, SearchRepoInputData, UpdateDataInputData,
    UpdateOrganizationInputData, UpdatePropertyInputData,
    UpdateRepoInputData, UpdateSourceInputData, ViewDataInputData,
    ViewDataListInputData, ViewOrgInputData, ViewRepoInputData,
};
use database_manager::domain::{Property, PropertyType};
use value_object::{LongText, Text, Url};

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const MCP_DEFAULT_SCOPES: &[&str] = &["openid", "email", "profile"];
type HmacSha256 = Hmac<Sha256>;

static MCP_OAUTH_STORE: Lazy<Mutex<McpOAuthStore>> =
    Lazy::new(|| Mutex::new(McpOAuthStore::default()));

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    pub(crate) id: Option<Value>,
    pub(crate) method: String,
    params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
struct OrgRepoArgs {
    org: String,
    repo: String,
}

#[derive(Debug, Deserialize)]
struct OrgArgs {
    org: String,
}

#[derive(Debug, Deserialize)]
struct CreateOrgArgs {
    name: String,
    username: String,
    description: Option<String>,
    website: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateOrgArgs {
    org: String,
    name: String,
    description: Option<String>,
    website: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetPropertyArgs {
    org: String,
    repo: String,
    property_id: String,
}

#[derive(Debug, Deserialize)]
struct SearchReposArgs {
    org: Option<String>,
    query: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CreateRepoArgs {
    org: String,
    name: String,
    username: String,
    is_public: bool,
    description: Option<String>,
    #[serde(default)]
    skip_sample_data: bool,
}

#[derive(Debug, Deserialize)]
struct UpdateRepoArgs {
    org: String,
    repo: String,
    name: Option<String>,
    description: Option<String>,
    is_public: Option<bool>,
    tags: Option<Vec<String>>,
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
    value: Value,
    #[serde(default)]
    value_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateDataArgs {
    org: String,
    repo: String,
    data_id: String,
    name: String,
    #[serde(default)]
    property_data: Vec<CreateDataPropertyArgs>,
}

#[derive(Debug, Deserialize)]
struct DeleteDataArgs {
    org: String,
    repo: String,
    data_id: String,
}

#[derive(Debug, Deserialize)]
struct CreatePropertyArgs {
    org: String,
    repo: String,
    name: String,
    property_type: String,
    #[serde(default)]
    meta: Value,
}

#[derive(Debug, Deserialize)]
struct UpdatePropertyArgs {
    org: String,
    repo: String,
    property_id: String,
    name: Option<String>,
    property_type: Option<String>,
    #[serde(default)]
    meta: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct DeletePropertyArgs {
    org: String,
    repo: String,
    property_id: String,
}

#[derive(Debug, Deserialize)]
struct GetSourceArgs {
    org: String,
    repo: String,
    source_id: String,
}

#[derive(Debug, Deserialize)]
struct CreateSourceArgs {
    org: String,
    repo: String,
    name: String,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateSourceArgs {
    org: String,
    repo: String,
    source_id: String,
    name: Option<String>,
    url: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct DeleteSourceArgs {
    org: String,
    repo: String,
    source_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct McpOAuthClientRegistrationRequest {
    redirect_uris: Vec<String>,
    #[serde(default)]
    token_endpoint_auth_method: Option<String>,
    #[serde(default)]
    grant_types: Vec<String>,
    #[serde(default)]
    response_types: Vec<String>,
    #[serde(default)]
    client_name: Option<String>,
    #[serde(default)]
    client_uri: Option<String>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct McpOAuthAuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    code_challenge_method: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    resource: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct McpOAuthAuthorizeForm {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    code_challenge_method: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    resource: Option<String>,
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct McpOAuthTokenRequest {
    grant_type: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    code_verifier: Option<String>,
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    client_id: Option<String>,
}

#[derive(Debug, Default)]
struct McpOAuthStore {
    clients: HashMap<String, McpOAuthClient>,
    codes: HashMap<String, McpOAuthCode>,
}

#[derive(Debug, Clone)]
struct McpOAuthClient {
    redirect_uris: Vec<String>,
    token_endpoint_auth_method: String,
    grant_types: Vec<String>,
    response_types: Vec<String>,
}

#[derive(Debug, Clone)]
struct McpOAuthCode {
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    scope: Option<String>,
    access_token: String,
    expires_in: i64,
    created_at: Instant,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CognitoInitiateAuthResponse {
    authentication_result: Option<CognitoAuthenticationResult>,
    challenge_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CognitoAuthenticationResult {
    access_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CognitoErrorResponse {
    #[serde(rename = "__type")]
    error_type: Option<String>,
    #[serde(alias = "message", alias = "Message")]
    message: Option<String>,
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

#[derive(Debug, Serialize)]
struct McpRepo {
    id: String,
    org: String,
    username: String,
    name: String,
    is_public: bool,
    description: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct McpProperty {
    id: String,
    name: String,
    property_type: String,
    meta: Option<Value>,
}

#[derive(Debug, Serialize)]
struct McpSource {
    id: String,
    repo_id: String,
    name: String,
    url: Option<String>,
}

#[derive(Debug, Serialize)]
struct McpOrganization {
    id: String,
    username: String,
    name: String,
    description: Option<String>,
    website: Option<String>,
}

#[axum::debug_handler]
pub async fn mcp_handler(
    headers: HeaderMap,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Extension(sdk): Extension<Arc<SdkAuthApp>>,
    Json(request): Json<JsonRpcRequest>,
) -> Response {
    match dispatch_rpc(&headers, library_app, sdk, request).await {
        // A notification carries no id, so JSON-RPC has nothing to answer
        // with. The plain HTTP transport still owes the caller a body, and
        // an empty object is what MCP clients expect there.
        Ok(None) => Json(json!({})).into_response(),
        Ok(Some(response)) => Json(response).into_response(),
        Err(challenge) => challenge,
    }
}

/// Authenticate one JSON-RPC request and run it, independent of the
/// transport that carried it. Both `POST /mcp` and the SSE pair
/// (`GET /sse` + `POST /messages`) go through here, so the two transports
/// cannot drift apart on which tools demand credentials.
///
/// `Ok(None)` means the request was a notification and owes no response.
/// `Err` carries the ready-made `401` challenge.
pub(crate) async fn dispatch_rpc(
    headers: &HeaderMap,
    library_app: Arc<LibraryApp>,
    sdk: Arc<SdkAuthApp>,
    request: JsonRpcRequest,
) -> Result<Option<Value>, Response> {
    if should_challenge(headers, &request) {
        return Err(auth_challenge_response());
    }

    let org_hint = request_org_hint(&request);
    let auth = resolve_auth_context(
        headers,
        sdk,
        library_app.clone(),
        org_hint.as_deref(),
    )
    .await;
    if mcp_auth_required() && !auth.is_authenticated() {
        return Err(auth_challenge_response());
    }

    let is_notification = request.id.is_none();
    let response = handle_rpc(library_app, auth, request).await;
    if is_notification {
        return Ok(None);
    }
    Ok(Some(response))
}

/// Whether a stream opened without credentials must be refused. The SSE
/// transport checks this at `GET /sse` so a client learns it needs to
/// authenticate before it holds an open session it can never use.
pub(crate) fn sse_requires_auth(headers: &HeaderMap) -> bool {
    mcp_auth_required() && bearer_token(headers).is_none()
}

pub(crate) fn unauthorized_response() -> Response {
    auth_challenge_response()
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
        "get_org" => {
            let args: OrgArgs = parse_tool_args(params.arguments)?;
            let output = get_org(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "get_property" => {
            let args: GetPropertyArgs = parse_tool_args(params.arguments)?;
            let output = get_property(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "create_org" => {
            let args: CreateOrgArgs = parse_tool_args(params.arguments)?;
            let output = create_org(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "update_org" => {
            let args: UpdateOrgArgs = parse_tool_args(params.arguments)?;
            let output = update_org(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "search_repos" => {
            let args: SearchReposArgs = parse_tool_args(params.arguments)?;
            let output = search_repos(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "get_repo" => {
            let args: OrgRepoArgs = parse_tool_args(params.arguments)?;
            let output = get_repo(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
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
        "list_properties" => {
            let args: OrgRepoArgs = parse_tool_args(params.arguments)?;
            let output = list_properties(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "list_sources" => {
            let args: OrgRepoArgs = parse_tool_args(params.arguments)?;
            let output = list_sources(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "get_source" => {
            let args: GetSourceArgs = parse_tool_args(params.arguments)?;
            let output = get_source(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "create_repo" => {
            let args: CreateRepoArgs = parse_tool_args(params.arguments)?;
            let output = create_repo(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "update_repo" => {
            let args: UpdateRepoArgs = parse_tool_args(params.arguments)?;
            let output = update_repo(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "delete_repo" => {
            let args: OrgRepoArgs = parse_tool_args(params.arguments)?;
            let output = delete_repo(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "create_data" => {
            let args: CreateDataArgs = parse_tool_args(params.arguments)?;
            let output = create_data(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "update_data" => {
            let args: UpdateDataArgs = parse_tool_args(params.arguments)?;
            let output = update_data(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "delete_data" => {
            let args: DeleteDataArgs = parse_tool_args(params.arguments)?;
            let output = delete_data(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "create_property" => {
            let args: CreatePropertyArgs =
                parse_tool_args(params.arguments)?;
            let output = create_property(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "update_property" => {
            let args: UpdatePropertyArgs =
                parse_tool_args(params.arguments)?;
            let output = update_property(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "delete_property" => {
            let args: DeletePropertyArgs =
                parse_tool_args(params.arguments)?;
            let output = delete_property(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "create_source" => {
            let args: CreateSourceArgs = parse_tool_args(params.arguments)?;
            let output = create_source(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "update_source" => {
            let args: UpdateSourceArgs = parse_tool_args(params.arguments)?;
            let output = update_source(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "delete_source" => {
            let args: DeleteSourceArgs = parse_tool_args(params.arguments)?;
            let output = delete_source(library_app, auth, args).await?;
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

async fn get_org(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: OrgArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = LibraryOrg::with_org(args.org.clone());
    let input = ViewOrgInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        organization_username: args.org,
    };
    let output = library_app
        .view_org
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;
    let repos = output.repos.iter().map(repo_to_mcp).collect::<Vec<_>>();

    Ok(json!({
        "organization": organization_to_mcp(&output.organization),
        "repos": repos,
    }))
}

async fn create_org(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: CreateOrgArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "create_org")?;
    // A new organization has no tenancy of its own yet, so it is created
    // against the Library platform tenant the same way `POST /v1beta/orgs`
    // does.
    let multi_tenancy = tachyon_sdk::auth::MultiTenancy::new_platform(
        crate::LIBRARY_TENANT.clone(),
    );
    let input = CreateOrganizationInputData {
        executor: &executor,
        multi_tenancy: &multi_tenancy,
        name: args.name,
        username: args.username,
        description: args.description,
        website: args.website,
    };
    let organization = library_app
        .create_organization
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "organization": organization_to_mcp(&organization) }))
}

async fn update_org(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: UpdateOrgArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "update_org")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let input = UpdateOrganizationInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        username: args.org,
        name: args.name,
        description: args.description,
        website: args.website,
    };
    let output = library_app
        .update_organization
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({
        "organization": organization_to_mcp(&output.organization),
    }))
}

async fn get_property(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: GetPropertyArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = LibraryOrg::with_org(args.org.clone());
    let input = GetPropertiesInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
    };
    let properties = library_app
        .get_properties
        .execute(input)
        .await
        .map_err(tool_execution_error)?;
    let property = properties
        .iter()
        .find(|property| *property.id() == args.property_id)
        .ok_or_else(|| {
            json_rpc_error(
                -32000,
                format!("Property not found: {}", args.property_id),
            )
        })?;

    Ok(json!({ "property": property_to_mcp(property) }))
}

async fn search_repos(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: SearchReposArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = args
        .org
        .clone()
        .map(LibraryOrg::with_org)
        .unwrap_or_default();
    let input = SearchRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        name: args.query,
        limit: Some(args.limit.unwrap_or(20).clamp(1, 100)),
    };
    let repos = library_app
        .search_repo
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;
    let repos = repos.iter().map(repo_to_mcp).collect::<Vec<_>>();

    Ok(json!({ "repos": repos }))
}

async fn get_repo(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: OrgRepoArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = LibraryOrg::with_org(args.org.clone());
    let input = ViewRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        organization_username: args.org,
        repo_username: args.repo,
    };
    let output = library_app
        .view_repo
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "repo": repo_to_mcp(&output.repo) }))
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

async fn list_properties(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: OrgRepoArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = LibraryOrg::with_org(args.org.clone());
    let input = GetPropertiesInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
    };
    let properties = library_app
        .get_properties
        .execute(input)
        .await
        .map_err(tool_execution_error)?;
    let properties =
        properties.iter().map(property_to_mcp).collect::<Vec<_>>();

    Ok(json!({ "properties": properties }))
}

async fn list_sources(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: OrgRepoArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = LibraryOrg::with_org(args.org.clone());
    let repo = library_app
        .view_repo
        .execute(&ViewRepoInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: args.org.clone(),
            repo_username: args.repo.clone(),
        })
        .await
        .map_err(tool_execution_error)?
        .repo;
    let input = FindSourcesInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        repo_id: repo.id(),
        org_username: args.org,
        repo_username: args.repo,
    };
    let sources = library_app
        .find_sources
        .execute(input)
        .await
        .map_err(tool_execution_error)?;
    let sources = sources.iter().map(source_to_mcp).collect::<Vec<_>>();

    Ok(json!({ "sources": sources }))
}

async fn get_source(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: GetSourceArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = LibraryOrg::with_org(args.org.clone());
    let source_id = args.source_id.parse().map_err(invalid_tool_arg)?;
    let input = GetSourceInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        source_id: &source_id,
        org_username: args.org,
        repo_username: args.repo,
    };
    let source = library_app
        .get_source
        .execute(input)
        .await
        .map_err(tool_execution_error)?
        .ok_or_else(|| json_rpc_error(-32004, "source not found"))?;

    Ok(json!({ "source": source_to_mcp(&source) }))
}

async fn create_repo(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: CreateRepoArgs,
) -> Result<Value, Value> {
    let caller_auth = auth
        .caller_auth
        .as_ref()
        .ok_or_else(|| {
            json_rpc_error(
                -32001,
                "Authentication required for create_repo",
            )
        })?
        .auth_app();
    let executor = require_executor(auth, "create_repo")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let input = CreateRepoInputData {
        auth: caller_auth,
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_name: args.name,
        repo_username: args.username,
        user_id: executor.get_id().to_string(),
        is_public: args.is_public,
        description: args.description,
        database_id: None,
        skip_sample_data: args.skip_sample_data,
    };
    let repo = library_app
        .create_repo
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "repo": repo_to_mcp(&repo) }))
}

async fn update_repo(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: UpdateRepoArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "update_repo")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let name = parse_optional::<Text>(args.name)?;
    let description = parse_optional::<LongText>(args.description)?;
    let tags = args
        .tags
        .map(|tags| {
            tags.into_iter()
                .map(|tag| tag.parse::<Text>().map_err(invalid_tool_arg))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let input = UpdateRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
        name,
        description,
        is_public: args.is_public,
        tags,
    };
    let repo = library_app
        .update_repo
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "repo": repo_to_mcp(&repo) }))
}

async fn delete_repo(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: OrgRepoArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "delete_repo")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let input = DeleteRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
    };
    library_app
        .delete_repo
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "deleted": true }))
}

async fn create_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: CreateDataArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "create_data")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let property_data = property_data_from_args(args.property_data)?;

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

async fn update_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: UpdateDataArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "update_data")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let property_data = property_data_from_args(args.property_data)?;
    let input = UpdateDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        actor: executor.get_id(),
        org_username: &args.org,
        repo_username: &args.repo,
        data_id: &args.data_id,
        data_name: &args.name,
        property_data,
    };
    let (data, properties) = library_app
        .update_data
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

async fn delete_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: DeleteDataArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "delete_data")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let input = DeleteDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        actor: executor.get_id().to_string(),
        org_username: args.org,
        repo_username: args.repo,
        data_id: args.data_id,
    };
    library_app
        .delete_data
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "deleted": true }))
}

async fn create_property(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: CreatePropertyArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "create_property")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let property_type =
        property_type_from_value(&args.property_type, args.meta)?;
    let input = AddPropertyInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
        property_name: args.name,
        property_type,
    };
    let property = library_app
        .add_property
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "property": property_to_mcp(&property) }))
}

async fn update_property(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: UpdatePropertyArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "update_property")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let property_type = args
        .property_type
        .as_deref()
        .map(|typ| {
            property_type_from_value(
                typ,
                args.meta.clone().unwrap_or(Value::Null),
            )
        })
        .transpose()?;
    let meta_json = args
        .meta
        .map(|meta| {
            if meta.is_null() {
                Ok(None)
            } else {
                serde_json::to_string(&meta)
                    .map(Some)
                    .map_err(invalid_tool_arg)
            }
        })
        .transpose()?;
    let input = UpdatePropertyInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
        property_id: args.property_id,
        property_name: args.name,
        property_type: property_type.as_ref(),
        meta_json,
    };
    let property = library_app
        .update_property
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "property": property_to_mcp(&property) }))
}

async fn delete_property(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: DeletePropertyArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "delete_property")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let input = DeletePropertyInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
        property_id: args.property_id,
    };
    let property = library_app
        .delete_property
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "deleted": true, "property": property_to_mcp(&property) }))
}

async fn create_source(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: CreateSourceArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "create_source")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let name = args.name.parse::<Text>().map_err(invalid_tool_arg)?;
    let url = parse_optional::<Url>(args.url)?;
    let input = CreateSourceInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org,
        repo_username: args.repo,
        name: &name,
        url,
    };
    let source = library_app
        .create_source
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "source": source_to_mcp(&source) }))
}

async fn update_source(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: UpdateSourceArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "update_source")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let source_id = args.source_id.parse().map_err(invalid_tool_arg)?;
    let name = parse_optional::<Text>(args.name)?;
    let url = args
        .url
        .map(|value| match value {
            Some(value) => {
                value.parse::<Url>().map(Some).map_err(invalid_tool_arg)
            }
            None => Ok(None),
        })
        .transpose()?;
    let input = UpdateSourceInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        source_id: &source_id,
        org_username: args.org,
        repo_username: args.repo,
        name,
        url,
    };
    let source = library_app
        .update_source
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "source": source_to_mcp(&source) }))
}

async fn delete_source(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: DeleteSourceArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "delete_source")?;
    let library_org = authenticated_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let source_id = args.source_id.parse().map_err(invalid_tool_arg)?;
    let input = DeleteSourceInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        source_id: &source_id,
        org_username: args.org,
        repo_username: args.repo,
    };
    library_app
        .delete_source
        .execute(input)
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "deleted": true }))
}

fn read_executor(auth: &McpAuthContext) -> LibraryExecutor {
    auth.executor.clone().unwrap_or_else(anonymous_executor)
}

fn require_executor(
    auth: McpAuthContext,
    tool_name: &str,
) -> Result<LibraryExecutor, Value> {
    auth.executor.ok_or_else(|| {
        json_rpc_error(
            -32001,
            format!("Authentication required for {tool_name}"),
        )
    })
}

fn property_data_from_args(
    properties: Vec<CreateDataPropertyArgs>,
) -> Result<Vec<PropertyDataInputData>, Value> {
    properties
        .into_iter()
        .map(|property| {
            Ok(PropertyDataInputData {
                property_id: property.property_id,
                value: property_data_value(
                    property.value,
                    property.value_type.as_deref(),
                )?,
            })
        })
        .collect()
}

fn property_data_value(
    value: Value,
    value_type: Option<&str>,
) -> Result<PropertyDataValueInputData, Value> {
    let as_string = |value: Value| -> Result<String, Value> {
        match value {
            Value::String(value) => Ok(value),
            value => Ok(value.to_string()),
        }
    };
    match value_type.unwrap_or("string") {
        "integer" => {
            Ok(PropertyDataValueInputData::Integer(as_string(value)?))
        }
        "html" => Ok(PropertyDataValueInputData::Html(as_string(value)?)),
        "markdown" => {
            Ok(PropertyDataValueInputData::Markdown(as_string(value)?))
        }
        "rich_text" => {
            Ok(PropertyDataValueInputData::RichText(as_string(value)?))
        }
        "relation" => {
            let values = serde_json::from_value::<Vec<String>>(value)
                .map_err(invalid_tool_arg)?;
            Ok(PropertyDataValueInputData::Relation(values))
        }
        "select" => {
            Ok(PropertyDataValueInputData::Select(as_string(value)?))
        }
        "multi_select" => {
            let values = serde_json::from_value::<Vec<String>>(value)
                .map_err(invalid_tool_arg)?;
            Ok(PropertyDataValueInputData::MultiSelect(values))
        }
        "boolean" => {
            let flag = match &value {
                Value::Bool(flag) => *flag,
                Value::String(text) => match text.trim() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(json_rpc_error(
                            -32602,
                            "boolean value must be true or false",
                        ));
                    }
                },
                _ => {
                    return Err(json_rpc_error(
                        -32602,
                        "boolean value must be true or false",
                    ));
                }
            };
            Ok(PropertyDataValueInputData::Boolean(flag))
        }
        "date" => Ok(PropertyDataValueInputData::Date(as_string(value)?)),
        "image" => Ok(PropertyDataValueInputData::Image(as_string(value)?)),
        "string" => {
            Ok(PropertyDataValueInputData::String(as_string(value)?))
        }
        other => Err(json_rpc_error(
            -32602,
            format!("Unsupported property value_type: {other}"),
        )),
    }
}

fn property_type_from_value(
    typ: &str,
    meta: Value,
) -> Result<PropertyType, Value> {
    let typ = typ.trim().replace('-', "_").to_ascii_uppercase();
    PropertyType::from_meta(&typ, meta).map_err(tool_execution_error)
}

fn parse_optional<T>(value: Option<String>) -> Result<Option<T>, Value>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .map(|value| value.parse::<T>().map_err(invalid_tool_arg))
        .transpose()
}

fn repo_to_mcp(repo: &crate::domain::Repo) -> McpRepo {
    McpRepo {
        id: repo.id().to_string(),
        org: repo.org_username().to_string(),
        username: repo.username().to_string(),
        name: repo.name().to_string(),
        is_public: *repo.is_public(),
        description: repo.description().as_ref().map(ToString::to_string),
        tags: repo.tags().iter().map(ToString::to_string).collect(),
    }
}

fn property_to_mcp(property: &Property) -> McpProperty {
    McpProperty {
        id: property.id().to_string(),
        name: property.name().to_string(),
        property_type: property.property_type().to_string(),
        meta: property
            .meta_json()
            .as_deref()
            .and_then(|meta| serde_json::from_str(meta).ok()),
    }
}

fn organization_to_mcp(
    organization: &crate::domain::Organization,
) -> McpOrganization {
    McpOrganization {
        id: organization.id().to_string(),
        username: organization.username().to_string(),
        name: organization.name().to_string(),
        description: organization
            .description()
            .as_ref()
            .map(ToString::to_string),
        website: organization.website().as_ref().map(ToString::to_string),
    }
}

fn source_to_mcp(source: &crate::domain::Source) -> McpSource {
    McpSource {
        id: source.id().to_string(),
        repo_id: source.repo_id().to_string(),
        name: source.name().to_string(),
        url: source.url().as_ref().map(ToString::to_string),
    }
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
            "name": "get_org",
            "description": "Get one Library organization and the repositories it owns.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" }
                },
                "required": ["org"]
            }
        }),
        json!({
            "name": "search_repos",
            "description": "Search Library repositories within one organization you belong to.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                },
                "required": ["org"]
            }
        }),
        json!({
            "name": "get_repo",
            "description": "Get one Library repository.",
            "inputSchema": org_repo_schema()
        }),
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
        json!({
            "name": "list_properties",
            "description": "List properties for a Library repository.",
            "inputSchema": org_repo_schema()
        }),
        json!({
            "name": "get_property",
            "description": "Get one property of a Library repository, including its type and meta.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "repo": { "type": "string" },
                    "property_id": { "type": "string" }
                },
                "required": ["org", "repo", "property_id"]
            }
        }),
        json!({
            "name": "list_sources",
            "description": "List sources attached to a Library repository.",
            "inputSchema": org_repo_schema()
        }),
        json!({
            "name": "get_source",
            "description": "Get one source attached to a Library repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "repo": { "type": "string" },
                    "source_id": { "type": "string" }
                },
                "required": ["org", "repo", "source_id"]
            }
        }),
    ];

    if is_authenticated {
        tools.extend([
            json!({
                "name": "create_org",
                "description": "Create a Library organization.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "username": { "type": "string" },
                        "description": { "type": "string" },
                        "website": { "type": "string" }
                    },
                    "required": ["name", "username"]
                }
            }),
            json!({
                "name": "update_org",
                "description": "Update a Library organization. `name` is required; omitting `description` or `website` clears it.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "name": { "type": "string" },
                        "description": { "type": "string" },
                        "website": { "type": "string" }
                    },
                    "required": ["org", "name"]
                }
            }),
            json!({
                "name": "create_repo",
                "description": "Create a Library repository.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "name": { "type": "string" },
                        "username": { "type": "string" },
                        "is_public": { "type": "boolean" },
                        "description": { "type": "string" },
                        "skip_sample_data": { "type": "boolean" }
                    },
                    "required": ["org", "name", "username", "is_public"]
                }
            }),
            json!({
                "name": "update_repo",
                "description": "Update repository settings.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "repo": { "type": "string" },
                        "name": { "type": "string" },
                        "description": { "type": "string" },
                        "is_public": { "type": "boolean" },
                        "tags": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["org", "repo"]
                }
            }),
            json!({
                "name": "delete_repo",
                "description": "Delete a Library repository.",
                "inputSchema": org_repo_schema()
            }),
            json!({
                "name": "create_data",
                "description": "Create a Library data record.",
                "inputSchema": data_write_schema(["org", "repo", "name"])
            }),
            json!({
                "name": "update_data",
                "description": "Update a Library data record.",
                "inputSchema": data_write_schema(["org", "repo", "data_id", "name"])
            }),
            json!({
                "name": "delete_data",
                "description": "Delete a Library data record.",
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
            json!({
                "name": "create_property",
                "description": "Create a repository property.",
                "inputSchema": property_write_schema(["org", "repo", "name", "property_type"])
            }),
            json!({
                "name": "update_property",
                "description": "Update a repository property.",
                "inputSchema": property_write_schema(["org", "repo", "property_id"])
            }),
            json!({
                "name": "delete_property",
                "description": "Delete a repository property.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "repo": { "type": "string" },
                        "property_id": { "type": "string" }
                    },
                    "required": ["org", "repo", "property_id"]
                }
            }),
            json!({
                "name": "create_source",
                "description": "Create a repository source.",
                "inputSchema": source_write_schema(["org", "repo", "name"])
            }),
            json!({
                "name": "update_source",
                "description": "Update a repository source. Set url to null to clear it.",
                "inputSchema": source_write_schema(["org", "repo", "source_id"])
            }),
            json!({
                "name": "delete_source",
                "description": "Delete a repository source.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "repo": { "type": "string" },
                        "source_id": { "type": "string" }
                    },
                    "required": ["org", "repo", "source_id"]
                }
            }),
        ]);
    }

    json!({ "tools": tools })
}

fn org_repo_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "org": { "type": "string" },
            "repo": { "type": "string" }
        },
        "required": ["org", "repo"]
    })
}

fn data_write_schema<const N: usize>(required: [&str; N]) -> Value {
    let required = required.to_vec();
    json!({
        "type": "object",
        "properties": {
            "org": { "type": "string" },
            "repo": { "type": "string" },
            "data_id": { "type": "string" },
            "name": { "type": "string" },
            "property_data": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "property_id": { "type": "string" },
                        "value": {},
                        "value_type": {
                            "type": "string",
                            "enum": [
                                "string",
                                "integer",
                                "html",
                                "markdown",
                                "relation",
                                "select",
                                "multi_select",
                                "date",
                                "image",
                                "rich_text",
                                "boolean"
                            ]
                        }
                    },
                    "required": ["property_id", "value"]
                }
            }
        },
        "required": required
    })
}

fn property_write_schema<const N: usize>(required: [&str; N]) -> Value {
    let required = required.to_vec();
    json!({
        "type": "object",
        "properties": {
            "org": { "type": "string" },
            "repo": { "type": "string" },
            "property_id": { "type": "string" },
            "name": { "type": "string" },
            "property_type": {
                "type": "string",
                "enum": [
                    "string",
                    "integer",
                    "html",
                    "markdown",
                    "relation",
                    "select",
                    "multi_select",
                    "id",
                    "location",
                    "date",
                    "image",
                    "rich_text",
                    "boolean"
                ]
            },
            "meta": {}
        },
        "required": required
    })
}

fn source_write_schema<const N: usize>(required: [&str; N]) -> Value {
    let required = required.to_vec();
    json!({
        "type": "object",
        "properties": {
            "org": { "type": "string" },
            "repo": { "type": "string" },
            "source_id": { "type": "string" },
            "name": { "type": "string" },
            "url": { "type": ["string", "null"] }
        },
        "required": required
    })
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

fn invalid_tool_arg(err: impl std::fmt::Display) -> Value {
    json_rpc_error(-32602, err.to_string())
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
    caller_auth: Option<CallerAuthApp>,
    accepted_credentials: bool,
    write_tools_available: bool,
}

impl McpAuthContext {
    fn anonymous() -> Self {
        Self {
            executor: None,
            caller_auth: None,
            accepted_credentials: false,
            write_tools_available: false,
        }
    }

    fn authenticated(
        executor: LibraryExecutor,
        caller_auth: CallerAuthApp,
    ) -> Self {
        Self {
            executor: Some(executor),
            caller_auth: Some(caller_auth),
            accepted_credentials: true,
            write_tools_available: true,
        }
    }

    fn accepted_without_executor(write_tools_available: bool) -> Self {
        Self {
            executor: None,
            caller_auth: None,
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
        // A key is verified against the organization that issued it, so
        // something has to name that organization. `tools/call` names it
        // in the tool arguments, but every other method names nothing —
        // `tools/list` above all, which is what an operator runs to see
        // whether a key reaches the write tools. Let the caller state it
        // through `x-operator-id`, exactly as the REST routes already
        // allow. With neither, there is nothing to verify the key
        // against and the request stays anonymous.
        let operator_id = match org_username {
            Some(org_username) => library_app
                .view_org
                .execute(&ViewOrgInputData {
                    executor: &tachyon_sdk::auth::Executor::SystemUser,
                    multi_tenancy: &LibraryOrg::with_org(
                        org_username.to_string(),
                    ),
                    organization_username: org_username.to_string(),
                })
                .await
                .ok()
                .map(|org| org.organization.id().clone()),
            None => operator_id_header(headers),
        };

        if let Some(operator_id) = operator_id {
            if let Ok(service_account) =
                sdk.verify_api_key(&operator_id, &token).await
            {
                let executor = LibraryExecutor {
                    inner: LibraryExecutorKind::ServiceAccount(Box::new(
                        service_account,
                    )),
                    original_token: Some(token),
                };
                let caller_auth = executor
                    .caller_auth_app(&sdk)
                    .expect("authenticated MCP executor has a token");
                return McpAuthContext::authenticated(
                    executor,
                    caller_auth,
                );
            }
        }
        return McpAuthContext::anonymous();
    }

    match sdk.verify_token(&token).await {
        Ok(user) => {
            let executor = LibraryExecutor {
                inner: LibraryExecutorKind::User(Box::new(user)),
                original_token: Some(token),
            };
            let caller_auth = executor
                .caller_auth_app(&sdk)
                .expect("authenticated MCP executor has a token");
            McpAuthContext::authenticated(executor, caller_auth)
        }
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

/// The organization a caller names when the request itself does not.
/// Mirrors the REST executor extractor so one key behaves the same way
/// on both surfaces.
fn operator_id_header(headers: &HeaderMap) -> Option<OperatorId> {
    headers
        .get("x-operator-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<OperatorId>().ok())
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
        .unwrap_or_else(|_| mcp_oauth_issuer())
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let scopes_supported = mcp_scopes_supported();

    Json(json!({
        "resource": mcp_resource_url(),
        "authorization_servers": authorization_servers,
        "scopes_supported": scopes_supported,
        "bearer_methods_supported": ["header"],
        "resource_name": "Library MCP"
    }))
}

pub async fn mcp_oauth_authorization_server_metadata() -> Json<Value> {
    let issuer = mcp_oauth_issuer();
    let scopes_supported = mcp_scopes_supported();

    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "registration_endpoint": format!("{issuer}/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "scopes_supported": scopes_supported
    }))
}

pub async fn mcp_oauth_register(
    Json(request): Json<McpOAuthClientRegistrationRequest>,
) -> Response {
    if request.redirect_uris.is_empty() {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "redirect_uris is required",
        );
    }

    let client_id = format!("mcp_client_{}", Uuid::new_v4().simple());
    let token_endpoint_auth_method = request
        .token_endpoint_auth_method
        .clone()
        .unwrap_or_else(|| "none".to_string());
    if token_endpoint_auth_method != "none" {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "Only public clients with token_endpoint_auth_method=none are supported",
        );
    }

    let client = McpOAuthClient {
        redirect_uris: request.redirect_uris.clone(),
        token_endpoint_auth_method,
        grant_types: if request.grant_types.is_empty() {
            vec!["authorization_code".to_string()]
        } else {
            request.grant_types.clone()
        },
        response_types: if request.response_types.is_empty() {
            vec!["code".to_string()]
        } else {
            request.response_types.clone()
        },
    };

    MCP_OAUTH_STORE
        .lock()
        .await
        .clients
        .insert(client_id.clone(), client);

    Json(json!({
        "client_id": client_id,
        "client_id_issued_at": chrono::Utc::now().timestamp(),
        "redirect_uris": request.redirect_uris,
        "token_endpoint_auth_method": "none",
        "grant_types": request.grant_types,
        "response_types": request.response_types,
        "client_name": request.client_name,
        "client_uri": request.client_uri,
        "scope": request.scope
    }))
    .into_response()
}

pub async fn mcp_oauth_authorize(
    Query(query): Query<McpOAuthAuthorizeQuery>,
) -> Response {
    if let Err(message) = validate_authorize_request(&query).await {
        return Html(render_login_page(&query, Some(&message)))
            .into_response();
    }

    Html(render_login_page(&query, None)).into_response()
}

pub async fn mcp_oauth_authorize_submit(
    Form(form): Form<McpOAuthAuthorizeForm>,
) -> Response {
    let query = McpOAuthAuthorizeQuery {
        response_type: form.response_type.clone(),
        client_id: form.client_id.clone(),
        redirect_uri: form.redirect_uri.clone(),
        code_challenge: form.code_challenge.clone(),
        code_challenge_method: form.code_challenge_method.clone(),
        state: form.state.clone(),
        scope: form.scope.clone(),
        resource: form.resource.clone(),
    };

    if let Err(message) = validate_authorize_request(&query).await {
        return Html(render_login_page(&query, Some(&message)))
            .into_response();
    }

    let auth =
        match cognito_user_password_auth(&form.username, &form.password)
            .await
        {
            Ok(auth) => auth,
            Err(message) => {
                return Html(render_login_page(&query, Some(&message)))
                    .into_response()
            }
        };

    let code = format!("mcp_code_{}", Uuid::new_v4().simple());
    MCP_OAUTH_STORE.lock().await.codes.insert(
        code.clone(),
        McpOAuthCode {
            client_id: form.client_id,
            redirect_uri: form.redirect_uri.clone(),
            code_challenge: form.code_challenge,
            scope: form.scope.clone(),
            access_token: auth
                .access_token
                .expect("cognito auth checked access token"),
            expires_in: auth.expires_in.unwrap_or(3600),
            created_at: Instant::now(),
        },
    );

    match redirect_with_code(
        &form.redirect_uri,
        &code,
        form.state.as_deref(),
    ) {
        Ok(location) => (
            StatusCode::FOUND,
            [(LOCATION, HeaderValue::from_str(&location).unwrap())],
        )
            .into_response(),
        Err(message) => {
            Html(render_login_page(&query, Some(&message))).into_response()
        }
    }
}

pub async fn mcp_oauth_token(
    Form(request): Form<McpOAuthTokenRequest>,
) -> Response {
    if request.grant_type != "authorization_code" {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "Only authorization_code is supported",
        );
    }

    let Some(code) = request.code else {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code is required",
        );
    };
    let Some(code_verifier) = request.code_verifier else {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_verifier is required",
        );
    };

    let stored = MCP_OAUTH_STORE.lock().await.codes.remove(&code);
    let Some(stored) = stored else {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "Authorization code is invalid or already used",
        );
    };

    if stored.created_at.elapsed().as_secs() > 600 {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "Authorization code expired",
        );
    }
    if request
        .client_id
        .as_deref()
        .is_some_and(|client_id| client_id != stored.client_id)
    {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "client_id does not match authorization code",
        );
    }
    if request
        .redirect_uri
        .as_deref()
        .is_some_and(|redirect_uri| redirect_uri != stored.redirect_uri)
    {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "redirect_uri does not match authorization code",
        );
    }
    if !verify_pkce(&code_verifier, &stored.code_challenge) {
        return oauth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "PKCE verification failed",
        );
    }

    Json(json!({
        "access_token": stored.access_token,
        "token_type": "Bearer",
        "expires_in": stored.expires_in,
        "scope": stored.scope.unwrap_or_else(|| mcp_scopes_supported().join(" "))
    }))
    .into_response()
}

fn mcp_oauth_issuer() -> String {
    std::env::var("MCP_OAUTH_ISSUER").unwrap_or_else(|_| {
        let base_url = std::env::var("LIBRARY_API_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:50053".to_string());
        format!("{}/mcp/oauth", base_url.trim_end_matches('/'))
    })
}

fn mcp_scopes_supported() -> Vec<String> {
    std::env::var("MCP_SCOPES_SUPPORTED")
        .ok()
        .map(|value| csv_env(&value))
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| {
            MCP_DEFAULT_SCOPES
                .iter()
                .map(|scope| (*scope).to_string())
                .collect()
        })
}

fn csv_env(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

async fn validate_authorize_request(
    request: &McpOAuthAuthorizeQuery,
) -> Result<(), String> {
    if request.response_type != "code" {
        return Err("Only response_type=code is supported.".to_string());
    }
    if request.code_challenge_method != "S256" {
        return Err("Only PKCE S256 is supported.".to_string());
    }

    let store = MCP_OAUTH_STORE.lock().await;
    let client = store
        .clients
        .get(&request.client_id)
        .ok_or_else(|| "OAuth client is not registered.".to_string())?;
    if !client.redirect_uris.contains(&request.redirect_uri) {
        return Err(
            "redirect_uri is not registered for this client.".to_string()
        );
    }
    if client.token_endpoint_auth_method != "none" {
        return Err("Only public OAuth clients are supported.".to_string());
    }
    if !client
        .grant_types
        .iter()
        .any(|grant| grant == "authorization_code")
    {
        return Err(
            "OAuth client does not allow authorization_code.".to_string()
        );
    }
    if !client
        .response_types
        .iter()
        .any(|response| response == "code")
    {
        return Err(
            "OAuth client does not allow response_type=code.".to_string()
        );
    }

    Ok(())
}

async fn cognito_user_password_auth(
    username: &str,
    password: &str,
) -> Result<CognitoAuthenticationResult, String> {
    let client_id = env_first(&[
        "MCP_COGNITO_CLIENT_ID",
        "COGNITO_CLIENT_ID",
        "VITE_COGNITO_CLIENT_ID",
    ])
    .ok_or_else(|| {
        "MCP_COGNITO_CLIENT_ID is not configured.".to_string()
    })?;
    let client_secret =
        env_first(&["MCP_COGNITO_CLIENT_SECRET", "COGNITO_CLIENT_SECRET"]);
    let region = env_first(&[
        "MCP_COGNITO_REGION",
        "COGNITO_REGION",
        "VITE_COGNITO_REGION",
    ])
    .unwrap_or_else(|| "ap-northeast-1".to_string());

    let mut auth_parameters = json!({
        "USERNAME": username,
        "PASSWORD": password
    });
    if let Some(secret) =
        client_secret.as_deref().filter(|value| !value.is_empty())
    {
        auth_parameters["SECRET_HASH"] =
            json!(cognito_secret_hash(username, &client_id, secret)?);
    }

    let endpoint = format!("https://cognito-idp.{region}.amazonaws.com/");
    let response = reqwest::Client::new()
        .post(endpoint)
        .header(
            "X-Amz-Target",
            "AWSCognitoIdentityProviderService.InitiateAuth",
        )
        .header("Content-Type", "application/x-amz-json-1.1")
        .json(&json!({
            "AuthFlow": "USER_PASSWORD_AUTH",
            "ClientId": client_id,
            "AuthParameters": auth_parameters
        }))
        .send()
        .await
        .map_err(|error| format!("Cognito request failed: {error}"))?;

    let status = response.status();
    let body = response.text().await.map_err(|error| {
        format!("Cognito response read failed: {error}")
    })?;
    if !status.is_success() {
        let detail = serde_json::from_str::<CognitoErrorResponse>(&body)
            .ok()
            .and_then(|error| {
                error
                    .message
                    .or(error.error_type)
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_else(|| {
                "Cognito authentication failed.".to_string()
            });
        return Err(detail);
    }

    let response: CognitoInitiateAuthResponse = serde_json::from_str(&body)
        .map_err(|error| {
            format!("Cognito response parse failed: {error}")
        })?;
    let auth = response.authentication_result.ok_or_else(|| {
        response
            .challenge_name
            .map(|challenge| {
                format!(
                    "Cognito returned unsupported challenge: {challenge}"
                )
            })
            .unwrap_or_else(|| "Cognito did not return tokens.".to_string())
    })?;
    if auth.access_token.is_none() {
        return Err("Cognito did not return an access token.".to_string());
    }
    Ok(auth)
}

fn env_first(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .filter(|value| !value.is_empty())
}

fn cognito_secret_hash(
    username: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<String, String> {
    let mut mac = HmacSha256::new_from_slice(client_secret.as_bytes())
        .map_err(|error| {
            format!("Invalid Cognito client secret: {error}")
        })?;
    mac.update(format!("{username}{client_id}").as_bytes());
    Ok(STANDARD.encode(mac.finalize().into_bytes()))
}

fn verify_pkce(code_verifier: &str, expected_challenge: &str) -> bool {
    let digest = Sha256::digest(code_verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest) == expected_challenge
}

fn redirect_with_code(
    redirect_uri: &str,
    code: &str,
    state: Option<&str>,
) -> Result<String, String> {
    let mut url = url::Url::parse(redirect_uri)
        .map_err(|error| format!("Invalid redirect_uri: {error}"))?;
    url.query_pairs_mut().append_pair("code", code);
    if let Some(state) = state {
        url.query_pairs_mut().append_pair("state", state);
    }
    Ok(url.to_string())
}

fn oauth_error_response(
    status: StatusCode,
    error: &str,
    description: &str,
) -> Response {
    (
        status,
        Json(json!({
            "error": error,
            "error_description": description
        })),
    )
        .into_response()
}

fn render_login_page(
    query: &McpOAuthAuthorizeQuery,
    error: Option<&str>,
) -> String {
    let error_html = error
        .map(|message| {
            format!("<p class=\"error\">{}</p>", html_escape(message))
        })
        .unwrap_or_default();
    let hidden = |name: &str, value: &str| {
        format!(
            "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
            html_escape(name),
            html_escape(value)
        )
    };
    let optional_hidden = |name: &str, value: &Option<String>| {
        value
            .as_deref()
            .map(|value| hidden(name, value))
            .unwrap_or_default()
    };

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Library MCP Login</title>
  <style>
    :root {{ color-scheme: light; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    body {{ margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f6f7f9; color: #17181c; }}
    main {{ width: min(420px, calc(100vw - 32px)); background: #fff; border: 1px solid #d9dde5; border-radius: 8px; padding: 28px; box-shadow: 0 16px 40px rgba(20, 28, 40, .08); }}
    h1 {{ font-size: 22px; margin: 0 0 6px; letter-spacing: 0; }}
    p {{ margin: 0 0 20px; color: #596070; line-height: 1.5; }}
    label {{ display: block; font-size: 13px; font-weight: 650; margin: 16px 0 6px; }}
    input[type="text"], input[type="password"] {{ box-sizing: border-box; width: 100%; height: 42px; border: 1px solid #c9ced8; border-radius: 6px; padding: 0 12px; font-size: 15px; }}
    button {{ width: 100%; height: 42px; margin-top: 22px; border: 0; border-radius: 6px; background: #17181c; color: #fff; font-weight: 700; font-size: 15px; cursor: pointer; }}
    .error {{ margin: 0 0 16px; color: #b42318; background: #fff1f0; border: 1px solid #ffccc7; border-radius: 6px; padding: 10px 12px; }}
  </style>
</head>
<body>
  <main>
    <h1>Library MCP</h1>
    <p>Sign in with your Library account to authorize this MCP client.</p>
    {error_html}
    <form method="post" action="/mcp/oauth/authorize">
      {}
      {}
      {}
      {}
      {}
      {}
      {}
      {}
      <label for="username">Username or email</label>
      <input id="username" name="username" type="text" autocomplete="username" required autofocus>
      <label for="password">Password</label>
      <input id="password" name="password" type="password" autocomplete="current-password" required>
      <button type="submit">Sign in</button>
    </form>
  </main>
</body>
</html>"#,
        hidden("response_type", &query.response_type),
        hidden("client_id", &query.client_id),
        hidden("redirect_uri", &query.redirect_uri),
        hidden("code_challenge", &query.code_challenge),
        hidden("code_challenge_method", &query.code_challenge_method),
        optional_hidden("state", &query.state),
        optional_hidden("scope", &query.scope),
        optional_hidden("resource", &query.resource),
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `pk_` key is verified against the organization that issued it,
    /// and only `tools/call` names one in its arguments. Without this
    /// header there is nothing for `tools/list` to verify against, so an
    /// operator checking a key's reach saw the anonymous tool list no
    /// matter how privileged the key was.
    #[test]
    fn an_operator_id_header_names_the_organization_to_verify_against() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-operator-id",
            "tn_01kxz0ytmhnab5vh53011cwctj".parse().unwrap(),
        );

        let operator_id = operator_id_header(&headers);

        assert_eq!(
            operator_id.map(|id| id.to_string()).as_deref(),
            Some("tn_01kxz0ytmhnab5vh53011cwctj")
        );
    }

    #[test]
    fn a_missing_or_blank_operator_id_header_names_nothing() {
        assert!(operator_id_header(&HeaderMap::new()).is_none());

        let mut headers = HeaderMap::new();
        headers.insert("x-operator-id", "   ".parse().unwrap());
        assert!(operator_id_header(&headers).is_none());
    }

    #[test]
    fn request_org_hint_only_reads_tool_call_arguments() {
        // `tools/list` carries no arguments, which is precisely why the
        // header fallback above exists.
        let list = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };

        assert_eq!(request_org_hint(&list), None);
    }

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
