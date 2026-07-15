use database_manager::domain::{DataId, DatabaseId, PropertyId};
use sqlx::{MySqlPool, Row};
use value_object::{DatabaseUrl, TenantId};

async fn constraint_columns(
    pool: &MySqlPool,
    table: &str,
    constraint: &str,
) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text
        FROM information_schema.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = ?
          AND CONSTRAINT_NAME = ?
        ORDER BY ORDINAL_POSITION
        "#,
    )
    .bind(table)
    .bind(constraint)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| row.try_get("column_name_text"))
        .collect::<Result<Vec<_>, _>>()?)
}

async fn foreign_key_columns(
    pool: &MySqlPool,
    constraint: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let rows = sqlx::query(
        r#"
        SELECT
            CAST(COLUMN_NAME AS CHAR) AS column_name_text,
            CAST(REFERENCED_COLUMN_NAME AS CHAR) AS referenced_column_name_text
        FROM information_schema.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'property_values'
          AND CONSTRAINT_NAME = ?
        ORDER BY ORDINAL_POSITION
        "#,
    )
    .bind(constraint)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            Ok((
                row.try_get("column_name_text")?,
                row.try_get("referenced_column_name_text")?,
            ))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?)
}

async fn index_columns(
    pool: &MySqlPool,
    table: &str,
    index: &str,
) -> anyhow::Result<Vec<String>> {
    let rows = sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text
        FROM information_schema.STATISTICS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = ?
          AND INDEX_NAME = ?
        ORDER BY SEQ_IN_INDEX
        "#,
    )
    .bind(table)
    .bind(index)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| row.try_get("column_name_text"))
        .collect::<Result<Vec<_>, _>>()?)
}

fn assert_fk_violation(error: sqlx::Error, expected: &str) {
    let database_error = error
        .as_database_error()
        .expect("the database must reject the insert");
    assert!(database_error.is_foreign_key_violation());
    assert!(
        database_error.message().contains(expected),
        "expected {expected}, got: {}",
        database_error.message()
    );
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn normalized_property_value_schema_is_scoped_and_expand_only(
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

    let widened_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED)
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'data'
          AND COLUMN_NAME REGEXP '^value(2[6-9]|[3-4][0-9]|50)$'
          AND DATA_TYPE = 'longtext'
        "#,
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(widened_count, 25);

    assert_eq!(
        constraint_columns(
            pool.as_ref(),
            "data",
            "uq_data_tenant_object_id_id"
        )
        .await?,
        ["tenant_id", "object_id", "id"]
    );
    assert_eq!(
        constraint_columns(
            pool.as_ref(),
            "fields",
            "uq_fields_tenant_object_field_num"
        )
        .await?,
        ["tenant_id", "object_id", "field_num"]
    );
    assert_eq!(
        constraint_columns(
            pool.as_ref(),
            "fields",
            "uq_fields_tenant_object_id_singleton"
        )
        .await?,
        ["tenant_id", "object_id", "id_singleton_marker"]
    );
    assert_eq!(
        constraint_columns(pool.as_ref(), "property_values", "PRIMARY")
            .await?,
        ["tenant_id", "database_id", "data_id", "property_id"]
    );
    assert_eq!(
        index_columns(
            pool.as_ref(),
            "property_values",
            "idx_property_values_property_scan"
        )
        .await?,
        ["tenant_id", "database_id", "property_id", "data_id"]
    );
    assert_eq!(
        foreign_key_columns(
            pool.as_ref(),
            "fk_property_values_tenant_database_data"
        )
        .await?,
        [
            ("tenant_id".to_string(), "tenant_id".to_string()),
            ("database_id".to_string(), "object_id".to_string()),
            ("data_id".to_string(), "id".to_string()),
        ]
    );
    assert_eq!(
        foreign_key_columns(
            pool.as_ref(),
            "fk_property_values_tenant_database_property"
        )
        .await?,
        [
            ("tenant_id".to_string(), "tenant_id".to_string()),
            ("database_id".to_string(), "object_id".to_string()),
            ("property_id".to_string(), "id".to_string()),
        ]
    );

    let property_value_columns = sqlx::query(
        r#"
        SELECT
            CAST(COLUMN_NAME AS CHAR) AS column_name_text,
            CAST(COLUMN_TYPE AS CHAR) AS column_type_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'property_values'
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
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    assert_eq!(
        property_value_columns,
        [
            ("tenant_id".to_string(), "varchar(29)".to_string()),
            ("database_id".to_string(), "varchar(29)".to_string()),
            ("data_id".to_string(), "varchar(31)".to_string()),
            ("property_id".to_string(), "varchar(31)".to_string()),
            ("type_key".to_string(), "varchar(64)".to_string()),
            ("type_version".to_string(), "smallint unsigned".to_string(),),
            (
                "value_encoding_version".to_string(),
                "smallint unsigned".to_string(),
            ),
            ("value".to_string(), "longtext".to_string()),
            ("created_at".to_string(), "timestamp(6)".to_string()),
            ("updated_at".to_string(), "timestamp(6)".to_string()),
        ]
    );

    let value_column = sqlx::query(
        r#"
        SELECT
            CAST(DATA_TYPE AS CHAR) AS data_type_text,
            CAST(IS_NULLABLE AS CHAR) AS is_nullable_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'property_values'
          AND COLUMN_NAME = 'value'
        "#,
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        value_column.try_get::<String, _>("data_type_text")?,
        "longtext"
    );
    assert_eq!(
        value_column.try_get::<String, _>("is_nullable_text")?,
        "NO"
    );

    let marker = sqlx::query(
        r#"
        SELECT
            CAST(EXTRA AS CHAR) AS extra_text,
            CAST(GENERATION_EXPRESSION AS CHAR) AS generation_expression_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'fields'
          AND COLUMN_NAME = 'id_singleton_marker'
        "#,
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert!(marker
        .try_get::<String, _>("extra_text")?
        .to_ascii_uppercase()
        .contains("STORED GENERATED"));
    assert!(marker
        .try_get::<String, _>("generation_expression_text")?
        .to_ascii_uppercase()
        .contains("DATATYPE"));

    let tenant_id = TenantId::default().to_string();
    let database_a = DatabaseId::default().to_string();
    let database_b = DatabaseId::default().to_string();
    let property_a = PropertyId::default().to_string();
    let property_b = PropertyId::default().to_string();
    let data_a = DataId::default().to_string();
    let mut transaction = pool.begin().await?;

    for (database_id, name) in
        [(&database_a, "database-a"), (&database_b, "database-b")]
    {
        sqlx::query(
            "INSERT INTO objects (id, tenant_id, object_name) VALUES (?, ?, ?)",
        )
        .bind(database_id)
        .bind(&tenant_id)
        .bind(name)
        .execute(&mut *transaction)
        .await?;
    }
    for (property_id, database_id) in
        [(&property_a, &database_a), (&property_b, &database_b)]
    {
        sqlx::query(
            r#"
            INSERT INTO fields (
                id, tenant_id, object_id, field_name, datatype,
                is_indexed, field_num
            )
            VALUES (?, ?, ?, 'field', 'STRING', FALSE, 0)
            "#,
        )
        .bind(property_id)
        .bind(&tenant_id)
        .bind(database_id)
        .execute(&mut *transaction)
        .await?;
    }
    sqlx::query(
        "INSERT INTO data (id, tenant_id, object_id, name) VALUES (?, ?, ?, 'data')",
    )
    .bind(&data_a)
    .bind(&tenant_id)
    .bind(&database_a)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO property_values (
            tenant_id, database_id, data_id, property_id,
            type_key, value
        )
        VALUES (?, ?, ?, ?, 'string', ?)
        "#,
    )
    .bind(&tenant_id)
    .bind(&database_a)
    .bind(&data_a)
    .bind(&property_a)
    .bind("canonical value")
    .execute(&mut *transaction)
    .await?;

    let wrong_data_scope = sqlx::query(
        r#"
        INSERT INTO property_values (
            tenant_id, database_id, data_id, property_id,
            type_key, value
        )
        VALUES (?, ?, ?, ?, 'string', 'wrong data scope')
        "#,
    )
    .bind(&tenant_id)
    .bind(&database_b)
    .bind(&data_a)
    .bind(&property_b)
    .execute(&mut *transaction)
    .await
    .expect_err("data scope must include its database");
    assert_fk_violation(
        wrong_data_scope,
        "fk_property_values_tenant_database_data",
    );

    let wrong_property_scope = sqlx::query(
        r#"
        INSERT INTO property_values (
            tenant_id, database_id, data_id, property_id,
            type_key, value
        )
        VALUES (?, ?, ?, ?, 'string', 'wrong property scope')
        "#,
    )
    .bind(&tenant_id)
    .bind(&database_a)
    .bind(&data_a)
    .bind(&property_b)
    .execute(&mut *transaction)
    .await
    .expect_err("property scope must include its database");
    assert_fk_violation(
        wrong_property_scope,
        "fk_property_values_tenant_database_property",
    );

    let stored_values = sqlx::query(
        r#"
        SELECT type_key, type_version, value_encoding_version, value
        FROM property_values
        WHERE tenant_id = ? AND database_id = ? AND data_id = ?
        "#,
    )
    .bind(&tenant_id)
    .bind(&database_a)
    .bind(&data_a)
    .fetch_all(&mut *transaction)
    .await?;
    assert_eq!(
        stored_values.len(),
        1,
        "failed cross-scope inserts must not leave rows"
    );
    let stored_value = &stored_values[0];
    assert_eq!(stored_value.try_get::<String, _>("type_key")?, "string");
    assert_eq!(stored_value.try_get::<u16, _>("type_version")?, 1);
    assert_eq!(
        stored_value.try_get::<u16, _>("value_encoding_version")?,
        1
    );
    assert_eq!(
        stored_value.try_get::<String, _>("value")?,
        "canonical value"
    );

    sqlx::query("DELETE FROM data WHERE id = ?")
        .bind(&data_a)
        .execute(&mut *transaction)
        .await?;
    let remaining_values = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM property_values WHERE data_id = ?",
    )
    .bind(&data_a)
    .fetch_one(&mut *transaction)
    .await?;
    assert_eq!(remaining_values, 0, "record deletion must cascade");

    // Prove the generated nullable marker, not just its metadata, protects the
    // Id singleton when a caller bypasses the application lock.
    let id_property = PropertyId::default().to_string();
    sqlx::query(
        r#"
        INSERT INTO fields (
            id, tenant_id, object_id, field_name, datatype,
            is_indexed, field_num
        )
        VALUES (?, ?, ?, 'id', 'ID', FALSE, 1)
        "#,
    )
    .bind(&id_property)
    .bind(&tenant_id)
    .bind(&database_a)
    .execute(&mut *transaction)
    .await?;
    let duplicate_id = sqlx::query(
        r#"
        INSERT INTO fields (
            id, tenant_id, object_id, field_name, datatype,
            is_indexed, field_num
        )
        VALUES (?, ?, ?, 'second-id', 'ID', FALSE, 2)
        "#,
    )
    .bind(PropertyId::default().to_string())
    .bind(&tenant_id)
    .bind(&database_a)
    .execute(&mut *transaction)
    .await
    .expect_err("the database must reject a second Id property");
    let database_error =
        duplicate_id.as_database_error().expect("unique violation");
    assert!(database_error.is_unique_violation());
    assert!(database_error
        .message()
        .contains("uq_fields_tenant_object_id_singleton"));

    transaction.rollback().await?;
    Ok(())
}
