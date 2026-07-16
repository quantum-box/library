use crate as database_manager;
use database_manager::domain::{
    Data, DataId, DatabaseId, DecideRecordPatchCommand, IndexDefinitionId,
    Property, PropertyType, PropertyValueCommand,
    ReconfigureRelationDefinitionCommand, RecordActor, RecordActorKind,
    RecordMutationDecision, RecordOperationId, RecordPatch,
    RecordPropertyPatch, RecordRejectionCode, RecordVersion,
    RelationGeneration, RelationInverseChange, RelationSchemaMutationPort,
    TypeRelation, VersionedRecordMutationUnitOfWork,
};
use database_manager::interface_adapter::gateway::{
    DataRepositoryImpl, PropertyRepositoryImpl,
};
use database_manager::property_definition_rollout::PropertyDefinitionStorageMode;
use database_manager::property_value_rollout::PropertyValueStorageMode;
use database_manager::relation_edge_rollout::RelationEdgeWriteMode;
use database_manager::{
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    PatchRecordInputData, PropertyDataInputData,
};
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

fn database_url() -> anyhow::Result<DatabaseUrl> {
    dotenvy::dotenv().ok();
    Ok(std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager"))
}

fn operation_id(
    tenant_id: &TenantId,
    label: &str,
) -> errors::Result<RecordOperationId> {
    RecordOperationId::new(format!("{tenant_id}:{label}"))
}

async fn add_record(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    relation: Option<&Property>,
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
            property_data: relation
                .map(|property| {
                    vec![PropertyDataInputData {
                        property_id: property.id().clone(),
                        value: PropertyValueCommand::Relation(Vec::new()),
                    }]
                })
                .unwrap_or_default(),
        })
        .await
}

#[allow(clippy::too_many_arguments)]
async fn decide_relation_patch(
    repository: &DataRepositoryImpl,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    property_id: &database_manager::domain::PropertyId,
    operation_id: &RecordOperationId,
    expected_version: RecordVersion,
    value: PropertyValueCommand,
) -> errors::Result<RecordMutationDecision> {
    let command = DecideRecordPatchCommand::new(
        tenant_id,
        database_id,
        data_id,
        operation_id,
        expected_version,
        RecordActor::new(RecordActorKind::System, "relation-edge-test")?,
        RecordPatch::new(
            None,
            vec![RecordPropertyPatch::new(property_id, value)],
        ),
    )?;
    repository.decide_patch_atomically(&command).await
}

fn accepted_version(decision: &RecordMutationDecision) -> Option<u64> {
    match decision {
        RecordMutationDecision::Accepted { record_version, .. } => {
            Some(record_version.get())
        }
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_relation_writer_dual_writes_and_serializes_cardinality(
) -> anyhow::Result<()> {
    const FAILURE_TRIGGER: &str = "test_relation_edge_outbox_rollback";

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
    let pool = db.pool();
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(pool.as_ref())
        .await?;

    let tenant_id = TenantId::default();
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let target_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "relation-edge-target",
        })
        .await?;
    let source_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "relation-edge-source",
        })
        .await?;
    let relation_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            name: "related",
            property_type: PropertyType::Relation(TypeRelation::new(
                target_database.id().clone(),
            )),
        })
        .await?;

    let target_a = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "target-a",
    )
    .await?;
    let target_b = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "target-b",
    )
    .await?;
    let target_c = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "target-c",
    )
    .await?;
    let source = add_record(
        &app,
        &tenant_id,
        source_database.id(),
        Some(&relation_property),
        "source",
    )
    .await?;

    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let relation_id = sqlx::query_scalar::<_, String>(
        "SELECT id FROM relationships WHERE tenant_id = ? \
         AND object_id = ? AND field_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(relation_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    let legacy_column =
        format!("value{}", relation_property.property_num());

    let set_operation = operation_id(&tenant_id, "edge-set")?;
    let set = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &set_operation,
        RecordVersion::INITIAL,
        PropertyValueCommand::Relation(vec![
            target_b.id().clone(),
            target_a.id().clone(),
        ]),
    )
    .await?;
    assert_eq!(accepted_version(&set), Some(2));

    let mut expected_targets =
        vec![target_a.id().to_string(), target_b.id().to_string()];
    expected_targets.sort();
    let row = sqlx::query(&format!(
        "SELECT record_version, {legacy_column} AS legacy_value \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(row.try_get::<u64, _>("record_version")?, 2);
    assert_eq!(
        row.try_get::<String, _>("legacy_value")?,
        std::iter::once(target_database.id().to_string())
            .chain(expected_targets.iter().cloned())
            .collect::<Vec<_>>()
            .join(",")
    );
    let canonical = sqlx::query_scalar::<_, String>(
        "SELECT value FROM property_values WHERE tenant_id = ? \
         AND database_id = ? AND data_id = ? AND property_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(relation_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    let canonical: serde_json::Value = serde_json::from_str(&canonical)?;
    assert_eq!(
        canonical
            .get("data_ids")
            .and_then(serde_json::Value::as_array)
            .expect("canonical relation ids")
            .iter()
            .map(|value| value.as_str().expect("string id").to_string())
            .collect::<Vec<_>>(),
        expected_targets
    );
    let edge_targets = sqlx::query_scalar::<_, String>(
        "SELECT target_data_id FROM relation_edges WHERE tenant_id = ? \
         AND source_database_id = ? AND source_data_id = ? \
         AND relation_id = ? ORDER BY target_data_id",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(&relation_id)
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(edge_targets, expected_targets);

    let replace_operation = operation_id(&tenant_id, "edge-replace")?;
    let replace = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &replace_operation,
        RecordVersion::new(2)?,
        PropertyValueCommand::Relation(vec![
            target_c.id().clone(),
            target_b.id().clone(),
        ]),
    )
    .await?;
    assert_eq!(accepted_version(&replace), Some(3));
    let edge_targets = sqlx::query_scalar::<_, String>(
        "SELECT target_data_id FROM relation_edges WHERE tenant_id = ? \
         AND source_database_id = ? AND source_data_id = ? \
         AND relation_id = ? ORDER BY target_data_id",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .bind(&relation_id)
    .fetch_all(pool.as_ref())
    .await?;
    let mut replaced =
        vec![target_b.id().to_string(), target_c.id().to_string()];
    replaced.sort();
    assert_eq!(edge_targets, replaced);

    let clear_operation = operation_id(&tenant_id, "edge-clear")?;
    let clear = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &clear_operation,
        RecordVersion::new(3)?,
        PropertyValueCommand::Clear,
    )
    .await?;
    assert_eq!(accepted_version(&clear), Some(4));
    let replay = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &clear_operation,
        RecordVersion::new(3)?,
        PropertyValueCommand::Clear,
    )
    .await?;
    assert_eq!(replay, clear);
    let cleared = sqlx::query(&format!(
        "SELECT record_version, {legacy_column} AS legacy_value, \
         (SELECT CAST(COUNT(*) AS SIGNED) FROM property_values pv \
          WHERE pv.tenant_id = data.tenant_id \
            AND pv.database_id = data.object_id \
            AND pv.data_id = data.id AND pv.property_id = ?) canonical_count, \
         (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges re \
          WHERE re.tenant_id = data.tenant_id \
            AND re.source_database_id = data.object_id \
            AND re.source_data_id = data.id AND re.relation_id = ?) edge_count \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(relation_property.id().to_string())
    .bind(&relation_id)
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(cleared.try_get::<u64, _>("record_version")?, 4);
    assert!(cleared
        .try_get::<Option<String>, _>("legacy_value")?
        .is_none());
    assert_eq!(cleared.try_get::<i64, _>("canonical_count")?, 0);
    assert_eq!(cleared.try_get::<i64, _>("edge_count")?, 0);

    // Empty Relation is a present typed set, while Clear removes the value.
    // Both have zero edges but must retain distinct storage and event deltas.
    let empty_operation = operation_id(&tenant_id, "edge-empty-set")?;
    let empty = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &empty_operation,
        RecordVersion::new(4)?,
        PropertyValueCommand::Relation(Vec::new()),
    )
    .await?;
    assert_eq!(accepted_version(&empty), Some(5));
    let empty_state = sqlx::query(&format!(
        "SELECT {legacy_column} AS legacy_value, \
         (SELECT CAST(COUNT(*) AS SIGNED) FROM property_values pv \
          WHERE pv.tenant_id = data.tenant_id \
            AND pv.database_id = data.object_id \
            AND pv.data_id = data.id AND pv.property_id = ?) canonical_count \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(relation_property.id().to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        empty_state.try_get::<String, _>("legacy_value")?,
        target_database.id().to_string()
    );
    assert_eq!(empty_state.try_get::<i64, _>("canonical_count")?, 1);
    let empty_event = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT payload FROM domain_outbox_events WHERE operation_id = ?",
    )
    .bind(empty_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        empty_event.pointer("/properties/0/action"),
        Some(&serde_json::Value::String("SET".to_string()))
    );
    assert_eq!(
        empty_event.pointer("/properties/0/value/raw_value/data_ids"),
        Some(&serde_json::Value::Array(Vec::new()))
    );

    let explicit_clear_operation =
        operation_id(&tenant_id, "edge-explicit-clear")?;
    let explicit_clear = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &explicit_clear_operation,
        RecordVersion::new(5)?,
        PropertyValueCommand::Clear,
    )
    .await?;
    assert_eq!(accepted_version(&explicit_clear), Some(6));
    let clear_event = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT payload FROM domain_outbox_events WHERE operation_id = ?",
    )
    .bind(explicit_clear_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        clear_event.pointer("/properties/0/action"),
        Some(&serde_json::Value::String("CLEAR".to_string()))
    );

    let one_target_operation = operation_id(&tenant_id, "edge-one")?;
    let one_target = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &one_target_operation,
        RecordVersion::new(6)?,
        PropertyValueCommand::Relation(vec![target_a.id().clone()]),
    )
    .await?;
    assert_eq!(accepted_version(&one_target), Some(7));
    let stale = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-stale")?,
        RecordVersion::new(6)?,
        PropertyValueCommand::Relation(vec![target_b.id().clone()]),
    )
    .await?;
    assert!(matches!(stale, RecordMutationDecision::Conflict { .. }));
    let clear = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-clear-before-one")?,
        RecordVersion::new(7)?,
        PropertyValueCommand::Clear,
    )
    .await?;
    assert_eq!(accepted_version(&clear), Some(8));

    // Normal factories have no RelationEdge activation path. Their stale
    // caller still receives the canonical CAS conflict before the capability
    // rejection returned to a fresh caller.
    let disabled_stale = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            data_id: source.id(),
            operation_id: &operation_id(&tenant_id, "edge-disabled-stale")?,
            expected_version: RecordVersion::new(7)?,
            name: None,
            properties: vec![PropertyDataInputData {
                property_id: relation_property.id().clone(),
                value: PropertyValueCommand::Relation(Vec::new()),
            }],
        })
        .await?;
    assert!(matches!(
        disabled_stale,
        RecordMutationDecision::Conflict { .. }
    ));
    let disabled_fresh = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            data_id: source.id(),
            operation_id: &operation_id(&tenant_id, "edge-disabled-fresh")?,
            expected_version: RecordVersion::new(8)?,
            name: None,
            properties: vec![PropertyDataInputData {
                property_id: relation_property.id().clone(),
                value: PropertyValueCommand::Relation(Vec::new()),
            }],
        })
        .await?;
    assert!(matches!(
        disabled_fresh,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::RelationProjectionRequired,
            ..
        }
    ));

    let missing = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-missing")?,
        RecordVersion::new(8)?,
        PropertyValueCommand::Relation(vec![DataId::default()]),
    )
    .await?;
    assert!(matches!(
        missing,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::ResourceNotFound,
            ..
        }
    ));
    let wrong_database = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-wrong-database")?,
        RecordVersion::new(8)?,
        PropertyValueCommand::Relation(vec![source.id().clone()]),
    )
    .await?;
    assert!(matches!(
        wrong_database,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::ResourceNotFound,
            ..
        }
    ));
    let duplicate = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-duplicate")?,
        RecordVersion::new(8)?,
        PropertyValueCommand::Relation(vec![
            target_a.id().clone(),
            target_a.id().clone(),
        ]),
    )
    .await?;
    assert!(matches!(
        duplicate,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::InvalidPropertyValue,
            ..
        }
    ));

    sqlx::query(
        "UPDATE relationships SET forward_cardinality = 'ONE', \
         reverse_cardinality = 'ONE' WHERE id = ?",
    )
    .bind(&relation_id)
    .execute(pool.as_ref())
    .await?;
    let forward_exceeded = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-forward-one")?,
        RecordVersion::new(8)?,
        PropertyValueCommand::Relation(vec![
            target_a.id().clone(),
            target_b.id().clone(),
        ]),
    )
    .await?;
    assert!(matches!(
        forward_exceeded,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::RelationCardinalityExceeded,
            ..
        }
    ));

    let source_two = add_record(
        &app,
        &tenant_id,
        source_database.id(),
        Some(&relation_property),
        "source-two",
    )
    .await?;
    let source_three = add_record(
        &app,
        &tenant_id,
        source_database.id(),
        Some(&relation_property),
        "source-three",
    )
    .await?;
    let op_two = operation_id(&tenant_id, "edge-reverse-one-a")?;
    let op_three = operation_id(&tenant_id, "edge-reverse-one-b")?;
    let (two, three) = tokio::join!(
        decide_relation_patch(
            repository.as_ref(),
            &tenant_id,
            source_database.id(),
            source_two.id(),
            relation_property.id(),
            &op_two,
            RecordVersion::INITIAL,
            PropertyValueCommand::Relation(vec![target_a.id().clone()]),
        ),
        decide_relation_patch(
            repository.as_ref(),
            &tenant_id,
            source_database.id(),
            source_three.id(),
            relation_property.id(),
            &op_three,
            RecordVersion::INITIAL,
            PropertyValueCommand::Relation(vec![target_a.id().clone()]),
        )
    );
    let decisions = [two?, three?];
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| accepted_version(decision) == Some(2))
            .count(),
        1
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(
                decision,
                RecordMutationDecision::Rejected {
                    code: RecordRejectionCode::RelationCardinalityExceeded,
                    ..
                }
            ))
            .count(),
        1
    );
    let target_a_edges = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
         WHERE tenant_id = ? AND relation_id = ? AND target_data_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(&relation_id)
    .bind(target_a.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(target_a_edges, 1);

    let property_repository =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let relation_with_inverse = property_repository
        .reconfigure_relation_atomically(
            &ReconfigureRelationDefinitionCommand::new(
                &tenant_id,
                source_database.id(),
                relation_property.id(),
                RelationGeneration::new(1)?,
                None,
                None,
                RelationInverseChange::SetAlias("related from".to_string()),
                None,
            ),
        )
        .await?;
    let inverse_property_id = relation_with_inverse
        .inverse_property_id()
        .as_ref()
        .expect("generated inverse Property");
    let inverse = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        target_b.id(),
        inverse_property_id,
        &operation_id(&tenant_id, "edge-inverse")?,
        RecordVersion::INITIAL,
        PropertyValueCommand::Relation(vec![source.id().clone()]),
    )
    .await?;
    assert!(matches!(
        inverse,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::RelationProjectionRequired,
            ..
        }
    ));

    let relation_index_id = IndexDefinitionId::default();
    sqlx::query(
        r#"
        INSERT INTO index_definitions (
            id, tenant_id, database_id, property_id, relation_id,
            policy, is_unique, definition_version, generation,
            projection_state
        )
        VALUES (?, ?, ?, NULL, ?, 'EXACT', FALSE, 1, 1, 'PENDING')
        "#,
    )
    .bind(relation_index_id.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(&relation_id)
    .execute(pool.as_ref())
    .await?;
    let stale_index = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-index-stale")?,
        RecordVersion::new(7)?,
        PropertyValueCommand::Relation(vec![target_b.id().clone()]),
    )
    .await?;
    assert!(matches!(
        stale_index,
        RecordMutationDecision::Conflict { .. }
    ));
    let guarded_index = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-index-fresh")?,
        RecordVersion::new(8)?,
        PropertyValueCommand::Relation(vec![target_b.id().clone()]),
    )
    .await?;
    assert!(matches!(
        guarded_index,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::IndexProjectionRequired,
            ..
        }
    ));
    sqlx::query("DELETE FROM index_definitions WHERE id = ?")
        .bind(relation_index_id.to_string())
        .execute(pool.as_ref())
        .await?;

    sqlx::query(
        "UPDATE relationships SET definition_version = 2 WHERE id = ?",
    )
    .bind(&relation_id)
    .execute(pool.as_ref())
    .await?;
    let stale_future = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-future-stale")?,
        RecordVersion::new(7)?,
        PropertyValueCommand::Relation(vec![target_b.id().clone()]),
    )
    .await?;
    assert!(matches!(
        stale_future,
        RecordMutationDecision::Conflict { .. }
    ));
    let guarded_future = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &operation_id(&tenant_id, "edge-future-fresh")?,
        RecordVersion::new(8)?,
        PropertyValueCommand::Relation(vec![target_b.id().clone()]),
    )
    .await?;
    assert!(matches!(
        guarded_future,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::RelationProjectionRequired,
            ..
        }
    ));
    sqlx::query(
        "UPDATE relationships SET definition_version = 1 WHERE id = ?",
    )
    .bind(&relation_id)
    .execute(pool.as_ref())
    .await?;

    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {FAILURE_TRIGGER} BEFORE INSERT ON domain_outbox_events \
         FOR EACH ROW SIGNAL SQLSTATE '45000' \
         SET MESSAGE_TEXT = 'forced RelationEdge outbox failure'"
    ))
    .execute(pool.as_ref())
    .await?;
    let rollback_operation = operation_id(&tenant_id, "edge-rollback")?;
    let rollback = decide_relation_patch(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        source.id(),
        relation_property.id(),
        &rollback_operation,
        RecordVersion::new(8)?,
        PropertyValueCommand::Relation(vec![target_b.id().clone()]),
    )
    .await;
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    assert!(rollback.is_err());
    let rollback_state = sqlx::query(&format!(
        "SELECT record_version, {legacy_column} AS legacy_value, \
         (SELECT CAST(COUNT(*) AS SIGNED) FROM property_values pv \
          WHERE pv.tenant_id = data.tenant_id \
            AND pv.database_id = data.object_id \
            AND pv.data_id = data.id AND pv.property_id = ?) canonical_count, \
         (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges re \
          WHERE re.tenant_id = data.tenant_id \
            AND re.source_database_id = data.object_id \
            AND re.source_data_id = data.id AND re.relation_id = ?) edge_count, \
         (SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations op \
          WHERE op.operation_id = ?) operation_count \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(relation_property.id().to_string())
    .bind(&relation_id)
    .bind(rollback_operation.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(source.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(rollback_state.try_get::<u64, _>("record_version")?, 8);
    assert!(rollback_state
        .try_get::<Option<String>, _>("legacy_value")?
        .is_none());
    assert_eq!(rollback_state.try_get::<i64, _>("canonical_count")?, 0);
    assert_eq!(rollback_state.try_get::<i64, _>("edge_count")?, 0);
    assert_eq!(rollback_state.try_get::<i64, _>("operation_count")?, 0);

    Ok(())
}
