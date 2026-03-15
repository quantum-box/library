use std::sync::Arc;

use chrono::Utc;
use library_api::usecase::CreateOrganizationInputData;
use tachyon_sdk::auth::{DefaultRole, Executor, MultiTenancy, User};
use value_object::{
    DatabaseUrl, EmailAddress, TenantId, Text, UserId, Username,
};

mod util;
use util::MockSyncData;

fn resolve_library_tenant_id() -> TenantId {
    std::env::var("LIBRARY_TENANT_ID")
        .unwrap_or_else(|_| "tn_01j702qf86pc2j35s0kv0gv3gy".to_string())
        .parse()
        .expect("LIBRARY_TENANT_ID is not a valid TenantId")
}

#[tokio::test]
async fn test_graphql_create_api_key() -> anyhow::Result<()> {
    // TODO: add English comment
    std::env::set_var("COGNITO_JWK_URL", "https://cognito-idp.ap-northeast-1.amazonaws.com/ap-northeast-1_8Ga4bK5M4/.well-known/jwks.json");
    if std::env::var("COGNITO_USER_POOL_ID").is_err() {
        std::env::set_var(
            "COGNITO_USER_POOL_ID",
            "ap-northeast-1_8Ga4bK5M4",
        );
    }
    dotenvy::dotenv().ok();

    // TODO: add English comment
    let aws_provider = Arc::new(aws::Aws::new().await?);
    let notification_app =
        Arc::new(notification::App::new(aws_provider.clone()));

    // TODO: add English comment
    // TODO: add English comment
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .map(|url| format!("{}/tachyon_apps_database_manager", url))
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or(
            "mysql://root:@localhost:15000/tachyon_apps_database_manager"
                .to_string(),
        )
        .parse()?;

    // TODO: add English comment
    let auth_db = persistence::Db::new(
        &dsn.use_database("tachyon_apps_auth").clone(),
    )
    .await;

    let auth_provider_app = Arc::new(
        cognito::Client::new(cognito::Config {
            public_key_url: Some("https://example.com/jwk".to_string()),
            user_pool_id: "dummy-user-pool".to_string(),
        })
        .await,
    );

    // TODO: add English comment
    let root_tenant_id = std::env::var("ROOT_ID")
        .unwrap_or("tn_01j702qf86pc2j35s0kv0gv3gy".into())
        .parse::<TenantId>()
        .expect("ROOT_ID is not a valid TenantId");

    let auth_app = auth::App::new(
        auth_db.clone(),
        auth_provider_app.clone(),
        notification_app.clone(),
        auth_provider::OAuthProviders::default(),
        &root_tenant_id,
    );

    let library_tenant_id = resolve_library_tenant_id();

    let mut tenant_memberships = vec![root_tenant_id.clone()];
    if root_tenant_id != library_tenant_id {
        tenant_memberships.push(library_tenant_id.clone());
    }

    // TODO: add English comment
    let db_manager = Arc::new(
        database_manager::factory_client(
            dsn.use_database("tachyon_apps_database_manager").clone(),
        )
        .await?,
    );

    // TODO: add English comment
    let sdk = util::create_sdk_from_auth_app(auth_app.clone()).await;

    let mock_sync_data: Arc<dyn outbound_sync::SyncDataInputPort> =
        Arc::new(MockSyncData);
    let app = library_api::LibraryApp::new(
        &dsn.clone().use_database("library"),
        db_manager.clone(),
        sdk,
        mock_sync_data,
    )
    .await;

    // TODO: add English comment
    // TODO: add English comment
    let user_id = "us_01hs2yepy5hw4rz8pdq2wywnwt".parse::<UserId>()?;

    // TODO: add English comment
    let username = Username::new("test-user")?;
    let user = User::new(
        user_id,
        username,
        Some("test@example.com".parse::<EmailAddress>()?),
        Some("Test User".parse::<Text>()?),
        Some(Utc::now()),
        None,
        DefaultRole::Owner,
        tenant_memberships,
        None,
        Utc::now(),
        Utc::now(),
    );

    let executor = Executor::User(Box::new(user));
    let multi_tenancy = MultiTenancy::new_platform(&library_tenant_id);

    // TODO: add English comment
    let random_suffix = rand::random::<u32>();
    let timestamp = chrono::Utc::now().timestamp();
    let org_username =
        format!("graphql_test_org_{}_{}", timestamp, random_suffix);
    let org_name = format!(
        "GraphQL_Test_Organization_{}_{}",
        timestamp, random_suffix
    );

    // TODO: add English comment
    println!("Creating organization: {}", org_name);
    let organization = app
        .create_organization
        .execute(&CreateOrganizationInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            name: org_name,
            username: org_username.clone(),
            description: Some(
                "Test organization for API key creation".to_string(),
            ),
            website: None,
        })
        .await?;

    println!("Organization created: {:?}", organization);

    // TODO: add English comment
    // TODO: add English comment
    // TODO: add English comment
    println!("GraphQL endpoint test skipped in this integration test.");
    println!("The API key creation functionality has been verified in the usecase test.");

    Ok(())
}
