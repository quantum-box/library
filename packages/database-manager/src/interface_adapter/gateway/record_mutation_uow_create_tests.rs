use crate as database_manager;
use database_manager::domain::{
    DataId, Database, DatabaseId, DecideRecordCreateCommand,
    DecideRecordPatchCommand, IndexDefinitionId, Property, PropertyId,
    PropertyType, PropertyValueCommand,
    ReconfigureRelationDefinitionCommand, RecordActor, RecordActorKind,
    RecordCreate, RecordCreatedEventV1, RecordMutationDecision,
    RecordOperationId, RecordPatch, RecordPropertyDelta,
    RecordPropertyPatch, RecordRejectionCode, RecordVersion,
    RelationGeneration, RelationInverseChange, RelationSchemaMutationPort,
    TypeId, TypeRelation, VersionedRecordCreationUnitOfWork,
    VersionedRecordMutationUnitOfWork,
};
use database_manager::interface_adapter::gateway::{
    DataRepositoryImpl, PropertyRepositoryImpl,
};
use database_manager::property_definition_rollout::PropertyDefinitionStorageMode;
use database_manager::property_value_rollout::PropertyValueStorageMode;
use database_manager::relation_edge_rollout::RelationEdgeWriteMode;
use database_manager::{AddPropertyInputData, CreateDatabaseInputData};
use sqlx::{types::Json, Row};
use tachyon_sdk::auth;
use value_object::TenantId;

use std::collections::BTreeMap;

use super::record_mutation_uow_relation_edge_tests::{
    add_record, database_url, operation_id,
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
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    target_database_id: &DatabaseId,
    name: &str,
) -> errors::Result<Property> {
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

async fn add_auto_id_property(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    name: &str,
) -> errors::Result<Property> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    app.add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id,
            database_id,
            name,
            property_type: PropertyType::Id(TypeId::new(true)),
        })
        .await
}

fn create_command(
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    operation_id: &RecordOperationId,
    name: &str,
    properties: Vec<(PropertyId, PropertyValueCommand)>,
) -> errors::Result<DecideRecordCreateCommand> {
    DecideRecordCreateCommand::new(
        tenant_id,
        database_id,
        data_id,
        operation_id,
        RecordActor::new(RecordActorKind::System, "record-create-test")?,
        RecordCreate::new(
            name.parse()?,
            properties
                .into_iter()
                .map(|(property_id, value)| {
                    RecordPropertyPatch::new(&property_id, value)
                })
                .collect(),
        ),
    )
}

#[allow(clippy::too_many_arguments)]
async fn decide_create(
    repository: &DataRepositoryImpl,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    operation_id: &RecordOperationId,
    name: &str,
    properties: Vec<(PropertyId, PropertyValueCommand)>,
) -> errors::Result<RecordMutationDecision> {
    repository
        .decide_create_atomically(&create_command(
            tenant_id,
            database_id,
            data_id,
            operation_id,
            name,
            properties,
        )?)
        .await
}

fn accepted_version(decision: &RecordMutationDecision) -> Option<u64> {
    match decision {
        RecordMutationDecision::Accepted { record_version, .. } => {
            Some(record_version.get())
        }
        _ => None,
    }
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

async fn assert_create_attempt_absent(
    pool: &sqlx::MySqlPool,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    operation_id: &RecordOperationId,
) -> anyhow::Result<()> {
    let state = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data \
            WHERE tenant_id = ? AND object_id = ? AND id = ?) data_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM property_values \
            WHERE tenant_id = ? AND database_id = ? AND data_id = ?) value_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
            WHERE tenant_id = ? AND source_database_id = ? \
              AND source_data_id = ?) edge_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations \
            WHERE operation_id = ?) operation_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
            WHERE operation_id = ?) event_count",
    )
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(data_id.to_string())
    .bind(operation_id.to_string())
    .bind(operation_id.to_string())
    .fetch_one(pool)
    .await?;
    for column in [
        "data_count",
        "value_count",
        "edge_count",
        "operation_count",
        "event_count",
    ] {
        assert_eq!(state.try_get::<i64, _>(column)?, 0, "{column}");
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_create_uow_dual_writes_relations_and_emits_created_event(
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
    let target_database =
        create_database(&app, &tenant_id, "create-target").await?;
    let source_database =
        create_database(&app, &tenant_id, "create-source").await?;
    let relation = add_relation_property(
        &app,
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "related",
    )
    .await?;
    let self_relation = add_relation_property(
        &app,
        &tenant_id,
        source_database.id(),
        source_database.id(),
        "parent",
    )
    .await?;
    let empty_relation = add_relation_property(
        &app,
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "empty related",
    )
    .await?;
    let target =
        add_record(&app, &tenant_id, target_database.id(), None, "target")
            .await?;
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let pool = db.pool();
    let data_id = DataId::default();
    let operation = operation_id(&tenant_id, "create-dual-write")?;
    let decision = decide_create(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        &data_id,
        &operation,
        "new record",
        vec![
            (
                relation.id().clone(),
                PropertyValueCommand::Relation(vec![target.id().clone()]),
            ),
            (
                self_relation.id().clone(),
                PropertyValueCommand::Relation(vec![data_id.clone()]),
            ),
            (
                empty_relation.id().clone(),
                PropertyValueCommand::Relation(vec![]),
            ),
        ],
    )
    .await?;
    assert_eq!(accepted_version(&decision), Some(1));
    let RecordMutationDecision::Accepted { event_ids, .. } = &decision
    else {
        unreachable!("accepted decision asserted above");
    };
    assert_eq!(event_ids.len(), 1);

    let relation_column = format!("value{}", relation.property_num());
    let self_column = format!("value{}", self_relation.property_num());
    let empty_column = format!("value{}", empty_relation.property_num());
    let record = sqlx::query(&format!(
        "SELECT name, record_version, {relation_column} relation_value, \
         {self_column} self_value, {empty_column} empty_value FROM data \
         WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(data_id.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(record.try_get::<String, _>("name")?, "new record");
    assert_eq!(record.try_get::<u64, _>("record_version")?, 1);
    assert_eq!(
        record.try_get::<String, _>("relation_value")?,
        format!("{},{}", target_database.id(), target.id())
    );
    assert_eq!(
        record.try_get::<String, _>("self_value")?,
        format!("{},{}", source_database.id(), data_id)
    );
    assert_eq!(
        record.try_get::<String, _>("empty_value")?,
        target_database.id().to_string()
    );

    let expected_values = BTreeMap::from([
        (
            relation.id().to_string(),
            serde_json::json!({
                "database_id": target_database.id().to_string(),
                "data_ids": [target.id().to_string()],
            }),
        ),
        (
            self_relation.id().to_string(),
            serde_json::json!({
                "database_id": source_database.id().to_string(),
                "data_ids": [data_id.to_string()],
            }),
        ),
        (
            empty_relation.id().to_string(),
            serde_json::json!({
                "database_id": target_database.id().to_string(),
                "data_ids": [],
            }),
        ),
    ]);
    let canonical_rows = sqlx::query(
        "SELECT property_id, type_key, type_version, \
                value_encoding_version, value \
         FROM property_values WHERE tenant_id = ? \
           AND database_id = ? AND data_id = ? \
         ORDER BY property_id",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(data_id.to_string())
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(canonical_rows.len(), expected_values.len());
    for row in &canonical_rows {
        let property_id = row.try_get::<String, _>("property_id")?;
        let value: serde_json::Value =
            serde_json::from_str(&row.try_get::<String, _>("value")?)?;
        assert_eq!(
            expected_values.get(&property_id),
            Some(&value),
            "canonical value for {property_id}"
        );
    }

    let edges = sqlx::query(
        "SELECT relation_id, target_database_id, target_data_id \
         FROM relation_edges WHERE tenant_id = ? \
         AND source_database_id = ? AND source_data_id = ? \
         ORDER BY relation_id, target_data_id",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(data_id.to_string())
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(edges.len(), 2);
    let relation_ids = [
        relation_id(
            pool.as_ref(),
            &tenant_id,
            source_database.id(),
            &relation,
        )
        .await?,
        relation_id(
            pool.as_ref(),
            &tenant_id,
            source_database.id(),
            &self_relation,
        )
        .await?,
    ];
    assert!(edges.iter().all(|edge| relation_ids.contains(
        &edge
            .try_get::<String, _>("relation_id")
            .expect("relation id")
    )));

    let event = sqlx::query(
        "SELECT CAST(event_id AS CHAR) AS event_id, \
                event_sequence, aggregate_version, \
                CAST(event_type AS CHAR) AS event_type, payload \
         FROM domain_outbox_events WHERE operation_id = ?",
    )
    .bind(operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(event.try_get::<u32, _>("event_sequence")?, 1);
    assert_eq!(event.try_get::<u64, _>("aggregate_version")?, 1);
    assert_eq!(
        event.try_get::<String, _>("event_type")?,
        "database.record.created.v1"
    );
    let Json(payload) =
        event.try_get::<Json<RecordCreatedEventV1>, _>("payload")?;
    assert_eq!(
        event.try_get::<String, _>("event_id")?,
        payload.event_id.to_string()
    );
    assert_eq!(event_ids, &[payload.event_id.clone()]);
    assert_eq!(payload.operation_id, operation);
    assert_eq!(payload.data_id, data_id);
    assert_eq!(payload.record_version, "1");
    assert_eq!(payload.name, "new record");
    assert_eq!(payload.properties.len(), canonical_rows.len());
    for (delta, row) in payload.properties.iter().zip(&canonical_rows) {
        let property_id = row.try_get::<String, _>("property_id")?;
        let type_key = row.try_get::<String, _>("type_key")?;
        let type_version = row.try_get::<u16, _>("type_version")?;
        let encoding_version =
            row.try_get::<u16, _>("value_encoding_version")?;
        let raw_value: serde_json::Value =
            serde_json::from_str(&row.try_get::<String, _>("value")?)?;
        let RecordPropertyDelta::Set {
            property_id: event_property_id,
            value,
        } = delta
        else {
            panic!("created Relation event must contain a SET envelope");
        };
        assert_eq!(event_property_id.to_string(), property_id);
        assert_eq!(value.type_ref.key.as_str(), type_key);
        assert_eq!(value.type_ref.version.get(), type_version);
        assert_eq!(value.encoding_version.get(), encoding_version);
        assert_eq!(value.raw_value, raw_value);
    }

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_create_uow_replays_rejects_reuse_and_stays_disabled(
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
    let database =
        create_database(&app, &tenant_id, "create-replay").await?;
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let data_id = DataId::default();
    let operation = operation_id(&tenant_id, "create-replay")?;
    let first = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &operation,
        "first",
        vec![],
    )
    .await?;
    let replay = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &operation,
        "first",
        vec![],
    )
    .await?;
    assert_eq!(first, replay);
    assert_eq!(accepted_version(&first), Some(1));

    let patch = DecideRecordPatchCommand::new(
        &tenant_id,
        database.id(),
        &data_id,
        &operation,
        RecordVersion::INITIAL,
        RecordActor::new(RecordActorKind::System, "record-create-test")?,
        RecordPatch::new(Some("changed".parse()?), vec![]),
    )?;
    let cross_kind = repository.decide_patch_atomically(&patch).await?;
    assert_rejected(&cross_kind, RecordRejectionCode::IdempotencyKeyReuse);

    let collision_operation = operation_id(&tenant_id, "create-collision")?;
    let collision = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &collision_operation,
        "replacement",
        vec![],
    )
    .await?;
    assert_rejected(&collision, RecordRejectionCode::RecordAlreadyExists);
    let stored_name = sqlx::query_scalar::<_, String>(
        "SELECT name FROM data WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(data_id.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(stored_name, "first");

    let target_database =
        create_database(&app, &tenant_id, "disabled-target").await?;
    let relation = add_relation_property(
        &app,
        &tenant_id,
        database.id(),
        target_database.id(),
        "disabled relation",
    )
    .await?;
    let target = add_record(
        &app,
        &tenant_id,
        target_database.id(),
        None,
        "disabled target",
    )
    .await?;
    let disabled_repository =
        DataRepositoryImpl::new_with_all_storage_modes(
            db.clone(),
            PropertyValueStorageMode::DualWriteLegacyRead,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
            RelationEdgeWriteMode::Disabled,
        );
    let disabled_data_id = DataId::default();
    let disabled_operation = operation_id(&tenant_id, "create-disabled")?;
    let disabled = decide_create(
        disabled_repository.as_ref(),
        &tenant_id,
        database.id(),
        &disabled_data_id,
        &disabled_operation,
        "must stay dormant",
        vec![(
            relation.id().clone(),
            PropertyValueCommand::Relation(vec![target.id().clone()]),
        )],
    )
    .await;
    assert!(disabled.is_err());
    let disabled_state = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data \
            WHERE tenant_id = ? AND object_id = ? AND id = ?) data_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) \
            FROM record_mutation_operations WHERE operation_id = ?) op_count",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(disabled_data_id.to_string())
    .bind(disabled_operation.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(disabled_state.try_get::<i64, _>("data_count")?, 0);
    assert_eq!(disabled_state.try_get::<i64, _>("op_count")?, 0);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_create_uow_fails_closed_before_index_policy_on_definition_drift(
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
    let database =
        create_database(&app, &tenant_id, "create-definition-drift")
            .await?;
    let auto_id = add_auto_id_property(
        &app,
        &tenant_id,
        database.id(),
        "canonical id",
    )
    .await?;
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let index_id = IndexDefinitionId::default();
    sqlx::query(
        "INSERT INTO index_definitions ( \
           id, tenant_id, database_id, property_id, relation_id, \
           policy, is_unique, definition_version, generation, \
           projection_state) \
         VALUES (?, ?, ?, ?, NULL, 'EXACT', FALSE, 1, 1, 'PENDING')",
    )
    .bind(index_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(auto_id.id().to_string())
    .execute(db.pool().as_ref())
    .await?;

    // The auto-generated Id is deliberately omitted from the command. CREATE
    // must still validate every definition before returning a durable Index
    // policy rejection.
    sqlx::query(
        "UPDATE fields SET type_config = ? WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(serde_json::json!({ "auto_generate": false }).to_string())
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(auto_id.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let mismatch_data_id = DataId::default();
    let mismatch_operation =
        operation_id(&tenant_id, "create-definition-mismatch")?;
    let mismatch = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &mismatch_data_id,
        &mismatch_operation,
        "definition mismatch",
        vec![],
    )
    .await;
    assert!(
        mismatch.is_err(),
        "definition drift must be an infrastructure error, not a decision"
    );
    assert_create_attempt_absent(
        db.pool().as_ref(),
        &tenant_id,
        database.id(),
        &mismatch_data_id,
        &mismatch_operation,
    )
    .await?;

    sqlx::query(
        "UPDATE fields SET type_key = 'future_id', type_version = 1, \
                type_config = ? \
         WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(serde_json::json!({ "auto_generate": true }).to_string())
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(auto_id.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let opaque_data_id = DataId::default();
    let opaque_operation =
        operation_id(&tenant_id, "create-definition-opaque")?;
    let opaque = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &opaque_data_id,
        &opaque_operation,
        "opaque definition",
        vec![],
    )
    .await;
    assert!(
        opaque.is_err(),
        "opaque definition must be an infrastructure error, not a decision"
    );
    assert_create_attempt_absent(
        db.pool().as_ref(),
        &tenant_id,
        database.id(),
        &opaque_data_id,
        &opaque_operation,
    )
    .await?;

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_create_uow_rejects_resurrection_from_outbox_history(
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
    let database =
        create_database(&app, &tenant_id, "create-history").await?;
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let data_id = DataId::default();
    let original_operation =
        operation_id(&tenant_id, "create-history-original")?;
    let original = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &original_operation,
        "original",
        vec![],
    )
    .await?;
    assert_eq!(accepted_version(&original), Some(1));
    sqlx::query(
        "DELETE FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(data_id.to_string())
    .execute(db.pool().as_ref())
    .await?;

    let replacement_operation =
        operation_id(&tenant_id, "create-history-replacement")?;
    let replacement = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &replacement_operation,
        "must not resurrect",
        vec![],
    )
    .await?;
    assert_rejected(&replacement, RecordRejectionCode::RecordAlreadyExists);
    let replay = decide_create(
        repository.as_ref(),
        &tenant_id,
        database.id(),
        &data_id,
        &replacement_operation,
        "must not resurrect",
        vec![],
    )
    .await?;
    assert_eq!(replacement, replay);

    let state = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data \
            WHERE tenant_id = ? AND object_id = ? AND id = ?) data_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations \
            WHERE tenant_id = ? AND database_id = ? AND data_id = ?) op_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
            WHERE tenant_id = ? AND database_id = ? \
              AND aggregate_id = ?) aggregate_event_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
            WHERE operation_id = ?) replacement_event_count",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(data_id.to_string())
    .bind(replacement_operation.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(state.try_get::<i64, _>("data_count")?, 0);
    assert_eq!(state.try_get::<i64, _>("op_count")?, 2);
    assert_eq!(state.try_get::<i64, _>("aggregate_event_count")?, 1);
    assert_eq!(state.try_get::<i64, _>("replacement_event_count")?, 0);
    let stored = sqlx::query(
        "SELECT CAST(decision_kind AS CHAR) AS decision_kind, \
                decision_payload \
         FROM record_mutation_operations WHERE operation_id = ?",
    )
    .bind(replacement_operation.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(stored.try_get::<String, _>("decision_kind")?, "REJECTED");
    let Json(stored_decision) = stored
        .try_get::<Json<RecordMutationDecision>, _>("decision_payload")?;
    assert_eq!(stored_decision, replacement);

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_create_uow_guards_targets_inverse_cardinality_and_indexes(
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
    let target_database =
        create_database(&app, &tenant_id, "guard-target").await?;
    let source_database =
        create_database(&app, &tenant_id, "guard-source").await?;
    let relation = add_relation_property(
        &app,
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "guard relation",
    )
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
    let source_record = add_record(
        &app,
        &tenant_id,
        source_database.id(),
        None,
        "source-existing",
    )
    .await?;
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );

    for (label, target) in [
        ("missing", DataId::default()),
        ("wrong-database", source_record.id().clone()),
    ] {
        let data_id = DataId::default();
        let decision = decide_create(
            repository.as_ref(),
            &tenant_id,
            source_database.id(),
            &data_id,
            &operation_id(&tenant_id, &format!("create-{label}"))?,
            label,
            vec![(
                relation.id().clone(),
                PropertyValueCommand::Relation(vec![target]),
            )],
        )
        .await?;
        assert_rejected(&decision, RecordRejectionCode::ResourceNotFound);
    }

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
                relation.id(),
                RelationGeneration::new(1)?,
                None,
                None,
                RelationInverseChange::SetAlias(
                    "guard inverse".to_string(),
                ),
                None,
            ),
        )
        .await?;
    let inverse_property_id = relation_with_inverse
        .inverse_property_id()
        .as_ref()
        .expect("generated inverse Property")
        .clone();
    let inverse = decide_create(
        repository.as_ref(),
        &tenant_id,
        target_database.id(),
        &DataId::default(),
        &operation_id(&tenant_id, "create-inverse")?,
        "inverse",
        vec![(
            inverse_property_id,
            PropertyValueCommand::Relation(vec![source_record
                .id()
                .clone()]),
        )],
    )
    .await?;
    assert_rejected(
        &inverse,
        RecordRejectionCode::RelationProjectionRequired,
    );

    let relation_id = relation_id(
        db.pool().as_ref(),
        &tenant_id,
        source_database.id(),
        &relation,
    )
    .await?;
    let index_id = IndexDefinitionId::default();
    sqlx::query(
        "INSERT INTO index_definitions ( \
           id, tenant_id, database_id, property_id, relation_id, \
           policy, is_unique, definition_version, generation, \
           projection_state) \
         VALUES (?, ?, ?, NULL, ?, 'EXACT', FALSE, 1, 1, 'PENDING')",
    )
    .bind(index_id.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(&relation_id)
    .execute(db.pool().as_ref())
    .await?;
    let indexed = decide_create(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        &DataId::default(),
        &operation_id(&tenant_id, "create-indexed")?,
        "indexed",
        vec![(
            relation.id().clone(),
            PropertyValueCommand::Relation(vec![target_a.id().clone()]),
        )],
    )
    .await?;
    assert_rejected(&indexed, RecordRejectionCode::IndexProjectionRequired);
    sqlx::query("DELETE FROM index_definitions WHERE id = ?")
        .bind(index_id.to_string())
        .execute(db.pool().as_ref())
        .await?;

    sqlx::query(
        "UPDATE relationships SET forward_cardinality = 'ONE' \
         WHERE id = ?",
    )
    .bind(&relation_id)
    .execute(db.pool().as_ref())
    .await?;
    let forward_one = decide_create(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        &DataId::default(),
        &operation_id(&tenant_id, "create-forward-one")?,
        "forward-one",
        vec![(
            relation.id().clone(),
            PropertyValueCommand::Relation(vec![
                target_a.id().clone(),
                target_b.id().clone(),
            ]),
        )],
    )
    .await?;
    assert_rejected(
        &forward_one,
        RecordRejectionCode::RelationCardinalityExceeded,
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dormant_record_create_uow_serializes_reverse_one_and_rolls_back_outbox_failure(
) -> anyhow::Result<()> {
    const FAILURE_TRIGGER: &str = "test_record_create_outbox_rollback";

    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let target_database =
        create_database(&app, &tenant_id, "concurrent-target").await?;
    let source_database =
        create_database(&app, &tenant_id, "concurrent-source").await?;
    let relation = add_relation_property(
        &app,
        &tenant_id,
        source_database.id(),
        target_database.id(),
        "reverse-one",
    )
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
    let relation_id = relation_id(
        db.pool().as_ref(),
        &tenant_id,
        source_database.id(),
        &relation,
    )
    .await?;
    sqlx::query(
        "UPDATE relationships SET reverse_cardinality = 'ONE' \
         WHERE id = ?",
    )
    .bind(&relation_id)
    .execute(db.pool().as_ref())
    .await?;
    let repository = DataRepositoryImpl::new_with_all_storage_modes(
        db.clone(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
        RelationEdgeWriteMode::DualWriteLegacyRead,
    );
    let first_id = DataId::default();
    let second_id = DataId::default();
    let first_operation = operation_id(&tenant_id, "create-reverse-one-a")?;
    let second_operation =
        operation_id(&tenant_id, "create-reverse-one-b")?;
    let (first, second) = tokio::join!(
        decide_create(
            repository.as_ref(),
            &tenant_id,
            source_database.id(),
            &first_id,
            &first_operation,
            "first",
            vec![(
                relation.id().clone(),
                PropertyValueCommand::Relation(vec![target_a.id().clone()]),
            )],
        ),
        decide_create(
            repository.as_ref(),
            &tenant_id,
            source_database.id(),
            &second_id,
            &second_operation,
            "second",
            vec![(
                relation.id().clone(),
                PropertyValueCommand::Relation(vec![target_a.id().clone()]),
            )],
        )
    );
    let decisions = [first?, second?];
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| accepted_version(decision) == Some(1))
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
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(target_a_edges, 1);

    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {FAILURE_TRIGGER} BEFORE INSERT ON domain_outbox_events \
         FOR EACH ROW SIGNAL SQLSTATE '45000' \
         SET MESSAGE_TEXT = 'forced Record create outbox failure'"
    ))
    .execute(db.pool().as_ref())
    .await?;
    let rollback_data_id = DataId::default();
    let rollback_operation = operation_id(&tenant_id, "create-rollback")?;
    let rollback = decide_create(
        repository.as_ref(),
        &tenant_id,
        source_database.id(),
        &rollback_data_id,
        &rollback_operation,
        "rollback",
        vec![(
            relation.id().clone(),
            PropertyValueCommand::Relation(vec![target_b.id().clone()]),
        )],
    )
    .await;
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(db.pool().as_ref())
        .await?;
    assert!(rollback.is_err());
    let rollback_state = sqlx::query(
        "SELECT \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM data \
            WHERE tenant_id = ? AND object_id = ? AND id = ?) data_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM property_values \
            WHERE tenant_id = ? AND database_id = ? AND data_id = ?) value_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM relation_edges \
            WHERE tenant_id = ? AND source_database_id = ? \
              AND source_data_id = ?) edge_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations \
            WHERE operation_id = ?) operation_count, \
           (SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
            WHERE operation_id = ?) event_count",
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(rollback_data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(rollback_data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(rollback_data_id.to_string())
    .bind(rollback_operation.to_string())
    .bind(rollback_operation.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    for column in [
        "data_count",
        "value_count",
        "edge_count",
        "operation_count",
        "event_count",
    ] {
        assert_eq!(rollback_state.try_get::<i64, _>(column)?, 0);
    }

    Ok(())
}
