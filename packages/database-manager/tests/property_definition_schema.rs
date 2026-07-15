use database_manager::domain::{DatabaseId, PropertyId};
use sqlx::{MySqlPool, Row};
use value_object::{DatabaseUrl, TenantId};

async fn insert_property(
    pool: &MySqlPool,
    type_key: Option<&str>,
    type_version: Option<u16>,
    type_config: Option<&str>,
) -> Result<(), sqlx::Error> {
    let tenant_id = TenantId::default().to_string();
    let database_id = DatabaseId::default().to_string();

    sqlx::query(
        "INSERT INTO objects (id, tenant_id, object_name) VALUES (?, ?, 'database')",
    )
    .bind(&database_id)
    .bind(&tenant_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO fields (
            id, tenant_id, object_id, field_name, datatype,
            is_indexed, field_num, type_key, type_version, type_config
        )
        VALUES (?, ?, ?, 'field', 'STRING', FALSE, 0, ?, ?, ?)
        "#,
    )
    .bind(PropertyId::default().to_string())
    .bind(&tenant_id)
    .bind(&database_id)
    .bind(type_key)
    .bind(type_version)
    .bind(type_config)
    .execute(pool)
    .await?;

    Ok(())
}

fn assert_check_violation(error: sqlx::Error, expected: &str) {
    let database_error = error
        .as_database_error()
        .expect("the database must reject the insert");
    assert!(database_error.is_check_violation());
    assert!(
        database_error.message().contains(expected),
        "expected {expected}, got: {}",
        database_error.message()
    );
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn property_definition_envelope_schema_is_additive_and_strict(
) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager");
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let pool = db.pool();

    let columns = sqlx::query(
        r#"
        SELECT
            CAST(COLUMN_NAME AS CHAR) AS column_name_text,
            CAST(COLUMN_TYPE AS CHAR) AS column_type_text,
            CAST(IS_NULLABLE AS CHAR) AS is_nullable_text,
            CHARACTER_MAXIMUM_LENGTH
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'fields'
          AND COLUMN_NAME IN ('type_key', 'type_version', 'type_config')
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
            row.try_get::<String, _>("is_nullable_text")?,
            row.try_get::<Option<u64>, _>("CHARACTER_MAXIMUM_LENGTH")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    assert_eq!(
        columns,
        [
            (
                "type_key".to_string(),
                "varchar(64)".to_string(),
                "YES".to_string(),
                Some(64),
            ),
            (
                "type_version".to_string(),
                "smallint unsigned".to_string(),
                "YES".to_string(),
                None,
            ),
            (
                "type_config".to_string(),
                "longtext".to_string(),
                "YES".to_string(),
                Some(4_294_967_295),
            ),
        ]
    );

    let constraints = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(CONSTRAINT_NAME AS CHAR)
        FROM information_schema.TABLE_CONSTRAINTS
        WHERE CONSTRAINT_SCHEMA = DATABASE()
          AND TABLE_NAME = 'fields'
          AND CONSTRAINT_TYPE = 'CHECK'
          AND CONSTRAINT_NAME LIKE 'ck_fields_property_definition_%'
        ORDER BY CONSTRAINT_NAME
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(
        constraints,
        [
            "ck_fields_property_definition_envelope_complete",
            "ck_fields_property_definition_type_config",
            "ck_fields_property_definition_type_key",
            "ck_fields_property_definition_type_version",
        ]
    );

    insert_property(pool.as_ref(), None, None, None).await?;
    insert_property(
        pool.as_ref(),
        Some("multi_select"),
        Some(1),
        Some(r#"{"items":[]}"#),
    )
    .await?;
    insert_property(pool.as_ref(), Some("boolean"), Some(1), Some("null"))
        .await?;

    for (type_key, type_version, type_config) in [
        (Some("string"), None, None),
        (None, Some(1), None),
        (None, None, Some("null")),
        (Some("string"), Some(1), None),
        (Some("string"), None, Some("null")),
        (None, Some(1), Some("null")),
    ] {
        let error = insert_property(
            pool.as_ref(),
            type_key,
            type_version,
            type_config,
        )
        .await
        .expect_err("a partial PropertyDefinition envelope must fail");
        assert_check_violation(
            error,
            "ck_fields_property_definition_envelope_complete",
        );
    }

    let zero_version =
        insert_property(pool.as_ref(), Some("string"), Some(0), Some("null"))
            .await
            .expect_err("type version zero must fail");
    assert_check_violation(
        zero_version,
        "ck_fields_property_definition_type_version",
    );

    for invalid_key in [
        "String",
        "1_string",
        "_string",
        "string_",
        "multi__select",
        "multi-select",
    ] {
        let error = insert_property(
            pool.as_ref(),
            Some(invalid_key),
            Some(1),
            Some("null"),
        )
        .await
        .expect_err("a non-lower-snake type key must fail");
        assert_check_violation(
            error,
            "ck_fields_property_definition_type_key",
        );
    }

    let overlong_key = "a".repeat(65);
    let overlong_error = insert_property(
        pool.as_ref(),
        Some(overlong_key.as_str()),
        Some(1),
        Some("null"),
    )
    .await
    .expect_err("a type key longer than 64 bytes must fail");
    let database_error = overlong_error
        .as_database_error()
        .expect("the database must reject the insert");
    assert_eq!(database_error.code().as_deref(), Some("1406"));
    assert!(database_error.message().contains("type_key"));

    let invalid_json = insert_property(
        pool.as_ref(),
        Some("string"),
        Some(1),
        Some("{not-json}"),
    )
    .await
    .expect_err("invalid type config JSON must fail");
    assert_check_violation(
        invalid_json,
        "ck_fields_property_definition_type_config",
    );

    Ok(())
}
