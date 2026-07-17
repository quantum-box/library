use database_manager::domain::{
    DataId, DatabaseId, RecordEventId, RecordOperationId,
};
use sqlx::Row;
use value_object::{DatabaseUrl, TenantId};

fn database_url() -> anyhow::Result<DatabaseUrl> {
    dotenvy::dotenv().ok();
    Ok(std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager"))
}

fn assert_check_violation(error: sqlx::Error, constraint: &str) {
    let database_error = error
        .as_database_error()
        .expect("MySQL must report a constraint violation");
    assert!(database_error.is_check_violation());
    assert!(
        database_error.message().contains(constraint),
        "expected {constraint}, got: {}",
        database_error.message()
    );
}

#[test]
fn record_mutation_tables_are_retryable_after_partial_tidb_ddl() {
    let migration = include_str!(
        "../migrations/20260716120000_create_record_mutation_journal.sql"
    );

    assert_eq!(migration.matches("CREATE TABLE IF NOT EXISTS").count(), 3);
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn record_mutation_journal_is_expand_only_and_strict(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let pool = db.pool();

    let tables = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(TABLE_NAME AS CHAR)
        FROM information_schema.TABLES
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME IN (
              'record_mutation_operations',
              'domain_outbox_events',
              'domain_outbox_deliveries'
          )
        ORDER BY TABLE_NAME
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(
        tables,
        [
            "domain_outbox_deliveries",
            "domain_outbox_events",
            "record_mutation_operations",
        ]
    );

    let operation_columns = sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text,
               CAST(COLUMN_TYPE AS CHAR) AS column_type_text,
               CAST(IS_NULLABLE AS CHAR) AS nullable_text,
               CAST(COLLATION_NAME AS CHAR) AS collation_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'record_mutation_operations'
          AND COLUMN_NAME IN (
              'operation_id', 'expected_version', 'request_fingerprint',
              'decision_kind', 'decision_version', 'decision_payload'
          )
        ORDER BY ORDINAL_POSITION
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?
    .into_iter()
    .map(|row| {
        Ok((
            row.try_get::<String, _>("column_name_text")?,
            row.try_get::<String, _>("column_type_text")?,
            row.try_get::<String, _>("nullable_text")?,
            row.try_get::<Option<String>, _>("collation_text")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    assert_eq!(
        operation_columns,
        [
            (
                "operation_id".to_string(),
                "varchar(64)".to_string(),
                "NO".to_string(),
                Some("ascii_bin".to_string()),
            ),
            (
                "expected_version".to_string(),
                "bigint unsigned".to_string(),
                "NO".to_string(),
                None,
            ),
            (
                "request_fingerprint".to_string(),
                "binary(32)".to_string(),
                "NO".to_string(),
                None,
            ),
            (
                "decision_kind".to_string(),
                "varchar(8)".to_string(),
                "NO".to_string(),
                Some("ascii_bin".to_string()),
            ),
            (
                "decision_version".to_string(),
                "smallint unsigned".to_string(),
                "YES".to_string(),
                None,
            ),
            (
                "decision_payload".to_string(),
                "json".to_string(),
                "YES".to_string(),
                None,
            ),
        ]
    );

    let outbox_version_columns = sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text,
               CAST(COLUMN_TYPE AS CHAR) AS column_type_text,
               CAST(IS_NULLABLE AS CHAR) AS nullable_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'domain_outbox_events'
          AND COLUMN_NAME IN ('event_sequence', 'aggregate_version')
        ORDER BY ORDINAL_POSITION
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?
    .into_iter()
    .map(|row| {
        Ok((
            row.try_get::<String, _>("column_name_text")?,
            row.try_get::<String, _>("column_type_text")?,
            row.try_get::<String, _>("nullable_text")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    assert_eq!(
        outbox_version_columns,
        [
            (
                "event_sequence".to_string(),
                "int unsigned".to_string(),
                "NO".to_string(),
            ),
            (
                "aggregate_version".to_string(),
                "bigint unsigned".to_string(),
                "NO".to_string(),
            ),
        ]
    );

    let foreign_keys = sqlx::query(
        r#"
        SELECT CAST(CONSTRAINT_NAME AS CHAR) AS constraint_name_text,
               CAST(TABLE_NAME AS CHAR) AS table_name_text,
               CAST(REFERENCED_TABLE_NAME AS CHAR) AS referenced_table_text,
               CAST(DELETE_RULE AS CHAR) AS delete_rule_text
        FROM information_schema.REFERENTIAL_CONSTRAINTS
        WHERE CONSTRAINT_SCHEMA = DATABASE()
          AND TABLE_NAME IN (
              'domain_outbox_events', 'domain_outbox_deliveries'
          )
        ORDER BY CONSTRAINT_NAME
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?
    .into_iter()
    .map(|row| {
        Ok((
            row.try_get::<String, _>("constraint_name_text")?,
            row.try_get::<String, _>("table_name_text")?,
            row.try_get::<String, _>("referenced_table_text")?,
            row.try_get::<String, _>("delete_rule_text")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    assert_eq!(
        foreign_keys,
        [
            (
                "fk_domain_outbox_delivery_event".to_string(),
                "domain_outbox_deliveries".to_string(),
                "domain_outbox_events".to_string(),
                "CASCADE".to_string(),
            ),
            (
                "fk_domain_outbox_operation".to_string(),
                "domain_outbox_events".to_string(),
                "record_mutation_operations".to_string(),
                "RESTRICT".to_string(),
            ),
        ]
    );

    let operation_id = RecordOperationId::default();
    let event_id = RecordEventId::default();
    let tenant_id = TenantId::default();
    let aggregate_database_id = DatabaseId::default();
    let data_id = DataId::default();
    sqlx::query(
        r#"
        INSERT INTO record_mutation_operations (
            operation_id, tenant_id, database_id, data_id,
            mutation_kind, actor_kind, actor_id, expected_version,
            fingerprint_version, request_fingerprint
        )
        VALUES (?, ?, ?, ?, 'PATCH', 'SYSTEM', 'system', 1, 1, ?)
        "#,
    )
    .bind(operation_id.to_string())
    .bind(tenant_id.to_string())
    .bind(aggregate_database_id.to_string())
    .bind(data_id.to_string())
    .bind(vec![0_u8; 32])
    .execute(pool.as_ref())
    .await?;

    let missing_decision_version = sqlx::query(
        r#"
        UPDATE record_mutation_operations
        SET decision_kind = 'REJECTED', decision_version = NULL,
            decision_payload = JSON_OBJECT('kind', 'REJECTED'),
            decided_at = CURRENT_TIMESTAMP(6)
        WHERE operation_id = ?
        "#,
    )
    .bind(operation_id.to_string())
    .execute(pool.as_ref())
    .await
    .expect_err("a final decision must have a non-null version");
    assert_check_violation(
        missing_decision_version,
        "chk_record_operations_decision_payload",
    );

    sqlx::query(
        r#"
        UPDATE record_mutation_operations
        SET decision_kind = 'REJECTED', decision_version = 1,
            decision_payload = JSON_OBJECT('kind', 'REJECTED'),
            decided_at = CURRENT_TIMESTAMP(6)
        WHERE operation_id = ?
        "#,
    )
    .bind(operation_id.to_string())
    .execute(pool.as_ref())
    .await?;

    let zero_sequence = sqlx::query(
        r#"
        INSERT INTO domain_outbox_events (
            event_id, operation_id, event_sequence, tenant_id,
            database_id, aggregate_type, aggregate_id,
            aggregate_version, event_type, payload, occurred_at
        )
        VALUES (
            ?, ?, 0, ?,
            ?, 'RECORD', ?, 1,
            'database.record.patched.v1', JSON_OBJECT(),
            CURRENT_TIMESTAMP(6)
        )
        "#,
    )
    .bind(event_id.to_string())
    .bind(operation_id.to_string())
    .bind(tenant_id.to_string())
    .bind(aggregate_database_id.to_string())
    .bind(data_id.to_string())
    .execute(pool.as_ref())
    .await
    .expect_err("outbox event sequence is one-origin");
    assert_check_violation(
        zero_sequence,
        "chk_domain_outbox_event_sequence",
    );

    Ok(())
}
