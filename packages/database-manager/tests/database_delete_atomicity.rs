use std::{sync::Arc, time::Duration};

use database_manager::domain::{
    DataId, Database, DatabaseId, Property, PropertyId, PropertyType,
    RelationId, TypeRelation,
};
use database_manager::{
    AddPropertyInputData, CreateDatabaseInputData, DeleteDatabaseInputData,
};
use sqlx::{MySql, Row, Transaction};
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

#[derive(Debug, PartialEq, Eq)]
struct ScopedCounts {
    objects: i64,
    fields: i64,
    data: i64,
    indexes: i64,
    inbound_relations: i64,
}

async fn mysql_fixture(
    name: &str,
) -> anyhow::Result<(
    Arc<persistence::Db>,
    database_manager::App,
    TenantId,
    Database,
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
    let database = create_database(&app, &tenant_id, name).await?;

    Ok((db, app, tenant_id, database))
}

async fn create_database(
    app: &database_manager::App,
    tenant_id: &TenantId,
    name: &str,
) -> errors::Result<Database> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    app.create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id,
            name,
        })
        .await
}

async fn add_property(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    name: &str,
    property_type: PropertyType,
) -> errors::Result<Property> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    app.add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id,
            database_id,
            name,
            property_type,
        })
        .await
}

async fn insert_data(
    db: &persistence::Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
) -> anyhow::Result<DataId> {
    let data_id = DataId::default();
    sqlx::query(
        r#"
        INSERT INTO data (id, tenant_id, object_id, name)
        VALUES (?, ?, ?, 'record')
        "#,
    )
    .bind(data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .execute(db.pool().as_ref())
    .await?;
    Ok(data_id)
}

async fn insert_legacy_index(
    db: &persistence::Db,
    tenant_id: &TenantId,
    data_id: &DataId,
) -> anyhow::Result<()> {
    let max_id =
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(id) FROM indexes")
            .fetch_one(db.pool().as_ref())
            .await?;
    let next_id = max_id
        .unwrap_or(0)
        .checked_add(1)
        .expect("the test fixture needs an unused indexes.id");
    sqlx::query(
        r#"
        INSERT INTO indexes (id, tenant_id, object_id, field_num)
        VALUES (?, ?, ?, 0)
        "#,
    )
    .bind(next_id)
    .bind(tenant_id.to_string())
    .bind(data_id.to_string())
    .execute(db.pool().as_ref())
    .await?;
    Ok(())
}

async fn relation_id_for_property(
    db: &persistence::Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    property_id: &PropertyId,
) -> anyhow::Result<String> {
    Ok(sqlx::query_scalar::<_, String>(
        r#"
        SELECT id FROM relationships
        WHERE tenant_id = ? AND object_id = ? AND field_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(property_id.to_string())
    .fetch_one(db.pool().as_ref())
    .await?)
}

#[allow(clippy::too_many_arguments)]
async fn insert_relation_edge(
    db: &persistence::Db,
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    source_data_id: &DataId,
    relation_id: &str,
    target_database_id: &DatabaseId,
    target_data_id: &DataId,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO relation_edges (
            tenant_id, source_database_id, source_data_id, relation_id,
            target_database_id, target_data_id
        )
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(source_data_id.to_string())
    .bind(relation_id)
    .bind(target_database_id.to_string())
    .bind(target_data_id.to_string())
    .execute(db.pool().as_ref())
    .await?;
    Ok(())
}

async fn delete_database(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
) -> errors::Result<Database> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let database_id = database_id.to_string();
    app.delete_database_usecase()
        .execute(&DeleteDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: tenant_id.as_ref(),
            database_id: &database_id,
        })
        .await
}

async fn scoped_counts(
    db: &persistence::Db,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
) -> anyhow::Result<ScopedCounts> {
    let pool = db.pool();
    let row = sqlx::query(
        r#"
        SELECT
            (SELECT CAST(COUNT(*) AS SIGNED) FROM objects
             WHERE tenant_id = ? AND id = ?) AS object_count,
            (SELECT CAST(COUNT(*) AS SIGNED) FROM fields
             WHERE tenant_id = ? AND object_id = ?) AS field_count,
            (SELECT CAST(COUNT(*) AS SIGNED) FROM data
             WHERE tenant_id = ? AND object_id = ?) AS data_count,
            (SELECT CAST(COUNT(*) AS SIGNED) FROM indexes AS index_projection
             INNER JOIN data AS indexed_data
               ON indexed_data.tenant_id = index_projection.tenant_id
              AND indexed_data.id = index_projection.object_id
             WHERE indexed_data.tenant_id = ?
               AND indexed_data.object_id = ?) AS index_count,
            (SELECT CAST(COUNT(*) AS SIGNED) FROM relationships
             WHERE tenant_id = ? AND target_object_id = ?
               AND object_id <> ?) AS inbound_relation_count
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(database_id.to_string())
    .fetch_one(pool.as_ref())
    .await?;

    Ok(ScopedCounts {
        objects: row.try_get("object_count")?,
        fields: row.try_get("field_count")?,
        data: row.try_get("data_count")?,
        indexes: row.try_get("index_count")?,
        inbound_relations: row.try_get("inbound_relation_count")?,
    })
}

async fn insert_relation_definition(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    target_database_id: &DatabaseId,
) -> anyhow::Result<()> {
    let property_id = PropertyId::default();
    sqlx::query(
        r#"
        INSERT INTO fields (
            id, tenant_id, object_id, field_name, datatype,
            datatype_meta, is_indexed, field_num
        )
        VALUES (?, ?, ?, 'relation', 'RELATION', ?, FALSE, 0)
        "#,
    )
    .bind(property_id.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(serde_json::json!({
        "database_id": target_database_id.to_string(),
    }))
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id
        )
        VALUES (?, ?, ?, ?, 0, ?)
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(property_id.to_string())
    .bind(target_database_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn external_inbound_relation_rejects_before_any_mutation(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, target) =
        mysql_fixture("delete-target-restricted").await?;
    add_property(
        &app,
        &tenant_id,
        target.id(),
        "target-value",
        PropertyType::String,
    )
    .await?;
    let data_id = insert_data(db.as_ref(), &tenant_id, target.id()).await?;
    insert_legacy_index(db.as_ref(), &tenant_id, &data_id).await?;
    let source = create_database(&app, &tenant_id, "delete-source").await?;
    add_property(
        &app,
        &tenant_id,
        source.id(),
        "target",
        PropertyType::Relation(TypeRelation::new(target.id().clone())),
    )
    .await?;

    let before =
        scoped_counts(db.as_ref(), &tenant_id, target.id()).await?;
    assert_eq!(before.inbound_relations, 1);

    let error = delete_database(&app, &tenant_id, target.id())
        .await
        .expect_err(
            "an external RelationDefinition must restrict deletion",
        );
    assert!(matches!(&error, errors::Error::Conflict { .. }));
    assert!(error.to_string().contains("external RelationDefinition"));
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, target.id()).await?,
        before,
        "preflight failure must not delete data, fields, or the Database"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn self_relation_is_deleted_with_its_owned_schema(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, database) =
        mysql_fixture("delete-self-relation").await?;
    let relation = add_property(
        &app,
        &tenant_id,
        database.id(),
        "parent",
        PropertyType::Relation(TypeRelation::new(database.id().clone())),
    )
    .await?;
    let data_id =
        insert_data(db.as_ref(), &tenant_id, database.id()).await?;
    let relation_id = relation_id_for_property(
        db.as_ref(),
        &tenant_id,
        database.id(),
        relation.id(),
    )
    .await?;
    insert_relation_edge(
        db.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &relation_id,
        database.id(),
        &data_id,
    )
    .await?;

    delete_database(&app, &tenant_id, database.id()).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges WHERE relation_id = ?",
        )
        .bind(&relation_id)
        .fetch_one(db.pool().as_ref())
        .await?,
        0
    );
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        ScopedCounts {
            objects: 0,
            fields: 0,
            data: 0,
            indexes: 0,
            inbound_relations: 0,
        }
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn source_database_delete_cleans_cross_database_edges_before_records(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, source) =
        mysql_fixture("delete-edge-source").await?;
    let target =
        create_database(&app, &tenant_id, "delete-edge-target").await?;
    let relation = add_property(
        &app,
        &tenant_id,
        source.id(),
        "targets",
        PropertyType::Relation(TypeRelation::new(target.id().clone())),
    )
    .await?;
    let source_data =
        insert_data(db.as_ref(), &tenant_id, source.id()).await?;
    let target_data =
        insert_data(db.as_ref(), &tenant_id, target.id()).await?;
    let relation_id = relation_id_for_property(
        db.as_ref(),
        &tenant_id,
        source.id(),
        relation.id(),
    )
    .await?;
    insert_relation_edge(
        db.as_ref(),
        &tenant_id,
        source.id(),
        &source_data,
        &relation_id,
        target.id(),
        &target_data,
    )
    .await?;

    delete_database(&app, &tenant_id, source.id()).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges WHERE relation_id = ?",
        )
        .bind(&relation_id)
        .fetch_one(db.pool().as_ref())
        .await?,
        0
    );
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, source.id()).await?,
        ScopedCounts {
            objects: 0,
            fields: 0,
            data: 0,
            indexes: 0,
            inbound_relations: 0,
        }
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?",
        )
        .bind(tenant_id.to_string())
        .bind(target.id().to_string())
        .bind(target_data.to_string())
        .fetch_one(db.pool().as_ref())
        .await?,
        1,
        "Database-owned edge cleanup must not delete the target Record"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn future_relation_definition_blocks_database_deletion(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, database) =
        mysql_fixture("delete-future-relation-definition").await?;
    add_property(
        &app,
        &tenant_id,
        database.id(),
        "future-self-relation",
        PropertyType::Relation(TypeRelation::new(database.id().clone())),
    )
    .await?;
    sqlx::query(
        r#"
        UPDATE relationships
        SET definition_version = 2
        WHERE tenant_id = ? AND object_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let before =
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?;

    let error = delete_database(&app, &tenant_id, database.id())
        .await
        .expect_err("a future RelationDefinition must remain read-only");
    assert!(matches!(error, errors::Error::Conflict { .. }));
    assert!(error.to_string().contains("read-only RelationDefinition"));
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        before,
        "future RelationDefinition rejection must precede every delete"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn late_root_delete_failure_rolls_back_descendant_deletes(
) -> anyhow::Result<()> {
    const TRIGGER: &str = "test_database_delete_atomicity_rollback";
    let (db, app, tenant_id, database) =
        mysql_fixture("delete-late-failure").await?;
    add_property(
        &app,
        &tenant_id,
        database.id(),
        "value",
        PropertyType::String,
    )
    .await?;
    let relation = add_property(
        &app,
        &tenant_id,
        database.id(),
        "self",
        PropertyType::Relation(TypeRelation::new(database.id().clone())),
    )
    .await?;
    let data_id =
        insert_data(db.as_ref(), &tenant_id, database.id()).await?;
    insert_legacy_index(db.as_ref(), &tenant_id, &data_id).await?;
    let relation_id = relation_id_for_property(
        db.as_ref(),
        &tenant_id,
        database.id(),
        relation.id(),
    )
    .await?;
    insert_relation_edge(
        db.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &relation_id,
        database.id(),
        &data_id,
    )
    .await?;
    let before =
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?;
    let pool = db.pool();
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    sqlx::raw_sql(&format!(
        r#"
        CREATE TRIGGER {TRIGGER}
        BEFORE DELETE ON objects
        FOR EACH ROW
        BEGIN
            IF OLD.object_name = 'delete-late-failure' THEN
                SIGNAL SQLSTATE '45000'
                    SET MESSAGE_TEXT = 'forced late Database delete failure';
            END IF;
        END
        "#
    ))
    .execute(pool.as_ref())
    .await?;

    let error = delete_database(&app, &tenant_id, database.id())
        .await
        .expect_err("the trigger must fail the final root delete");
    assert!(matches!(error, errors::Error::InternalServerError { .. }));
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        before,
        "indexes, data, and fields deleted earlier in the UoW must be restored"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges WHERE relation_id = ?",
        )
        .bind(&relation_id)
        .fetch_one(pool.as_ref())
        .await?,
        1,
        "edge cleanup must roll back with the later Database delete failure"
    );

    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    delete_database(&app, &tenant_id, database.id()).await?;
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        ScopedCounts {
            objects: 0,
            fields: 0,
            data: 0,
            indexes: 0,
            inbound_relations: 0,
        }
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges WHERE relation_id = ?",
        )
        .bind(&relation_id)
        .fetch_one(pool.as_ref())
        .await?,
        0
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn deletion_waits_for_endpoint_ordered_relation_writer(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, target) =
        mysql_fixture("delete-concurrent-target").await?;
    add_property(
        &app,
        &tenant_id,
        target.id(),
        "target-value",
        PropertyType::String,
    )
    .await?;
    insert_data(db.as_ref(), &tenant_id, target.id()).await?;
    let source =
        create_database(&app, &tenant_id, "delete-concurrent-source")
            .await?;

    // This is the RelationDefinition writer protocol used by the production
    // adapter: lock both object rows in primary-key order before persisting a
    // field/definition pair.
    let pool = db.pool();
    let mut writer = pool.begin().await?;
    let mut endpoint_ids =
        [source.id().to_string(), target.id().to_string()];
    endpoint_ids.sort();
    let locked = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id FROM objects
        WHERE tenant_id = ? AND id IN (?, ?)
        ORDER BY id
        FOR UPDATE
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(&endpoint_ids[0])
    .bind(&endpoint_ids[1])
    .fetch_all(&mut *writer)
    .await?;
    assert_eq!(locked, endpoint_ids);

    let delete_app = app.clone();
    let delete_tenant = tenant_id.clone();
    let delete_target = target.id().clone();
    let deleting = tokio::spawn(async move {
        delete_database(&delete_app, &delete_tenant, &delete_target).await
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !deleting.is_finished(),
        "delete must wait for the endpoint serialization lock"
    );

    insert_relation_definition(
        &mut writer,
        &tenant_id,
        source.id(),
        target.id(),
    )
    .await?;
    writer.commit().await?;

    let error = tokio::time::timeout(Duration::from_secs(10), deleting)
        .await
        .expect("relation writer and Database delete must not deadlock")
        .expect("delete task must not panic")
        .expect_err("the committed inbound definition must be re-read");
    assert!(matches!(error, errors::Error::Conflict { .. }));
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, target.id()).await?,
        ScopedCounts {
            objects: 1,
            fields: 1,
            data: 1,
            indexes: 0,
            inbound_relations: 1,
        }
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn wrong_tenant_is_not_found_without_revealing_or_mutating_database(
) -> anyhow::Result<()> {
    let (db, app, tenant_id, database) =
        mysql_fixture("delete-tenant-concealment").await?;
    add_property(
        &app,
        &tenant_id,
        database.id(),
        "value",
        PropertyType::String,
    )
    .await?;
    insert_data(db.as_ref(), &tenant_id, database.id()).await?;
    let before =
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?;
    let other_tenant = TenantId::default();

    let error = delete_database(&app, &other_tenant, database.id())
        .await
        .expect_err("a Database outside the tenant scope is concealed");
    assert!(error.is_not_found());
    assert_eq!(error.to_string(), "NotFoundError: resource not found");
    assert_eq!(
        scoped_counts(db.as_ref(), &tenant_id, database.id()).await?,
        before
    );
    Ok(())
}
