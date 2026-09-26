use std::{sync::Arc, time::Duration};

use database_manager::domain::{
    AddPropertyCommand, DatabaseId, Property, PropertySchemaMutationPort,
    PropertyType, TypeId, TypeRelation,
};
use database_manager::interface_adapter::gateway::PropertyRepositoryImpl;
use database_manager::{AddPropertyInputData, CreateDatabaseInputData};
use sqlx::Row;
use tachyon_sdk::auth;
use tokio::sync::Barrier;
use value_object::{DatabaseUrl, TenantId};

async fn mysql_fixture(
    name: &str,
) -> anyhow::Result<(
    Arc<persistence::Db>,
    database_manager::App,
    TenantId,
    database_manager::domain::Database,
)> {
    dotenvy::dotenv().ok();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager");
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client(dsn.to_string()).await?;
    let tenant_id = TenantId::default();
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name,
        })
        .await?;

    Ok((db, app, tenant_id, database))
}

async fn add_after_barrier(
    app: database_manager::App,
    barrier: Arc<Barrier>,
    tenant_id: TenantId,
    database_id: DatabaseId,
    name: &'static str,
    property_type: PropertyType,
) -> errors::Result<Property> {
    barrier.wait().await;
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    app.add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: &database_id,
            name,
            property_type,
        })
        .await
}

async fn scoped_counts(
    db: &persistence::Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
) -> anyhow::Result<(i64, i64, i64)> {
    let pool = db.pool();
    let field_row = sqlx::query(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED) AS field_count,
               CAST(COUNT(DISTINCT field_num) AS SIGNED) AS slot_count
        FROM fields
        WHERE tenant_id = ? AND object_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    let relation_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED)
        FROM relationships
        WHERE tenant_id = ? AND object_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .fetch_one(pool.as_ref())
    .await?;

    Ok((
        field_row.try_get("field_count")?,
        field_row.try_get("slot_count")?,
        relation_count,
    ))
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn concurrent_string_properties_receive_distinct_legacy_slots(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, database) =
        mysql_fixture("concurrent-string-slots").await?;
    let barrier = Arc::new(Barrier::new(2));

    let (first, second) = tokio::join!(
        add_after_barrier(
            app.clone(),
            barrier.clone(),
            tenant_id.clone(),
            database.id().clone(),
            "first",
            PropertyType::String,
        ),
        add_after_barrier(
            app,
            barrier,
            tenant_id.clone(),
            database.id().clone(),
            "second",
            PropertyType::String,
        ),
    );
    let first = first?;
    let second = second?;

    assert_ne!(first.property_num(), second.property_num());
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        (2, 2, 0)
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn concurrent_id_properties_have_one_winner_and_no_residue(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, database) =
        mysql_fixture("concurrent-id-singleton").await?;
    let barrier = Arc::new(Barrier::new(2));

    let (first, second) = tokio::join!(
        add_after_barrier(
            app.clone(),
            barrier.clone(),
            tenant_id.clone(),
            database.id().clone(),
            "first-id",
            PropertyType::Id(TypeId::new(true)),
        ),
        add_after_barrier(
            app,
            barrier,
            tenant_id.clone(),
            database.id().clone(),
            "second-id",
            PropertyType::Id(TypeId::new(false)),
        ),
    );

    let success_count = [&first, &second]
        .into_iter()
        .filter(|result| result.is_ok())
        .count();
    let conflict_count = [&first, &second]
        .into_iter()
        .filter(|result| {
            matches!(result, Err(errors::Error::Conflict { .. }))
        })
        .count();
    assert_eq!(success_count, 1);
    assert_eq!(conflict_count, 1);
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        (1, 1, 0),
        "the losing mutation must leave neither a field nor relation metadata"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn opposite_relation_additions_lock_endpoints_without_deadlock(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, database_a) =
        mysql_fixture("opposite-relation-a").await?;
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let database_b = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "opposite-relation-b",
        })
        .await?;
    let barrier = Arc::new(Barrier::new(2));

    let (a_to_b, b_to_a) =
        tokio::time::timeout(Duration::from_secs(10), async {
            tokio::join!(
                add_after_barrier(
                    app.clone(),
                    barrier.clone(),
                    tenant_id.clone(),
                    database_a.id().clone(),
                    "a-to-b",
                    PropertyType::Relation(TypeRelation::new(
                        database_b.id().clone(),
                    )),
                ),
                add_after_barrier(
                    app,
                    barrier,
                    tenant_id.clone(),
                    database_b.id().clone(),
                    "b-to-a",
                    PropertyType::Relation(TypeRelation::new(
                        database_a.id().clone(),
                    )),
                ),
            )
        })
        .await
        .expect("opposite Relation additions must not deadlock");
    a_to_b?;
    b_to_a?;

    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database_a.id()).await?,
        (1, 1, 1)
    );
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database_b.id()).await?,
        (1, 1, 1)
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn relation_metadata_failure_rolls_back_its_field(
) -> anyhow::Result<()> {
    let (db, _app, tenant_id, database) =
        mysql_fixture("relation-uow-rollback").await?;
    let repository = PropertyRepositoryImpl::new(db.clone());
    let missing_target = DatabaseId::default();
    let command = AddPropertyCommand::new(
        &tenant_id,
        database.id(),
        "invalid-relation",
        &PropertyType::Relation(TypeRelation::new(missing_target)),
    );

    repository
        .add_property_atomically(&command)
        .await
        .expect_err("the relation FK must reject a missing target");

    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        (0, 0, 0),
        "field and relation metadata must roll back as one unit"
    );
    Ok(())
}
