use database_manager::{
    domain::PropertyType, AddDataInputData, AddPropertyInputData,
    CreateDatabaseInputData, PropertyDataInputData, SearchDataInputData,
};
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

#[tokio::test]
#[ignore]
async fn test_paging() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let tenant_id = TenantId::default();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()
        .unwrap()
        .use_database("tachyon_apps_database_manager");
    let app = database_manager::factory_client(dsn).await?;

    let executor = &auth::Executor::SystemUser;
    let multi_tenancy =
        &auth::MultiTenancy::new(None, Some(tenant_id.clone()));

    let database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: None,
            name: "test",
        })
        .await?;
    let property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "test",
            property_type: PropertyType::String,
        })
        .await?;
    for i in 0..50 {
        app.add_data_usecase()
            .execute(AddDataInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                database_id: database.id(),
                name: &format!("test{}", i),
                property_data: vec![PropertyDataInputData {
                    property_id: property.id().to_string(),
                    value: format!("test{}", i),
                }],
            })
            .await?;
    }
    let (data, paginator) = app
        .search_data()
        .execute(&SearchDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: Some(database.id().clone()),
            query: "",
            page: Some(1),
            page_size: Some(10),
        })
        .await?;
    assert_eq!(data.len(), 10);
    assert_eq!(paginator.total_items, 50);
    assert_eq!(paginator.total_pages, 5);
    assert_eq!(paginator.current_page, 1);

    let (data, paginator) = app
        .search_data()
        .execute(&SearchDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: Some(database.id().clone()),
            query: "",
            page: Some(2),
            page_size: Some(20),
        })
        .await?;
    assert_eq!(data.len(), 20);
    assert_eq!(paginator.total_items, 50);
    assert_eq!(paginator.total_pages, 3);
    assert_eq!(paginator.current_page, 2);

    let (data, paginator) = app
        .search_data()
        .execute(&SearchDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: Some(database.id().clone()),
            query: "",
            page: Some(3),
            page_size: Some(20),
        })
        .await?;
    assert_eq!(data.len(), 10);
    assert_eq!(paginator.total_items, 50);
    assert_eq!(paginator.total_pages, 3);
    assert_eq!(paginator.current_page, 3);
    Ok(())
}
