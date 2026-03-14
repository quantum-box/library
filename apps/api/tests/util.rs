#![allow(dead_code)]

use dotenvy::dotenv;
use inbound_sync::interface_adapter::InMemoryOAuthTokenRepository;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use value_object::DatabaseUrl;

/// Mock implementation of OAuthService for testing
#[derive(Debug)]
pub struct MockOAuthService;

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
        unimplemented!("Mock OAuth service does not support code exchange")
    }

    async fn refresh_token(
        &self,
        _tenant_id: &value_object::TenantId,
        _provider: inbound_sync_domain::OAuthProvider,
    ) -> errors::Result<inbound_sync_domain::StoredOAuthToken> {
        unimplemented!("Mock OAuth service does not support token refresh")
    }

    async fn revoke_token(
        &self,
        _tenant_id: &value_object::TenantId,
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

/// Mock implementation of SyncDataInputPort for testing
pub struct MockSyncData;

#[async_trait::async_trait]
impl outbound_sync::SyncDataInputPort for MockSyncData {
    async fn execute<'a>(
        &self,
        _input: &outbound_sync::SyncDataInputData<'a>,
    ) -> errors::Result<outbound_sync::SyncDataResult> {
        Ok(outbound_sync::SyncDataResult {
            status: outbound_sync::SyncStatus::Synced,
            result_id: Some("mock-result-id".to_string()),
            url: Some("https://github.com/mock/repo".to_string()),
            diff: None,
        })
    }

    async fn delete<'a>(
        &self,
        _input: &outbound_sync::DeleteDataInputData<'a>,
    ) -> errors::Result<outbound_sync::SyncDataResult> {
        Ok(outbound_sync::SyncDataResult {
            status: outbound_sync::SyncStatus::Synced,
            result_id: None,
            url: None,
            diff: None,
        })
    }
}

/// TODO: add English documentation
#[tracing::instrument]
pub async fn setup_test_server() -> (String, oneshot::Sender<()>) {
    std::env::set_var(
        "COGNITO_JWK_URL",
        "https://cognito-idp.ap-northeast-1.amazonaws.com/ap-northeast-1_8Ga4bK5M4/.well-known/jwks.json",
    );
    std::env::set_var("ENVIRONMENT", "test");
    std::env::set_var("SKIP_MINIO_SETUP", "1");
    dotenv().ok();

    // TODO: add English comment
    let dsn = std::env::var("LIBRARY_DATABASE_URL")
        .or_else(|_| {
            std::env::var("DEV_DATABASE_URL")
                .map(|url| format!("{}/library", url))
        })
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or("mysql://root:@localhost:15000/library".to_string())
        .parse::<DatabaseUrl>()
        .unwrap();

    // TODO: add English comment
    let database_app = Arc::new(
        database_manager::factory_client(
            &dsn.use_database("tachyon_apps_database_manager"),
        )
        .await
        .unwrap(),
    );

    // TODO: add English comment
    let aws_client = Arc::new(aws::Aws::new().await.unwrap());
    let notification_app =
        Arc::new(notification::App::new(aws_client.clone()));

    // TODO: add English comment
    let auth_db =
        persistence::Db::new(&dsn.use_database("tachyon_apps_auth")).await;
    let cognito = Arc::new(
        cognito::Client::new(cognito::Config {
            public_key_url: None,
            user_pool_id: "test-user-pool".to_string(),
        })
        .await,
    );

    let root_id = std::env::var("ROOT_ID")
        .unwrap_or("tn_01hjryxysgey07h5jz5wagqj0m".to_string())
        .parse()
        .expect("ROOT_ID is not a valid TenantId");

    let auth_app = auth::App::new(
        auth_db.clone(),
        cognito.clone(),
        notification_app.clone(),
        auth_provider::OAuthProviders::default(),
        &root_id,
    );
    // auth::App::new returns Arc<auth::App>; extract inner
    let auth_app = Arc::into_inner(auth_app).unwrap();
    let auth_app = Arc::new(auth_app);

    // Start auth REST server so SdkAuthApp can call it
    let auth_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let auth_addr = auth_listener.local_addr().unwrap();
    let auth_url = format!("http://{}", auth_addr);

    let auth_router = auth::interface_adapter::axum::create_router()
        .layer(axum::Extension(auth_app.clone()));
    tokio::spawn(async move {
        axum::serve(auth_listener, auth_router).await.unwrap();
    });

    // Create SdkAuthApp pointing to the auth REST server
    let sdk = Arc::new(library_api::sdk_auth::SdkAuthApp::new(
        &auth_url,
        &root_id,
        "dummy-token",
    ));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://{}", addr);

    let _library_db =
        persistence::Db::new(&dsn.use_database("library")).await;
    let mock_sync_data: Arc<dyn outbound_sync::SyncDataInputPort> =
        Arc::new(MockSyncData);
    let library_app = Arc::new(
        library_api::LibraryApp::new(
            &dsn.use_database("library"),
            database_app.clone(),
            sdk.clone(),
            mock_sync_data,
        )
        .await,
    );

    // GitHub provider (test mode without OAuth)
    let github = Arc::new(github_provider::GitHub::new(None));

    // Mock OAuth service for testing
    let oauth_service: Arc<dyn inbound_sync_domain::OAuthService> =
        Arc::new(MockOAuthService);

    let oauth_token_repo: Arc<
        dyn inbound_sync_domain::OAuthTokenRepository,
    > = Arc::new(InMemoryOAuthTokenRepository::default());
    let provider_secrets =
        Arc::new(inbound_sync::WebhookSecretStore::new());

    let app = library_api::router(
        dsn.to_string(),
        sdk,
        database_app.clone(),
        github,
        oauth_service,
        oauth_token_repo,
        provider_secrets,
    )
    .await
    .unwrap();

    // TODO: add English comment
    let app = app.layer(axum::Extension(library_app));

    // TODO: add English comment
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // TODO: add English comment
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            })
            .await
            .unwrap();
    });

    (server_url, shutdown_tx)
}

/// Start an auth REST server and create an SdkAuthApp
/// pointing to it. Returns the SdkAuthApp.
pub async fn create_sdk_from_auth_app(
    auth_app: Arc<auth::App>,
) -> Arc<library_api::sdk_auth::SdkAuthApp> {
    let auth_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let auth_addr = auth_listener.local_addr().unwrap();
    let auth_url = format!("http://{}", auth_addr);

    let auth_router = auth::interface_adapter::axum::create_router()
        .layer(axum::Extension(auth_app));
    tokio::spawn(async move {
        axum::serve(auth_listener, auth_router).await.unwrap();
    });

    let default_operator_id: value_object::TenantId =
        std::env::var("ROOT_ID")
            .unwrap_or("tn_01hjryxysgey07h5jz5wagqj0m".to_string())
            .parse()
            .expect("ROOT_ID is not a valid TenantId");

    Arc::new(library_api::sdk_auth::SdkAuthApp::new(
        &auth_url,
        &default_operator_id,
        "dummy-token",
    ))
}

/// TODO: add English documentation
pub fn generate_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// TODO: add English documentation
pub fn create_test_client() -> Client {
    Client::new()
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
pub async fn send_api_request<T>(
    client: &Client,
    method: reqwest::Method,
    url: &str,
    body: Option<T>,
    auth_token: &str,
) -> Result<Response, reqwest::Error>
where
    T: serde::Serialize,
{
    // TODO: add English comment
    let mut request_builder = client
        .request(method, url)
        .header("Authorization", format!("Bearer {}", auth_token));

    // TODO: add English comment
    if let Some(body) = body {
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .json(&body);
    }

    // TODO: add English comment
    request_builder.send().await
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
pub async fn process_response(
    response: Response,
    expected_status: StatusCode,
) -> anyhow::Result<(StatusCode, String)> {
    // TODO: add English comment
    let status = response.status();

    // TODO: add English comment
    let text = response.text().await?;

    // TODO: add English comment
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        return Err(anyhow::anyhow!("サーバーエラー (500): {}", text));
    }

    // TODO: add English comment
    // TODO: add English comment
    if expected_status != StatusCode::OK && status != expected_status {
        return Err(anyhow::anyhow!(
            "期待するステータスコード({})と実際のステータスコード({})が一致しません。レスポンス: {}",
            expected_status,
            status,
            text
        ));
    }

    Ok((status, text))
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
pub fn parse_json_response(response_text: &str) -> anyhow::Result<Value> {
    serde_json::from_str::<Value>(response_text).map_err(Into::into)
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
pub fn deserialize_response<T>(response_text: &str) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str::<T>(response_text).map_err(Into::into)
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
pub async fn send_and_process_request<T>(
    client: &Client,
    method: reqwest::Method,
    url: &str,
    body: Option<T>,
    auth_token: &str,
    expected_status: StatusCode,
) -> anyhow::Result<(StatusCode, String)>
where
    T: serde::Serialize,
{
    // TODO: add English comment
    let response =
        send_api_request(client, method, url, body, auth_token).await?;

    // TODO: add English comment
    process_response(response, expected_status).await
}

/// TODO: add English documentation
///
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
/// TODO: add English documentation
pub fn print_response_result(
    operation_name: &str,
    status: StatusCode,
    text: &str,
) {
    println!("{} レスポンスステータス: {}", operation_name, status);
    println!("{} レスポンスボディ: {}", operation_name, text);

    if status.is_success() {
        println!("✅ {} - 成功", operation_name);
    } else {
        println!(
            "⚠️ {} - 警告: 期待するステータスコードではありません: {}",
            operation_name, status
        );
    }
}
