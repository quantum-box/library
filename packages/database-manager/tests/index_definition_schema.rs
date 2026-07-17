use database_manager::domain::{
    DataId, DatabaseId, IndexDefinitionId, PropertyId, RelationId,
};
use sqlx::{MySql, MySqlPool, Row, Transaction};
use value_object::{DatabaseUrl, TenantId};

async fn key_columns(
    pool: &MySqlPool,
    table: &str,
    key: &str,
) -> anyhow::Result<Vec<(String, i64)>> {
    Ok(sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text,
               CAST(NON_UNIQUE AS SIGNED) AS non_unique
        FROM information_schema.STATISTICS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = ?
          AND INDEX_NAME = ?
        ORDER BY SEQ_IN_INDEX
        "#,
    )
    .bind(table)
    .bind(key)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        Ok((
            row.try_get::<String, _>("column_name_text")?,
            row.try_get::<i64, _>("non_unique")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?)
}

async fn foreign_key_columns(
    pool: &MySqlPool,
    constraint: &str,
) -> anyhow::Result<Vec<(String, String, String)>> {
    Ok(sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text,
               CAST(REFERENCED_TABLE_NAME AS CHAR) AS table_name_text,
               CAST(REFERENCED_COLUMN_NAME AS CHAR) AS referenced_column_text
        FROM information_schema.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'index_definitions'
          AND CONSTRAINT_NAME = ?
        ORDER BY ORDINAL_POSITION
        "#,
    )
    .bind(constraint)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| {
        Ok((
            row.try_get::<String, _>("column_name_text")?,
            row.try_get::<String, _>("table_name_text")?,
            row.try_get::<String, _>("referenced_column_text")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?)
}

async fn insert_object(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO objects (id, tenant_id, object_name) VALUES (?, ?, ?)",
    )
    .bind(database_id.to_string())
    .bind(tenant_id.to_string())
    .bind(name)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_field(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    property_id: &PropertyId,
    field_num: u32,
    datatype: &str,
    is_indexed: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO fields (
            id, tenant_id, object_id, field_name, datatype,
            datatype_meta, is_indexed, field_num
        )
        VALUES (?, ?, ?, 'index-contract-field', ?, NULL, ?, ?)
        "#,
    )
    .bind(property_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(datatype)
    .bind(is_indexed)
    .bind(field_num)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_definition(
    transaction: &mut Transaction<'_, MySql>,
    id: &IndexDefinitionId,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    property_id: Option<&PropertyId>,
    relation_id: Option<&RelationId>,
    policy: &str,
    unique: bool,
    definition_version: u16,
    generation: u64,
    projection_state: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO index_definitions (
            id, tenant_id, database_id, property_id, relation_id,
            policy, is_unique, definition_version, generation,
            projection_state
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(property_id.map(ToString::to_string))
    .bind(relation_id.map(ToString::to_string))
    .bind(policy)
    .bind(unique)
    .bind(definition_version)
    .bind(generation)
    .bind(projection_state)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn assert_check_violation(error: sqlx::Error, constraint: &str) {
    let database_error = error
        .as_database_error()
        .expect("the database must reject the invalid definition");
    assert!(database_error.is_check_violation());
    assert!(
        database_error.message().contains(constraint),
        "expected {constraint}, got: {}",
        database_error.message()
    );
}

fn assert_foreign_key_violation(error: sqlx::Error, constraint: &str) {
    let database_error = error
        .as_database_error()
        .expect("the database must reject the out-of-scope target");
    assert!(database_error.is_foreign_key_violation());
    assert!(
        database_error.message().contains(constraint),
        "expected {constraint}, got: {}",
        database_error.message()
    );
}

#[test]
fn index_definition_migration_resumes_after_partial_tidb_ddl() {
    let migration = include_str!(
        "../migrations/20260715160000_create_index_definitions.sql"
    );

    assert!(migration.contains("information_schema.STATISTICS"));
    assert!(migration
        .contains("INDEX_NAME = 'uq_relationships_tenant_object_id'"));
    assert!(
        migration.contains("CREATE TABLE IF NOT EXISTS index_definitions")
    );
    assert!(migration.contains("ON DELETE CASCADE"));
    assert!(!migration.contains("chk_index_definitions_exactly_one_target"));
    assert!(!migration.contains("chk_index_definitions_relation_policy"));
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn index_definition_schema_is_scoped_additive_and_strict(
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
    let migration = include_str!(
        "../migrations/20260715160000_create_index_definitions.sql"
    );
    assert!(
        !migration.contains("INSERT INTO index_definitions"),
        "the expand migration must never derive declarations from legacy rows"
    );

    let columns = sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text,
               CAST(COLUMN_TYPE AS CHAR) AS column_type_text,
               CAST(IS_NULLABLE AS CHAR) AS is_nullable_text,
               CAST(COLUMN_DEFAULT AS CHAR) AS column_default_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'index_definitions'
          AND COLUMN_NAME IN (
              'property_id', 'relation_id', 'policy', 'is_unique',
              'definition_version', 'generation', 'projection_state'
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
            row.try_get::<String, _>("is_nullable_text")?,
            row.try_get::<Option<String>, _>("column_default_text")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    assert_eq!(
        columns,
        [
            (
                "property_id".to_string(),
                "varchar(31)".to_string(),
                "YES".to_string(),
                None,
            ),
            (
                "relation_id".to_string(),
                "varchar(31)".to_string(),
                "YES".to_string(),
                None,
            ),
            (
                "policy".to_string(),
                "varchar(9)".to_string(),
                "NO".to_string(),
                Some("NONE".to_string()),
            ),
            (
                "is_unique".to_string(),
                "tinyint(1)".to_string(),
                "NO".to_string(),
                Some("0".to_string()),
            ),
            (
                "definition_version".to_string(),
                "smallint unsigned".to_string(),
                "NO".to_string(),
                Some("1".to_string()),
            ),
            (
                "generation".to_string(),
                "bigint unsigned".to_string(),
                "NO".to_string(),
                Some("1".to_string()),
            ),
            (
                "projection_state".to_string(),
                "varchar(8)".to_string(),
                "NO".to_string(),
                Some("DISABLED".to_string()),
            ),
        ]
    );

    let checks = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(CONSTRAINT_NAME AS CHAR)
        FROM information_schema.TABLE_CONSTRAINTS
        WHERE CONSTRAINT_SCHEMA = DATABASE()
          AND TABLE_NAME = 'index_definitions'
          AND CONSTRAINT_TYPE = 'CHECK'
        ORDER BY CONSTRAINT_NAME
        "#,
    )
    .fetch_all(pool.as_ref())
    .await?;
    assert_eq!(
        checks,
        [
            "chk_index_definitions_generation",
            "chk_index_definitions_policy",
            "chk_index_definitions_policy_projection",
            "chk_index_definitions_projection_state",
            "chk_index_definitions_unique_policy",
            "chk_index_definitions_version",
        ]
    );

    assert_eq!(
        key_columns(
            pool.as_ref(),
            "index_definitions",
            "uq_index_definitions_tenant_database_id",
        )
        .await?,
        [
            ("tenant_id".to_string(), 0),
            ("database_id".to_string(), 0),
            ("id".to_string(), 0),
        ]
    );
    assert_eq!(
        key_columns(
            pool.as_ref(),
            "index_definitions",
            "uq_index_definitions_property_target",
        )
        .await?,
        [
            ("tenant_id".to_string(), 0),
            ("database_id".to_string(), 0),
            ("property_id".to_string(), 0),
        ]
    );
    assert_eq!(
        key_columns(
            pool.as_ref(),
            "index_definitions",
            "uq_index_definitions_relation_target",
        )
        .await?,
        [
            ("tenant_id".to_string(), 0),
            ("database_id".to_string(), 0),
            ("relation_id".to_string(), 0),
        ]
    );
    assert_eq!(
        key_columns(
            pool.as_ref(),
            "index_definitions",
            "idx_index_definitions_tenant_database_state",
        )
        .await?,
        [
            ("tenant_id".to_string(), 1),
            ("database_id".to_string(), 1),
            ("projection_state".to_string(), 1),
            ("id".to_string(), 1),
        ]
    );
    assert_eq!(
        key_columns(
            pool.as_ref(),
            "relationships",
            "uq_relationships_tenant_object_id",
        )
        .await?,
        [
            ("tenant_id".to_string(), 0),
            ("object_id".to_string(), 0),
            ("id".to_string(), 0),
        ]
    );

    assert_eq!(
        foreign_key_columns(
            pool.as_ref(),
            "fk_index_definitions_tenant_database",
        )
        .await?,
        [
            (
                "tenant_id".to_string(),
                "objects".to_string(),
                "tenant_id".to_string(),
            ),
            (
                "database_id".to_string(),
                "objects".to_string(),
                "id".to_string(),
            ),
        ]
    );
    assert_eq!(
        foreign_key_columns(
            pool.as_ref(),
            "fk_index_definitions_property_target",
        )
        .await?,
        [
            (
                "tenant_id".to_string(),
                "fields".to_string(),
                "tenant_id".to_string(),
            ),
            (
                "database_id".to_string(),
                "fields".to_string(),
                "object_id".to_string(),
            ),
            (
                "property_id".to_string(),
                "fields".to_string(),
                "id".to_string(),
            ),
        ]
    );
    assert_eq!(
        foreign_key_columns(
            pool.as_ref(),
            "fk_index_definitions_relation_target",
        )
        .await?,
        [
            (
                "tenant_id".to_string(),
                "relationships".to_string(),
                "tenant_id".to_string(),
            ),
            (
                "database_id".to_string(),
                "relationships".to_string(),
                "object_id".to_string(),
            ),
            (
                "relation_id".to_string(),
                "relationships".to_string(),
                "id".to_string(),
            ),
        ]
    );
    for constraint in [
        "fk_index_definitions_tenant_database",
        "fk_index_definitions_property_target",
        "fk_index_definitions_relation_target",
    ] {
        let delete_rule = sqlx::query_scalar::<_, String>(
            r#"
            SELECT CAST(DELETE_RULE AS CHAR)
            FROM information_schema.REFERENTIAL_CONSTRAINTS
            WHERE CONSTRAINT_SCHEMA = DATABASE()
              AND TABLE_NAME = 'index_definitions'
              AND CONSTRAINT_NAME = ?
            "#,
        )
        .bind(constraint)
        .fetch_one(pool.as_ref())
        .await?;
        assert_eq!(delete_rule, "CASCADE", "{constraint}");
    }

    let tenant_a = TenantId::default();
    let tenant_b = TenantId::default();
    let database_a = DatabaseId::default();
    let target_a = DatabaseId::default();
    let database_b = DatabaseId::default();
    let property_a = PropertyId::default();
    let relation_property = PropertyId::default();
    let property_b = PropertyId::default();
    let relation_a = RelationId::default();
    let data_a = DataId::default();
    let mut transaction = pool.begin().await?;

    insert_object(&mut transaction, &tenant_a, &database_a, "source-a")
        .await?;
    insert_object(&mut transaction, &tenant_a, &target_a, "target-a")
        .await?;
    insert_object(&mut transaction, &tenant_b, &database_b, "source-b")
        .await?;
    insert_field(
        &mut transaction,
        &tenant_a,
        &database_a,
        &property_a,
        0,
        "STRING",
        true,
    )
    .await?;
    insert_field(
        &mut transaction,
        &tenant_a,
        &database_a,
        &relation_property,
        1,
        "RELATION",
        false,
    )
    .await?;
    insert_field(
        &mut transaction,
        &tenant_b,
        &database_b,
        &property_b,
        0,
        "STRING",
        false,
    )
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
    .bind(relation_a.to_string())
    .bind(tenant_a.to_string())
    .bind(database_a.to_string())
    .bind(relation_property.to_string())
    .bind(target_a.to_string())
    .execute(&mut *transaction)
    .await?;

    // Legacy declarations remain data, not migration input. Creating both
    // legacy signals must not synthesize an IndexDefinition.
    sqlx::query(
        "INSERT INTO data (id, tenant_id, object_id, name) VALUES (?, ?, ?, 'legacy-index-record')",
    )
    .bind(data_a.to_string())
    .bind(tenant_a.to_string())
    .bind(database_a.to_string())
    .execute(&mut *transaction)
    .await?;
    let legacy_index_id = sqlx::query_scalar::<_, u64>(
        "SELECT CAST(COALESCE(MAX(id), 0) + 1 AS UNSIGNED) FROM indexes",
    )
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO indexes (id, tenant_id, object_id, field_num) VALUES (?, ?, ?, 0)",
    )
    .bind(legacy_index_id)
    .bind(tenant_a.to_string())
    .bind(data_a.to_string())
    .execute(&mut *transaction)
    .await?;
    let legacy_field_flag = sqlx::query_scalar::<_, bool>(
        "SELECT is_indexed FROM fields WHERE id = ?",
    )
    .bind(property_a.to_string())
    .fetch_one(&mut *transaction)
    .await?;
    assert!(legacy_field_flag);
    let legacy_index_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM indexes WHERE tenant_id = ?",
    )
    .bind(tenant_a.to_string())
    .fetch_one(&mut *transaction)
    .await?;
    assert_eq!(legacy_index_count, 1);
    let auto_definition_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM index_definitions WHERE tenant_id = ?",
    )
    .bind(tenant_a.to_string())
    .fetch_one(&mut *transaction)
    .await?;
    assert_eq!(
        auto_definition_count, 0,
        "legacy is_indexed/indexes state must not be auto-backfilled"
    );

    for (
        property_id,
        relation_id,
        policy,
        unique,
        version,
        generation,
        state,
        constraint,
    ) in [
        (
            None,
            Some(&relation_a),
            "MAGIC",
            false,
            1,
            1,
            "PENDING",
            "chk_index_definitions_policy",
        ),
        (
            None,
            Some(&relation_a),
            "exact",
            false,
            1,
            1,
            "PENDING",
            "chk_index_definitions_policy",
        ),
        (
            Some(&property_a),
            None,
            "FULL_TEXT",
            true,
            1,
            1,
            "PENDING",
            "chk_index_definitions_unique_policy",
        ),
        (
            None,
            Some(&relation_a),
            "NONE",
            false,
            1,
            1,
            "PENDING",
            "chk_index_definitions_policy_projection",
        ),
        (
            None,
            Some(&relation_a),
            "EXACT",
            false,
            1,
            1,
            "DISABLED",
            "chk_index_definitions_policy_projection",
        ),
        (
            None,
            Some(&relation_a),
            "EXACT",
            false,
            0,
            1,
            "PENDING",
            "chk_index_definitions_version",
        ),
        (
            None,
            Some(&relation_a),
            "EXACT",
            false,
            1,
            0,
            "PENDING",
            "chk_index_definitions_generation",
        ),
        (
            None,
            Some(&relation_a),
            "EXACT",
            false,
            1,
            1,
            "MAGIC",
            "chk_index_definitions_projection_state",
        ),
        (
            None,
            Some(&relation_a),
            "EXACT",
            false,
            1,
            1,
            "pending",
            "chk_index_definitions_projection_state",
        ),
    ] {
        let error = insert_definition(
            &mut transaction,
            &IndexDefinitionId::default(),
            &tenant_a,
            &database_a,
            property_id,
            relation_id,
            policy,
            unique,
            version,
            generation,
            state,
        )
        .await
        .expect_err("the invalid definition must be rejected");
        assert_check_violation(error, constraint);
    }

    let foreign_target_error = insert_definition(
        &mut transaction,
        &IndexDefinitionId::default(),
        &tenant_a,
        &database_a,
        Some(&property_b),
        None,
        "EXACT",
        false,
        1,
        1,
        "PENDING",
    )
    .await
    .expect_err("a Property target must belong to the declared scope");
    assert_foreign_key_violation(
        foreign_target_error,
        "fk_index_definitions_property_target",
    );

    let property_definition = IndexDefinitionId::default();
    insert_definition(
        &mut transaction,
        &property_definition,
        &tenant_a,
        &database_a,
        Some(&property_a),
        None,
        "EXACT",
        true,
        1,
        1,
        "PENDING",
    )
    .await?;
    let relation_definition = IndexDefinitionId::default();
    insert_definition(
        &mut transaction,
        &relation_definition,
        &tenant_a,
        &database_a,
        None,
        Some(&relation_a),
        "EXACT",
        false,
        1,
        1,
        "PENDING",
    )
    .await?;

    let duplicate_error = insert_definition(
        &mut transaction,
        &IndexDefinitionId::default(),
        &tenant_a,
        &database_a,
        Some(&property_a),
        None,
        "EXACT",
        false,
        1,
        1,
        "PENDING",
    )
    .await
    .expect_err("one Property target may have only one definition");
    let duplicate_database_error = duplicate_error
        .as_database_error()
        .expect("duplicate target must be rejected by MySQL");
    assert!(duplicate_database_error.is_unique_violation());
    assert!(duplicate_database_error
        .message()
        .contains("uq_index_definitions_property_target"));

    sqlx::query("DELETE FROM relationships WHERE id = ?")
        .bind(relation_a.to_string())
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM fields WHERE id = ?")
        .bind(property_a.to_string())
        .execute(&mut *transaction)
        .await?;
    let cascaded_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED)
        FROM index_definitions
        WHERE id IN (?, ?)
        "#,
    )
    .bind(property_definition.to_string())
    .bind(relation_definition.to_string())
    .fetch_one(&mut *transaction)
    .await?;
    assert_eq!(cascaded_count, 0);
    let preserved_legacy_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM indexes WHERE tenant_id = ?",
    )
    .bind(tenant_a.to_string())
    .fetch_one(&mut *transaction)
    .await?;
    assert_eq!(preserved_legacy_count, 1);

    transaction.rollback().await?;
    Ok(())
}
