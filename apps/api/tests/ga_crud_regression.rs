extern crate library_api;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use inbound_sync::interface_adapter::InMemoryOAuthTokenRepository;
use reqwest::{Client, Method, StatusCode};
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::oneshot};
use value_object::{DatabaseUrl, TenantId};

const TOKEN: &str = "caller-user-token";
const BASE_AUTH_TOKEN: &str = "base-service-token";
const USER_ID: &str = "us_01hs2yepy5hw4rz8pdq2wywnwt";
const LIBRARY_TENANT_ID: &str = "tn_01j702qf86pc2j35s0kv0gv3gy";

#[tokio::test]
async fn rest_core_crud_lifecycle_is_stable() -> anyhow::Result<()> {
    let (server_url, shutdown_tx, _) = setup_test_server().await?;
    let client = create_test_client();
    let suffix = unique_suffix();

    let org = format!("ga-rest-org-{suffix}");
    let repo = format!("ga-rest-repo-{suffix}");

    post_json(
        &client,
        &format!("{server_url}/v1beta/orgs"),
        json!({
            "name": format!("GA REST Org {suffix}"),
            "username": org,
            "description": "GA REST org",
            "website": "https://example.com/rest"
        }),
        StatusCode::OK,
    )
    .await?;

    let org_view = get_json(
        &client,
        &format!("{server_url}/v1beta/orgs/{org}"),
        StatusCode::OK,
    )
    .await?;
    assert_eq!(org_view["username"], org);
    assert_eq!(org_view["website"], "https://example.com/rest");

    let org_updated = put_json(
        &client,
        &format!("{server_url}/v1beta/orgs/{org}"),
        json!({
            "name": format!("GA REST Org Updated {suffix}"),
            "description": "GA REST org updated",
            "website": "https://example.com/rest-updated"
        }),
        StatusCode::OK,
    )
    .await?;
    assert_eq!(org_updated["description"], "GA REST org updated");

    let created_repo = post_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}"),
        json!({
            "name": format!("GA REST Repo {suffix}"),
            "username": repo,
            "description": "GA REST repo",
            "is_public": false,
            "database_id": null
        }),
        StatusCode::OK,
    )
    .await?;
    assert_eq!(created_repo["username"], repo);

    let repo_updated = put_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}"),
        json!({
            "name": format!("GA REST Repo Updated {suffix}"),
            "description": "GA REST repo updated",
            "is_public": true,
            "tags": ["ga", "rest"]
        }),
        StatusCode::OK,
    )
    .await?;
    assert_eq!(repo_updated["is_public"], true);

    let property = post_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/properties"),
        json!({"name": "ga_title", "property_type": "string"}),
        StatusCode::OK,
    )
    .await?;
    let property_id = string_field(&property, "id")?;

    let property_updated = put_json(
        &client,
        &format!(
            "{server_url}/v1beta/repos/{org}/{repo}/properties/{property_id}"
        ),
        json!({"name": "ga_title_updated"}),
        StatusCode::OK,
    )
    .await?;
    assert_eq!(property_updated["name"], "ga_title_updated");

    let properties = get_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/properties"),
        StatusCode::OK,
    )
    .await?;
    let id_property = properties
        .as_array()
        .and_then(|properties| {
            properties
                .iter()
                .find(|property| property["property_type"] == "ID")
        })
        .ok_or_else(|| {
            anyhow::anyhow!("default typed Id property is missing")
        })?;
    let id_property_id = string_field(id_property, "id")?;
    assert_eq!(id_property["auto_generate"], true);

    post_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/properties"),
        json!({
            "name": "ga_after_id",
            "property_type": "string"
        }),
        StatusCode::OK,
    )
    .await?;

    post_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/properties"),
        json!({
            "name": "ga_duplicate_id",
            "property_type": "id",
            "auto_generate": false
        }),
        StatusCode::CONFLICT,
    )
    .await?;

    let data = post_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/data"),
        json!({
            "name": format!("GA REST Data {suffix}"),
            "property_data": [{
                "property_id": property_id,
                "value": "rest-value"
            }]
        }),
        StatusCode::OK,
    )
    .await?;
    let data_id = string_field(&data, "id")?;
    assert_eq!(data["recordVersion"], "1");
    let generated_id = data["items"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["property_id"] == id_property_id)
        })
        .and_then(|item| item["value"]["id"].as_str())
        .ok_or_else(|| anyhow::anyhow!("generated Id value is missing"))?;
    assert_eq!(generated_id, data_id);

    let data_updated = put_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/data/{data_id}"),
        json!({
            "name": format!("GA REST Data Updated {suffix}"),
            "property_data": [{
                "property_id": property_id,
                "value": "rest-value-updated"
            }]
        }),
        StatusCode::OK,
    )
    .await?;
    assert_eq!(
        data_updated["name"],
        format!("GA REST Data Updated {suffix}")
    );
    assert_eq!(data_updated["recordVersion"], "1");

    let data_view = get_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/data/{data_id}"),
        StatusCode::OK,
    )
    .await?;
    assert_eq!(data_view["recordVersion"], "1");

    let data_list = get_json(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/data-list"),
        StatusCode::OK,
    )
    .await?;
    let listed_data = data_list["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("REST data list is not an array"))?
        .iter()
        .find(|item| item["id"] == data_id)
        .ok_or_else(|| {
            anyhow::anyhow!("REST data list is missing record")
        })?;
    assert_eq!(listed_data["recordVersion"], "1");

    delete(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/data/{data_id}"),
        StatusCode::NO_CONTENT,
    )
    .await?;
    request_text(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}/data/{data_id}"),
        Method::GET,
        None,
        StatusCode::NOT_FOUND,
    )
    .await?;

    delete(
        &client,
        &format!(
            "{server_url}/v1beta/repos/{org}/{repo}/properties/{property_id}"
        ),
        StatusCode::OK,
    )
    .await?;
    request_text(
        &client,
        &format!(
            "{server_url}/v1beta/repos/{org}/{repo}/properties/{property_id}"
        ),
        Method::GET,
        None,
        StatusCode::NOT_FOUND,
    )
    .await?;

    delete(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}"),
        StatusCode::NO_CONTENT,
    )
    .await?;
    request_text(
        &client,
        &format!("{server_url}/v1beta/repos/{org}/{repo}"),
        Method::GET,
        None,
        StatusCode::NOT_FOUND,
    )
    .await?;

    shutdown_tx.send(()).ok();
    Ok(())
}

#[tokio::test]
async fn graphql_core_crud_lifecycle_is_stable() -> anyhow::Result<()> {
    let (server_url, shutdown_tx, _) = setup_test_server().await?;
    let client = create_test_client();
    let suffix = unique_suffix();

    let org = format!("ga-gql-org-{suffix}");
    let repo = format!("ga-gql-repo-{suffix}");

    let created_org = graphql(
        &client,
        &server_url,
        "mutation CreateOrg($input: CreateOrganizationInput!) {
            createOrganization(input: $input) { id username website }
        }",
        json!({
            "input": {
                "name": format!("GA GraphQL Org {suffix}"),
                "username": org,
                "description": "GA GraphQL org",
                "website": "https://example.com/graphql"
            }
        }),
    )
    .await?;
    assert_eq!(created_org["createOrganization"]["username"], org);

    let updated_org = graphql(
        &client,
        &server_url,
        "mutation UpdateOrg($input: UpdateOrganizationInput!) {
            updateOrganization(input: $input) { username description website }
        }",
        json!({
            "input": {
                "username": org,
                "name": format!("GA GraphQL Org Updated {suffix}"),
                "description": "GA GraphQL org updated",
                "website": "https://example.com/graphql-updated"
            }
        }),
    )
    .await?;
    assert_eq!(
        updated_org["updateOrganization"]["description"],
        "GA GraphQL org updated"
    );

    graphql(
        &client,
        &server_url,
        "mutation CreateRepo($input: CreateRepoInput!) {
            createRepo(input: $input) { id username isPublic }
        }",
        json!({
            "input": {
                "orgUsername": org,
                "repoName": format!("GA GraphQL Repo {suffix}"),
                "repoUsername": repo,
                "userId": USER_ID,
                "isPublic": false,
                "description": "GA GraphQL repo"
            }
        }),
    )
    .await?;

    let repo_update = graphql(
        &client,
        &server_url,
        "mutation UpdateRepo($input: UpdateRepoInput!) {
            updateRepo(input: $input) { username isPublic tags }
        }",
        json!({
            "input": {
                "orgUsername": org,
                "repoUsername": repo,
                "name": format!("GA GraphQL Repo Updated {suffix}"),
                "description": "GA GraphQL repo updated",
                "isPublic": true,
                "tags": ["ga", "graphql"]
            }
        }),
    )
    .await?;
    assert_eq!(repo_update["updateRepo"]["isPublic"], true);

    let initial_properties = graphql(
        &client,
        &server_url,
        "query Properties($org: String!, $repo: String!) {
            properties(orgUsername: $org, repoUsername: $repo) {
                id
                typ
                meta { ... on IdType { autoGenerate } }
            }
        }",
        json!({"org": org, "repo": repo}),
    )
    .await?;
    let id_property = initial_properties["properties"]
        .as_array()
        .and_then(|properties| {
            properties.iter().find(|property| property["typ"] == "ID")
        })
        .ok_or_else(|| {
            anyhow::anyhow!("GraphQL typed Id property is missing")
        })?;
    let id_property_id = string_field(id_property, "id")?;
    assert_eq!(id_property["meta"]["autoGenerate"], true);

    let integration_admin = graphql(
        &client,
        &server_url,
        "query IntegrationAdmin($tenantId: String!) {
            integrations { id provider }
            connections(tenantId: $tenantId) { id provider }
        }",
        json!({"tenantId": LIBRARY_TENANT_ID}),
    )
    .await?;
    assert!(integration_admin["integrations"]
        .as_array()
        .is_some_and(|integrations| !integrations.is_empty()));
    assert!(integration_admin["connections"].is_array());

    let property = graphql(
        &client,
        &server_url,
        "mutation AddProperty($input: PropertyInput!) {
            addProperty(input: $input) { id name typ }
        }",
        json!({
            "input": {
                "orgUsername": org,
                "repoUsername": repo,
                "propertyName": "ga_title",
                "propertyType": "STRING"
            }
        }),
    )
    .await?;
    let property_id = string_at(&property, &["addProperty", "id"])?;

    let duplicate_id_response = post_json(
        &client,
        &format!("{server_url}/v1/graphql"),
        json!({
            "query": "mutation AddProperty($input: PropertyInput!) {
                addProperty(input: $input) { id }
            }",
            "variables": {
                "input": {
                    "orgUsername": org,
                    "repoUsername": repo,
                    "propertyName": "ga_duplicate_id",
                    "propertyType": "ID",
                    "meta": {"id": false}
                }
            }
        }),
        StatusCode::OK,
    )
    .await?;
    let duplicate_errors =
        duplicate_id_response["errors"].as_array().ok_or_else(|| {
            anyhow::anyhow!("GraphQL duplicate Id did not return errors")
        })?;
    assert!(!duplicate_errors.is_empty());
    assert!(duplicate_id_response
        .to_string()
        .contains(database_manager::domain::ID_PROPERTY_ALREADY_EXISTS));

    let property_update = graphql(
        &client,
        &server_url,
        "mutation UpdateProperty($id: String!, $input: PropertyInput!) {
            updateProperty(id: $id, input: $input) { id name typ }
        }",
        json!({
            "id": property_id,
            "input": {
                "orgUsername": org,
                "repoUsername": repo,
                "propertyName": "ga_title_updated",
                "propertyType": "STRING"
            }
        }),
    )
    .await?;
    assert_eq!(
        property_update["updateProperty"]["name"],
        "ga_title_updated"
    );

    let data = graphql(
        &client,
        &server_url,
        "mutation AddData($input: AddDataInputData!) {
            addData(input: $input) {
                id
                name
                recordVersion
                propertyData {
                    propertyId
                    value { ... on IdValue { id } }
                }
            }
        }",
        json!({
            "input": {
                "actor": USER_ID,
                "orgUsername": org,
                "repoUsername": repo,
                "dataName": format!("GA GraphQL Data {suffix}"),
                "propertyData": [{
                    "propertyId": property_id,
                    "value": {"string": "graphql-value"}
                }]
            }
        }),
    )
    .await?;
    let data_id = string_at(&data, &["addData", "id"])?;
    assert_eq!(data["addData"]["recordVersion"], "1");
    let generated_id = data["addData"]["propertyData"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["propertyId"] == id_property_id)
        })
        .and_then(|item| item["value"]["id"].as_str())
        .ok_or_else(|| {
            anyhow::anyhow!("GraphQL generated Id value is missing")
        })?;
    assert_eq!(generated_id, data_id);

    let data_update = graphql(
        &client,
        &server_url,
        "mutation UpdateData($input: UpdateDataInputData!) {
            updateData(input: $input) {
                id
                name
                recordVersion
                propertyData {
                    propertyId
                    value { __typename ... on StringValue { string } }
                }
            }
        }",
        json!({
            "input": {
                "actor": USER_ID,
                "orgUsername": org,
                "repoUsername": repo,
                "dataId": data_id,
                "dataName": format!("GA GraphQL Data Updated {suffix}"),
                "propertyData": [{
                    "propertyId": property_id,
                    "value": {"string": "graphql-value-updated"}
                }]
            }
        }),
    )
    .await?;
    assert_eq!(
        data_update["updateData"]["name"],
        format!("GA GraphQL Data Updated {suffix}")
    );
    assert_eq!(data_update["updateData"]["recordVersion"], "1");

    let data_list = graphql(
        &client,
        &server_url,
        "query DataList($org: String!, $repo: String!) {
            dataList(orgUsername: $org, repoUsername: $repo, pageSize: 20, page: 1) {
                items { id name recordVersion }
            }
        }",
        json!({"org": org, "repo": repo}),
    )
    .await?;
    let listed_data = data_list["dataList"]["items"]
        .as_array()
        .ok_or_else(|| {
            anyhow::anyhow!("GraphQL data list items is not an array")
        })?
        .iter()
        .find(|item| item["id"] == data_id)
        .ok_or_else(|| {
            anyhow::anyhow!("GraphQL data list is missing record")
        })?;
    assert_eq!(listed_data["recordVersion"], "1");

    let deleted_data = graphql(
        &client,
        &server_url,
        "mutation DeleteData($org: String!, $repo: String!, $id: String!) {
            deleteData(orgUsername: $org, repoUsername: $repo, dataId: $id)
        }",
        json!({"org": org, "repo": repo, "id": data_id}),
    )
    .await?;
    assert_eq!(deleted_data["deleteData"], data_id);

    let deleted_property = graphql(
        &client,
        &server_url,
        "mutation DeleteProperty($org: String!, $repo: String!, $id: String!) {
            deleteProperty(orgUsername: $org, repoUsername: $repo, propertyId: $id)
        }",
        json!({"org": org, "repo": repo, "id": property_id}),
    )
    .await?;
    assert_eq!(deleted_property["deleteProperty"], property_id);

    let deleted_repo = graphql(
        &client,
        &server_url,
        "mutation DeleteRepo($org: String!, $repo: String!) {
            deleteRepo(orgUsername: $org, repoUsername: $repo)
        }",
        json!({"org": org, "repo": repo}),
    )
    .await?;
    assert_eq!(deleted_repo["deleteRepo"], "ok");

    shutdown_tx.send(()).ok();
    Ok(())
}

#[tokio::test]
async fn create_repo_forwards_general_users_caller_token_and_operator(
) -> anyhow::Result<()> {
    let (server_url, shutdown_tx, auth_state) = setup_test_server().await?;
    let client = create_test_client();
    let suffix = unique_suffix();
    let org = format!("ca-org-{suffix}");
    let repo = format!("ca-repo-{suffix}");

    let created_org = graphql(
        &client,
        &server_url,
        "mutation CreateOrg($input: CreateOrganizationInput!) {
            createOrganization(input: $input) { id username }
        }",
        json!({
            "input": {
                "name": format!("Caller Auth Org {suffix}"),
                "username": org,
                "description": "caller auth regression",
                "website": null
            }
        }),
    )
    .await?;
    let operator_id =
        string_at(&created_org, &["createOrganization", "id"])?;
    assert_ne!(operator_id, LIBRARY_TENANT_ID);

    let created_repo = graphql_for_operator(
        &client,
        &server_url,
        &operator_id,
        "mutation CreateRepo($input: CreateRepoInput!) {
            createRepo(input: $input) { id username }
        }",
        json!({
            "input": {
                "orgUsername": org,
                "repoName": format!("Caller Auth Repo {suffix}"),
                "repoUsername": repo,
                "userId": USER_ID,
                "isPublic": false,
                "description": "caller auth regression"
            }
        }),
    )
    .await?;
    assert_eq!(created_repo["createRepo"]["username"], repo);

    let checks = auth_state.policy_checks.lock().unwrap();
    let create_repo_check = checks
        .iter()
        .find(|check| check.actions == ["library:CreateRepo"])
        .ok_or_else(|| {
            anyhow::anyhow!("CreateRepo policy check missing")
        })?;
    assert_eq!(
        create_repo_check.authorization.as_deref(),
        Some("Bearer caller-user-token")
    );
    assert_ne!(
        create_repo_check.authorization.as_deref(),
        Some("Bearer base-service-token")
    );
    assert_eq!(
        create_repo_check.operator_id.as_deref(),
        Some(operator_id.as_str())
    );
    drop(checks);

    shutdown_tx.send(()).ok();
    Ok(())
}

#[derive(Clone)]
struct FakeAuthState {
    library_tenant: String,
    operators: Arc<Mutex<HashMap<String, String>>>,
    policy_checks: Arc<Mutex<Vec<PolicyCheck>>>,
}

#[derive(Debug, Clone)]
struct PolicyCheck {
    authorization: Option<String>,
    operator_id: Option<String>,
    actions: Vec<String>,
}

async fn setup_test_server(
) -> anyhow::Result<(String, oneshot::Sender<()>, FakeAuthState)> {
    std::env::set_var("ENVIRONMENT", "test");
    std::env::set_var("SKIP_MINIO_SETUP", "1");
    std::env::set_var("AWS_EC2_METADATA_DISABLED", "true");
    std::env::set_var("ROOT_ID", LIBRARY_TENANT_ID);
    std::env::set_var("LIBRARY_TENANT_ID", LIBRARY_TENANT_ID);
    dotenvy::dotenv().ok();
    let dsn = std::env::var("LIBRARY_DATABASE_URL")
        .or_else(|_| {
            std::env::var("DEV_DATABASE_URL")
                .map(|url| format!("{}/library", url))
        })
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| {
            "mysql://root:@localhost:15000/library".to_string()
        })
        .parse::<DatabaseUrl>()?;

    let (fake_auth_url, fake_auth_state) = spawn_fake_auth_server().await?;
    let library_tenant = LIBRARY_TENANT_ID.parse::<TenantId>()?;
    let sdk = Arc::new(library_api::sdk_auth::SdkAuthApp::new(
        fake_auth_url,
        &library_tenant,
        BASE_AUTH_TOKEN,
    ));
    let database_app = Arc::new(
        database_manager::factory_client(
            dsn.use_database("tachyon_apps_database_manager"),
        )
        .await?,
    );
    let github = Arc::new(github_provider::GitHub::new(None));
    let oauth_service: Arc<dyn inbound_sync_domain::OAuthService> =
        Arc::new(MockOAuthService);
    let oauth_token_repo: Arc<
        dyn inbound_sync_domain::OAuthTokenRepository,
    > = Arc::new(InMemoryOAuthTokenRepository::default());
    let provider_secrets =
        Arc::new(inbound_sync::WebhookSecretStore::new());
    let database_layout =
        library_api::DatabaseLayout::resolve(&dsn, Some("test"))?;

    let app = library_api::router(
        database_layout.open_pools(),
        sdk,
        database_app,
        github,
        oauth_service,
        oauth_token_repo,
        provider_secrets,
    )
    .await
    .map_err(|error| anyhow::anyhow!("{error}"))?;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server_url = format!("http://{addr}");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
            .unwrap();
    });

    Ok((server_url, shutdown_tx, fake_auth_state))
}

async fn spawn_fake_auth_server() -> anyhow::Result<(String, FakeAuthState)>
{
    let state = FakeAuthState {
        library_tenant: LIBRARY_TENANT_ID.to_string(),
        operators: Arc::new(Mutex::new(HashMap::new())),
        policy_checks: Arc::new(Mutex::new(Vec::new())),
    };
    let app = Router::new()
        .route("/auth/v1beta/verify", post(fake_verify))
        .route("/v1/auth/policies/check", post(fake_check_policy))
        .route(
            "/v1/auth/policies/check-for-resource",
            post(fake_check_policy_for_resource),
        )
        .route("/v1/auth/operators", post(fake_create_operator))
        .route(
            "/v1/auth/operators/by-alias",
            get(fake_get_operator_by_alias),
        )
        .route("/v1/auth/operators/{id}", get(fake_get_operator))
        .route("/v1/auth/users/{id}", get(fake_get_user))
        .route("/v1/auth/user-policies/attach", post(fake_ok))
        .route("/v1/auth/user-policies/attach-with-scope", post(fake_ok))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Ok((format!("http://{addr}"), state))
}

async fn fake_check_policy(
    State(state): State<FakeAuthState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    let actions = body["actions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|action| action.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    state.policy_checks.lock().unwrap().push(PolicyCheck {
        authorization: headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
        operator_id: headers
            .get("x-operator-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
        actions: actions.clone(),
    });
    let results = body["actions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|action| {
            json!({
                "action": action.as_str().unwrap_or_default(),
                "allowed": true
            })
        })
        .collect::<Vec<_>>();
    Json(json!({ "results": results }))
}

async fn fake_check_policy_for_resource() -> Json<Value> {
    Json(json!({ "allowed": true }))
}

async fn fake_verify() -> Json<Value> {
    Json(json!({
        "user": {
            "id": USER_ID,
            "email": "ga-crud@example.com",
            "name": "GA CRUD Test User",
            "role": "General",
            "tenants": [LIBRARY_TENANT_ID]
        }
    }))
}

async fn fake_create_operator(
    State(state): State<FakeAuthState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let operator_alias = body["operatorAlias"].as_str().unwrap_or("ga-org");
    let operator_name =
        body["operatorName"].as_str().unwrap_or(operator_alias);
    let operator_id = TenantId::default().to_string();
    state
        .operators
        .lock()
        .unwrap()
        .insert(operator_alias.to_string(), operator_id.clone());
    Json(json!({
        "operator": {
            "id": operator_id,
            "name": operator_name,
            "operatorName": operator_alias,
            "platformId": state.library_tenant
        },
        "ownerId": USER_ID
    }))
}

async fn fake_get_user(
    State(state): State<FakeAuthState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Json<Value> {
    let tenant = headers
        .get("x-operator-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or(&state.library_tenant);
    Json(json!({
        "id": id,
        "email": "ga-crud@example.com",
        "name": "GA CRUD Test User",
        "role": "Owner",
        "tenants": [tenant]
    }))
}

async fn fake_get_operator(
    State(state): State<FakeAuthState>,
    Path(id): Path<String>,
) -> Json<Value> {
    Json(json!({
        "id": id,
        "name": "GA CRUD Operator",
        "operatorName": "ga-crud-operator",
        "platformId": state.library_tenant
    }))
}

async fn fake_get_operator_by_alias(
    State(state): State<FakeAuthState>,
    Query(query): Query<HashMap<String, String>>,
) -> Json<Value> {
    let alias = query
        .get("alias")
        .map(String::as_str)
        .unwrap_or("ga-crud-operator");
    let operator_id = state
        .operators
        .lock()
        .unwrap()
        .get(alias)
        .cloned()
        .unwrap_or_else(|| TenantId::default().to_string());
    Json(json!({
        "id": operator_id,
        "name": "GA CRUD Operator",
        "operatorName": alias,
        "platformId": state.library_tenant
    }))
}

async fn fake_ok() -> Json<Value> {
    Json(json!({}))
}

#[derive(Debug)]
struct MockOAuthService;

#[async_trait::async_trait]
impl inbound_sync_domain::OAuthService for MockOAuthService {
    async fn init_authorization(
        &self,
        _input: inbound_sync_domain::InitOAuthInput,
    ) -> errors::Result<inbound_sync_domain::InitOAuthOutput> {
        Ok(inbound_sync_domain::InitOAuthOutput {
            authorization_url: "https://example.com/oauth".to_string(),
            state: "mock-state".to_string(),
        })
    }

    async fn exchange_code(
        &self,
        _input: inbound_sync_domain::ExchangeOAuthCodeInput,
    ) -> errors::Result<inbound_sync_domain::StoredOAuthToken> {
        unimplemented!("mock OAuth exchange is outside CRUD regression")
    }

    async fn refresh_token(
        &self,
        _tenant_id: &TenantId,
        _provider: inbound_sync_domain::OAuthProvider,
    ) -> errors::Result<inbound_sync_domain::StoredOAuthToken> {
        unimplemented!("mock OAuth refresh is outside CRUD regression")
    }

    async fn revoke_token(
        &self,
        _tenant_id: &TenantId,
        _provider: inbound_sync_domain::OAuthProvider,
    ) -> errors::Result<()> {
        Ok(())
    }

    fn get_credentials(
        &self,
        _provider: inbound_sync_domain::OAuthProvider,
    ) -> Option<&inbound_sync_domain::OAuthClientCredentials> {
        None
    }
}

fn create_test_client() -> Client {
    Client::new()
}

async fn post_json(
    client: &Client,
    url: &str,
    body: Value,
    expected: StatusCode,
) -> anyhow::Result<Value> {
    request_json(client, Method::POST, url, Some(body), expected).await
}

async fn put_json(
    client: &Client,
    url: &str,
    body: Value,
    expected: StatusCode,
) -> anyhow::Result<Value> {
    request_json(client, Method::PUT, url, Some(body), expected).await
}

async fn get_json(
    client: &Client,
    url: &str,
    expected: StatusCode,
) -> anyhow::Result<Value> {
    request_json(client, Method::GET, url, None, expected).await
}

async fn request_json(
    client: &Client,
    method: Method,
    url: &str,
    body: Option<Value>,
    expected: StatusCode,
) -> anyhow::Result<Value> {
    let text = request_text(client, url, method, body, expected).await?;
    serde_json::from_str(&text).map_err(Into::into)
}

async fn delete(
    client: &Client,
    url: &str,
    expected: StatusCode,
) -> anyhow::Result<()> {
    request_text(client, url, Method::DELETE, None, expected).await?;
    Ok(())
}

async fn request_text(
    client: &Client,
    url: &str,
    method: Method,
    body: Option<Value>,
    expected: StatusCode,
) -> anyhow::Result<String> {
    let mut request = client
        .request(method, url)
        .header("Authorization", format!("Bearer {TOKEN}"))
        .header("Content-Type", "application/json")
        .header("x-platform-id", LIBRARY_TENANT_ID)
        .header("x-user-id", USER_ID);
    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await?;
    let status = response.status();
    let text = response.text().await?;
    if status != expected {
        anyhow::bail!(
            "expected status {expected}, got {status}; response: {text}"
        );
    }
    Ok(text)
}

async fn graphql(
    client: &Client,
    server_url: &str,
    query: &str,
    variables: Value,
) -> anyhow::Result<Value> {
    graphql_with_optional_operator(
        client, server_url, None, query, variables,
    )
    .await
}

async fn graphql_for_operator(
    client: &Client,
    server_url: &str,
    operator_id: &str,
    query: &str,
    variables: Value,
) -> anyhow::Result<Value> {
    graphql_with_optional_operator(
        client,
        server_url,
        Some(operator_id),
        query,
        variables,
    )
    .await
}

async fn graphql_with_optional_operator(
    client: &Client,
    server_url: &str,
    operator_id: Option<&str>,
    query: &str,
    variables: Value,
) -> anyhow::Result<Value> {
    let mut request = client
        .post(format!("{server_url}/v1/graphql"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .header("Content-Type", "application/json")
        .header("x-platform-id", LIBRARY_TENANT_ID)
        .header("x-user-id", USER_ID)
        .json(&json!({"query": query, "variables": variables}));
    if let Some(operator_id) = operator_id {
        request = request.header("x-operator-id", operator_id);
    }
    let response = request.send().await?;
    let status = response.status();
    let text = response.text().await?;
    if status != StatusCode::OK {
        anyhow::bail!("expected 200 OK, got {status}; response: {text}");
    }
    let response: Value = serde_json::from_str(&text)?;

    if let Some(errors) = response.get("errors") {
        anyhow::bail!("GraphQL errors: {errors}");
    }

    response
        .get("data")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("GraphQL response missing data"))
}

fn string_field(value: &Value, field: &str) -> anyhow::Result<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("missing string field: {field}"))
}

fn string_at(value: &Value, path: &[&str]) -> anyhow::Result<String> {
    let mut current = value;
    for segment in path {
        current = current.get(segment).ok_or_else(|| {
            anyhow::anyhow!("missing path segment: {segment}")
        })?;
    }
    current
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("path is not a string: {path:?}"))
}

fn unique_suffix() -> String {
    format!(
        "{}-{}",
        chrono::Utc::now().timestamp_millis(),
        rand::random::<u32>()
    )
}
