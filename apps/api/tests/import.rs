use csv_importer::{CSVImporter, CSVImporterClient, DatabaseConfig};
use database_manager::domain::DatabaseId;
use std::io::Read;
use std::sync::Arc;
use value_object::DatabaseUrl;

mod util;
use util::MockSyncData;

#[tokio::test]
#[ignore]
async fn test_big_csv_preview() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let dsn = std::env::var("DEV_DATABASE_URL")
        .map(|url| format!("{}/tachyon_apps_database_manager", url))
        .unwrap_or_else(|_| {
            "mysql://root:@localhost:15000/tachyon_apps_database_manager"
                .to_string()
        });
    let db_manager = database_manager::factory_client(dsn).await?;
    let csv_importer = CSVImporterClient::new(Arc::new(db_manager));
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("kihonjoho_20241007.csv");
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    let buffer = std::io::Cursor::new(buffer);
    let preview = csv_importer.preview(buffer, Some(10)).await?;
    for record in preview.iter() {
        println!("{:?}", record);
    }
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_import_and_create_database() -> anyhow::Result<()> {
    std::env::set_var("COGNITO_JWK_URL", "https://cognito-idp.ap-northeast-1.amazonaws.com/your-cognito-user-pool-id/.well-known/jwks.json");
    dotenvy::dotenv().ok();

    let aws_provider = Arc::new(aws::Aws::new().await?);
    let notification_app =
        Arc::new(notification::App::new(aws_provider.clone()));

    // use clap::Parser;
    // let config = library_api::Config::parse();
    // if config.environment == "development" {
    //     println!("{config:#?}");
    // }
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .map(|url| format!("{}/tachyon_apps_database_manager", url))
        .unwrap_or_else(|_| {
            "mysql://root:@localhost:15000/tachyon_apps_database_manager"
                .to_string()
        })
        .parse()?;

    let auth_db = persistence::Db::new(
        &dsn.use_database("tachyon_apps_auth").clone(),
    )
    .await;
    let auth_provider_app = Arc::new(
        cognito::Client::new(cognito::Config {
            ..Default::default()
        })
        .await,
    );
    let auth_app = auth::App::new(
        auth_db.clone(),
        auth_provider_app.clone(),
        notification_app.clone(),
        auth_provider::OAuthProviders::default(),
        &std::env::var("ROOT_ID")
            .unwrap_or("tn_01j702qf86pc2j35s0kv0gv3gy".into())
            .parse()
            .expect("ROOT_ID is not a valid TenantId"),
    );
    let db_manager = Arc::new(
        database_manager::factory_client(
            dsn.use_database("tachyon_apps_database_manager").clone(),
        )
        .await?,
    );

    let sdk = util::create_sdk_from_auth_app(auth_app).await;

    let mock_sync_data: Arc<dyn outbound_sync::SyncDataInputPort> =
        Arc::new(MockSyncData);
    let app = library_api::LibraryApp::new(
        &dsn.clone().use_database("library"),
        db_manager.clone(),
        sdk,
        mock_sync_data,
    )
    .await;

    let csv_importer = CSVImporterClient::new(db_manager);

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        // TODO: add English comment
        .join("kihonjoho_20241007.csv");
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    let buffer = std::io::Cursor::new(buffer);

    let database_id = DatabaseId::default();
    app.create_repo
        .execute(library_api::usecase::CreateRepoInputData {
            executor: &tachyon_sdk::auth::Executor::SystemUser,
            multi_tenancy: &tachyon_sdk::auth::MultiTenancy::default(),
            database_id: Some(database_id.to_string()),
            org_username: "library-sandbox".to_string(),
            repo_username: "kihonjoho-corporate".to_string(),
            repo_name: "kihonjoho corporate".to_string(),
            user_id: "takanori.fukuyama@quantum-box.com".to_string(),
            is_public: true,
            description: None,
            skip_sample_data: false,
        })
        .await?;

    let database = csv_importer
        .import(
            &tachyon_sdk::auth::Executor::SystemUser,
            &tachyon_sdk::auth::MultiTenancy::default(),
            buffer,
            DatabaseConfig {
                name: "big_csv_import",
                tenant_id: &"tn_01j702qf86pc2j35s0kv0gv3gy".parse()?, // library-sandbox
                database_id: Some(database_id),
            },
        )
        .await?;
    println!("{:#?}", database);

    Ok(())
}
