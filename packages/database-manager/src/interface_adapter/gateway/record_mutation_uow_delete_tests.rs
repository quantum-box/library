use crate as database_manager;
use database_manager::domain::{
    Data, DataId, Database, DatabaseId, DecideRecordDeleteCommand,
    IndexDefinitionId, Property, PropertyType, PropertyValueCommand,
    ReconfigureRelationDefinitionCommand, RecordActor, RecordActorKind,
    RecordMutationDecision, RecordOperationId, RecordRejectionCode,
    RecordVersion, RelationGeneration, RelationInverseChange,
    RelationOnDelete, RelationSchemaMutationPort, TypeRelation,
    VersionedRecordDeletionUnitOfWork,
};
use database_manager::interface_adapter::gateway::{
    DataRepositoryImpl, PropertyRepositoryImpl,
};
use database_manager::property_definition_rollout::PropertyDefinitionStorageMode;
use database_manager::property_value_rollout::PropertyValueStorageMode;
use database_manager::relation_edge_rollout::RelationEdgeWriteMode;
use database_manager::{
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    PropertyDataInputData,
};
use sqlx::{types::Json, Row};
use tachyon_sdk::auth;
use value_object::TenantId;

use super::record_mutation_uow_relation_edge_tests::{
    accepted_version, add_record, database_url, decide_relation_patch,
    operation_id,
};

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

async fn add_relation_property(
    app: &database_manager::App,
    property_repository: &PropertyRepositoryImpl,
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    target_database_id: &DatabaseId,
    name: &str,
    on_target_delete: RelationOnDelete,
) -> errors::Result<Property> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let property = app
        .add_property()
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
        .await?;
    if on_target_delete != RelationOnDelete::Restrict {
        property_repository
            .reconfigure_relation_atomically(
                &ReconfigureRelationDefinitionCommand::new(
                    tenant_id,
                    source_database_id,
                    property.id(),
                    RelationGeneration::new(1)?,
                    None,
                    None,
                    RelationInverseChange::Keep,
                    Some(on_target_delete),
                ),
            )
            .await?;
    }
    Ok(property)
}

async fn add_relation_record(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    properties: &[&Property],
    name: &str,
) -> errors::Result<Data> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    app.add_data_usecase()
        .execute(AddDataInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id,
            database_id,
            name,
            property_data: properties
                .iter()
                .map(|property| PropertyDataInputData {
                    property_id: property.id().clone(),
                    value: PropertyValueCommand::Relation(Vec::new()),
                })
                .collect(),
        })
        .await
}

fn delete_command(
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    operation_id: &RecordOperationId,
    expected_version: RecordVersion,
) -> errors::Result<DecideRecordDeleteCommand> {
    DecideRecordDeleteCommand::new(
        tenant_id,
        database_id,
        data_id,
        operation_id,
        expected_version,
        RecordActor::new(RecordActorKind::System, "record-delete-test")?,
    )
}

#[allow(clippy::too_many_arguments)]
async fn decide_delete(
    repository: &DataRepositoryImpl,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    operation_id: &RecordOperationId,
    expected_version: RecordVersion,
) -> errors::Result<RecordMutationDecision> {
    repository
        .decide_delete_atomically(&delete_command(
            tenant_id,
            database_id,
            data_id,
            operation_id,
            expected_version,
        )?)
        .await
}

fn assert_rejected(
    decision: &RecordMutationDecision,
    expected: RecordRejectionCode,
) {
    assert!(matches!(
        decision,
        RecordMutationDecision::Rejected { code, .. } if code == &expected
    ));
}

async fn relation_id(
    pool: &sqlx::MySqlPool,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    property: &Property,
) -> anyhow::Result<String> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT id FROM relationships WHERE tenant_id = ? \
         AND object_id = ? AND field_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(property.id().to_string())
    .fetch_one(pool)
    .await?)
}

async fn wait_for_delete_object_lock(
    pool: &sqlx::MySqlPool,
) -> anyhow::Result<()> {
    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let waiting = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT CAST(COUNT(*) AS SIGNED)
            FROM performance_schema.data_lock_waits AS waits
            INNER JOIN performance_schema.data_locks AS requested
              ON requested.ENGINE = waits.ENGINE
             AND requested.ENGINE_LOCK_ID =
                 waits.REQUESTING_ENGINE_LOCK_ID
            WHERE requested.OBJECT_SCHEMA =
                  'tachyon_apps_database_manager'
              AND requested.OBJECT_NAME = 'objects'
            "#,
        )
        .fetch_one(pool)
        .await?;
        if waiting > 0 {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!(
                "Record delete did not wait on the shared endpoint object lock"
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_delete_uow_nullifies_normalized_relations_and_orders_events(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();

    let target_database =
        create_database(&app, &tenant_id, "delete-nullify-target").await?;
    let source_database =
        create_database(&app, &tenant_id, "delete-nullify-source").await?;
    let relation_a = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "related-a",
        RelationOnDelete::Nullify,
    )
    .await?;
    let relation_b = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "related-b",
        RelationOnDelete::Nullify,
    )
    .await?;
    let deleted_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "delete-me",
    )
    .await?;
    let remaining_target =
        add_record(&app, &tenant_id, target_database.id(), None, "keep-me")
            .await?;
    let source_a = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&relation_a, &relation_b],
        "source-a",
    )
    .await?;
    let source_b = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&relation_a],
        "source-b",
    )
    .await?;

    let source_a_first = operation_id(&tenant_id, "delete-nullify-a-1")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                source_a.id(),
                relation_a.id(),
                &source_a_first,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![
                    deleted_target.id().clone(),
                    remaining_target.id().clone(),
                ]),
            )
            .await?
        ),
        Some(2)
    );
    let source_a_second = operation_id(&tenant_id, "delete-nullify-a-2")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                source_a.id(),
                relation_b.id(),
                &source_a_second,
                RecordVersion::new(2)?,
                PropertyValueCommand::Relation(vec![deleted_target
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(3)
    );
    let source_b_first = operation_id(&tenant_id, "delete-nullify-b-1")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                source_b.id(),
                relation_a.id(),
                &source_b_first,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![deleted_target
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(2)
    );

    let delete_operation = operation_id(&tenant_id, "delete-nullify")?;
    let decision = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        deleted_target.id(),
        &delete_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    let event_ids = match &decision {
        RecordMutationDecision::Accepted {
            record_version,
            event_ids,
            ..
        } => {
            assert_eq!(record_version.get(), 2);
            assert_eq!(event_ids.len(), 3);
            event_ids.clone()
        }
        other => panic!("expected accepted delete, got {other:?}"),
    };

    let deleted_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM data \
         WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(deleted_target.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(deleted_count, 0);
    let source_versions = sqlx::query(
        "SELECT id, record_version FROM data WHERE tenant_id = ? \
         AND object_id = ? AND id IN (?, ?) ORDER BY id",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source_a.id().to_string())
    .bind(source_b.id().to_string())
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(source_versions.len(), 2);
    for row in &source_versions {
        let id = row.try_get::<String, _>("id")?;
        let expected = if id.as_str() == source_a.id().as_str() {
            4
        } else {
            3
        };
        assert_eq!(row.try_get::<u64, _>("record_version")?, expected);
    }

    let legacy_a = format!("value{}", relation_a.property_num());
    let legacy_b = format!("value{}", relation_b.property_num());
    let source_a_values = sqlx::query(&format!(
        "SELECT {legacy_a} AS relation_a, {legacy_b} AS relation_b \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source_a.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        source_a_values.try_get::<String, _>("relation_a")?,
        format!("{},{}", target_database.id(), remaining_target.id())
    );
    assert_eq!(
        source_a_values.try_get::<String, _>("relation_b")?,
        target_database.id().to_string(),
        "last-target Nullify must preserve a typed empty Relation"
    );
    let canonical_b = sqlx::query_scalar::<_, String>(
        "SELECT value FROM property_values WHERE tenant_id = ? \
         AND database_id = ? AND data_id = ? AND property_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source_a.id().to_string())
    .bind(relation_b.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&canonical_b)?
            .pointer("/data_ids"),
        Some(&serde_json::json!([]))
    );

    let relation_a_id = relation_id(
        pool.as_ref(),
        &tenant_id,
        source_database.id(),
        &relation_a,
    )
    .await?;
    let relation_b_id = relation_id(
        pool.as_ref(),
        &tenant_id,
        source_database.id(),
        &relation_b,
    )
    .await?;
    let edges = sqlx::query(
        "SELECT source_data_id, relation_id, target_data_id \
         FROM relation_edges WHERE tenant_id = ? \
         AND source_database_id = ? AND source_data_id IN (?, ?) \
         ORDER BY source_data_id, relation_id, target_data_id",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source_a.id().to_string())
    .bind(source_b.id().to_string())
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(edges.len(), 1);
    assert_eq!(
        edges[0].try_get::<String, _>("source_data_id")?,
        source_a.id().to_string()
    );
    assert_eq!(
        edges[0].try_get::<String, _>("relation_id")?,
        relation_a_id
    );
    assert_eq!(
        edges[0].try_get::<String, _>("target_data_id")?,
        remaining_target.id().to_string()
    );
    assert_ne!(relation_b_id, "");

    let outbox = sqlx::query(
        "SELECT CAST(event_id AS CHAR) event_id, event_sequence, database_id, aggregate_id, \
                aggregate_version, CAST(event_type AS CHAR) event_type, payload \
         FROM domain_outbox_events WHERE operation_id = ? \
         ORDER BY event_sequence",
    )
    .bind(delete_operation.to_string())
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(outbox.len(), 3);
    let mut expected_sources =
        [source_a.id().to_string(), source_b.id().to_string()];
    expected_sources.sort();
    for (index, source_id) in expected_sources.iter().enumerate() {
        assert_eq!(
            outbox[index].try_get::<u32, _>("event_sequence")?,
            (index + 1) as u32
        );
        assert_eq!(
            outbox[index].try_get::<String, _>("database_id")?,
            source_database.id().to_string()
        );
        assert_eq!(
            outbox[index].try_get::<String, _>("aggregate_id")?,
            *source_id
        );
        assert_eq!(
            outbox[index].try_get::<String, _>("event_type")?,
            "database.record.patched.v1"
        );
        let Json(payload) = outbox[index]
            .try_get::<Json<serde_json::Value>, _>("payload")?;
        let property_count = payload
            .get("properties")
            .and_then(serde_json::Value::as_array)
            .expect("properties")
            .len();
        assert_eq!(
            property_count,
            if source_id == &source_a.id().to_string() {
                2
            } else {
                1
            },
            "all Nullify changes on one source share one event"
        );
    }
    assert_eq!(outbox[2].try_get::<u32, _>("event_sequence")?, 3);
    assert_eq!(
        outbox[2].try_get::<String, _>("database_id")?,
        target_database.id().to_string()
    );
    assert_eq!(
        outbox[2].try_get::<String, _>("aggregate_id")?,
        deleted_target.id().to_string()
    );
    assert_eq!(outbox[2].try_get::<u64, _>("aggregate_version")?, 2);
    assert_eq!(
        outbox[2].try_get::<String, _>("event_type")?,
        "database.record.deleted.v1"
    );
    let persisted_event_ids = outbox
        .iter()
        .map(|row| row.try_get::<String, _>("event_id"))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        persisted_event_ids,
        event_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_delete_uow_prioritizes_cas_and_guards_lifecycle(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();

    let target_database =
        create_database(&app, &tenant_id, "delete-restrict-target").await?;
    let source_database =
        create_database(&app, &tenant_id, "delete-restrict-source").await?;
    let restrict_relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "restricted",
        RelationOnDelete::Restrict,
    )
    .await?;
    let target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "restricted-target",
    )
    .await?;
    let source = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&restrict_relation],
        "restrict-source",
    )
    .await?;
    let patch_operation = operation_id(&tenant_id, "delete-restrict-edge")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                source.id(),
                restrict_relation.id(),
                &patch_operation,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![target.id().clone()]),
            )
            .await?
        ),
        Some(2)
    );

    let stale_operation =
        operation_id(&tenant_id, "delete-restrict-stale")?;
    let stale = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        target.id(),
        &stale_operation,
        RecordVersion::new(2)?,
    )
    .await?;
    assert!(matches!(stale, RecordMutationDecision::Conflict { .. }));
    let restrict_operation = operation_id(&tenant_id, "delete-restrict")?;
    let restricted = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        target.id(),
        &restrict_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    assert_rejected(
        &restricted,
        RecordRejectionCode::RelationDeleteRestricted,
    );
    let restrict_state = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) data_count, \
           (SELECT record_version FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) source_version, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
             WHERE tenant_id = ? AND target_database_id = ? \
               AND target_data_id = ?) edge_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
             WHERE operation_id = ?) event_count",
    )
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(target.id().to_string())
    .bind(restrict_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(restrict_state.try_get::<i64, _>("data_count")?, 1);
    assert_eq!(restrict_state.try_get::<u64, _>("source_version")?, 2);
    assert_eq!(restrict_state.try_get::<i64, _>("edge_count")?, 1);
    assert_eq!(restrict_state.try_get::<i64, _>("event_count")?, 0);

    let self_database =
        create_database(&app, &tenant_id, "delete-self-restrict").await?;
    let self_relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        self_database.id(),
        self_database.id(),
        "self",
        RelationOnDelete::Restrict,
    )
    .await?;
    let self_record = add_relation_record(
        &app,
        &tenant_id,
        self_database.id(),
        &[&self_relation],
        "self-record",
    )
    .await?;
    let self_patch_operation =
        operation_id(&tenant_id, "delete-self-edge")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                self_database.id(),
                self_record.id(),
                self_relation.id(),
                &self_patch_operation,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![self_record
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(2)
    );
    let self_delete_operation = operation_id(&tenant_id, "delete-self")?;
    let self_deleted = decide_delete(
        repository.as_ref(),
        &tenant_id,
        self_database.id(),
        self_record.id(),
        &self_delete_operation,
        RecordVersion::new(2)?,
    )
    .await?;
    assert_eq!(accepted_version(&self_deleted), Some(3));
    let self_rows = sqlx::query_scalar::<_, i64>(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) + \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
             WHERE tenant_id = ? AND source_database_id = ? \
               AND source_data_id = ?)",
    )
    .bind(tenant_id.to_string())
    .bind(self_database.id().to_string())
    .bind(self_record.id().to_string())
    .bind(tenant_id.to_string())
    .bind(self_database.id().to_string())
    .bind(self_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(self_rows, 0, "a self-loop must not self-restrict");

    let indexed_database =
        create_database(&app, &tenant_id, "delete-indexed-target").await?;
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let indexed_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: indexed_database.id(),
            name: "indexed",
            property_type: PropertyType::String,
        })
        .await?;
    let indexed_record = add_record(
        &app,
        &tenant_id,
        indexed_database.id(),
        None,
        "indexed-record",
    )
    .await?;
    sqlx::query(
        "INSERT INTO index_definitions ( \
           id, tenant_id, database_id, property_id, relation_id, policy, \
           is_unique, definition_version, generation, projection_state \
         ) VALUES (?, ?, ?, ?, NULL, 'EXACT', FALSE, 1, 1, 'PENDING')",
    )
    .bind(IndexDefinitionId::default().to_string())
    .bind(tenant_id.to_string())
    .bind(indexed_database.id().to_string())
    .bind(indexed_property.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let stale_index_operation =
        operation_id(&tenant_id, "delete-index-stale")?;
    let stale_index = decide_delete(
        repository.as_ref(),
        &tenant_id,
        indexed_database.id(),
        indexed_record.id(),
        &stale_index_operation,
        RecordVersion::new(2)?,
    )
    .await?;
    assert!(matches!(
        stale_index,
        RecordMutationDecision::Conflict { .. }
    ));
    let index_operation = operation_id(&tenant_id, "delete-index")?;
    let index_guard = decide_delete(
        repository.as_ref(),
        &tenant_id,
        indexed_database.id(),
        indexed_record.id(),
        &index_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    assert_rejected(
        &index_guard,
        RecordRejectionCode::IndexProjectionRequired,
    );
    let indexed_record_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(indexed_database.id().to_string())
    .bind(indexed_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(indexed_record_count, 1);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_delete_uow_replays_rejects_cross_kind_reuse_and_stays_dormant(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();

    let database =
        create_database(&app, &tenant_id, "delete-replay").await?;
    let replay_record =
        add_record(&app, &tenant_id, database.id(), None, "replay").await?;
    let replay_operation = operation_id(&tenant_id, "delete-replay")?;
    let accepted = decide_delete(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        replay_record.id(),
        &replay_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    assert_eq!(accepted_version(&accepted), Some(2));
    let replay = decide_delete(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        replay_record.id(),
        &replay_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    assert_eq!(
        replay, accepted,
        "lost-ack replay survives physical delete"
    );

    let cross_kind_database =
        create_database(&app, &tenant_id, "delete-cross-kind").await?;
    let self_relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        cross_kind_database.id(),
        cross_kind_database.id(),
        "related",
        RelationOnDelete::Restrict,
    )
    .await?;
    let cross_kind_record = add_relation_record(
        &app,
        &tenant_id,
        cross_kind_database.id(),
        &[&self_relation],
        "cross-kind",
    )
    .await?;
    let shared_operation = operation_id(&tenant_id, "patch-delete-reuse")?;
    let patched = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        cross_kind_database.id(),
        cross_kind_record.id(),
        self_relation.id(),
        &shared_operation,
        RecordVersion::INITIAL,
        PropertyValueCommand::Relation(Vec::new()),
    )
    .await?;
    assert_eq!(accepted_version(&patched), Some(2));
    let reused = decide_delete(
        repository.as_ref(),
        &tenant_id,
        cross_kind_database.id(),
        cross_kind_record.id(),
        &shared_operation,
        RecordVersion::new(2)?,
    )
    .await?;
    assert_rejected(&reused, RecordRejectionCode::IdempotencyKeyReuse);
    let mutation_kind = sqlx::query_scalar::<_, String>(
        "SELECT CAST(mutation_kind AS CHAR) FROM record_mutation_operations \
         WHERE operation_id = ?",
    )
    .bind(shared_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(mutation_kind, "PATCH");

    let dormant_record =
        add_record(&app, &tenant_id, database.id(), None, "dormant-gate")
            .await?;
    let disabled_repository =
        DataRepositoryImpl::new_with_all_storage_modes(
            db.clone(),
            PropertyValueStorageMode::DualWriteLegacyRead,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
            RelationEdgeWriteMode::default(),
        );
    let disabled_operation = operation_id(&tenant_id, "delete-disabled")?;
    let disabled = decide_delete(
        disabled_repository.as_ref(),
        &tenant_id,
        database.id(),
        dormant_record.id(),
        &disabled_operation,
        RecordVersion::INITIAL,
    )
    .await;
    assert!(
        disabled.is_err(),
        "the normal construction path stays dormant"
    );
    let disabled_state = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) data_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) \
             FROM record_mutation_operations WHERE operation_id = ?) op_count",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(dormant_record.id().to_string())
    .bind(disabled_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(disabled_state.try_get::<i64, _>("data_count")?, 1);
    assert_eq!(disabled_state.try_get::<i64, _>("op_count")?, 0);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_delete_uow_rejects_version_exhaustion_and_rolls_back_outbox_failure(
) -> anyhow::Result<()> {
    const FAILURE_TRIGGER: &str = "test_record_delete_outbox_rollback";

    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(pool.as_ref())
        .await?;

    let target_database =
        create_database(&app, &tenant_id, "delete-exhaust-target").await?;
    let target_exhausted = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "target-exhausted",
    )
    .await?;
    sqlx::query(
        "UPDATE data SET record_version = ? WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(u64::MAX)
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(target_exhausted.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let target_exhaust_operation =
        operation_id(&tenant_id, "delete-target-exhausted")?;
    let target_exhaustion = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        target_exhausted.id(),
        &target_exhaust_operation,
        RecordVersion::new(u64::MAX)?,
    )
    .await?;
    assert_rejected(
        &target_exhaustion,
        RecordRejectionCode::VersionExhausted,
    );

    let source_database =
        create_database(&app, &tenant_id, "delete-exhaust-source").await?;
    let nullify_relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "nullify",
        RelationOnDelete::Nullify,
    )
    .await?;
    let source_exhaust_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "source-exhaust-target",
    )
    .await?;
    let source_exhausted = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&nullify_relation],
        "source-exhausted",
    )
    .await?;
    let source_exhaust_patch =
        operation_id(&tenant_id, "delete-source-exhaust-edge")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                source_exhausted.id(),
                nullify_relation.id(),
                &source_exhaust_patch,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![source_exhaust_target
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(2)
    );
    sqlx::query(
        "UPDATE data SET record_version = ? WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(u64::MAX)
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source_exhausted.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let source_exhaust_operation =
        operation_id(&tenant_id, "delete-source-exhausted")?;
    let source_exhaustion = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        source_exhaust_target.id(),
        &source_exhaust_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    assert_rejected(
        &source_exhaustion,
        RecordRejectionCode::VersionExhausted,
    );
    let exhausted_state = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) target_count, \
           (SELECT record_version FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) source_version, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
             WHERE tenant_id = ? AND target_database_id = ? \
               AND target_data_id = ?) edge_count",
    )
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(source_exhaust_target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source_exhausted.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(source_exhaust_target.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(exhausted_state.try_get::<i64, _>("target_count")?, 1);
    assert_eq!(
        exhausted_state.try_get::<u64, _>("source_version")?,
        u64::MAX
    );
    assert_eq!(exhausted_state.try_get::<i64, _>("edge_count")?, 1);

    let rollback_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "rollback-target",
    )
    .await?;
    let rollback_source = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&nullify_relation],
        "rollback-source",
    )
    .await?;
    let rollback_patch = operation_id(&tenant_id, "delete-rollback-edge")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                rollback_source.id(),
                nullify_relation.id(),
                &rollback_patch,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![rollback_target
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(2)
    );
    let legacy_index_id = sqlx::query_scalar::<_, u64>(
        "SELECT CAST(COALESCE(MAX(id), 0) + 1 AS UNSIGNED) FROM indexes",
    )
    .fetch_one(pool.as_ref())
    .await?;
    sqlx::query(
        "INSERT INTO indexes (id, tenant_id, object_id, field_num) \
         VALUES (?, ?, ?, 1)",
    )
    .bind(legacy_index_id)
    .bind(tenant_id.to_string())
    .bind(rollback_target.id().to_string())
    .execute(pool.as_ref())
    .await?;
    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {FAILURE_TRIGGER} BEFORE INSERT ON domain_outbox_events \
         FOR EACH ROW SIGNAL SQLSTATE '45000' \
         SET MESSAGE_TEXT = 'forced Record delete outbox failure'"
    ))
    .execute(pool.as_ref())
    .await?;
    let rollback_operation = operation_id(&tenant_id, "delete-rollback")?;
    let rollback = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        rollback_target.id(),
        &rollback_operation,
        RecordVersion::INITIAL,
    )
    .await;
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    assert!(rollback.is_err());

    let legacy_column = format!("value{}", nullify_relation.property_num());
    let rollback_state = sqlx::query(&format!(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) target_count, \
           record_version AS source_version, {legacy_column} AS legacy_value, \
           (SELECT value FROM property_values WHERE tenant_id = ? \
             AND database_id = ? AND data_id = ? AND property_id = ?) canonical_value, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
             WHERE tenant_id = ? AND target_database_id = ? \
               AND target_data_id = ?) edge_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM indexes WHERE id = ?) index_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations \
             WHERE operation_id = ?) operation_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
             WHERE operation_id = ?) event_count \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(rollback_target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(rollback_source.id().to_string())
    .bind(nullify_relation.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(rollback_target.id().to_string())
    .bind(legacy_index_id)
    .bind(rollback_operation.to_string())
    .bind(rollback_operation.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(rollback_source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(rollback_state.try_get::<i64, _>("target_count")?, 1);
    assert_eq!(rollback_state.try_get::<u64, _>("source_version")?, 2);
    assert_eq!(
        rollback_state.try_get::<String, _>("legacy_value")?,
        format!("{},{}", target_database.id(), rollback_target.id())
    );
    let canonical = serde_json::from_str::<serde_json::Value>(
        &rollback_state.try_get::<String, _>("canonical_value")?,
    )?;
    assert_eq!(
        canonical
            .pointer("/data_ids/0")
            .and_then(|value| value.as_str()),
        Some(rollback_target.id().as_str())
    );
    assert_eq!(rollback_state.try_get::<i64, _>("edge_count")?, 1);
    assert_eq!(rollback_state.try_get::<i64, _>("index_count")?, 1);
    assert_eq!(rollback_state.try_get::<i64, _>("operation_count")?, 0);
    assert_eq!(rollback_state.try_get::<i64, _>("event_count")?, 0);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_delete_uow_serializes_concurrent_replay_and_source_patch(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();

    let replay_database =
        create_database(&app, &tenant_id, "delete-concurrent-replay")
            .await?;
    let replay_record = add_record(
        &app,
        &tenant_id,
        replay_database.id(),
        None,
        "concurrent-replay",
    )
    .await?;
    let replay_operation =
        operation_id(&tenant_id, "delete-concurrent-replay")?;
    let (left, right) = tokio::join!(
        decide_delete(
            repository.as_ref(),
            &tenant_id,
            replay_database.id(),
            replay_record.id(),
            &replay_operation,
            RecordVersion::INITIAL,
        ),
        decide_delete(
            repository.as_ref(),
            &tenant_id,
            replay_database.id(),
            replay_record.id(),
            &replay_operation,
            RecordVersion::INITIAL,
        )
    );
    let left = left?;
    let right = right?;
    assert_eq!(left, right);
    assert_eq!(accepted_version(&left), Some(2));
    let replay_rows = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations \
             WHERE operation_id = ?) operation_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
             WHERE operation_id = ?) event_count",
    )
    .bind(replay_operation.to_string())
    .bind(replay_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(replay_rows.try_get::<i64, _>("operation_count")?, 1);
    assert_eq!(replay_rows.try_get::<i64, _>("event_count")?, 1);

    let target_database =
        create_database(&app, &tenant_id, "delete-race-target").await?;
    let source_database =
        create_database(&app, &tenant_id, "delete-race-source").await?;
    let relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "race-relation",
        RelationOnDelete::Nullify,
    )
    .await?;
    let deleted_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "race-delete",
    )
    .await?;
    let replacement_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "race-replacement",
    )
    .await?;
    let source = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&relation],
        "race-source",
    )
    .await?;
    let seed_operation = operation_id(&tenant_id, "delete-race-seed")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                source.id(),
                relation.id(),
                &seed_operation,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![deleted_target
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(2)
    );

    let patch_operation = operation_id(&tenant_id, "delete-race-patch")?;
    let delete_operation = operation_id(&tenant_id, "delete-race-delete")?;
    let (patch, delete) = tokio::join!(
        decide_relation_patch(
            repository.as_ref(),
            &tenant_id,
            source_database.id(),
            source.id(),
            relation.id(),
            &patch_operation,
            RecordVersion::new(2)?,
            PropertyValueCommand::Relation(vec![replacement_target
                .id()
                .clone()]),
        ),
        decide_delete(
            repository.as_ref(),
            &tenant_id,
            target_database.id(),
            deleted_target.id(),
            &delete_operation,
            RecordVersion::INITIAL,
        )
    );
    let patch = patch?;
    let delete = delete?;
    assert_eq!(accepted_version(&delete), Some(2));
    assert!(
        accepted_version(&patch) == Some(3)
            || matches!(patch, RecordMutationDecision::Conflict { .. }),
        "source PATCH either wins at v3 or observes DELETE's v3 Nullify"
    );

    let legacy_column = format!("value{}", relation.property_num());
    let final_state = sqlx::query(&format!(
        "SELECT {legacy_column} AS legacy_value, \
           (SELECT value FROM property_values WHERE tenant_id = ? \
             AND database_id = ? AND data_id = ? AND property_id = ?) canonical_value, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
             WHERE tenant_id = ? AND target_database_id = ? \
               AND target_data_id = ?) deleted_edge_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) deleted_data_count \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(relation.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(deleted_target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(deleted_target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert!(!final_state
        .try_get::<String, _>("legacy_value")?
        .split(',')
        .any(|value| value == deleted_target.id().as_str()));
    let canonical = serde_json::from_str::<serde_json::Value>(
        &final_state.try_get::<String, _>("canonical_value")?,
    )?;
    assert!(!canonical
        .get("data_ids")
        .and_then(serde_json::Value::as_array)
        .expect("typed Relation")
        .iter()
        .any(|value| value.as_str() == Some(deleted_target.id().as_str())));
    assert_eq!(final_state.try_get::<i64, _>("deleted_edge_count")?, 0);
    assert_eq!(final_state.try_get::<i64, _>("deleted_data_count")?, 0);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_delete_uow_treats_legacy_as_authoritative_before_edge_backfill(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();

    let target_database =
        create_database(&app, &tenant_id, "delete-prebackfill-target")
            .await?;
    let source_database =
        create_database(&app, &tenant_id, "delete-prebackfill-source")
            .await?;
    let restrict_relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "prebackfill-restrict",
        RelationOnDelete::Restrict,
    )
    .await?;
    let restrict_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "prebackfill-restrict-target",
    )
    .await?;
    let restrict_source = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&restrict_relation],
        "prebackfill-restrict-source",
    )
    .await?;
    let restrict_patch =
        operation_id(&tenant_id, "delete-prebackfill-restrict-edge")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                restrict_source.id(),
                restrict_relation.id(),
                &restrict_patch,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![restrict_target
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(2)
    );
    let restrict_relation_id = relation_id(
        pool.as_ref(),
        &tenant_id,
        source_database.id(),
        &restrict_relation,
    )
    .await?;
    sqlx::query(
        "DELETE FROM relation_edges WHERE tenant_id = ? \
         AND source_database_id = ? AND source_data_id = ? \
         AND relation_id = ? AND target_data_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(restrict_source.id().to_string())
    .bind(&restrict_relation_id)
    .bind(restrict_target.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let restrict_operation =
        operation_id(&tenant_id, "delete-prebackfill-restrict")?;
    let restricted = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        restrict_target.id(),
        &restrict_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    assert_rejected(
        &restricted,
        RecordRejectionCode::RelationDeleteRestricted,
    );

    let extra_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "prebackfill-extra-edge",
    )
    .await?;
    sqlx::query(
        "INSERT INTO relation_edges ( \
           tenant_id, source_database_id, source_data_id, relation_id, \
           target_database_id, target_data_id \
         ) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(restrict_source.id().to_string())
    .bind(&restrict_relation_id)
    .bind(target_database.id().to_string())
    .bind(extra_target.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let extra_operation =
        operation_id(&tenant_id, "delete-prebackfill-extra-edge")?;
    let extra = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        extra_target.id(),
        &extra_operation,
        RecordVersion::INITIAL,
    )
    .await;
    assert!(extra.is_err(), "an edge absent from legacy is corruption");
    let extra_journal = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations \
         WHERE operation_id = ?",
    )
    .bind(extra_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(extra_journal, 0, "corruption does not become a decision");

    let nullify_relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "prebackfill-nullify",
        RelationOnDelete::Nullify,
    )
    .await?;
    let nullify_target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "prebackfill-nullify-target",
    )
    .await?;
    let nullify_source = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&nullify_relation],
        "prebackfill-nullify-source",
    )
    .await?;
    let nullify_patch =
        operation_id(&tenant_id, "delete-prebackfill-nullify-edge")?;
    assert_eq!(
        accepted_version(
            &decide_relation_patch(
                repository.as_ref(),
                &tenant_id,
                source_database.id(),
                nullify_source.id(),
                nullify_relation.id(),
                &nullify_patch,
                RecordVersion::INITIAL,
                PropertyValueCommand::Relation(vec![nullify_target
                    .id()
                    .clone()]),
            )
            .await?
        ),
        Some(2)
    );
    let nullify_relation_id = relation_id(
        pool.as_ref(),
        &tenant_id,
        source_database.id(),
        &nullify_relation,
    )
    .await?;
    sqlx::query(
        "DELETE FROM relation_edges WHERE tenant_id = ? \
         AND source_database_id = ? AND source_data_id = ? \
         AND relation_id = ? AND target_data_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(nullify_source.id().to_string())
    .bind(&nullify_relation_id)
    .bind(nullify_target.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let nullify_operation =
        operation_id(&tenant_id, "delete-prebackfill-nullify")?;
    let nullified = decide_delete(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        nullify_target.id(),
        &nullify_operation,
        RecordVersion::INITIAL,
    )
    .await?;
    assert_eq!(accepted_version(&nullified), Some(2));

    let legacy_column = format!("value{}", nullify_relation.property_num());
    let nullified_state = sqlx::query(&format!(
        "SELECT record_version, {legacy_column} AS legacy_value, \
           (SELECT value FROM property_values WHERE tenant_id = ? \
             AND database_id = ? AND data_id = ? AND property_id = ?) canonical_value, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) target_count \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(nullify_source.id().to_string())
    .bind(nullify_relation.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(nullify_target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(nullify_source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(nullified_state.try_get::<u64, _>("record_version")?, 3);
    assert_eq!(
        nullified_state.try_get::<String, _>("legacy_value")?,
        target_database.id().to_string()
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &nullified_state.try_get::<String, _>("canonical_value")?
        )?
        .pointer("/data_ids"),
        Some(&serde_json::json!([]))
    );
    assert_eq!(nullified_state.try_get::<i64, _>("target_count")?, 0);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_delete_uow_refreshes_repeatable_read_after_endpoint_lock_wait(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();

    let target_database =
        create_database(&app, &tenant_id, "delete-rr-target").await?;
    let source_database =
        create_database(&app, &tenant_id, "delete-rr-source").await?;
    let relation = add_relation_property(
        &app,
        property_repository.as_ref(),
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "rr-nullify",
        RelationOnDelete::Nullify,
    )
    .await?;
    let target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "rr-target",
    )
    .await?;
    let source = add_relation_record(
        &app,
        &tenant_id,
        source_database.id(),
        &[&relation],
        "rr-source",
    )
    .await?;
    let relation_id = relation_id(
        pool.as_ref(),
        &tenant_id,
        source_database.id(),
        &relation,
    )
    .await?;

    // Model the endpoint-serialized writer's transaction directly so the
    // object mutex can be held while DELETE advances to its lock wait. The
    // writer commits all three Relation representations atomically.
    let mut writer = pool.begin().await?;
    let mut endpoints = [
        source_database.id().to_string(),
        target_database.id().to_string(),
    ];
    endpoints.sort();
    for endpoint in endpoints {
        sqlx::query_scalar::<_, String>(
            "SELECT id FROM objects WHERE tenant_id = ? AND id = ? \
             FOR UPDATE",
        )
        .bind(tenant_id.to_string())
        .bind(endpoint)
        .fetch_one(&mut *writer)
        .await?;
    }

    let delete_operation = operation_id(&tenant_id, "delete-rr-wait")?;
    let delete_repository = repository.clone();
    let delete_tenant_id = tenant_id.clone();
    let delete_database_id = target_database.id().clone();
    let delete_data_id = target.id().clone();
    let spawned_operation = delete_operation.clone();
    let delete_task = tokio::spawn(async move {
        decide_delete(
            delete_repository.as_ref(),
            &delete_tenant_id,
            &delete_database_id,
            &delete_data_id,
            &spawned_operation,
            RecordVersion::INITIAL,
        )
        .await
    });

    // This is a condition wait, not a timing guess: claim_operation has run
    // before DELETE can block on the endpoint object row.
    wait_for_delete_object_lock(pool.as_ref()).await?;

    let legacy_column = format!("value{}", relation.property_num());
    let legacy_value = format!("{},{}", target_database.id(), target.id());
    let updated = sqlx::query(&format!(
        "UPDATE data SET {legacy_column} = ?, record_version = 2 \
         WHERE tenant_id = ? AND object_id = ? AND id = ? \
           AND record_version = 1"
    ))
    .bind(&legacy_value)
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .execute(&mut *writer)
    .await?;
    assert_eq!(updated.rows_affected(), 1);
    let canonical_value = serde_json::json!({
        "database_id": target_database.id().to_string(),
        "data_ids": [target.id().to_string()]
    });
    let updated = sqlx::query(
        "UPDATE property_values SET value = ? \
         WHERE tenant_id = ? AND database_id = ? \
           AND data_id = ? AND property_id = ?",
    )
    .bind(canonical_value.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(relation.id().to_string())
    .execute(&mut *writer)
    .await?;
    assert_eq!(updated.rows_affected(), 1);
    sqlx::query(
        "INSERT INTO relation_edges ( \
           tenant_id, source_database_id, source_data_id, relation_id, \
           target_database_id, target_data_id \
         ) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(&relation_id)
    .bind(target_database.id().to_string())
    .bind(target.id().to_string())
    .execute(&mut *writer)
    .await?;
    writer.commit().await?;

    let joined = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        delete_task,
    )
    .await
    .map_err(|_| anyhow::anyhow!("Record delete stayed blocked"))?;
    let decision = joined??;
    let event_ids = match &decision {
        RecordMutationDecision::Accepted {
            record_version,
            event_ids,
            ..
        } => {
            assert_eq!(record_version.get(), 2);
            assert_eq!(event_ids.len(), 2);
            event_ids
        }
        other => {
            panic!("DELETE missed the committed Nullify edge: {other:?}")
        }
    };
    assert_eq!(event_ids.len(), 2);

    let final_state = sqlx::query(&format!(
        "SELECT record_version, {legacy_column} AS legacy_value, \
           (SELECT value FROM property_values WHERE tenant_id = ? \
             AND database_id = ? AND data_id = ? AND property_id = ?) canonical_value, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
             WHERE tenant_id = ? AND target_database_id = ? \
               AND target_data_id = ?) edge_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?) target_count \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(relation.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(target_database.id().to_string())
    .bind(target.id().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(final_state.try_get::<u64, _>("record_version")?, 3);
    assert_eq!(
        final_state.try_get::<String, _>("legacy_value")?,
        target_database.id().to_string(),
        "committed inbound reference must be Nullified to typed empty"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            &final_state.try_get::<String, _>("canonical_value")?
        )?
        .pointer("/data_ids"),
        Some(&serde_json::json!([]))
    );
    assert_eq!(final_state.try_get::<i64, _>("edge_count")?, 0);
    assert_eq!(final_state.try_get::<i64, _>("target_count")?, 0);

    Ok(())
}
