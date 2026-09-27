use database_manager::{
    domain::{PropertyType, PropertyValueCommand},
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    DeleteDataInputData, PropertyDataInputData, UpdateDataInputData,
};
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

/// Every kind of record write must move the revision a search index keys
/// on, and a read with no write in between must not.
#[tokio::test]
#[ignore]
async fn revision_moves_with_every_record_write() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let tenant_id = TenantId::default();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager");
    let app = database_manager::factory_client(&dsn).await?;

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
            name: "snapshot revision",
        })
        .await?;
    let property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            display_name: None,
            name: "location",
            property_type: PropertyType::Location(Default::default()),
        })
        .await?;
    let snapshot = app.data_snapshot();

    let empty = snapshot.revision(&tenant_id, database.id()).await?;
    assert_eq!(empty.record_count, 0);
    assert_eq!(empty.token(), "0-0-0-none");

    let add = |name: &'static str, location: &'static str| {
        app.add_data_usecase().execute(AddDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name,
            property_data: vec![PropertyDataInputData {
                property_id: property.id().clone(),
                value: PropertyValueCommand::Location(
                    location.parse().unwrap(),
                ),
            }],
        })
    };
    let first = add("札幌駅", "43.0687,141.3508").await?;
    add("小樽", "43.1907,140.9947").await?;

    let created = snapshot.revision(&tenant_id, database.id()).await?;
    assert_eq!(created.record_count, 2);
    assert_ne!(created, empty);
    assert_eq!(
        snapshot.revision(&tenant_id, database.id()).await?,
        created,
        "reading must not move the revision"
    );

    let all = snapshot.load_all(&tenant_id, database.id()).await?;
    assert_eq!(all.len(), 2);

    app.update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: first.id(),
            name: "札幌駅",
            data: vec![PropertyDataInputData {
                property_id: property.id().clone(),
                value: PropertyValueCommand::Location(
                    "43.0690,141.3510".parse().unwrap(),
                ),
            }],
        })
        .await?;
    let updated = snapshot.revision(&tenant_id, database.id()).await?;
    assert_eq!(updated.record_count, 2);
    assert_ne!(updated, created, "an update must move the revision");

    app.delete_data_usecase()
        .execute(&DeleteDataInputData {
            executor,
            multi_tenancy,
            tenant_id: tenant_id.as_ref(),
            database_id: database.id().as_ref(),
            data_id: first.id().as_ref(),
        })
        .await?;
    let deleted = snapshot.revision(&tenant_id, database.id()).await?;
    assert_eq!(deleted.record_count, 1);
    assert_ne!(deleted, updated);

    // Another tenant cannot read this Database's revision.
    assert!(snapshot
        .revision(&TenantId::default(), database.id())
        .await
        .is_err());
    Ok(())
}
