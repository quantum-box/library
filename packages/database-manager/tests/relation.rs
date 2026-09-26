use database_manager::{
    domain::PropertyType, AddPropertyInputData, CreateDatabaseInputData,
};
use persistence::tests::clean_up;
use tachyon_sdk::auth;
use value_object::TenantId;

#[tokio::test]
#[ignore]
async fn test_create_new_database() {
    dotenvy::dotenv().ok();
    let dsn = std::env::var("DEV_DATABASE_URL")
        .map(|url| format!("{}/database_test", url))
        .unwrap_or_else(|_| {
            "mysql://root:@localhost:15000/database_test".to_string()
        });
    let db = persistence::Db::new(&dsn).await;

    let tenant_id = TenantId::default();

    let executor = &auth::Executor::SystemUser;
    let multi_tenancy =
        &auth::MultiTenancy::new_operator(tenant_id.clone());

    // TODO: add English comment
    // TODO: add English comment
    // TODO: add English comment
    clean_up(db.pool()).await.expect("clean failed");
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await
        .expect("migration failed");

    let client = database_manager::factory_client(dsn)
        .await
        .expect("failed to create client");
    let database = client
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: None,
            name: "test",
        })
        .await
        .expect("failed to create database");
    client
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "property1",
            property_type: PropertyType::String,
        })
        .await
        .expect("failed to add property");
}
