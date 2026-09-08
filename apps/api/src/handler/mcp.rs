use std::collections::{BTreeSet, HashMap};
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
use tachyon_sdk::auth::{ExecutorAction, MultiTenancyAction, OperatorId};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::{
    CallerAuthApp, LibraryExecutor, LibraryExecutorKind,
};
use crate::sdk_auth::SdkAuthApp;
use crate::usecase::library_client_url::{data_url, share_url};
use crate::usecase::markdown_composer::compose_markdown;
use crate::usecase::{
    AddDataInputData, AddPropertyInputData, ChangeRepoUsernameInputData,
    CreateOrganizationInputData, CreateRepoInputData,
    CreateShareLinkInputData, CreateSourceInputData, DeleteDataInputData,
    DeletePropertyInputData, DeleteRepoInputData, DeleteSourceInputData,
    FindSourcesInputData, GetPropertiesInputData, GetSourceInputData,
    LibraryOrg, ListShareLinksInputData, PropertyDataInputData,
    PropertyDataValueInputData, RevokeShareLinkInputData,
    SearchDataInputData, SearchRepoInputData, ShareLinkRepoTarget,
    UpdateDataInputData, UpdateOrganizationInputData,
    UpdatePropertyInputData, UpdateRepoInputData, UpdateSourceInputData,
    UpsertDataInputData, ViewDataInputData, ViewDataListInputData,
    ViewOrgInputData, ViewRepoInputData,
};
use database_manager::domain::{
    Data, Property, PropertyDataValue, PropertyType,
};
use value_object::{LongText, OffsetPage, OffsetPaginator, Text, Url};

mod oauth_resource;

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

#[derive(Debug, Default, Deserialize)]
struct PaginationArgs {
    page: Option<u32>,
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ListReposArgs {
    org: String,
    #[serde(flatten)]
    pagination: PaginationArgs,
}

#[derive(Debug, Deserialize)]
struct RenameRepoArgs {
    org: String,
    repo: String,
    new_username: String,
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
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable")]
    website: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct GetPropertyArgs {
    org: String,
    repo: String,
    property_id: String,
}

#[derive(Debug, Deserialize)]
struct SearchReposArgs {
    org: String,
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
struct CreateShareLinkArgs {
    org: String,
    repo: String,
    data_id: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RevokeShareLinkArgs {
    org: String,
    repo: String,
    share_link_id: String,
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
    #[serde(default, deserialize_with = "deserialize_nullable")]
    url: Option<Option<String>>,
}

/// Preserve the difference between an omitted patch field and explicit null.
fn deserialize_nullable<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
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
    url: String,
}

#[derive(Debug, Serialize)]
struct McpData {
    id: String,
    title: String,
    markdown: String,
    url: String,
    record_version: String,
    property_data: Vec<McpPropertyData>,
}

#[derive(Debug, Serialize)]
struct McpPropertyData {
    property_id: String,
    value_type: String,
    value: Value,
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
    if (bearer_token(headers).is_some() || mcp_auth_required())
        && !auth.is_authenticated()
    {
        return Err(auth_challenge_response());
    }
    if request_requires_auth(&request) && auth.executor.is_none() {
        return Err(auth_challenge_response());
    }

    if let Some(scope) = missing_oauth_scope(&auth, &request) {
        return Err(insufficient_scope_response(scope));
    }

    let is_notification = request.id.is_none();
    let was_authenticated = auth.executor.is_some();
    // The SSE transport can supply credentials from its session rather than
    // the POST header. Scope the verified token for every downstream policy
    // evaluation, on both transports.
    let caller_token = auth
        .executor
        .as_ref()
        .and_then(|executor| executor.original_token.clone());
    let response = crate::sdk_auth::caller_token_scope(
        caller_token,
        handle_rpc(library_app, auth, request),
    )
    .await;
    // Public tools can also address private resources. Let a client begin
    // OAuth when an anonymous read reaches a protected resource.
    if !was_authenticated && response["error"]["code"] == -32001 {
        return Err(auth_challenge_response());
    }
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
        "tools/list" => Ok(scoped_tools_list(&auth)),
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

    if !auth.allows_tool(&params.name) {
        return Err(json_rpc_error(-32001, "Insufficient OAuth scope"));
    }
    match params.name.as_str() {
        "get_me" => Ok(tool_text_result(get_me(auth)?)),
        "list_orgs" => {
            let args: PaginationArgs = parse_tool_args(params.arguments)?;
            let output = list_orgs(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "list_repos" => {
            let args: ListReposArgs = parse_tool_args(params.arguments)?;
            let output = list_repos(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "rename_repo" => {
            let args: RenameRepoArgs = parse_tool_args(params.arguments)?;
            let output = rename_repo(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "upsert_data" => {
            let args: UpdateDataArgs = parse_tool_args(params.arguments)?;
            let output = upsert_data(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "create_share_link" => {
            let args: CreateShareLinkArgs =
                parse_tool_args(params.arguments)?;
            let output = create_share_link(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "list_share_links" => {
            let args: DeleteDataArgs = parse_tool_args(params.arguments)?;
            let output = list_share_links(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "revoke_share_link" => {
            let args: RevokeShareLinkArgs =
                parse_tool_args(params.arguments)?;
            let output = revoke_share_link(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
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
            let output = list_data(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "search_data" => {
            let args: SearchDataArgs = parse_tool_args(params.arguments)?;
            let output = search_data(library_app, auth, args).await?;
            Ok(tool_text_result(output))
        }
        "get_data" => {
            let args: GetDataArgs = parse_tool_args(params.arguments)?;
            let output = get_data(library_app, auth, args).await?;
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

fn get_me(auth: McpAuthContext) -> Result<Value, Value> {
    let executor = require_executor(auth, "get_me")?;
    match executor.inner {
        LibraryExecutorKind::User(user) => Ok(json!({
            "id": user.id(), "type": "user", "name": user.name(),
            "username": user.username(),
        })),
        LibraryExecutorKind::ServiceAccount(account) => Ok(json!({
            "id": account.id(), "type": "service_account", "name": account.name(),
        })),
        LibraryExecutorKind::None => Err(json_rpc_error(
            -32001,
            "Authentication required for get_me",
        )),
    }
}

async fn member_organizations(
    repository: &dyn crate::domain::OrganizationRepository,
    executor: &LibraryExecutor,
) -> Result<Vec<McpOrganization>, Value> {
    // Enumerate only verified memberships. Never use find_all(): a Tachyon
    // account may belong to other products' tenants as well as Library.
    let tenants: BTreeSet<String> = match &executor.inner {
        LibraryExecutorKind::User(user) => {
            user.tenants().iter().map(ToString::to_string).collect()
        }
        LibraryExecutorKind::ServiceAccount(account) => {
            BTreeSet::from([account.tenant_id().to_string()])
        }
        LibraryExecutorKind::None => {
            return Err(json_rpc_error(
                -32001,
                "Authentication required for list_orgs",
            ))
        }
    };
    let mut organizations = Vec::new();
    for tenant in tenants {
        if let Some(organization) = repository
            .get_by_id(&tenant.parse().map_err(invalid_tool_arg)?)
            .await
            .map_err(tool_execution_error)?
        {
            organizations.push(organization_to_mcp(&organization));
        }
    }
    organizations.sort_by(|a, b| {
        a.username.cmp(&b.username).then_with(|| a.id.cmp(&b.id))
    });
    Ok(organizations)
}

fn paginate<T>(
    items: Vec<T>,
    page: OffsetPage,
) -> (Vec<T>, OffsetPaginator) {
    let paginator = OffsetPaginator::new(page, items.len() as u32);
    let items = items
        .into_iter()
        .skip(page.offset() as usize)
        .take(page.items_per_page() as usize)
        .collect();
    (items, paginator)
}

async fn list_orgs(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: PaginationArgs,
) -> Result<Value, Value> {
    let page = OffsetPage::from_options(args.page, args.page_size)
        .map_err(invalid_tool_arg)?;
    let executor = require_executor(auth, "list_orgs")?;
    let organizations = member_organizations(
        library_app.organization_repo.as_ref(),
        &executor,
    )
    .await?;
    let (organizations, paginator) = paginate(organizations, page);
    Ok(json!({ "organizations": organizations, "paginator": paginator }))
}

async fn list_repos(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: ListReposArgs,
) -> Result<Value, Value> {
    let page = OffsetPage::from_options(
        args.pagination.page,
        args.pagination.page_size,
    )
    .map_err(invalid_tool_arg)?;
    let executor = read_executor(&auth);
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let output = library_app
        .view_org
        .execute(&ViewOrgInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            organization_username: args.org,
        })
        .await
        .map_err(tool_execution_error)?;
    let repos = output.repos.iter().map(repo_to_mcp).collect();
    let (repos, paginator) = paginate(repos, page);
    Ok(json!({ "repos": repos, "paginator": paginator }))
}

async fn rename_repo(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: RenameRepoArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "rename_repo")?;
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let repo = library_app
        .change_repo_username
        .execute(ChangeRepoUsernameInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            org_username: args.org,
            old_repo_username: args.repo,
            new_repo_username: args.new_username,
        })
        .await
        .map_err(tool_execution_error)?;
    Ok(json!({ "repo": repo_to_mcp(&repo) }))
}

async fn upsert_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: UpdateDataArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "upsert_data")?;
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let (data, properties, outcome) = library_app
        .upsert_data
        .execute(UpsertDataInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            actor: executor.get_id(),
            org_username: &args.org,
            repo_username: &args.repo,
            data_id: &args.data_id,
            data_name: &args.name,
            property_data: property_data_from_args(args.property_data)?,
        })
        .await
        .map_err(tool_execution_error)?;
    let outcome = match outcome {
        database_manager::usecase::UpsertOutcome::Created => "created",
        database_manager::usecase::UpsertOutcome::Updated => "updated",
    };
    Ok(
        json!({ "data": data_to_mcp(&data, &properties, &args.org, &args.repo), "outcome": outcome }),
    )
}

/// The one place a share token is ever visible.
///
/// library-api stores only its SHA-256, so a caller that loses this
/// response cannot recover the link -- it has to mint another and revoke
/// this one.
async fn create_share_link(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: CreateShareLinkArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "create_share_link")?;
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let created = library_app
        .share_links
        .create(&CreateShareLinkInputData {
            target: ShareLinkRepoTarget {
                executor: &executor,
                multi_tenancy: &library_org,
                org_username: &args.org,
                repo_username: &args.repo,
            },
            data_id: &args.data_id,
            name: args.name.as_deref(),
        })
        .await
        .map_err(tool_execution_error)?;

    let token = created.token.as_str();
    Ok(json!({
        "share_link": share_link_to_mcp(&created.link),
        "url": share_url(token),
        "token": token,
    }))
}

async fn list_share_links(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: DeleteDataArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "list_share_links")?;
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let links = library_app
        .share_links
        .list(&ListShareLinksInputData {
            target: ShareLinkRepoTarget {
                executor: &executor,
                multi_tenancy: &library_org,
                org_username: &args.org,
                repo_username: &args.repo,
            },
            data_id: &args.data_id,
        })
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({
        "share_links": links.iter().map(share_link_to_mcp).collect::<Vec<_>>(),
    }))
}

async fn revoke_share_link(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: RevokeShareLinkArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "revoke_share_link")?;
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let link = library_app
        .share_links
        .revoke(&RevokeShareLinkInputData {
            target: ShareLinkRepoTarget {
                executor: &executor,
                multi_tenancy: &library_org,
                org_username: &args.org,
                repo_username: &args.repo,
            },
            share_link_id: &args.share_link_id,
        })
        .await
        .map_err(tool_execution_error)?;

    Ok(json!({ "share_link": share_link_to_mcp(&link) }))
}

/// A link without its secret -- the shape every response but the create
/// one can carry.
fn share_link_to_mcp(link: &crate::domain::ShareLink) -> Value {
    json!({
        "id": link.id().to_string(),
        "name": link.name().as_ref().map(|name| name.to_string()),
        "data_id": link.data_id(),
        "created_at": link.created_at().to_rfc3339(),
        "revoked_at": link.revoked_at().map(|at| at.to_rfc3339()),
        "active": !link.is_revoked(),
    })
}

async fn list_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: ListDataArgs,
) -> Result<Value, Value> {
    OffsetPage::from_options(args.page, args.page_size)
        .map_err(invalid_tool_arg)?;
    let executor = read_executor(&auth);
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let input = ViewDataListInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org.clone(),
        repo_username: args.repo.clone(),
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
            url: data_url(&args.org, &args.repo, data.id().as_str()),
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
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
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
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let current = library_app
        .organization_repo
        .get_by_username(&args.org.parse().map_err(invalid_tool_arg)?)
        .await
        .map_err(tool_execution_error)?
        .ok_or_else(|| json_rpc_error(-32000, "Organization not found"))?;
    let input = UpdateOrganizationInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        username: args.org,
        name: args.name.unwrap_or_else(|| current.name().to_string()),
        description: args.description.unwrap_or_else(|| {
            current.description().as_ref().map(ToString::to_string)
        }),
        website: args.website.unwrap_or_else(|| {
            current.website().as_ref().map(ToString::to_string)
        }),
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
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
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
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    if args.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
        return Err(invalid_tool_arg("limit must be between 1 and 100"));
    }
    let input = SearchRepoInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: Some(args.org),
        name: args.query,
        limit: Some(args.limit.unwrap_or(20)),
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
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
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
    auth: McpAuthContext,
    args: SearchDataArgs,
) -> Result<Value, Value> {
    OffsetPage::from_options(args.page, args.page_size)
        .map_err(invalid_tool_arg)?;
    let executor = read_executor(&auth);
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
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
            url: data_url(&args.org, &args.repo, data.id().as_str()),
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "data": data_list,
        "paginator": paginator,
    }))
}

async fn get_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: GetDataArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
    let input = ViewDataInputData {
        executor: &executor,
        multi_tenancy: &library_org,
        org_username: args.org.clone(),
        repo_username: args.repo.clone(),
        data_id: args.data_id,
    };

    let (data, properties) = library_app
        .view_data
        .execute(&input)
        .await
        .map_err(tool_execution_error)?;
    let data = data_to_mcp(&data, &properties, &args.org, &args.repo);

    Ok(json!({ "data": data }))
}

async fn list_properties(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: OrgRepoArgs,
) -> Result<Value, Value> {
    let executor = read_executor(&auth);
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
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
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
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
    let library_org = resolve_library_org(&library_app, &args.org)
        .await
        .map_err(tool_execution_error)?;
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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

    let mut response = serde_json::to_value(data_to_mcp(
        &data,
        &properties,
        &args.org,
        &args.repo,
    ))
    .map_err(|err| json_rpc_error(-32603, err.to_string()))?;
    response["property_count"] = json!(properties.len());
    Ok(json!({ "data": response }))
}

async fn update_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: UpdateDataArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "update_data")?;
    let library_org = resolve_library_org(&library_app, &args.org)
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

    let mut response = serde_json::to_value(data_to_mcp(
        &data,
        &properties,
        &args.org,
        &args.repo,
    ))
    .map_err(|err| json_rpc_error(-32603, err.to_string()))?;
    response["property_count"] = json!(properties.len());
    Ok(json!({ "data": response }))
}

async fn delete_data(
    library_app: Arc<LibraryApp>,
    auth: McpAuthContext,
    args: DeleteDataArgs,
) -> Result<Value, Value> {
    let executor = require_executor(auth, "delete_data")?;
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
    let library_org = resolve_library_org(&library_app, &args.org)
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
            Value::Null => Ok(String::new()),
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
            let values =
                serde_json::from_value::<Vec<String>>(if value.is_null() {
                    json!([])
                } else {
                    value
                })
                .map_err(invalid_tool_arg)?;
            Ok(PropertyDataValueInputData::Relation(values))
        }
        "select" => {
            Ok(PropertyDataValueInputData::Select(as_string(value)?))
        }
        "multi_select" => {
            let values =
                serde_json::from_value::<Vec<String>>(if value.is_null() {
                    json!([])
                } else {
                    value
                })
                .map_err(invalid_tool_arg)?;
            Ok(PropertyDataValueInputData::MultiSelect(values))
        }
        "boolean" => {
            if value.is_null() {
                return Ok(PropertyDataValueInputData::String(
                    String::new(),
                ));
            }
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
        "location" => {
            let location: value_object::Location =
                serde_json::from_value(value).map_err(invalid_tool_arg)?;
            let location = value_object::Location::new(
                location.latitude(),
                location.longitude(),
            )
            .map_err(invalid_tool_arg)?;
            Ok(PropertyDataValueInputData::Location(location))
        }
        "string" | "id" => {
            Ok(PropertyDataValueInputData::String(as_string(value)?))
        }
        other => Err(json_rpc_error(
            -32602,
            format!("Unsupported property value_type: {other}"),
        )),
    }
}

fn data_to_mcp(
    data: &Data,
    properties: &[Property],
    org: &str,
    repo: &str,
) -> McpData {
    let property_data = data
        .property_data()
        .iter()
        .map(|property| {
            let (value_type, value) = match property.value() {
                Some(value) => property_value_to_mcp(value),
                None => (
                    properties
                        .iter()
                        .find(|p| p.id() == property.property_id())
                        .map(|p| {
                            p.property_type()
                                .to_string()
                                .to_ascii_lowercase()
                        })
                        .unwrap_or_else(|| "unknown".to_string()),
                    Value::Null,
                ),
            };
            McpPropertyData {
                property_id: property.property_id().to_string(),
                value_type,
                value,
            }
        })
        .collect();
    McpData {
        id: data.id().to_string(),
        title: data.name().to_string(),
        markdown: compose_markdown(data, properties),
        url: data_url(org, repo, data.id().as_str()),
        record_version: data.record_version().to_string(),
        property_data,
    }
}

fn property_value_to_mcp(value: &PropertyDataValue) -> (String, Value) {
    let (typ, value) = match value {
        PropertyDataValue::String(v) => ("string", json!(v)),
        PropertyDataValue::Integer(v) => ("integer", json!(v)),
        PropertyDataValue::Html(v) => ("html", json!(v)),
        PropertyDataValue::Markdown(v) => ("markdown", json!(v)),
        PropertyDataValue::Relation(_, ids) => (
            "relation",
            json!(ids.iter().map(ToString::to_string).collect::<Vec<_>>()),
        ),
        PropertyDataValue::Id(v) => ("id", json!(v)),
        PropertyDataValue::Location(v) => ("location", json!(v)),
        PropertyDataValue::Select(v) => ("select", json!(v.to_string())),
        PropertyDataValue::MultiSelect(v) => (
            "multi_select",
            json!(v.iter().map(ToString::to_string).collect::<Vec<_>>()),
        ),
        PropertyDataValue::Date(v) => ("date", json!(v)),
        PropertyDataValue::Image(v) => ("image", json!(v)),
        PropertyDataValue::RichText(v) => ("rich_text", v.clone()),
        PropertyDataValue::Boolean(v) => ("boolean", json!(v)),
    };
    (typ.to_string(), value)
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

fn scoped_tools_list(auth: &McpAuthContext) -> Value {
    let mut result = tools_list_result(auth.can_use_write_tools());
    if let Some(tools) = result["tools"].as_array_mut() {
        tools.retain(|tool| {
            auth.allows_tool(tool["name"].as_str().unwrap_or_default())
        });
    }
    result
}

fn tools_list_result(is_authenticated: bool) -> Value {
    let mut tools = vec![
        json!({
            "name": "get_me",
            "description": "Identify the signed-in Library user or API-key service account. Requires authentication.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "list_orgs",
            "description": "List Library organizations belonging to the signed-in user, or the API key's organization. Requires authentication; no org slug is needed.",
            "inputSchema": pagination_schema()
        }),
        json!({
            "name": "list_repos",
            "description": "List repositories in an organization. Anonymous callers and non-members see only public repositories.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org": { "type": "string" },
                    "page": { "type": "integer", "minimum": 1 },
                    "page_size": { "type": "integer", "minimum": 1, "maximum": 100 }
                },
                "required": ["org"]
            }
        }),
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
            "description": "Search Library repositories within one organization you belong to. Requires authentication.",
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
            "description": "List data records in a public repository or a private repository the caller is authorized to read.",
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
            "description": "Find data records by exact name in a repository the caller is authorized to read. An empty query lists records; this is not full-text search.",
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
            "description": "Get a record as Markdown and typed property_data with its canonical URL and informational record_version (current MCP CRUD does not advance this counter). Private records require read permission.",
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
                "name": "rename_repo",
                "description": "Change a repository's username (slug), preserving its identity and content.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "repo": { "type": "string" },
                        "new_username": { "type": "string" }
                    },
                    "required": ["org", "repo", "new_username"]
                }
            }),
            json!({
                "name": "upsert_data",
                "description": "Create or update a record at a caller-supplied valid data_id. Reusing the same id avoids duplicate records on retry; this is not a compare-and-swap operation.",
                "inputSchema": data_write_schema(["org", "repo", "data_id", "name"])
            }),
            json!({
                "name": "create_share_link",
                "description": "Mint a read-only link to one record, openable without a Library account. Made for handing a document in a private repository to someone outside the tenant. The token is returned once and cannot be shown again; reuse the returned url rather than minting a link per message.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "repo": { "type": "string" },
                        "data_id": { "type": "string" },
                        "name": { "type": "string", "description": "Label shown beside the link in the owner's list." }
                    },
                    "required": ["org", "repo", "data_id"]
                }
            }),
            json!({
                "name": "list_share_links",
                "description": "List the share links a record has, without their tokens. Use it to find the link to revoke.",
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
                "name": "revoke_share_link",
                "description": "Stop a share link from opening its record. The link stays listed, marked inactive.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "repo": { "type": "string" },
                        "share_link_id": { "type": "string" }
                    },
                    "required": ["org", "repo", "share_link_id"]
                }
            }),
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
                "description": "Patch a Library organization. Omitted fields are preserved; null description or website clears that field.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "org": { "type": "string" },
                        "name": { "type": "string" },
                        "description": { "type": ["string", "null"] },
                        "website": { "type": ["string", "null"] }
                    },
                    "required": ["org"]
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

    for tool in &mut tools {
        let name = tool["name"].as_str().unwrap_or_default();
        let read_only = is_read_tool(name);
        tool["annotations"] = json!({
            "readOnlyHint": read_only,
            "destructiveHint": !read_only,
            "openWorldHint": true,
        });
    }
    json!({ "tools": tools })
}

fn is_read_tool(name: &str) -> bool {
    matches!(
        name,
        "get_me"
            | "list_orgs"
            | "get_org"
            | "list_repos"
            | "search_repos"
            | "get_repo"
            | "list_data"
            | "search_data"
            | "get_data"
            | "list_properties"
            | "get_property"
            | "list_sources"
            | "get_source"
    )
}

fn missing_oauth_scope(
    auth: &McpAuthContext,
    request: &JsonRpcRequest,
) -> Option<&'static str> {
    let scopes = auth.oauth_scopes?;
    if request.method != "tools/call" {
        return None;
    }
    let name = request.params.as_ref()?.get("name")?.as_str()?;
    let read_only = is_read_tool(name);
    if scopes.allows(read_only) {
        None
    } else if read_only {
        Some("mcp:read")
    } else {
        Some("mcp:write")
    }
}

fn insufficient_scope_response(scope: &str) -> Response {
    let mut response =
        (StatusCode::FORBIDDEN, "Insufficient OAuth scope").into_response();
    if let Ok(header) = HeaderValue::from_str(&format!(
        "Bearer error=\"insufficient_scope\", scope=\"{scope}\", resource_metadata=\"{}\"",
        mcp_resource_metadata_url()
    )) {
        response.headers_mut().insert(WWW_AUTHENTICATE, header);
    }
    response
}

fn pagination_schema() -> Value {
    json!({ "type": "object", "properties": {
        "page": { "type": "integer", "minimum": 1 },
        "page_size": { "type": "integer", "minimum": 1, "maximum": 100 }
    } })
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
                                "boolean",
                                "id",
                                "location"
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
    serde_json::from_value(if arguments.is_null() {
        json!({})
    } else {
        arguments
    })
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
    let code = match &err {
        errors::Error::Unauthorized { .. }
        | errors::Error::Forbidden { .. } => -32001,
        errors::Error::BadRequest { .. } => -32602,
        _ => -32000,
    };
    json_rpc_error(code, err.to_string())
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
    oauth_scopes: Option<oauth_resource::Scopes>,
}

impl McpAuthContext {
    fn anonymous() -> Self {
        Self {
            executor: None,
            caller_auth: None,
            accepted_credentials: false,
            write_tools_available: false,
            oauth_scopes: None,
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
            oauth_scopes: None,
        }
    }

    fn accepted_without_executor(write_tools_available: bool) -> Self {
        Self {
            executor: None,
            caller_auth: None,
            accepted_credentials: true,
            write_tools_available,
            oauth_scopes: None,
        }
    }

    fn is_authenticated(&self) -> bool {
        self.accepted_credentials
    }

    fn can_use_write_tools(&self) -> bool {
        self.write_tools_available
            && self.oauth_scopes.is_none_or(|scopes| scopes.write)
    }

    fn allows_tool(&self, name: &str) -> bool {
        self.oauth_scopes
            .is_none_or(|scopes| scopes.allows(is_read_tool(name)))
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
            Some(org_username) => {
                resolve_library_org(&library_app, org_username)
                    .await
                    .ok()
                    .and_then(|org| org.operator_id())
            }
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

    let oauth_token = if oauth_resource::enabled() {
        let config = match oauth_resource::Config::from_env() {
            Ok(config) => config,
            Err(error) => {
                tracing::warn!(error, "Invalid MCP OAuth configuration");
                return McpAuthContext::anonymous();
            }
        };
        match config.verify(&token).await {
            Ok(verified) => Some(verified),
            Err(error) => {
                tracing::warn!(
                    error,
                    "MCP OAuth token verification failed"
                );
                return McpAuthContext::anonymous();
            }
        }
    } else {
        None
    };

    match sdk.verify_token(&token).await {
        Ok(user) => {
            if oauth_token
                .as_ref()
                .is_some_and(|verified| user.id != verified.subject)
            {
                return McpAuthContext::anonymous();
            }
            let executor = LibraryExecutor {
                inner: LibraryExecutorKind::User(Box::new(user)),
                original_token: Some(token),
            };
            let caller_auth = executor
                .caller_auth_app(&sdk)
                .expect("authenticated MCP executor has a token");
            let mut auth =
                McpAuthContext::authenticated(executor, caller_auth);
            auth.oauth_scopes = oauth_token.map(|verified| verified.scopes);
            auth
        }
        Err(error) => {
            tracing::warn!("MCP bearer token verification failed: {error}");
            McpAuthContext::anonymous()
        }
    }
}

async fn resolve_library_org(
    library_app: &LibraryApp,
    org_username: &str,
) -> errors::Result<LibraryOrg> {
    let org = library_app
        .organization_repo
        .get_by_username(&org_username.parse()?)
        .await?
        .ok_or_else(|| errors::Error::not_found("organization"))?;
    Ok(LibraryOrg::with_org_and_operator(
        org_username.to_string(),
        org.id().clone(),
    ))
}

fn should_challenge(headers: &HeaderMap, request: &JsonRpcRequest) -> bool {
    if bearer_token(headers).is_some() {
        return false;
    }

    if mcp_auth_required() {
        return true;
    }

    request_requires_auth(request)
}

fn request_requires_auth(request: &JsonRpcRequest) -> bool {
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
    matches!(
        name,
        "get_me"
            | "list_orgs"
            | "search_repos"
            | "create_org"
            | "update_org"
            | "create_repo"
            | "update_repo"
            | "rename_repo"
            | "delete_repo"
            | "create_data"
            | "update_data"
            | "upsert_data"
            | "delete_data"
            | "create_share_link"
            | "list_share_links"
            | "revoke_share_link"
            | "create_property"
            | "update_property"
            | "delete_property"
            | "create_source"
            | "update_source"
            | "delete_source"
    )
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
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(char::is_whitespace)?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_string())
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

pub async fn protected_resource_metadata() -> Response {
    let (resource, authorization_servers) = if oauth_resource::enabled() {
        match oauth_resource::Config::from_env() {
            Ok(config) => (config.resource, vec![config.issuer]),
            Err(error) => {
                tracing::warn!(error, "Invalid MCP OAuth configuration");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "MCP OAuth is not configured",
                )
                    .into_response();
            }
        }
    } else {
        (mcp_resource_url(), vec![mcp_oauth_issuer()])
    };
    Json(json!({
        "resource": resource,
        "authorization_servers": authorization_servers,
        "scopes_supported": mcp_scopes_supported(),
        "bearer_methods_supported": ["header"],
        "resource_name": "Library MCP"
    }))
    .into_response()
}

fn retired_oauth_response() -> Response {
    (
        StatusCode::GONE,
        Json(json!({
            "error": "authorization_server_moved",
            "resource_metadata": mcp_resource_metadata_url()
        })),
    )
        .into_response()
}

pub async fn mcp_oauth_authorization_server_metadata() -> Response {
    if oauth_resource::enabled() {
        return retired_oauth_response();
    }
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
    .into_response()
}

pub async fn mcp_oauth_register(
    Json(request): Json<McpOAuthClientRegistrationRequest>,
) -> Response {
    if oauth_resource::enabled() {
        return retired_oauth_response();
    }
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
    if oauth_resource::enabled() {
        return retired_oauth_response();
    }
    if let Err(message) = validate_authorize_request(&query).await {
        return Html(render_login_page(&query, Some(&message)))
            .into_response();
    }

    Html(render_login_page(&query, None)).into_response()
}

pub async fn mcp_oauth_authorize_submit(
    Form(form): Form<McpOAuthAuthorizeForm>,
) -> Response {
    if oauth_resource::enabled() {
        return retired_oauth_response();
    }
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
    if oauth_resource::enabled() {
        return retired_oauth_response();
    }
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
    if oauth_resource::enabled() {
        return vec!["mcp:read".into(), "mcp:write".into()];
    }
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

    #[test]
    fn oauth_read_scope_hides_writes_and_rejects_direct_write_calls() {
        let mut auth = McpAuthContext::accepted_without_executor(true);
        auth.oauth_scopes = Some(oauth_resource::Scopes {
            read: true,
            write: false,
        });
        assert!(!auth.can_use_write_tools());
        let list = scoped_tools_list(&auth);
        let names = list["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(names.contains(&"get_data"));
        assert!(!names.contains(&"create_data"));
        for tool in tools_list_result(true)["tools"].as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            let request = JsonRpcRequest {
                jsonrpc: Some("2.0".into()),
                id: Some(json!(1)),
                method: "tools/call".into(),
                params: Some(json!({"name":name})),
            };
            assert_eq!(
                auth.allows_tool(name),
                tool["annotations"]["readOnlyHint"] == true,
                "{name}"
            );
            assert_eq!(
                missing_oauth_scope(&auth, &request),
                if is_read_tool(name) {
                    None
                } else {
                    Some("mcp:write")
                },
                "{name}"
            );
        }
        let response = insufficient_scope_response("mcp:write");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(response.headers()[WWW_AUTHENTICATE]
            .to_str()
            .unwrap()
            .contains("insufficient_scope"));
        assert!(McpAuthContext::accepted_without_executor(true)
            .can_use_write_tools());
    }

    #[test]
    fn oauth_write_scope_does_not_grant_read_access() {
        let mut auth = McpAuthContext::accepted_without_executor(true);
        auth.oauth_scopes = Some(oauth_resource::Scopes {
            read: false,
            write: true,
        });
        assert!(auth.can_use_write_tools());
        assert!(!auth.allows_tool("get_data"));
        assert!(!auth.allows_tool("get_me"));
        assert!(auth.allows_tool("create_data"));
        assert!(!McpAuthContext::anonymous().can_use_write_tools());
    }

    // Run environment-dependent handlers in a subprocess so parallel tests
    // never observe a temporary production-like OAuth configuration.
    #[test]
    fn external_oauth_http_contract_subprocess() {
        let result =
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "handler::mcp::tests::external_oauth_http_contract",
                    "--nocapture",
                ])
                .env("PLT4366_HTTP_TEST", "1")
                .env_remove("MCP_AUTHORIZATION_SERVERS")
                .env(
                    "MCP_AUTHORIZATION_SERVER",
                    "https://issuer.example.test",
                )
                .env(
                    "MCP_OAUTH_JWKS_URL",
                    "https://issuer.example.test/oauth2/jwks",
                )
                .env("MCP_RESOURCE_URL", "https://library.example.test/mcp")
                .output()
                .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[tokio::test]
    async fn external_oauth_http_contract() {
        if std::env::var("PLT4366_HTTP_TEST").as_deref() != Ok("1") {
            return;
        }
        use axum::{
            body::{to_bytes, Body},
            http::Request,
            routing::{get, post},
            Router,
        };
        use tower::ServiceExt;
        let router = Router::new()
            .route("/metadata", get(protected_resource_metadata))
            .route(
                "/discovery",
                get(mcp_oauth_authorization_server_metadata),
            )
            .route("/register", post(mcp_oauth_register))
            .route(
                "/authorize",
                get(mcp_oauth_authorize).post(mcp_oauth_authorize_submit),
            )
            .route("/token", post(mcp_oauth_token));
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/metadata")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let metadata: Value = serde_json::from_slice(
            &to_bytes(response.into_body(), 8192).await.unwrap(),
        )
        .unwrap();
        assert_eq!(
            metadata["authorization_servers"],
            json!(["https://issuer.example.test"])
        );
        assert_eq!(
            metadata["resource"],
            "https://library.example.test/mcp"
        );
        assert_eq!(
            metadata["scopes_supported"],
            json!(["mcp:read", "mcp:write"])
        );
        let query = "response_type=code&client_id=test&redirect_uri=https%3A%2F%2Fclient.example.test%2Fcallback&code_challenge=test&code_challenge_method=S256";
        for (method, uri, content_type, body) in [
            ("GET", "/discovery".to_string(), "application/json", String::new()),
            ("POST", "/register".to_string(), "application/json", "{\"redirect_uris\":[\"https://client.example.test/callback\"]}".to_string()),
            ("GET", format!("/authorize?{query}"), "application/json", String::new()),
            ("POST", "/authorize".to_string(), "application/x-www-form-urlencoded", format!("{query}&username=test&password=test")),
            ("POST", "/token".to_string(), "application/x-www-form-urlencoded", "grant_type=authorization_code&code=test".to_string()),
        ] {
            let response = router.clone().oneshot(Request::builder().method(method).uri(&uri).header("content-type", content_type).body(Body::from(body)).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::GONE, "{method} {uri}");
        }
        std::env::set_var("MCP_AUTHORIZATION_SERVER", "");
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metadata")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

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

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod workflow_tests;
