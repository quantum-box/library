use database_manager::domain::{
    Data, DatabaseId, IndexDefinitionId, Property, PropertyType,
    PropertyValueCommand, RecordMutationDecision, RecordOperationId,
    RecordRejectionCode, RecordVersion, TypeId, TypeRelation,
};
use database_manager::property_value_rollout::PropertyValueStorageMode;
use database_manager::{
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    PatchRecordInputData, PropertyDataInputData,
};
use sqlx::types::Json;
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

#[derive(Debug)]
struct TenantUserExecutor {
    id: String,
    tenant_id: TenantId,
}

impl auth::ExecutorAction for TenantUserExecutor {
    fn get_id(&self) -> &str {
        &self.id
    }

    fn has_tenant_id(&self, tenant_id: &TenantId) -> bool {
        &self.tenant_id == tenant_id
    }

    fn is_system_user(&self) -> bool {
        false
    }

    fn is_user(&self) -> bool {
        true
    }

    fn is_service_account(&self) -> bool {
        false
    }

    fn is_none(&self) -> bool {
        false
    }
}

fn database_url() -> anyhow::Result<DatabaseUrl> {
    dotenvy::dotenv().ok();
    Ok(std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager"))
}

fn test_operation_id(
    tenant_id: &TenantId,
    label: &str,
) -> errors::Result<RecordOperationId> {
    RecordOperationId::new(format!("{tenant_id}:{label}"))
}

async fn add_string_record(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    property: &Property,
    name: &str,
    value: &str,
) -> errors::Result<Data> {
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    app.add_data_usecase()
        .execute(AddDataInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id,
            database_id,
            name,
            property_data: vec![PropertyDataInputData {
                property_id: property.id().clone(),
                value: PropertyValueCommand::String(value.to_string()),
            }],
        })
        .await
}

#[allow(clippy::too_many_arguments)]
async fn patch_string_record(
    app: &database_manager::App,
    executor: &dyn auth::ExecutorAction,
    multi_tenancy: &auth::MultiTenancy,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    record: &Data,
    property: &Property,
    operation_id: &RecordOperationId,
    expected_version: RecordVersion,
    name: &str,
    value: &str,
) -> errors::Result<RecordMutationDecision> {
    app.patch_record_usecase()
        .execute(PatchRecordInputData {
            executor,
            multi_tenancy,
            tenant_id,
            database_id,
            data_id: record.id(),
            operation_id,
            expected_version,
            name: Some(name),
            properties: vec![PropertyDataInputData {
                property_id: property.id().clone(),
                value: PropertyValueCommand::String(value.to_string()),
            }],
        })
        .await
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn record_patch_decision_is_atomic_cas_idempotent_and_fail_closed(
) -> anyhow::Result<()> {
    const FAILURE_TRIGGER: &str = "test_record_outbox_rollback";

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
    // A prior interrupted test may have failed after creating the fault
    // injection trigger but before removing it. Clear it before the first
    // successful mutation so this integration test is safe to rerun.
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    let tenant_id = TenantId::default();
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let user = TenantUserExecutor {
        id: "record-editor".to_string(),
        tenant_id: tenant_id.clone(),
    };

    let database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "record-patch-decisions",
        })
        .await?;
    let string_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "content",
            property_type: PropertyType::String,
        })
        .await?;
    let record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "before",
        "before",
    )
    .await?;

    let operation_id = test_operation_id(&tenant_id, "accepted")?;
    let accepted = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &operation_id,
        RecordVersion::INITIAL,
        "accepted-name",
        "after",
    )
    .await?;
    let event_id = match &accepted {
        RecordMutationDecision::Accepted {
            decision_version,
            operation_id: decided_operation_id,
            record_version,
            event_ids,
        } => {
            assert_eq!(*decision_version, 1);
            assert_eq!(decided_operation_id, &operation_id);
            assert_eq!(record_version.get(), 2);
            assert_eq!(event_ids.len(), 1);
            event_ids[0].clone()
        }
        other => panic!("expected ACCEPTED, got {other:?}"),
    };

    let legacy_column = format!("value{}", string_property.property_num());
    let persisted = sqlx::query(&format!(
        "SELECT name, record_version, {legacy_column} AS legacy_value \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(persisted.try_get::<String, _>("name")?, "accepted-name");
    assert_eq!(persisted.try_get::<u64, _>("record_version")?, 2);
    assert_eq!(persisted.try_get::<String, _>("legacy_value")?, "after");
    let canonical = sqlx::query_scalar::<_, String>(
        r#"
        SELECT value FROM property_values
        WHERE tenant_id = ? AND database_id = ?
          AND data_id = ? AND property_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .bind(string_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(canonical, "\"after\"");

    let operation = sqlx::query(
        r#"
        SELECT CAST(actor_kind AS CHAR) AS actor_kind,
               CAST(actor_id AS CHAR) AS actor_id,
               CAST(decision_kind AS CHAR) AS decision_kind,
               decision_version, decision_payload
        FROM record_mutation_operations
        WHERE operation_id = ?
        "#,
    )
    .bind(operation_id.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(operation.try_get::<String, _>("actor_kind")?, "USER");
    assert_eq!(
        operation.try_get::<String, _>("actor_id")?,
        "record-editor"
    );
    assert_eq!(
        operation.try_get::<String, _>("decision_kind")?,
        "ACCEPTED"
    );
    assert_eq!(operation.try_get::<u16, _>("decision_version")?, 1);
    let Json(decision_payload) = operation
        .try_get::<Json<serde_json::Value>, _>("decision_payload")?;
    assert_eq!(
        decision_payload
            .get("record_version")
            .and_then(serde_json::Value::as_str),
        Some("2")
    );

    let outbox = sqlx::query(
        r#"
        SELECT event_sequence, aggregate_version,
               CAST(event_type AS CHAR) AS event_type, payload
        FROM domain_outbox_events
        WHERE event_id = ?
        "#,
    )
    .bind(event_id.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(outbox.try_get::<u32, _>("event_sequence")?, 1);
    assert_eq!(outbox.try_get::<u64, _>("aggregate_version")?, 2);
    assert_eq!(
        outbox.try_get::<String, _>("event_type")?,
        "database.record.patched.v1"
    );
    let Json(event_payload) =
        outbox.try_get::<Json<serde_json::Value>, _>("payload")?;
    assert_eq!(
        event_payload
            .get("previous_version")
            .and_then(serde_json::Value::as_str),
        Some("1")
    );
    assert_eq!(
        event_payload
            .get("record_version")
            .and_then(serde_json::Value::as_str),
        Some("2")
    );
    assert_eq!(
        event_payload
            .pointer("/actor/kind")
            .and_then(|v| v.as_str()),
        Some("USER")
    );
    assert_eq!(
        event_payload
            .get("operation_id")
            .and_then(serde_json::Value::as_str),
        Some(operation_id.as_str())
    );
    assert_eq!(
        event_payload
            .pointer("/name/previous")
            .and_then(|v| v.as_str()),
        Some("before")
    );
    assert_eq!(
        event_payload
            .pointer("/name/current")
            .and_then(|v| v.as_str()),
        Some("accepted-name")
    );
    assert_eq!(
        event_payload
            .pointer("/properties/0/action")
            .and_then(|v| v.as_str()),
        Some("SET")
    );
    assert_eq!(
        event_payload
            .pointer("/properties/0/property_id")
            .and_then(|v| v.as_str()),
        Some(string_property.id().as_str())
    );
    assert_eq!(
        event_payload.pointer("/properties/0/value/raw_value"),
        Some(&serde_json::json!("after"))
    );

    let stale_operation = test_operation_id(&tenant_id, "stale")?;
    let conflict = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &stale_operation,
        RecordVersion::INITIAL,
        "stale-name",
        "stale-value",
    )
    .await?;
    match conflict {
        RecordMutationDecision::Conflict { current, .. } => {
            assert_eq!(current.record_version().get(), 2);
            assert_eq!(current.name(), "accepted-name");
            let value = current
                .properties()
                .iter()
                .find(|snapshot| {
                    snapshot.property_id() == string_property.id()
                })
                .and_then(|snapshot| snapshot.value().as_ref())
                .expect("conflict snapshot contains current value");
            assert_eq!(value.raw_value, serde_json::json!("after"));
            let persisted_updated_at =
                sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
                    "SELECT updated_at FROM data WHERE tenant_id = ? \
                 AND object_id = ? AND id = ?",
                )
                .bind(tenant_id.to_string())
                .bind(database.id().to_string())
                .bind(record.id().to_string())
                .fetch_one(pool.as_ref())
                .await?;
            assert_eq!(current.updated_at(), &persisted_updated_at);
        }
        other => panic!("expected CONFLICT, got {other:?}"),
    }
    let record_event_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events
        WHERE tenant_id = ? AND database_id = ? AND aggregate_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(record_event_count, 1, "conflicts emit no domain event");

    let replay = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &operation_id,
        RecordVersion::INITIAL,
        "accepted-name",
        "after",
    )
    .await?;
    assert_eq!(replay, accepted);
    let reused = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &operation_id,
        RecordVersion::INITIAL,
        "different-name",
        "different-value",
    )
    .await?;
    assert!(matches!(
        reused,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::IdempotencyKeyReuse,
            ..
        }
    ));
    let actor_reuse = patch_string_record(
        &app,
        &auth::Executor::SystemUser,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &operation_id,
        RecordVersion::INITIAL,
        "accepted-name",
        "after",
    )
    .await?;
    let version_reuse = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &operation_id,
        RecordVersion::new(2)?,
        "accepted-name",
        "after",
    )
    .await?;
    for decision in [actor_reuse, version_reuse] {
        assert!(matches!(
            decision,
            RecordMutationDecision::Rejected {
                code: RecordRejectionCode::IdempotencyKeyReuse,
                ..
            }
        ));
    }

    let advance_operation = test_operation_id(&tenant_id, "advance-to-v3")?;
    let advanced = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &advance_operation,
        RecordVersion::new(2)?,
        "advanced-name",
        "advanced-value",
    )
    .await?;
    assert!(matches!(
        advanced,
        RecordMutationDecision::Accepted {
            record_version,
            ..
        } if record_version.get() == 3
    ));
    let replay_after_advance = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &operation_id,
        RecordVersion::INITIAL,
        "accepted-name",
        "after",
    )
    .await?;
    assert_eq!(
        replay_after_advance, accepted,
        "lost-ack replay returns the original v2 decision after v3 exists"
    );
    let advanced_version = sqlx::query_scalar::<_, u64>(
        "SELECT record_version FROM data WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(advanced_version, 3);

    let anonymous_operation = test_operation_id(&tenant_id, "anonymous")?;
    let anonymous_error = patch_string_record(
        &app,
        &auth::Executor::None,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &anonymous_operation,
        RecordVersion::new(2)?,
        "anonymous",
        "anonymous",
    )
    .await
    .expect_err("an anonymous executor must not enter the mutation UoW");
    assert!(anonymous_error.is_not_found());
    let anonymous_operation_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM record_mutation_operations \
         WHERE operation_id = ?",
    )
    .bind(anonymous_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(anonymous_operation_count, 0);

    let foreign_tenant_id = TenantId::default();
    let foreign_user = TenantUserExecutor {
        id: "foreign-editor".to_string(),
        tenant_id: foreign_tenant_id,
    };
    let cross_tenant_operation =
        test_operation_id(&tenant_id, "cross-tenant")?;
    let cross_tenant_error = patch_string_record(
        &app,
        &foreign_user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &cross_tenant_operation,
        RecordVersion::new(3)?,
        "cross-tenant",
        "cross-tenant",
    )
    .await
    .expect_err("a foreign executor must not enter the mutation UoW");
    assert!(cross_tenant_error.is_not_found());
    let cross_tenant_journal_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) \
         FROM record_mutation_operations WHERE operation_id = ?",
    )
    .bind(cross_tenant_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(cross_tenant_journal_count, 0);

    let concurrent_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "concurrent-before",
        "concurrent-before",
    )
    .await?;
    let scope_reuse = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &concurrent_record,
        &string_property,
        &operation_id,
        RecordVersion::INITIAL,
        "scope-reuse",
        "scope-reuse",
    )
    .await?;
    assert!(matches!(
        scope_reuse,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::IdempotencyKeyReuse,
            ..
        }
    ));
    let left_operation = test_operation_id(&tenant_id, "concurrent-left")?;
    let right_operation =
        test_operation_id(&tenant_id, "concurrent-right")?;
    let left = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &concurrent_record,
        &string_property,
        &left_operation,
        RecordVersion::INITIAL,
        "concurrent-left",
        "left",
    );
    let right = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &concurrent_record,
        &string_property,
        &right_operation,
        RecordVersion::INITIAL,
        "concurrent-right",
        "right",
    );
    let (left, right) = tokio::join!(left, right);
    let decisions = [left?, right?];
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(
                decision,
                RecordMutationDecision::Accepted { .. }
            ))
            .count(),
        1
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(
                decision,
                RecordMutationDecision::Conflict { .. }
            ))
            .count(),
        1
    );
    let concurrent_version = sqlx::query_scalar::<_, u64>(
        "SELECT record_version FROM data WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(concurrent_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(concurrent_version, 2);
    let concurrent_event_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
         WHERE aggregate_id = ?",
    )
    .bind(concurrent_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(concurrent_event_count, 1);

    let duplicate_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "duplicate-before",
        "duplicate-before",
    )
    .await?;
    let duplicate_operation =
        test_operation_id(&tenant_id, "concurrent-same-op")?;
    let first = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &duplicate_record,
        &string_property,
        &duplicate_operation,
        RecordVersion::INITIAL,
        "duplicate-after",
        "duplicate-after",
    );
    let second = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &duplicate_record,
        &string_property,
        &duplicate_operation,
        RecordVersion::INITIAL,
        "duplicate-after",
        "duplicate-after",
    );
    let (first, second) = tokio::join!(first, second);
    let first = first?;
    let second = second?;
    assert_eq!(first, second);
    assert!(matches!(
        first,
        RecordMutationDecision::Accepted {
            record_version,
            ..
        } if record_version.get() == 2
    ));
    let duplicate_counts = sqlx::query(
        r#"
        SELECT
            (SELECT CAST(COUNT(*) AS SIGNED)
             FROM record_mutation_operations WHERE operation_id = ?)
                AS operation_count,
            (SELECT CAST(COUNT(*) AS SIGNED)
             FROM domain_outbox_events WHERE operation_id = ?)
                AS event_count,
            (SELECT record_version FROM data WHERE tenant_id = ?
             AND object_id = ? AND id = ?) AS record_version
        "#,
    )
    .bind(duplicate_operation.to_string())
    .bind(duplicate_operation.to_string())
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(duplicate_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(duplicate_counts.try_get::<i64, _>("operation_count")?, 1);
    assert_eq!(duplicate_counts.try_get::<i64, _>("event_count")?, 1);
    assert_eq!(duplicate_counts.try_get::<u64, _>("record_version")?, 2);

    let rollback_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "rollback-before",
        "rollback-before",
    )
    .await?;
    let rollback_operation =
        test_operation_id(&tenant_id, "outbox-failure")?;
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {FAILURE_TRIGGER} \
         BEFORE INSERT ON domain_outbox_events FOR EACH ROW \
         BEGIN \
           IF NEW.operation_id = '{}' THEN \
             SIGNAL SQLSTATE '45000' \
               SET MESSAGE_TEXT = 'forced outbox failure'; \
           END IF; \
         END",
        rollback_operation.as_str()
    ))
    .execute(pool.as_ref())
    .await?;
    let rollback_error = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &rollback_record,
        &string_property,
        &rollback_operation,
        RecordVersion::INITIAL,
        "rollback-must-not-persist",
        "rollback-must-not-persist",
    )
    .await
    .expect_err("outbox failure must fail the whole unit of work");
    assert!(matches!(
        rollback_error,
        errors::Error::InternalServerError { .. }
    ));
    sqlx::raw_sql(&format!("DROP TRIGGER IF EXISTS {FAILURE_TRIGGER}"))
        .execute(pool.as_ref())
        .await?;
    let rollback_state = sqlx::query(&format!(
        "SELECT name, record_version, {legacy_column} AS legacy_value \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(rollback_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        rollback_state.try_get::<String, _>("name")?,
        "rollback-before"
    );
    assert_eq!(rollback_state.try_get::<u64, _>("record_version")?, 1);
    assert_eq!(
        rollback_state.try_get::<String, _>("legacy_value")?,
        "rollback-before"
    );
    let rollback_canonical = sqlx::query_scalar::<_, String>(
        "SELECT value FROM property_values WHERE data_id = ? AND property_id = ?",
    )
    .bind(rollback_record.id().to_string())
    .bind(string_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(rollback_canonical, "\"rollback-before\"");
    for table in ["record_mutation_operations", "domain_outbox_events"] {
        let count = sqlx::query_scalar::<_, i64>(&format!(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM {table} \
             WHERE operation_id = ?"
        ))
        .bind(rollback_operation.to_string())
        .fetch_one(pool.as_ref())
        .await?;
        assert_eq!(count, 0, "{table} must roll back");
    }

    let opaque_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "opaque-before",
        "opaque-before",
    )
    .await?;
    sqlx::query(
        r#"
        UPDATE property_values
        SET type_key = 'future_string', value = '{"future":true}'
        WHERE tenant_id = ? AND database_id = ?
          AND data_id = ? AND property_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(opaque_record.id().to_string())
    .bind(string_property.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let opaque_operation = test_operation_id(&tenant_id, "opaque-target")?;
    let opaque_decision = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &opaque_record,
        &string_property,
        &opaque_operation,
        RecordVersion::INITIAL,
        "opaque-must-not-change",
        "opaque-must-not-change",
    )
    .await?;
    assert!(matches!(
        opaque_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::InvalidPropertyValue,
            ..
        }
    ));
    let opaque_state = sqlx::query(&format!(
        "SELECT record_version, {legacy_column} AS legacy_value \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(opaque_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(opaque_state.try_get::<u64, _>("record_version")?, 1);
    assert_eq!(
        opaque_state.try_get::<String, _>("legacy_value")?,
        "opaque-before"
    );
    let opaque_canonical = sqlx::query_as::<_, (String, String)>(
        "SELECT type_key, value FROM property_values \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(opaque_record.id().to_string())
    .bind(string_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        opaque_canonical,
        ("future_string".to_string(), "{\"future\":true}".to_string())
    );

    let auto_id_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "record-id",
            property_type: PropertyType::Id(TypeId::new(true)),
        })
        .await?;
    let auto_id_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "auto-id-before",
        "auto-id-before",
    )
    .await?;
    let auto_id_operation = test_operation_id(&tenant_id, "auto-id")?;
    let auto_id_decision = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &user,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: auto_id_record.id(),
            operation_id: &auto_id_operation,
            expected_version: RecordVersion::INITIAL,
            name: None,
            properties: vec![PropertyDataInputData {
                property_id: auto_id_property.id().clone(),
                value: PropertyValueCommand::Id("external-id".to_string()),
            }],
        })
        .await?;
    assert!(matches!(
        auto_id_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::InvalidPropertyValue,
            ..
        }
    ));
    let auto_id_column =
        format!("value{}", auto_id_property.property_num());
    let auto_id_state = sqlx::query(&format!(
        "SELECT record_version, {auto_id_column} AS id_value \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(auto_id_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(auto_id_state.try_get::<u64, _>("record_version")?, 1);
    assert_eq!(
        auto_id_state.try_get::<String, _>("id_value")?,
        auto_id_record.id().to_string()
    );

    // A pre-policy Record may expose an external auto-ID. The versioned
    // writer preserves that value until an explicit repair migration; it
    // must not treat the canonical DataId as permission to rewrite it.
    const EXTERNAL_LEGACY_ID: &str = "external-legacy-id";
    sqlx::query(&format!(
        "UPDATE data SET {auto_id_column} = ? \
         WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(EXTERNAL_LEGACY_ID)
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(auto_id_record.id().to_string())
    .execute(pool.as_ref())
    .await?;
    sqlx::query(
        r#"
        UPDATE property_values
        SET value = ?
        WHERE tenant_id = ? AND database_id = ?
          AND data_id = ? AND property_id = ?
        "#,
    )
    .bind(serde_json::to_string(EXTERNAL_LEGACY_ID)?)
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(auto_id_record.id().to_string())
    .bind(auto_id_property.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let legacy_auto_id_operation =
        test_operation_id(&tenant_id, "legacy-auto-id")?;
    let legacy_auto_id_decision = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &user,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: auto_id_record.id(),
            operation_id: &legacy_auto_id_operation,
            expected_version: RecordVersion::INITIAL,
            name: None,
            properties: vec![PropertyDataInputData {
                property_id: auto_id_property.id().clone(),
                value: PropertyValueCommand::Id(
                    auto_id_record.id().to_string(),
                ),
            }],
        })
        .await?;
    assert!(matches!(
        legacy_auto_id_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::InvalidPropertyValue,
            ..
        }
    ));
    let preserved_legacy_auto_id = sqlx::query(&format!(
        "SELECT record_version, {auto_id_column} AS id_value \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(auto_id_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        preserved_legacy_auto_id.try_get::<u64, _>("record_version")?,
        1
    );
    assert_eq!(
        preserved_legacy_auto_id.try_get::<String, _>("id_value")?,
        EXTERNAL_LEGACY_ID
    );
    let preserved_canonical_auto_id = sqlx::query_scalar::<_, String>(
        "SELECT value FROM property_values \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(auto_id_record.id().to_string())
    .bind(auto_id_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        preserved_canonical_auto_id,
        serde_json::to_string(EXTERNAL_LEGACY_ID)?
    );

    let future_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "future-value",
            property_type: PropertyType::String,
        })
        .await?;
    let unrelated_opaque_record = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "unrelated-opaque-before",
            property_data: vec![
                PropertyDataInputData {
                    property_id: string_property.id().clone(),
                    value: PropertyValueCommand::String(
                        "writable-before".to_string(),
                    ),
                },
                PropertyDataInputData {
                    property_id: future_property.id().clone(),
                    value: PropertyValueCommand::String(
                        "future-before".to_string(),
                    ),
                },
            ],
        })
        .await?;
    sqlx::query(
        r#"
        UPDATE property_values
        SET type_key = 'future_string', value = '{"future":true}'
        WHERE tenant_id = ? AND database_id = ?
          AND data_id = ? AND property_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(unrelated_opaque_record.id().to_string())
    .bind(future_property.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let unrelated_operation =
        test_operation_id(&tenant_id, "unrelated-opaque")?;
    let unrelated_decision = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &unrelated_opaque_record,
        &string_property,
        &unrelated_operation,
        RecordVersion::INITIAL,
        "unrelated-opaque-after",
        "writable-after",
    )
    .await?;
    assert!(matches!(
        unrelated_decision,
        RecordMutationDecision::Accepted {
            record_version,
            ..
        } if record_version.get() == 2
    ));
    let preserved_opaque = sqlx::query_as::<_, (String, String)>(
        "SELECT type_key, value FROM property_values \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(unrelated_opaque_record.id().to_string())
    .bind(future_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        preserved_opaque,
        ("future_string".to_string(), "{\"future\":true}".to_string())
    );
    let Json(unrelated_event) = sqlx::query_scalar::<
        _,
        Json<serde_json::Value>,
    >(
        "SELECT payload FROM domain_outbox_events WHERE operation_id = ?",
    )
    .bind(unrelated_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        unrelated_event
            .get("properties")
            .and_then(serde_json::Value::as_array)
            .map(Vec::len),
        Some(1)
    );

    let maximum_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "maximum-before",
        "maximum-before",
    )
    .await?;
    sqlx::query(
        "UPDATE data SET record_version = ? WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(u64::MAX)
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(maximum_record.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let maximum_operation =
        test_operation_id(&tenant_id, "maximum-version")?;
    let maximum_decision = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &maximum_record,
        &string_property,
        &maximum_operation,
        RecordVersion::new(u64::MAX)?,
        "maximum-must-not-change",
        "maximum-must-not-change",
    )
    .await?;
    assert!(matches!(
        maximum_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::VersionExhausted,
            ..
        }
    ));
    let maximum_state = sqlx::query(&format!(
        "SELECT record_version, {legacy_column} AS legacy_value \
         FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(maximum_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        maximum_state.try_get::<u64, _>("record_version")?,
        u64::MAX
    );
    assert_eq!(
        maximum_state.try_get::<String, _>("legacy_value")?,
        "maximum-before"
    );
    let maximum_event_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
         WHERE operation_id = ?",
    )
    .bind(maximum_operation.to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(maximum_event_count, 0);

    let validation_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "validation-before",
        "validation-before",
    )
    .await?;
    let empty_operation = test_operation_id(&tenant_id, "empty-patch")?;
    let empty_decision = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &user,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: validation_record.id(),
            operation_id: &empty_operation,
            expected_version: RecordVersion::INITIAL,
            name: None,
            properties: Vec::new(),
        })
        .await?;
    assert!(matches!(
        empty_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::EmptyPatch,
            ..
        }
    ));
    let duplicate_operation =
        test_operation_id(&tenant_id, "duplicate-property")?;
    let duplicate_decision = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &user,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: validation_record.id(),
            operation_id: &duplicate_operation,
            expected_version: RecordVersion::INITIAL,
            name: None,
            properties: vec![
                PropertyDataInputData {
                    property_id: string_property.id().clone(),
                    value: PropertyValueCommand::String("one".to_string()),
                },
                PropertyDataInputData {
                    property_id: string_property.id().clone(),
                    value: PropertyValueCommand::String("two".to_string()),
                },
            ],
        })
        .await?;
    assert!(matches!(
        duplicate_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::DuplicateProperty,
            ..
        }
    ));
    let validation_version = sqlx::query_scalar::<_, u64>(
        "SELECT record_version FROM data WHERE tenant_id = ? \
         AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(validation_record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(validation_version, 1);

    let indexed_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "indexed-before",
        "indexed-before",
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO index_definitions (
            id, tenant_id, database_id, property_id, relation_id,
            policy, is_unique, projection_state
        )
        VALUES (?, ?, ?, ?, NULL, 'EXACT', FALSE, 'PENDING')
        "#,
    )
    .bind(IndexDefinitionId::default().to_string())
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(string_property.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let index_operation =
        test_operation_id(&tenant_id, "index-fail-closed")?;
    let index_decision = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &indexed_record,
        &string_property,
        &index_operation,
        RecordVersion::INITIAL,
        "indexed-must-not-change",
        "indexed-must-not-change",
    )
    .await?;
    assert!(matches!(
        index_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::IndexProjectionRequired,
            ..
        }
    ));
    let stale_index_operation =
        test_operation_id(&tenant_id, "stale-index-conflict-first")?;
    let stale_index_decision = patch_string_record(
        &app,
        &user,
        &multi_tenancy,
        &tenant_id,
        database.id(),
        &record,
        &string_property,
        &stale_index_operation,
        RecordVersion::INITIAL,
        "stale-index",
        "stale-index",
    )
    .await?;
    assert!(matches!(
        stale_index_decision,
        RecordMutationDecision::Conflict { .. }
    ));

    let target_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "relation-target",
        })
        .await?;
    let relation_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "blocked-relation",
            property_type: PropertyType::Relation(TypeRelation::new(
                target_database.id().clone(),
            )),
        })
        .await?;
    let relation_record = add_string_record(
        &app,
        &tenant_id,
        database.id(),
        &string_property,
        "relation-before",
        "relation-before",
    )
    .await?;
    let relation_operation =
        test_operation_id(&tenant_id, "relation-fail-closed")?;
    let relation_decision = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &user,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: relation_record.id(),
            operation_id: &relation_operation,
            expected_version: RecordVersion::INITIAL,
            name: None,
            properties: vec![PropertyDataInputData {
                property_id: relation_property.id().clone(),
                value: PropertyValueCommand::Relation(Vec::new()),
            }],
        })
        .await?;
    assert!(matches!(
        relation_decision,
        RecordMutationDecision::Rejected {
            code: RecordRejectionCode::RelationProjectionRequired,
            ..
        }
    ));
    let stale_relation_operation =
        test_operation_id(&tenant_id, "stale-relation-conflict-first")?;
    let stale_relation_decision = app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &user,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            operation_id: &stale_relation_operation,
            expected_version: RecordVersion::INITIAL,
            name: None,
            properties: vec![PropertyDataInputData {
                property_id: relation_property.id().clone(),
                value: PropertyValueCommand::Relation(Vec::new()),
            }],
        })
        .await?;
    assert!(matches!(
        stale_relation_decision,
        RecordMutationDecision::Conflict { .. }
    ));

    for (record_id, operation_id) in [
        (indexed_record.id(), &index_operation),
        (relation_record.id(), &relation_operation),
    ] {
        let version = sqlx::query_scalar::<_, u64>(
            "SELECT record_version FROM data WHERE tenant_id = ? \
             AND object_id = ? AND id = ?",
        )
        .bind(tenant_id.to_string())
        .bind(database.id().to_string())
        .bind(record_id.to_string())
        .fetch_one(pool.as_ref())
        .await?;
        assert_eq!(version, 1);
        let event_count = sqlx::query_scalar::<_, i64>(
            "SELECT CAST(COUNT(*) AS SIGNED) FROM domain_outbox_events \
             WHERE operation_id = ?",
        )
        .bind(operation_id.to_string())
        .fetch_one(pool.as_ref())
        .await?;
        assert_eq!(event_count, 0);
        let decision_kind = sqlx::query_scalar::<_, String>(
            "SELECT CAST(decision_kind AS CHAR) \
             FROM record_mutation_operations \
             WHERE operation_id = ?",
        )
        .bind(operation_id.to_string())
        .fetch_one(pool.as_ref())
        .await?;
        assert_eq!(decision_kind, "REJECTED");
    }

    Ok(())
}
