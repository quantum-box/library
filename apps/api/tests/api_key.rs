use std::sync::Arc;

use tachyon_sdk::auth::{DefaultRole, Executor, MultiTenancy, User};
use chrono::Utc;
use library_api::usecase::{
    CreateApiKeyInputData, CreateOrganizationInputData, CreateRepoInputData,
};
use tachyon_sdk::auth::MultiTenancyAction;
use value_object::{
    DatabaseUrl, EmailAddress, Identifier, TenantId, Text, UserId, Username,
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
async fn test_create_api_key() -> anyhow::Result<()> {
    // TODO: add English comment
    std::env::set_var("COGNITO_JWK_URL", "https://cognito-idp.ap-northeast-1.amazonaws.com/your-cognito-user-pool-id/.well-known/jwks.json");
    if std::env::var("COGNITO_USER_POOL_ID").is_err() {
        std::env::set_var(
            "COGNITO_USER_POOL_ID",
            "your-cognito-user-pool-id",
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
        user_id.clone(),
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
        format!("api_test_org_{}_{}", timestamp, random_suffix);
    let org_name =
        format!("API_Test_Organization_{}_{}", timestamp, random_suffix);

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
    println!("Creating API key for organization: {}", organization.id());
    let result = app
        .create_api_key
        .execute(&CreateApiKeyInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            org_name: &organization.username().to_string().parse().unwrap(),
            name: "Test API Key",
            service_account_name: Some("default"),
        })
        .await?;

    println!("API key created: {:?}", result);

    // TODO: add English comment
    assert!(!result.api_key.value().to_string().is_empty());
    assert_eq!(result.api_key.name().to_string(), "Test API Key");
    assert_eq!(result.service_account.name().to_string(), "default");

    Ok(())
}

#[tokio::test]
async fn test_api_key_access_control() -> anyhow::Result<()> {
    // TODO: add English comment
    std::env::set_var("COGNITO_JWK_URL", "https://cognito-idp.ap-northeast-1.amazonaws.com/your-cognito-user-pool-id/.well-known/jwks.json");
    if std::env::var("COGNITO_USER_POOL_ID").is_err() {
        std::env::set_var(
            "COGNITO_USER_POOL_ID",
            "your-cognito-user-pool-id",
        );
    }
    dotenvy::dotenv().ok();

    // TODO: add English comment
    let aws_provider = Arc::new(aws::Aws::new().await?);
    let notification_app =
        Arc::new(notification::App::new(aws_provider.clone()));

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

    let mock_sync_data2: Arc<dyn outbound_sync::SyncDataInputPort> =
        Arc::new(MockSyncData);
    let app = library_api::LibraryApp::new(
        &dsn.clone().use_database("library"),
        db_manager.clone(),
        sdk,
        mock_sync_data2,
    )
    .await;

    // TODO: add English comment
    let user_id = "us_01hs2yepy5hw4rz8pdq2wywnwt".parse::<UserId>()?;
    let username = Username::new("test-user")?;
    let user = User::new(
        user_id.clone(),
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
    let executor = Executor::User(Box::new(user.clone()));
    let multi_tenancy = MultiTenancy::new_platform(&library_tenant_id);

    // TODO: add English comment
    let timestamp = chrono::Utc::now().timestamp();
    let random_suffix1 = rand::random::<u32>();
    let random_suffix2 = rand::random::<u32>();

    // TODO: add English comment
    let org1_username =
        format!("api_test_org1_{}_{}", timestamp, random_suffix1);
    let org1_name =
        format!("API_Test_Organization1_{}_{}", timestamp, random_suffix1);

    println!("Creating first organization: {}", org1_name);
    let organization1 = app
        .create_organization
        .execute(&CreateOrganizationInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            name: org1_name,
            username: org1_username.clone(),
            description: Some("First test organization".to_string()),
            website: None,
        })
        .await?;
    println!("First organization created: {:?}", organization1);

    let user = user.grant_tenant_access(organization1.id())?;
    let executor = Executor::User(Box::new(user.clone()));

    let multi_tenancy = MultiTenancy::new(
        multi_tenancy.platform_id().clone(),
        Some(organization1.id().clone()),
    );

    // TODO: add English comment
    let repo1_name = format!("test-repo1-{}-{}", timestamp, random_suffix1);
    let repo1_username =
        format!("test-repo1-{}-{}", timestamp, random_suffix1);

    println!("Creating repository in first organization");
    let repo1 = app
        .create_repo
        .execute(CreateRepoInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            org_username: organization1.username().to_string(),
            repo_name: repo1_name.clone(),
            repo_username: repo1_username.clone(),
            user_id: user_id.to_string(),
            is_public: true,
            database_id: None,
            description: None,
            skip_sample_data: false,
        })
        .await?;
    println!("Repository created in first organization: {:?}", repo1);

    let multi_tenancy2 = MultiTenancy::new_platform(&library_tenant_id);

    // TODO: add English comment
    let org2_username =
        format!("api_test_org2_{}_{}", timestamp, random_suffix2);
    let org2_name =
        format!("API_Test_Organization2_{}_{}", timestamp, random_suffix2);

    println!("Creating second organization: {}", org2_name);
    let organization2 = app
        .create_organization
        .execute(&CreateOrganizationInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            name: org2_name,
            username: org2_username.clone(),
            description: Some("Second test organization".to_string()),
            website: None,
        })
        .await?;
    println!("Second organization created: {:?}", organization2);

    let multi_tenancy2 = MultiTenancy::new(
        multi_tenancy2.platform_id().clone(),
        Some(organization2.id().clone()),
    );

    // TODO: add English comment
    let repo2_name = format!("test-repo2-{}-{}", timestamp, random_suffix2);
    let repo2_username =
        format!("test-repo2-{}-{}", timestamp, random_suffix2);

    println!("Creating repository in second organization");

    let user = user.grant_tenant_access(organization2.id())?;
    let executor = Executor::User(Box::new(user));

    let repo2 = app
        .create_repo
        .execute(CreateRepoInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy2,
            org_username: organization2.username().to_string(),
            repo_name: repo2_name.clone(),
            repo_username: repo2_username.clone(),
            user_id: user_id.to_string(),
            is_public: true,
            database_id: None,
            description: None,
            skip_sample_data: false,
        })
        .await?;
    println!("Repository created in second organization: {:?}", repo2);

    // TODO: add English comment
    println!(
        "Creating API key for first organization: {}",
        organization1.id()
    );
    let api_key_result = app
        .create_api_key
        .execute(&CreateApiKeyInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            org_name: &organization1
                .username()
                .to_string()
                .parse()
                .unwrap(),
            name: "Test API Key",
            service_account_name: Some("default"),
        })
        .await?;
    println!(
        "API key created for first organization: {:?}",
        api_key_result
    );
    assert_eq!(
        api_key_result.service_account.tenant_id().clone(),
        multi_tenancy.operator_id().unwrap()
    );

    // TODO: add English comment
    let api_key_executor = Executor::ServiceAccount(Box::new(
        api_key_result.service_account.clone(),
    ));

    // TODO: add English comment
    println!("Accessing repository in first organization using API key");
    let repo1_alias = org1_username.parse::<Identifier>()?;
    let repo1_username_alias = repo1_username.parse::<Identifier>()?;

    let repo1_result = app
        .view_repo
        .execute(&library_api::usecase::ViewRepoInputData {
            executor: &api_key_executor,
            multi_tenancy: &multi_tenancy,
            organization_username: repo1_alias.to_string(),
            repo_username: repo1_username_alias.to_string(),
        })
        .await;

    assert!(
        repo1_result.is_ok(),
        "APIキーは同じ組織内のリポジトリにアクセスできるべきです"
    );
    println!("Successfully accessed repository in first organization using API key");

    // TODO: add English comment
    println!("Attempting to access repository in second organization using API key");
    let org2_alias = org2_username.parse::<Identifier>()?;
    let repo2_username_alias = repo2_username.parse::<Identifier>()?;

    let repo2_result = app
        .view_repo
        .execute(&library_api::usecase::ViewRepoInputData {
            executor: &api_key_executor,
            multi_tenancy: &multi_tenancy,
            organization_username: org2_alias.to_string(),
            repo_username: repo2_username_alias.to_string(),
        })
        .await;

    // TODO: add English comment
    dbg!(&repo2_result);
    assert!(
        repo2_result.is_err(),
        "APIキーは異なる組織のリポジトリにアクセスできない"
    );
    println!("As expected, could not access repository in second organization using API key");

    Ok(())
}
