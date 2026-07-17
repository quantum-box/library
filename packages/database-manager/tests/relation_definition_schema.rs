use database_manager::domain::{DatabaseId, PropertyId, RelationId};
use sqlx::{MySql, MySqlPool, Row, Transaction};
use value_object::{DatabaseUrl, TenantId};

async fn constraint_columns(
    pool: &MySqlPool,
    constraint: &str,
) -> anyhow::Result<Vec<String>> {
    Ok(sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text
        FROM information_schema.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND CONSTRAINT_NAME = ?
        ORDER BY ORDINAL_POSITION
        "#,
    )
    .bind(constraint)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.try_get("column_name_text"))
    .collect::<Result<Vec<_>, _>>()?)
}

async fn insert_object(
    transaction: &mut Transaction<'_, MySql>,
    id: &str,
    tenant_id: &str,
    name: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO objects (id, tenant_id, object_name) VALUES (?, ?, ?)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(name)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_relation_field(
    transaction: &mut Transaction<'_, MySql>,
    id: &str,
    tenant_id: &str,
    database_id: &str,
    field_num: u32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO fields (
            id, tenant_id, object_id, field_name, datatype,
            datatype_meta, is_indexed, field_num
        )
        VALUES (?, ?, ?, 'relation', 'RELATION', NULL, FALSE, ?)
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(database_id)
    .bind(field_num)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[test]
fn relation_definition_migrations_keep_each_tidb_check_change_isolated() {
    let expansion = include_str!(
        "../migrations/20260715140000_expand_relation_definitions.sql"
    );
    let expansion_checks = [
        include_str!(
            "../migrations/20260715140001_check_relation_forward_cardinality.sql"
        ),
        include_str!(
            "../migrations/20260715140002_check_relation_reverse_cardinality.sql"
        ),
        include_str!(
            "../migrations/20260715140003_check_relation_target_delete.sql"
        ),
    ];
    let versioning_migrations = [
        include_str!(
            "../migrations/20260716100000_version_relation_definitions.sql"
        ),
        include_str!(
            "../migrations/20260716100001_add_relation_definition_version.sql"
        ),
        include_str!(
            "../migrations/20260716100002_add_relation_generation.sql"
        ),
        include_str!(
            "../migrations/20260716100003_check_relation_forward_cardinality_strict.sql"
        ),
        include_str!(
            "../migrations/20260716100004_check_relation_reverse_cardinality_strict.sql"
        ),
        include_str!(
            "../migrations/20260716100005_check_relation_target_delete_strict.sql"
        ),
        include_str!(
            "../migrations/20260716100006_check_relation_owned_inverse.sql"
        ),
        include_str!(
            "../migrations/20260716100007_add_relation_source_restrict_foreign_key.sql"
        ),
        include_str!(
            "../migrations/20260716100008_check_relation_definition_version.sql"
        ),
        include_str!(
            "../migrations/20260716100009_check_relation_generation.sql"
        ),
        include_str!(
            "../migrations/20260716100010_drop_legacy_relation_source_foreign_key.sql"
        ),
        include_str!(
            "../migrations/20260716100011_document_relation_self_inverse_guard.sql"
        ),
        include_str!(
            "../migrations/20260716100012_drop_legacy_relation_forward_check.sql"
        ),
        include_str!(
            "../migrations/20260716100013_drop_legacy_relation_reverse_check.sql"
        ),
        include_str!(
            "../migrations/20260716100014_drop_legacy_relation_target_delete_check.sql"
        ),
    ];

    assert_eq!(
        expansion
            .matches("\nPREPARE library_relation_column_stmt")
            .count(),
        4
    );
    assert_eq!(
        expansion
            .matches("EXECUTE library_relation_column_stmt")
            .count(),
        4
    );
    assert!(!expansion.contains("ADD CONSTRAINT"));

    for migration in
        expansion_checks.iter().chain(versioning_migrations.iter())
    {
        assert!(
            migration.matches("ADD CONSTRAINT chk_").count() <= 1,
            "each TiDB migration may add at most one CHECK constraint"
        );
    }

    let versioning_plan = versioning_migrations.join("\n");
    for (new_guard, old_guard) in [
        (
            "ADD CONSTRAINT fk_relationships_tenant_source_property_restrict",
            "DROP FOREIGN KEY fk_relationships_tenant_source_property",
        ),
        (
            "ADD CONSTRAINT chk_relationships_forward_cardinality_strict",
            "DROP CHECK chk_relationships_forward_cardinality",
        ),
        (
            "ADD CONSTRAINT chk_relationships_reverse_cardinality_strict",
            "DROP CHECK chk_relationships_reverse_cardinality",
        ),
        (
            "ADD CONSTRAINT chk_relationships_on_target_delete_strict",
            "DROP CHECK chk_relationships_on_target_delete",
        ),
    ] {
        let add_position = versioning_plan
            .find(new_guard)
            .expect("the stronger guard must be installed");
        let drop_position = versioning_plan
            .find(old_guard)
            .expect("the legacy guard must be contracted");
        assert!(
            add_position < drop_position,
            "{new_guard} must be active before {old_guard}"
        );
    }

    assert!(!versioning_plan.contains(
        "ADD CONSTRAINT chk_relationships_distinct_self_inverse"
    ));
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn relation_definition_schema_enforces_control_plane_invariants(
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
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text,
               CAST(COLUMN_DEFAULT AS CHAR) AS column_default_text,
               CAST(IS_NULLABLE AS CHAR) AS is_nullable_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND COLUMN_NAME IN (
              'forward_cardinality', 'reverse_cardinality',
              'inverse_field_id', 'inverse_owned', 'on_target_delete',
              'definition_version', 'generation'
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
            row.try_get::<Option<String>, _>("column_default_text")?,
            row.try_get::<String, _>("is_nullable_text")?,
        ))
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()?;
    assert_eq!(
        columns,
        [
            (
                "forward_cardinality".to_string(),
                Some("MANY".to_string()),
                "NO".to_string(),
            ),
            (
                "reverse_cardinality".to_string(),
                Some("MANY".to_string()),
                "NO".to_string(),
            ),
            ("inverse_field_id".to_string(), None, "YES".to_string()),
            (
                "inverse_owned".to_string(),
                Some("0".to_string()),
                "NO".to_string(),
            ),
            (
                "on_target_delete".to_string(),
                Some("RESTRICT".to_string()),
                "NO".to_string(),
            ),
            (
                "definition_version".to_string(),
                Some("1".to_string()),
                "NO".to_string(),
            ),
            (
                "generation".to_string(),
                Some("1".to_string()),
                "NO".to_string(),
            ),
        ]
    );

    assert_eq!(
        constraint_columns(
            pool.as_ref(),
            "uq_relationships_tenant_source_field"
        )
        .await?,
        ["tenant_id", "object_id", "field_id"]
    );
    assert_eq!(
        constraint_columns(
            pool.as_ref(),
            "uq_relationships_tenant_inverse_field"
        )
        .await?,
        ["tenant_id", "target_object_id", "inverse_field_id"]
    );
    assert_eq!(
        constraint_columns(
            pool.as_ref(),
            "fk_relationships_tenant_target_inverse_field"
        )
        .await?,
        ["tenant_id", "target_object_id", "inverse_field_id"]
    );

    for (constraint, expected_fragments) in [
        (
            "chk_relationships_forward_cardinality_strict",
            &["forward_cardinality", "ONE", "MANY"][..],
        ),
        (
            "chk_relationships_reverse_cardinality_strict",
            &["reverse_cardinality", "ONE", "MANY"][..],
        ),
        (
            "chk_relationships_on_target_delete_strict",
            &["on_target_delete", "RESTRICT", "NULLIFY"][..],
        ),
        (
            "chk_relationships_owned_inverse",
            &["inverse_owned", "inverse_field_id"][..],
        ),
        (
            "chk_relationships_definition_version",
            &["definition_version", "0"][..],
        ),
        ("chk_relationships_generation", &["generation", "0"][..]),
    ] {
        let clause = sqlx::query_scalar::<_, String>(
            r#"
            SELECT CAST(CHECK_CLAUSE AS CHAR)
            FROM information_schema.CHECK_CONSTRAINTS
            WHERE CONSTRAINT_SCHEMA = DATABASE()
              AND CONSTRAINT_NAME = ?
            "#,
        )
        .bind(constraint)
        .fetch_one(pool.as_ref())
        .await?;
        let clause = clause.to_ascii_uppercase();
        for fragment in expected_fragments {
            assert!(
                clause.contains(&fragment.to_ascii_uppercase()),
                "{constraint} must contain {fragment}: {clause}"
            );
        }
    }

    let source_delete_rule = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(DELETE_RULE AS CHAR)
        FROM information_schema.REFERENTIAL_CONSTRAINTS
        WHERE CONSTRAINT_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relationships'
          AND CONSTRAINT_NAME = 'fk_relationships_tenant_source_property_restrict'
        "#,
    )
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(source_delete_rule, "RESTRICT");

    let tenant_a = TenantId::default().to_string();
    let tenant_b = TenantId::default().to_string();
    let database_a = DatabaseId::default().to_string();
    let target_a = DatabaseId::default().to_string();
    let database_b = DatabaseId::default().to_string();
    let source_default = PropertyId::default().to_string();
    let source_duplicate = source_default.clone();
    let source_bad_cardinality = PropertyId::default().to_string();
    let source_bad_policy = PropertyId::default().to_string();
    let source_bad_inverse = PropertyId::default().to_string();
    let inverse_a = PropertyId::default().to_string();
    let inverse_b = PropertyId::default().to_string();
    let mut transaction = pool.begin().await?;

    insert_object(&mut transaction, &database_a, &tenant_a, "source-a")
        .await?;
    insert_object(&mut transaction, &target_a, &tenant_a, "target-a")
        .await?;
    insert_object(&mut transaction, &database_b, &tenant_b, "target-b")
        .await?;
    for (id, tenant, database, slot) in [
        (&source_default, &tenant_a, &database_a, 0),
        (&source_bad_cardinality, &tenant_a, &database_a, 1),
        (&source_bad_policy, &tenant_a, &database_a, 2),
        (&source_bad_inverse, &tenant_a, &database_a, 3),
        (&inverse_a, &tenant_a, &target_a, 0),
        (&inverse_b, &tenant_b, &database_b, 0),
    ] {
        insert_relation_field(&mut transaction, id, tenant, database, slot)
            .await?;
    }

    let default_definition = RelationId::default().to_string();
    sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id
        )
        VALUES (?, ?, ?, ?, 0, ?)
        "#,
    )
    .bind(&default_definition)
    .bind(&tenant_a)
    .bind(&database_a)
    .bind(&source_default)
    .bind(&database_a)
    .execute(&mut *transaction)
    .await?;
    let defaults = sqlx::query(
        r#"
        SELECT forward_cardinality, reverse_cardinality,
               inverse_field_id, inverse_owned, on_target_delete,
               definition_version, generation
        FROM relationships
        WHERE id = ?
        "#,
    )
    .bind(&default_definition)
    .fetch_one(&mut *transaction)
    .await?;
    assert_eq!(
        defaults.try_get::<String, _>("forward_cardinality")?,
        "MANY"
    );
    assert_eq!(
        defaults.try_get::<String, _>("reverse_cardinality")?,
        "MANY"
    );
    assert_eq!(
        defaults.try_get::<Option<String>, _>("inverse_field_id")?,
        None
    );
    assert!(!defaults.try_get::<bool, _>("inverse_owned")?);
    assert_eq!(
        defaults.try_get::<String, _>("on_target_delete")?,
        "RESTRICT"
    );
    assert_eq!(defaults.try_get::<u16, _>("definition_version")?, 1);
    assert_eq!(defaults.try_get::<u64, _>("generation")?, 1);

    let duplicate_error = sqlx::query(
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
    .bind(&database_a)
    .bind(&source_duplicate)
    .bind(&target_a)
    .execute(&mut *transaction)
    .await
    .expect_err("one source Property must own only one definition");
    assert!(duplicate_error
        .as_database_error()
        .expect("unique violation")
        .is_unique_violation());

    let bad_cardinality = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id, forward_cardinality
        )
        VALUES (?, ?, ?, ?, 0, ?, 'ZERO')
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&database_a)
    .bind(&source_bad_cardinality)
    .bind(&target_a)
    .execute(&mut *transaction)
    .await
    .expect_err("unknown cardinality must be rejected");
    assert!(bad_cardinality
        .as_database_error()
        .expect("check violation")
        .message()
        .contains("chk_relationships_forward_cardinality_strict"));

    let lowercase_cardinality = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id, forward_cardinality
        )
        VALUES (?, ?, ?, ?, 0, ?, 'one')
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&database_a)
    .bind(&source_bad_cardinality)
    .bind(&target_a)
    .execute(&mut *transaction)
    .await
    .expect_err("stored enum spelling must be case-sensitive");
    assert!(lowercase_cardinality
        .as_database_error()
        .expect("check violation")
        .message()
        .contains("chk_relationships_forward_cardinality_strict"));

    let bad_policy = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id, on_target_delete
        )
        VALUES (?, ?, ?, ?, 0, ?, 'CASCADE')
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&database_a)
    .bind(&source_bad_policy)
    .bind(&target_a)
    .execute(&mut *transaction)
    .await
    .expect_err("unknown delete policy must be rejected");
    assert!(bad_policy
        .as_database_error()
        .expect("check violation")
        .message()
        .contains("chk_relationships_on_target_delete_strict"));

    let owned_without_inverse = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id, inverse_owned
        )
        VALUES (?, ?, ?, ?, 0, ?, TRUE)
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&database_a)
    .bind(&source_bad_policy)
    .bind(&target_a)
    .execute(&mut *transaction)
    .await
    .expect_err("owned inverse metadata requires an inverse Property");
    assert!(owned_without_inverse
        .as_database_error()
        .expect("check violation")
        .message()
        .contains("chk_relationships_owned_inverse"));

    let cross_tenant_inverse = sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id, inverse_field_id
        )
        VALUES (?, ?, ?, ?, 0, ?, ?)
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&database_a)
    .bind(&source_bad_inverse)
    .bind(&target_a)
    .bind(&inverse_b)
    .execute(&mut *transaction)
    .await
    .expect_err("inverse Property must stay in the target tenant/database");
    let inverse_error = cross_tenant_inverse
        .as_database_error()
        .expect("foreign key violation");
    assert!(inverse_error.is_foreign_key_violation());
    assert!(inverse_error
        .message()
        .contains("fk_relationships_tenant_target_inverse_field"));

    let legacy_source_delete =
        sqlx::query("DELETE FROM fields WHERE id = ?")
            .bind(&source_default)
            .execute(&mut *transaction)
            .await
            .expect_err(
                "source ownership must not cascade in a mixed fleet",
            );
    assert!(legacy_source_delete
        .as_database_error()
        .expect("foreign key violation")
        .is_foreign_key_violation());
    sqlx::query("DELETE FROM relationships WHERE id = ?")
        .bind(&default_definition)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM fields WHERE id = ?")
        .bind(&source_default)
        .execute(&mut *transaction)
        .await?;
    let remaining = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM relationships WHERE id = ?",
    )
    .bind(&default_definition)
    .fetch_one(&mut *transaction)
    .await?;
    assert_eq!(remaining, 0, "source Property owns its definition");

    // A valid inverse proves the composite target scope accepts the correct
    // tenant/database/property tuple. Use a fresh source after the failures.
    let valid_inverse_source = PropertyId::default().to_string();
    insert_relation_field(
        &mut transaction,
        &valid_inverse_source,
        &tenant_a,
        &database_a,
        4,
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id, forward_cardinality,
            reverse_cardinality, inverse_field_id, on_target_delete
        )
        VALUES (?, ?, ?, ?, 0, ?, 'ONE', 'MANY', ?, 'NULLIFY')
        "#,
    )
    .bind(RelationId::default().to_string())
    .bind(&tenant_a)
    .bind(&database_a)
    .bind(&valid_inverse_source)
    .bind(&target_a)
    .bind(&inverse_a)
    .execute(&mut *transaction)
    .await?;

    transaction.rollback().await?;
    Ok(())
}
