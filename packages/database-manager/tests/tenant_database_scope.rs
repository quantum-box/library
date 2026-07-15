use chrono::Utc;
use database_manager::domain::{
    Data, DataId, DataRepository, PropertyType, RelationRepository,
    TypeRelation,
};
use database_manager::interface_adapter::gateway::{
    data_repository::DataRepositoryImpl, RelationRepositoryImpl,
};
use database_manager::{
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    DeleteDataInputData, GetDataInputData, PropertyDataInputData,
    SearchDataInputData, UpdateDataInputData,
};
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

fn assert_generic_not_found(error: errors::Error) {
    assert!(error.is_not_found());
    assert_eq!(error.to_string(), "NotFoundError: resource not found");
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn two_tenants_cannot_cross_database_or_record_boundaries(
) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager");
    let db = persistence::Db::new(&dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;

    let app = database_manager::factory_client(dsn.to_string()).await?;
    let data_repository = DataRepositoryImpl::new(db.clone());
    let relation_repository = RelationRepositoryImpl::new(db);
    let tenant_a = TenantId::default();
    let tenant_b = TenantId::default();
    let executor = &auth::Executor::SystemUser;
    let multi_tenancy_a =
        &auth::MultiTenancy::new_operator(tenant_a.clone());
    let multi_tenancy_b =
        &auth::MultiTenancy::new_operator(tenant_b.clone());

    let database_a = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            database_id: None,
            tenant_id: &tenant_a,
            name: "tenant-a",
        })
        .await?;
    let other_database_a = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            database_id: None,
            tenant_id: &tenant_a,
            name: "tenant-a-other",
        })
        .await?;
    let record_a = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            name: "tenant-a-record",
            property_data: vec![],
            database_id: database_a.id(),
        })
        .await?;
    let database_b = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy: multi_tenancy_b,
            database_id: None,
            tenant_id: &tenant_b,
            name: "tenant-b",
        })
        .await?;
    let record_b = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy: multi_tenancy_b,
            tenant_id: &tenant_b,
            name: "tenant-b-record",
            property_data: vec![],
            database_id: database_b.id(),
        })
        .await?;

    let foreign_relation_database_error = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            database_id: database_a.id(),
            name: "foreign-relation",
            property_type: PropertyType::Relation(TypeRelation::new(
                database_b.id().clone(),
            )),
        })
        .await
        .expect_err(
            "a Relation property must not target another tenant's database",
        );
    assert_generic_not_found(foreign_relation_database_error);

    let relation_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            database_id: database_a.id(),
            name: "relation",
            property_type: PropertyType::Relation(TypeRelation::new(
                other_database_a.id().clone(),
            )),
        })
        .await?;
    let stored_relations = relation_repository
        .find_all_by_database(database_a.id(), &tenant_a)
        .await?;
    assert_eq!(stored_relations.len(), 1);
    assert_eq!(
        stored_relations[0].target_database_id(),
        other_database_a.id()
    );
    assert!(relation_repository
        .find_all_by_database(database_a.id(), &tenant_b)
        .await?
        .is_empty());
    let relation_target_a = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            name: "tenant-a-relation-target",
            property_data: vec![],
            database_id: other_database_a.id(),
        })
        .await?;
    let related_record_a = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            name: "tenant-a-related-record",
            property_data: vec![PropertyDataInputData {
                property_id: relation_property.id().to_string(),
                value: relation_target_a.id().to_string(),
            }],
            database_id: database_a.id(),
        })
        .await?;

    let foreign_relation_data_error = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            name: "foreign-relation-target",
            property_data: vec![PropertyDataInputData {
                property_id: relation_property.id().to_string(),
                value: record_b.id().to_string(),
            }],
            database_id: database_a.id(),
        })
        .await
        .expect_err(
            "a Relation value must not target another tenant's record",
        );
    assert_generic_not_found(foreign_relation_data_error);

    let missing_relation_data_error = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            name: "missing-relation-target",
            property_data: vec![PropertyDataInputData {
                property_id: relation_property.id().to_string(),
                value: DataId::default().to_string(),
            }],
            database_id: database_a.id(),
        })
        .await
        .expect_err("a Relation value must point to an existing record");
    assert_generic_not_found(missing_relation_data_error);

    let update_relation_data_error = app
        .update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            database_id: database_a.id(),
            data_id: related_record_a.id(),
            name: "foreign-relation-update",
            data: vec![PropertyDataInputData {
                property_id: relation_property.id().to_string(),
                value: record_b.id().to_string(),
            }],
        })
        .await
        .expect_err(
            "UpdateData must reject a Relation to another tenant's record",
        );
    assert_generic_not_found(update_relation_data_error);

    let add_error = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy: multi_tenancy_b,
            tenant_id: &tenant_b,
            name: "cross-tenant-add",
            property_data: vec![],
            database_id: database_a.id(),
        })
        .await
        .expect_err("another tenant must not add to this database");
    assert_generic_not_found(add_error);

    let get_error = app
        .get_data_usecase()
        .execute(&GetDataInputData {
            executor,
            multi_tenancy: multi_tenancy_b,
            tenant_id: &tenant_b,
            database_id: database_a.id(),
            data_id: record_a.id(),
        })
        .await
        .expect_err("another tenant must not read this record");
    assert_generic_not_found(get_error);

    let update_error = app
        .update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy: multi_tenancy_b,
            tenant_id: &tenant_b,
            database_id: database_a.id(),
            data_id: record_a.id(),
            name: "cross-tenant-update",
            data: vec![],
        })
        .await
        .expect_err("another tenant must not update this record");
    assert_generic_not_found(update_error);

    let delete_error = app
        .delete_data_usecase()
        .execute(&DeleteDataInputData {
            executor,
            multi_tenancy: multi_tenancy_b,
            tenant_id: &tenant_b.to_string(),
            database_id: &database_a.id().to_string(),
            data_id: &record_a.id().to_string(),
        })
        .await
        .expect_err("another tenant must not delete this record");
    assert_generic_not_found(delete_error);

    let search_error = app
        .search_data()
        .execute(&SearchDataInputData {
            executor,
            multi_tenancy: multi_tenancy_b,
            tenant_id: &tenant_b,
            database_id: Some(database_a.id().clone()),
            query: "",
            page: Some(1),
            page_size: Some(20),
        })
        .await
        .expect_err("another tenant must not search this database");
    assert_generic_not_found(search_error);

    let wrong_database_error = app
        .get_data_usecase()
        .execute(&GetDataInputData {
            executor,
            multi_tenancy: multi_tenancy_a,
            tenant_id: &tenant_a,
            database_id: other_database_a.id(),
            data_id: record_a.id(),
        })
        .await
        .expect_err(
            "a record must not be readable through another database",
        );
    assert_generic_not_found(wrong_database_error);

    let unowned_data = Data::new(
        &DataId::default(),
        &tenant_b,
        database_a.id(),
        "unowned",
        vec![],
        Utc::now(),
        Utc::now(),
    )?;
    let repository_scope_error = data_repository
        .create(&unowned_data)
        .await
        .expect_err("the scoped insert must reject an unowned database");
    assert_generic_not_found(repository_scope_error);

    let collision_id = DataId::default();
    let first = Data::new(
        &collision_id,
        &tenant_a,
        database_a.id(),
        "first",
        vec![],
        Utc::now(),
        Utc::now(),
    )?;
    let collision = Data::new(
        &collision_id,
        &tenant_a,
        database_a.id(),
        "collision",
        vec![],
        Utc::now(),
        Utc::now(),
    )?;
    data_repository.create(&first).await?;
    let collision_error = data_repository
        .create(&collision)
        .await
        .expect_err("create must not overwrite an existing data id");
    assert!(matches!(collision_error, errors::Error::Conflict { .. }));
    assert!(collision_error
        .to_string()
        .contains("data id already exists"));

    Ok(())
}
