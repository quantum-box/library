use std::{sync::Arc, time::Duration};

use database_manager::domain::{
    DataId, PropertyRepository, PropertyType, RelationCardinality,
    RelationDefinitionRepository, RelationInverseChange, RelationOnDelete,
    TypeRelation, RELATION_GENERATION_V1,
};
use database_manager::interface_adapter::gateway::{
    DatabaseRepositoryImpl, PropertyRepositoryImpl,
    RelationDefinitionRepositoryImpl,
};
use database_manager::usecase::RelationDefinitionMutationInteractor;
use database_manager::{
    AddPropertyInputData, CreateDatabaseInputData, DeleteDatabaseInputData,
    DeletePropertyInputData, DeleteRelationDefinitionInputData,
    ReconfigureRelationDefinitionInputData,
    RelationDefinitionMutationInputPort, UpdatePropertyInputData,
};
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

async fn fixture() -> anyhow::Result<(
    Arc<persistence::Db>,
    database_manager::App,
    Arc<RelationDefinitionMutationInteractor>,
    Arc<RelationDefinitionRepositoryImpl>,
    TenantId,
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
    let database_repository = DatabaseRepositoryImpl::new(db.clone());
    let property_repository = PropertyRepositoryImpl::new(db.clone());
    let mutation = RelationDefinitionMutationInteractor::new(
        database_repository,
        property_repository,
    );
    let definitions = RelationDefinitionRepositoryImpl::new(db.clone());
    Ok((db, app, mutation, definitions, TenantId::default()))
}

async fn create_database(
    app: &database_manager::App,
    tenant_id: &TenantId,
    name: &str,
) -> errors::Result<database_manager::domain::Database> {
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

async fn add_relation(
    app: &database_manager::App,
    tenant_id: &TenantId,
    source_database_id: &database_manager::domain::DatabaseId,
    target_database_id: &database_manager::domain::DatabaseId,
    name: &str,
) -> errors::Result<database_manager::domain::Property> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    app.add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id,
            database_id: source_database_id,
            name,
            property_type: PropertyType::Relation(TypeRelation::new(
                target_database_id.clone(),
            )),
        })
        .await
}

async fn insert_record(
    db: &persistence::Db,
    tenant_id: &TenantId,
    database_id: &database_manager::domain::DatabaseId,
    name: &str,
) -> anyhow::Result<DataId> {
    let data_id = DataId::default();
    sqlx::query(
        r#"
        INSERT INTO data (id, tenant_id, object_id, name)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(name)
    .execute(db.pool().as_ref())
    .await?;
    Ok(data_id)
}

async fn insert_edge(
    db: &persistence::Db,
    definition: &database_manager::domain::RelationDefinition,
    source_data_id: &DataId,
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
    .bind(definition.tenant_id().to_string())
    .bind(definition.source_database_id().to_string())
    .bind(source_data_id.to_string())
    .bind(definition.id().to_string())
    .bind(definition.target_database_id().to_string())
    .bind(target_data_id.to_string())
    .execute(db.pool().as_ref())
    .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn versioned_mutation_owns_inverse_and_guards_public_property_writes(
) -> anyhow::Result<()> {
    let (db, app, mutation, definitions, tenant_id) = fixture().await?;
    let source =
        create_database(&app, &tenant_id, "relation-schema-source").await?;
    let target =
        create_database(&app, &tenant_id, "relation-schema-target").await?;
    let relation =
        add_relation(&app, &tenant_id, source.id(), target.id(), "parent")
            .await?;
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());

    let configured = mutation
        .reconfigure(ReconfigureRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: source.id(),
            source_property_id: relation.id(),
            expected_generation: RELATION_GENERATION_V1,
            forward_cardinality: Some(RelationCardinality::One),
            reverse_cardinality: Some(RelationCardinality::Many),
            inverse: RelationInverseChange::SetAlias(
                "children".to_string(),
            ),
            on_target_delete: Some(RelationOnDelete::Nullify),
        })
        .await?;
    assert_eq!(configured.generation().get(), 2);
    assert!(*configured.inverse_property_owned());
    assert_eq!(*configured.forward_cardinality(), RelationCardinality::One);
    assert_eq!(*configured.on_target_delete(), RelationOnDelete::Nullify);
    let inverse_id = configured
        .inverse_property_id()
        .clone()
        .expect("generated inverse id");

    let inverse = sqlx::query(
        r#"
        SELECT field_name, datatype, datatype_meta, type_key,
               type_version, type_config
        FROM fields
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(target.id().to_string())
    .bind(inverse_id.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(inverse.try_get::<String, _>("field_name")?, "children");
    assert_eq!(inverse.try_get::<String, _>("datatype")?, "RELATION");
    assert_eq!(inverse.try_get::<String, _>("type_key")?, "relation");
    assert_eq!(inverse.try_get::<u16, _>("type_version")?, 1);
    for column in ["datatype_meta", "type_config"] {
        let config = match column {
            "datatype_meta" => {
                inverse.try_get::<serde_json::Value, _>(column)?
            }
            _ => serde_json::from_str(
                &inverse.try_get::<String, _>(column)?,
            )?,
        };
        assert_eq!(
            config.get("database_id").and_then(|value| value.as_str()),
            Some(source.id().as_str())
        );
    }
    let mirrored_definition_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED)
        FROM relationships
        WHERE tenant_id = ? AND object_id = ? AND field_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(target.id().to_string())
    .bind(inverse_id.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(mirrored_definition_count, 0);

    let same_relation_type =
        PropertyType::Relation(TypeRelation::new(target.id().clone()));
    let source_type_error = app
        .update_property()
        .execute(UpdatePropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            property_id: relation.id(),
            name: None,
            property_type: Some(&same_relation_type),
            meta_json: None,
        })
        .await
        .expect_err(
            "Relation source type/config changes must use the schema UoW",
        );
    assert!(matches!(source_type_error, errors::Error::Conflict { .. }));

    let update_error = app
        .update_property()
        .execute(UpdatePropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: target.id(),
            property_id: &inverse_id,
            name: Some("direct rename"),
            property_type: None,
            meta_json: None,
        })
        .await
        .expect_err("generated inverse must be read-only directly");
    assert!(matches!(update_error, errors::Error::Conflict { .. }));

    let target_id = target.id().to_string();
    let inverse_id_text = inverse_id.to_string();
    let delete_error = app
        .delete_property_usecase()
        .execute(&DeletePropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: tenant_id.as_ref(),
            database_id: &target_id,
            property_id: &inverse_id_text,
        })
        .await
        .expect_err("generated inverse must not be deleted directly");
    assert!(matches!(delete_error, errors::Error::Conflict { .. }));

    let renamed = mutation
        .reconfigure(ReconfigureRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: source.id(),
            source_property_id: relation.id(),
            expected_generation: *configured.generation(),
            forward_cardinality: None,
            reverse_cardinality: None,
            inverse: RelationInverseChange::SetAlias(
                "descendants".to_string(),
            ),
            on_target_delete: None,
        })
        .await?;
    assert_eq!(renamed.inverse_property_id(), &Some(inverse_id.clone()));
    assert_eq!(renamed.generation().get(), 3);
    let name = sqlx::query_scalar::<_, String>(
        "SELECT field_name FROM fields WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(inverse_id.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(name, "descendants");

    let stale_error = mutation
        .reconfigure(ReconfigureRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: source.id(),
            source_property_id: relation.id(),
            expected_generation: *configured.generation(),
            forward_cardinality: None,
            reverse_cardinality: None,
            inverse: RelationInverseChange::Keep,
            on_target_delete: None,
        })
        .await
        .expect_err("stale generation must be rejected");
    assert!(matches!(stale_error, errors::Error::Conflict { .. }));

    let source_id = source.id().to_string();
    let database_delete_error = app
        .delete_database_usecase()
        .execute(&DeleteDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: tenant_id.as_ref(),
            database_id: &source_id,
        })
        .await
        .expect_err(
            "Database delete must not orphan an externally owned inverse",
        );
    assert!(matches!(
        database_delete_error,
        errors::Error::Conflict { .. }
    ));

    mutation
        .delete(DeleteRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: source.id(),
            source_property_id: relation.id(),
            expected_generation: *renamed.generation(),
        })
        .await?;
    assert!(definitions
        .find_by_source_property(&tenant_id, source.id(), relation.id())
        .await?
        .is_none());
    for property_id in [relation.id(), &inverse_id] {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM fields WHERE tenant_id = ? AND id = ?",
        )
        .bind(tenant_id.to_string())
        .bind(property_id.to_string())
        .fetch_one(db.pool().as_ref())
        .await?;
        assert_eq!(count, 0);
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn self_relation_inverse_and_late_failure_are_atomic(
) -> anyhow::Result<()> {
    const TRIGGER: &str = "test_relation_schema_mutation_rollback";
    let (db, app, mutation, definitions, tenant_id) = fixture().await?;
    let database =
        create_database(&app, &tenant_id, "relation-schema-self").await?;
    let relation = add_relation(
        &app,
        &tenant_id,
        database.id(),
        database.id(),
        "parent",
    )
    .await?;
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let pool = db.pool();
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    sqlx::raw_sql(&format!(
        r#"
        CREATE TRIGGER {TRIGGER}
        BEFORE UPDATE ON relationships
        FOR EACH ROW
        SIGNAL SQLSTATE '45000'
            SET MESSAGE_TEXT = 'forced RelationDefinition update failure'
        "#
    ))
    .execute(pool.as_ref())
    .await?;

    mutation
        .reconfigure(ReconfigureRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: database.id(),
            source_property_id: relation.id(),
            expected_generation: RELATION_GENERATION_V1,
            forward_cardinality: None,
            reverse_cardinality: None,
            inverse: RelationInverseChange::SetAlias(
                "children".to_string(),
            ),
            on_target_delete: None,
        })
        .await
        .expect_err("late definition update must fail");
    let fields_after_rollback = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM fields WHERE tenant_id = ? AND object_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        fields_after_rollback, 1,
        "the inverse insert must roll back with the definition update"
    );
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {TRIGGER}"))
        .execute(pool.as_ref())
        .await?;

    let configured = mutation
        .reconfigure(ReconfigureRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: database.id(),
            source_property_id: relation.id(),
            expected_generation: RELATION_GENERATION_V1,
            forward_cardinality: None,
            reverse_cardinality: None,
            inverse: RelationInverseChange::SetAlias(
                "children".to_string(),
            ),
            on_target_delete: None,
        })
        .await?;
    let inverse_id = configured
        .inverse_property_id()
        .as_ref()
        .expect("self inverse");
    assert_ne!(inverse_id, relation.id());
    assert_eq!(
        definitions
            .find_all_by_source_database(&tenant_id, database.id())
            .await?
            .len(),
        1,
        "the generated inverse must not create a mirrored definition"
    );
    let database_id = database.id().to_string();
    let relation_id = relation.id().to_string();
    app.delete_property_usecase()
        .execute(&DeletePropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: tenant_id.as_ref(),
            database_id: &database_id,
            property_id: &relation_id,
        })
        .await?;
    let remaining_fields = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM fields WHERE tenant_id = ? AND object_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        remaining_fields, 0,
        "legacy source Property delete must route through the Relation schema UoW"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn mismatched_operator_scope_is_concealed() -> anyhow::Result<()> {
    let (_db, app, mutation, definitions, tenant_id) = fixture().await?;
    let source =
        create_database(&app, &tenant_id, "relation-schema-auth-source")
            .await?;
    let target =
        create_database(&app, &tenant_id, "relation-schema-auth-target")
            .await?;
    let relation =
        add_relation(&app, &tenant_id, source.id(), target.id(), "parent")
            .await?;
    let wrong_tenant = TenantId::default();
    let wrong_scope = auth::MultiTenancy::new_operator(wrong_tenant);

    let error = mutation
        .reconfigure(ReconfigureRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &wrong_scope,
            tenant_id: &tenant_id,
            source_database_id: source.id(),
            source_property_id: relation.id(),
            expected_generation: RELATION_GENERATION_V1,
            forward_cardinality: None,
            reverse_cardinality: None,
            inverse: RelationInverseChange::SetAlias(
                "children".to_string(),
            ),
            on_target_delete: None,
        })
        .await
        .expect_err("a mismatched operator must not probe the definition");
    assert!(matches!(error, errors::Error::NotFound { .. }));
    let unchanged = definitions
        .find_by_source_property(&tenant_id, source.id(), relation.id())
        .await?
        .expect("definition remains");
    assert_eq!(unchanged.generation(), &RELATION_GENERATION_V1);
    assert!(unchanged.inverse_property_id().is_none());
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn legacy_canonical_target_mismatch_never_gets_overwritten(
) -> anyhow::Result<()> {
    let (db, app, mutation, definitions, tenant_id) = fixture().await?;
    let source =
        create_database(&app, &tenant_id, "relation-schema-parity-source")
            .await?;
    let legacy_target =
        create_database(&app, &tenant_id, "relation-schema-legacy-target")
            .await?;
    let canonical_target = create_database(
        &app,
        &tenant_id,
        "relation-schema-canonical-target",
    )
    .await?;
    let relation = add_relation(
        &app,
        &tenant_id,
        source.id(),
        canonical_target.id(),
        "parent",
    )
    .await?;
    sqlx::query(
        r#"
        UPDATE fields
        SET datatype_meta = ?
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(serde_json::json!({
        "database_id": legacy_target.id().to_string(),
    }))
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(relation.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());

    let rename_error = app
        .update_property()
        .execute(UpdatePropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            property_id: relation.id(),
            name: Some("must-not-overwrite"),
            property_type: None,
            meta_json: None,
        })
        .await
        .expect_err("a mismatched dual-write definition must fail closed");
    assert!(matches!(rename_error, errors::Error::Conflict { .. }));

    let mutation_error = mutation
        .reconfigure(ReconfigureRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: source.id(),
            source_property_id: relation.id(),
            expected_generation: RELATION_GENERATION_V1,
            forward_cardinality: Some(RelationCardinality::One),
            reverse_cardinality: None,
            inverse: RelationInverseChange::Keep,
            on_target_delete: None,
        })
        .await
        .expect_err(
            "schema mutation must not choose one mismatched target",
        );
    assert!(matches!(mutation_error, errors::Error::Conflict { .. }));
    let unchanged = definitions
        .find_by_source_property(&tenant_id, source.id(), relation.id())
        .await?
        .expect("definition remains");
    assert_eq!(unchanged.generation(), &RELATION_GENERATION_V1);
    assert_eq!(*unchanged.forward_cardinality(), RelationCardinality::Many);
    let name = sqlx::query_scalar::<_, String>(
        "SELECT field_name FROM fields WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(relation.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(name, "parent");
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn legacy_and_canonical_values_guard_narrowing_before_edge_backfill(
) -> anyhow::Result<()> {
    let (db, app, mutation, definitions, tenant_id) = fixture().await?;
    let source =
        create_database(&app, &tenant_id, "relation-value-guard-source")
            .await?;
    let target =
        create_database(&app, &tenant_id, "relation-value-guard-target")
            .await?;
    let property =
        add_relation(&app, &tenant_id, source.id(), target.id(), "targets")
            .await?;
    let definition = definitions
        .find_by_source_property(&tenant_id, source.id(), property.id())
        .await?
        .expect("RelationDefinition");
    let source_data =
        insert_record(db.as_ref(), &tenant_id, source.id(), "source")
            .await?;
    let target_a =
        insert_record(db.as_ref(), &tenant_id, target.id(), "target-a")
            .await?;
    let target_b =
        insert_record(db.as_ref(), &tenant_id, target.id(), "target-b")
            .await?;
    let legacy_column = format!("value{}", property.property_num());
    let set_legacy = |value: String| {
        let db = db.clone();
        let tenant_id = tenant_id.clone();
        let source_database_id = source.id().clone();
        let source_data_id = source_data.clone();
        let legacy_column = legacy_column.clone();
        async move {
            sqlx::query(&format!(
                "UPDATE data SET {legacy_column} = ? WHERE tenant_id = ? AND object_id = ? AND id = ?",
            ))
            .bind(value)
            .bind(tenant_id.to_string())
            .bind(source_database_id.to_string())
            .bind(source_data_id.to_string())
            .execute(db.pool().as_ref())
            .await?;
            Ok::<_, anyhow::Error>(())
        }
    };
    set_legacy(format!("{},{},{}", target.id(), target_a, target_b))
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges WHERE relation_id = ?",
        )
        .bind(definition.id().to_string())
        .fetch_one(db.pool().as_ref())
        .await?,
        0,
        "this contract exercises the pre-backfill state"
    );
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let narrow = || ReconfigureRelationDefinitionInputData {
        executor: &auth::Executor::SystemUser,
        multi_tenancy: &multi_tenancy,
        tenant_id: &tenant_id,
        source_database_id: source.id(),
        source_property_id: property.id(),
        expected_generation: RELATION_GENERATION_V1,
        forward_cardinality: Some(RelationCardinality::One),
        reverse_cardinality: None,
        inverse: RelationInverseChange::Keep,
        on_target_delete: None,
    };

    let legacy_error = mutation
        .reconfigure(narrow())
        .await
        .expect_err("legacy values must be checked before edge backfill");
    assert!(matches!(legacy_error, errors::Error::Conflict { .. }));

    set_legacy(format!("{},{}", target.id(), target_a)).await?;
    sqlx::query(
        r#"
        INSERT INTO property_values (
            tenant_id, database_id, data_id, property_id, type_key,
            type_version, value_encoding_version, value
        )
        VALUES (?, ?, ?, ?, 'relation', 1, 1, ?)
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(source_data.to_string())
    .bind(property.id().to_string())
    .bind(
        serde_json::json!({
            "database_id": target.id().to_string(),
            "data_ids": [target_b.to_string()],
        })
        .to_string(),
    )
    .execute(db.pool().as_ref())
    .await?;
    let parity_error = mutation
        .reconfigure(narrow())
        .await
        .expect_err("canonical mismatch must fail closed");
    assert!(matches!(parity_error, errors::Error::Conflict { .. }));

    sqlx::query(
        r#"
        DELETE FROM property_values
        WHERE tenant_id = ? AND database_id = ?
          AND data_id = ? AND property_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(source_data.to_string())
    .bind(property.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let narrowed = mutation.reconfigure(narrow()).await?;
    assert_eq!(narrowed.generation().get(), 2);
    assert_eq!(*narrowed.forward_cardinality(), RelationCardinality::One);
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn persisted_edges_guard_cardinality_narrowing_and_are_deleted_with_schema(
) -> anyhow::Result<()> {
    let (db, app, mutation, definitions, tenant_id) = fixture().await?;
    let source =
        create_database(&app, &tenant_id, "relation-edge-guard-source")
            .await?;
    let target =
        create_database(&app, &tenant_id, "relation-edge-guard-target")
            .await?;
    let property =
        add_relation(&app, &tenant_id, source.id(), target.id(), "targets")
            .await?;
    let definition = definitions
        .find_by_source_property(&tenant_id, source.id(), property.id())
        .await?
        .expect("RelationDefinition");
    let source_a =
        insert_record(db.as_ref(), &tenant_id, source.id(), "source-a")
            .await?;
    let source_b =
        insert_record(db.as_ref(), &tenant_id, source.id(), "source-b")
            .await?;
    let target_a =
        insert_record(db.as_ref(), &tenant_id, target.id(), "target-a")
            .await?;
    let target_b =
        insert_record(db.as_ref(), &tenant_id, target.id(), "target-b")
            .await?;
    for (source_data_id, target_data_id) in [
        (&source_a, &target_a),
        (&source_a, &target_b),
        (&source_b, &target_a),
    ] {
        insert_edge(
            db.as_ref(),
            &definition,
            source_data_id,
            target_data_id,
        )
        .await?;
    }
    let legacy_column = format!("value{}", property.property_num());
    for (source_data_id, target_data_ids) in [
        (&source_a, vec![&target_a, &target_b]),
        (&source_b, vec![&target_a]),
    ] {
        let value = format!(
            "{},{}",
            target.id(),
            target_data_ids
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        sqlx::query(&format!(
            "UPDATE data SET {legacy_column} = ? WHERE tenant_id = ? AND object_id = ? AND id = ?",
        ))
        .bind(value)
        .bind(tenant_id.to_string())
        .bind(source.id().to_string())
        .bind(source_data_id.to_string())
        .execute(db.pool().as_ref())
        .await?;
    }
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());

    for (forward_cardinality, reverse_cardinality) in [
        (Some(RelationCardinality::One), None),
        (None, Some(RelationCardinality::One)),
    ] {
        let error = mutation
            .reconfigure(ReconfigureRelationDefinitionInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &multi_tenancy,
                tenant_id: &tenant_id,
                source_database_id: source.id(),
                source_property_id: property.id(),
                expected_generation: RELATION_GENERATION_V1,
                forward_cardinality,
                reverse_cardinality,
                inverse: RelationInverseChange::Keep,
                on_target_delete: None,
            })
            .await
            .expect_err(
                "persisted edges must reject Many to One narrowing",
            );
        assert!(matches!(error, errors::Error::Conflict { .. }));
    }

    let unchanged = definitions
        .find_by_source_property(&tenant_id, source.id(), property.id())
        .await?
        .expect("definition remains");
    assert_eq!(unchanged.generation(), &RELATION_GENERATION_V1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges WHERE relation_id = ?",
        )
        .bind(definition.id().to_string())
        .fetch_one(db.pool().as_ref())
        .await?,
        3
    );

    mutation
        .delete(DeleteRelationDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            source_database_id: source.id(),
            source_property_id: property.id(),
            expected_generation: RELATION_GENERATION_V1,
        })
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges WHERE relation_id = ?",
        )
        .bind(definition.id().to_string())
        .fetch_one(db.pool().as_ref())
        .await?,
        0
    );
    assert!(definitions
        .find_by_source_property(&tenant_id, source.id(), property.id())
        .await?
        .is_none());
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn cardinality_narrowing_waits_for_edge_writer_and_rechecks_complete_set(
) -> anyhow::Result<()> {
    let (db, app, mutation, definitions, tenant_id) = fixture().await?;
    let source =
        create_database(&app, &tenant_id, "relation-edge-race-source")
            .await?;
    let target =
        create_database(&app, &tenant_id, "relation-edge-race-target")
            .await?;
    let property =
        add_relation(&app, &tenant_id, source.id(), target.id(), "targets")
            .await?;
    let definition = definitions
        .find_by_source_property(&tenant_id, source.id(), property.id())
        .await?
        .expect("RelationDefinition");
    let source_data =
        insert_record(db.as_ref(), &tenant_id, source.id(), "source")
            .await?;
    let target_a =
        insert_record(db.as_ref(), &tenant_id, target.id(), "target-a")
            .await?;
    let target_b =
        insert_record(db.as_ref(), &tenant_id, target.id(), "target-b")
            .await?;

    let pool = db.pool();
    let mut writer = pool.begin().await?;
    let mut endpoint_ids =
        [source.id().to_string(), target.id().to_string()];
    endpoint_ids.sort();
    sqlx::query_scalar::<_, String>(
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

    let narrowing = {
        let mutation = mutation.clone();
        let tenant_id = tenant_id.clone();
        let source_database_id = source.id().clone();
        let source_property_id = property.id().clone();
        tokio::spawn(async move {
            let multi_tenancy =
                auth::MultiTenancy::new_operator(tenant_id.clone());
            mutation
                .reconfigure(ReconfigureRelationDefinitionInputData {
                    executor: &auth::Executor::SystemUser,
                    multi_tenancy: &multi_tenancy,
                    tenant_id: &tenant_id,
                    source_database_id: &source_database_id,
                    source_property_id: &source_property_id,
                    expected_generation: RELATION_GENERATION_V1,
                    forward_cardinality: Some(RelationCardinality::One),
                    reverse_cardinality: None,
                    inverse: RelationInverseChange::Keep,
                    on_target_delete: None,
                })
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !narrowing.is_finished(),
        "schema mutation must wait for the endpoint-ordered edge writer"
    );

    for target_data_id in [&target_a, &target_b] {
        sqlx::query(
            r#"
            INSERT INTO relation_edges (
                tenant_id, source_database_id, source_data_id,
                relation_id, target_database_id, target_data_id
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tenant_id.to_string())
        .bind(source.id().to_string())
        .bind(source_data.to_string())
        .bind(definition.id().to_string())
        .bind(target.id().to_string())
        .bind(target_data_id.to_string())
        .execute(&mut *writer)
        .await?;
    }
    writer.commit().await?;

    let error = tokio::time::timeout(Duration::from_secs(10), narrowing)
        .await
        .expect("writer and schema mutation must not deadlock")
        .expect("schema mutation task must not panic")
        .expect_err("the committed edge set must reject narrowing");
    assert!(matches!(error, errors::Error::Conflict { .. }));
    let unchanged = definitions
        .find_by_source_property(&tenant_id, source.id(), property.id())
        .await?
        .expect("definition remains");
    assert_eq!(unchanged.generation(), &RELATION_GENERATION_V1);
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn future_relation_definition_blocks_bulk_property_deletion(
) -> anyhow::Result<()> {
    let (db, app, _mutation, _definitions, tenant_id) = fixture().await?;
    let database = create_database(
        &app,
        &tenant_id,
        "relation-schema-future-bulk-delete",
    )
    .await?;
    add_relation(
        &app,
        &tenant_id,
        database.id(),
        database.id(),
        "future-self-relation",
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
    let properties = PropertyRepositoryImpl::new(db.clone());

    let error = properties
        .delete_all(&tenant_id, database.id())
        .await
        .expect_err("bulk delete must not erase a future definition");
    assert!(matches!(error, errors::Error::Conflict { .. }));
    assert!(error.to_string().contains("read-only RelationDefinition"));
    let field_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM fields WHERE tenant_id = ? AND object_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    let definition_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM relationships WHERE tenant_id = ? AND object_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(field_count, 1);
    assert_eq!(definition_count, 1);
    Ok(())
}
