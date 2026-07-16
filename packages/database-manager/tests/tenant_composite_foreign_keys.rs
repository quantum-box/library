use database_manager::domain::{
    DataId, DatabaseId, PropertyId, RelationId,
};
use sqlx::{MySqlPool, Row};
use value_object::{DatabaseUrl, TenantId};

fn assert_fk_violation(error: sqlx::Error, constraints: &[&str]) {
    let database_error = error
        .as_database_error()
        .expect("the database must reject the insert");
    assert!(database_error.is_foreign_key_violation());
    assert!(
        constraints.iter().any(|constraint| {
            database_error.message().contains(constraint)
        }),
        "expected one of {constraints:?}, got: {}",
        database_error.message()
    );
}

async fn assert_constraint_columns(
    pool: &MySqlPool,
    table: &str,
    constraint: &str,
    expected: &[&str],
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT COLUMN_NAME
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
    let actual = rows
        .iter()
        .map(|row| row.try_get::<String, _>("COLUMN_NAME"))
        .collect::<Result<Vec<_>, _>>()?;
    let expected = expected
        .iter()
        .map(|column| (*column).to_string())
        .collect::<Vec<_>>();

    assert_eq!(actual, expected, "column order for {constraint}");
    Ok(())
}

async fn assert_foreign_key_columns(
    pool: &MySqlPool,
    table: &str,
    constraint: &str,
    expected: &[(&str, &str)],
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT COLUMN_NAME, REFERENCED_COLUMN_NAME
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
    let actual = rows
        .iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("COLUMN_NAME")?,
                row.try_get::<String, _>("REFERENCED_COLUMN_NAME")?,
            ))
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    let expected = expected
        .iter()
        .map(|(column, referenced_column)| {
            ((*column).to_string(), (*referenced_column).to_string())
        })
        .collect::<Vec<_>>();

    assert_eq!(actual, expected, "column order for {constraint}");
    Ok(())
}

async fn assert_constraint_absent(
    pool: &MySqlPool,
    table: &str,
    constraint: &str,
) -> anyhow::Result<()> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM information_schema.TABLE_CONSTRAINTS
        WHERE CONSTRAINT_SCHEMA = DATABASE()
          AND TABLE_NAME = ?
          AND CONSTRAINT_NAME = ?
        "#,
    )
    .bind(table)
    .bind(constraint)
    .fetch_one(pool)
    .await?;

    assert_eq!(count, 0, "legacy constraint {constraint} must be absent");
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn tenant_composite_foreign_keys_enforce_physical_scope(
) -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager");
    let db = persistence::Db::new(&dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let pool = db.pool();

    let foreign_key_checks = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(@@SESSION.foreign_key_checks AS SIGNED)",
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(foreign_key_checks, 1);

    assert_constraint_columns(
        pool.as_ref(),
        "objects",
        "uq_objects_tenant_id_id",
        &["tenant_id", "id"],
    )
    .await?;
    assert_constraint_columns(
        pool.as_ref(),
        "fields",
        "uq_fields_tenant_object_id_id",
        &["tenant_id", "object_id", "id"],
    )
    .await?;
    assert_constraint_columns(
        pool.as_ref(),
        "data",
        "uq_data_tenant_id_id",
        &["tenant_id", "id"],
    )
    .await?;

    let object_columns = &[("tenant_id", "tenant_id"), ("object_id", "id")];
    assert_foreign_key_columns(
        pool.as_ref(),
        "fields",
        "fk_fields_tenant_object",
        object_columns,
    )
    .await?;
    assert_foreign_key_columns(
        pool.as_ref(),
        "data",
        "fk_data_tenant_object",
        object_columns,
    )
    .await?;
    assert_foreign_key_columns(
        pool.as_ref(),
        "indexes",
        "fk_indexes_tenant_data",
        &[("tenant_id", "tenant_id"), ("object_id", "id")],
    )
    .await?;
    assert_foreign_key_columns(
        pool.as_ref(),
        "relationships",
        "fk_relationships_tenant_object",
        object_columns,
    )
    .await?;
    assert_foreign_key_columns(
        pool.as_ref(),
        "relationships",
        "fk_relationships_tenant_target_object",
        &[("tenant_id", "tenant_id"), ("target_object_id", "id")],
    )
    .await?;
    assert_foreign_key_columns(
        pool.as_ref(),
        "relationships",
        "fk_relationships_tenant_source_property_restrict",
        &[
            ("tenant_id", "tenant_id"),
            ("object_id", "object_id"),
            ("field_id", "id"),
        ],
    )
    .await?;
    assert_foreign_key_columns(
        pool.as_ref(),
        "relationships",
        "fk_relationships_tenant_target_inverse_field",
        &[
            ("tenant_id", "tenant_id"),
            ("target_object_id", "object_id"),
            ("inverse_field_id", "id"),
        ],
    )
    .await?;
    assert_constraint_columns(
        pool.as_ref(),
        "relationships",
        "uq_relationships_tenant_source_field",
        &["tenant_id", "object_id", "field_id"],
    )
    .await?;

    for (table, constraint) in [
        ("fields", "fk_fields_objects"),
        ("data", "fk_data_objects"),
        ("indexes", "fk_indexes_data"),
        ("relationships", "fk_relationships_object_id"),
        ("relationships", "fk_relationships_target_object_id"),
        ("relationships", "fk_relationships_field_id"),
    ] {
        assert_constraint_absent(pool.as_ref(), table, constraint).await?;
    }

    let tenant_a = TenantId::default().to_string();
    let tenant_b = TenantId::default().to_string();
    let object_a = DatabaseId::default().to_string();
    let object_a_other = DatabaseId::default().to_string();
    let object_b = DatabaseId::default().to_string();
    let field_a = PropertyId::default().to_string();
    let field_a_cross_target = PropertyId::default().to_string();
    let field_a_other = PropertyId::default().to_string();
    let field_b = PropertyId::default().to_string();
    let data_a = DataId::default().to_string();
    let mut transaction = pool.begin().await?;

    for (id, tenant_id, name) in [
        (&object_a, &tenant_a, "tenant-a"),
        (&object_a_other, &tenant_a, "tenant-a-other"),
        (&object_b, &tenant_b, "tenant-b"),
    ] {
        sqlx::query(
            "INSERT INTO objects (id, tenant_id, object_name) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(tenant_id)
        .bind(name)
        .execute(&mut *transaction)
        .await?;
    }

    for (id, tenant_id, object_id, field_num) in [
        (&field_a, &tenant_a, &object_a, 0_u32),
        (&field_a_cross_target, &tenant_a, &object_a, 1_u32),
        (&field_a_other, &tenant_a, &object_a_other, 0_u32),
        (&field_b, &tenant_b, &object_b, 0_u32),
    ] {
        sqlx::query(
            r#"
            INSERT INTO fields (
                id, tenant_id, object_id, field_name, datatype,
                is_indexed, field_num
            )
            VALUES (?, ?, ?, 'field', 'string', FALSE, ?)
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(object_id)
        .bind(field_num)
        .execute(&mut *transaction)
        .await?;
    }

    sqlx::query(
        "INSERT INTO data (id, tenant_id, object_id, name) VALUES (?, ?, ?, ?)",
    )
    .bind(&data_a)
    .bind(&tenant_a)
    .bind(&object_a)
    .bind("tenant-a-data")
    .execute(&mut *transaction)
    .await?;

    let cross_field_error = sqlx::query(
        r#"
        INSERT INTO fields (
            id, tenant_id, object_id, field_name, datatype,
            is_indexed, field_num
        )
        VALUES (?, ?, ?, 'cross-tenant', 'string', FALSE, 1)
        "#,
    )
    .bind(PropertyId::default().to_string())
    .bind(&tenant_b)
    .bind(&object_a)
    .execute(&mut *transaction)
    .await
    .expect_err("a field must not reference another tenant's object");
    assert_fk_violation(cross_field_error, &["fk_fields_tenant_object"]);

    let cross_data_error = sqlx::query(
        "INSERT INTO data (id, tenant_id, object_id, name) VALUES (?, ?, ?, ?)",
    )
    .bind(DataId::default().to_string())
    .bind(&tenant_b)
    .bind(&object_a)
    .bind("cross-tenant")
    .execute(&mut *transaction)
    .await
    .expect_err("data must not reference another tenant's object");
    assert_fk_violation(cross_data_error, &["fk_data_tenant_object"]);

    let max_index_id =
        sqlx::query_scalar::<_, Option<u32>>("SELECT MAX(id) FROM indexes")
            .fetch_one(&mut *transaction)
            .await?;
    let next_index_id = max_index_id
        .unwrap_or(0)
        .checked_add(1)
        .expect("the test fixture needs an unused indexes.id");
    let cross_index_error = sqlx::query(
        r#"
        INSERT INTO indexes (id, tenant_id, object_id, field_num)
        VALUES (?, ?, ?, 0)
        "#,
    )
    .bind(next_index_id)
    .bind(&tenant_b)
    .bind(&data_a)
    .execute(&mut *transaction)
    .await
    .expect_err("an index row must not reference another tenant's data");
    assert_fk_violation(cross_index_error, &["fk_indexes_tenant_data"]);

    let valid_relation = RelationId::default().to_string();
    sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id
        )
        VALUES (?, ?, ?, ?, 0, ?)
        "#,
    )
    .bind(&valid_relation)
    .bind(&tenant_a)
    .bind(&object_a)
    .bind(&field_a_cross_target)
    .bind(&object_a)
    .execute(&mut *transaction)
    .await?;

    let cross_target_error = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id
        )
        VALUES (?, ?, ?, ?, 0, ?)
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&object_a)
    .bind(&field_a)
    .bind(&object_b)
    .execute(&mut *transaction)
    .await
    .expect_err("a relation target must stay in the tenant");
    assert_fk_violation(
        cross_target_error,
        &["fk_relationships_tenant_target_object"],
    );

    let wrong_field_error = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id
        )
        VALUES (?, ?, ?, ?, 0, ?)
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&object_a)
    .bind(&field_a_other)
    .bind(&object_a)
    .execute(&mut *transaction)
    .await
    .expect_err("a relation field must belong to its source object");
    assert_fk_violation(
        wrong_field_error,
        &["fk_relationships_tenant_source_property_restrict"],
    );

    let cross_source_error = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id
        )
        VALUES (?, ?, ?, ?, 0, ?)
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_b)
    .bind(&object_a)
    .bind(&field_b)
    .bind(&object_b)
    .execute(&mut *transaction)
    .await
    .expect_err("a relation source must stay in the tenant");
    assert_fk_violation(
        cross_source_error,
        &[
            "fk_relationships_tenant_object",
            "fk_relationships_tenant_source_property_restrict",
        ],
    );

    transaction.rollback().await?;
    Ok(())
}
