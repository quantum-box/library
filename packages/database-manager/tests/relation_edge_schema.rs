use database_manager::domain::{
    DataId, DatabaseId, PropertyId, RelationId,
};
use sqlx::{MySql, MySqlPool, Row, Transaction};
use value_object::{DatabaseUrl, TenantId};

async fn insert_object(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    name: &str,
) -> anyhow::Result<()> {
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

async fn insert_field(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    property_id: &PropertyId,
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
    .bind(property_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(field_num)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_data(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    name: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO data (id, tenant_id, object_id, name) VALUES (?, ?, ?, ?)",
    )
    .bind(data_id.to_string())
    .bind(tenant_id.to_string())
    .bind(database_id.to_string())
    .bind(name)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn insert_definition(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    source_property_id: &PropertyId,
    relation_id: &RelationId,
    target_database_id: &DatabaseId,
    legacy_relation_id: u32,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id
        )
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(relation_id.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(source_property_id.to_string())
    .bind(legacy_relation_id)
    .bind(target_database_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_edge(
    pool: &MySqlPool,
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    source_data_id: &DataId,
    relation_id: &RelationId,
    target_database_id: &DatabaseId,
    target_data_id: &DataId,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO relation_edges (
            tenant_id, source_database_id, source_data_id, relation_id,
            target_database_id, target_data_id
        )
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(source_data_id.to_string())
    .bind(relation_id.to_string())
    .bind(target_database_id.to_string())
    .bind(target_data_id.to_string())
    .execute(pool)
    .await
    .map(|_| ())
}

async fn key_columns(
    pool: &MySqlPool,
    table: &str,
    key: &str,
) -> anyhow::Result<Vec<String>> {
    Ok(sqlx::query(
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
    .bind(key)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.try_get("column_name_text"))
    .collect::<Result<Vec<_>, _>>()?)
}

async fn index_names(pool: &MySqlPool) -> anyhow::Result<Vec<String>> {
    Ok(sqlx::query(
        r#"
        SELECT DISTINCT CAST(INDEX_NAME AS CHAR) AS index_name_text
        FROM information_schema.STATISTICS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relation_edges'
        ORDER BY index_name_text
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.try_get("index_name_text"))
    .collect::<Result<Vec<_>, _>>()?)
}

async fn foreign_key_columns(
    pool: &MySqlPool,
    constraint: &str,
) -> anyhow::Result<Vec<String>> {
    Ok(sqlx::query(
        r#"
        SELECT CAST(COLUMN_NAME AS CHAR) AS column_name_text
        FROM information_schema.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'relation_edges'
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

#[test]
fn relation_edge_migration_resumes_after_partial_tidb_ddl() {
    let migration = include_str!(
        "../migrations/20260716110000_create_relation_edges.sql"
    );

    assert!(migration.contains("information_schema.STATISTICS"));
    assert!(
        migration.contains("INDEX_NAME = 'uq_relationships_edge_scope'")
    );
    assert!(migration.contains("CREATE TABLE IF NOT EXISTS relation_edges"));
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn relation_edge_schema_enforces_complete_scoped_identity(
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
        "../migrations/20260716110000_create_relation_edges.sql"
    );
    let candidate_key_position = migration
        .find("ADD CONSTRAINT uq_relationships_edge_scope")
        .expect("definition candidate key must be explicit");
    let table_position = migration
        .find("CREATE TABLE IF NOT EXISTS relation_edges")
        .expect("edge table must be created");
    assert!(candidate_key_position < table_position);
    assert!(!migration
        .to_uppercase()
        .contains("INSERT INTO RELATION_EDGES"));
    let runbook = include_str!(
        "../../../docs/specs/operations/relation-edge-expand-rollout.md"
    );
    for recovery_contract in [
        "Partial DDL recovery",
        "INDEX_NAME = 'uq_relationships_edge_scope'",
        "TABLE_NAME = 'relation_edges'",
        "DROP TABLE relation_edges",
        "DROP INDEX uq_relationships_edge_scope",
        "do not repair automatically or erase the evidence",
    ] {
        assert!(
            runbook.contains(recovery_contract),
            "runbook must preserve partial-DDL recovery: {recovery_contract}"
        );
    }

    assert_eq!(
        key_columns(
            pool.as_ref(),
            "relationships",
            "uq_relationships_edge_scope"
        )
        .await?,
        ["tenant_id", "object_id", "id", "target_object_id"]
    );
    assert_eq!(
        key_columns(pool.as_ref(), "relation_edges", "PRIMARY").await?,
        [
            "tenant_id",
            "source_database_id",
            "source_data_id",
            "relation_id",
            "target_database_id",
            "target_data_id",
        ]
    );
    assert_eq!(
        key_columns(
            pool.as_ref(),
            "relation_edges",
            "idx_relation_edges_forward"
        )
        .await?,
        [
            "tenant_id",
            "source_database_id",
            "relation_id",
            "target_database_id",
            "source_data_id",
            "target_data_id",
        ]
    );
    assert_eq!(
        key_columns(
            pool.as_ref(),
            "relation_edges",
            "idx_relation_edges_backlink"
        )
        .await?,
        [
            "tenant_id",
            "target_database_id",
            "target_data_id",
            "relation_id",
            "source_database_id",
            "source_data_id",
        ]
    );
    assert_eq!(
        index_names(pool.as_ref()).await?,
        [
            "idx_relation_edges_backlink",
            "idx_relation_edges_forward",
            "PRIMARY",
        ],
        "the explicit forward index must support the definition FK without an implicit index"
    );

    for (constraint, expected) in [
        (
            "fk_relation_edges_definition_scope",
            vec![
                "tenant_id",
                "source_database_id",
                "relation_id",
                "target_database_id",
            ],
        ),
        (
            "fk_relation_edges_source_record",
            vec!["tenant_id", "source_database_id", "source_data_id"],
        ),
        (
            "fk_relation_edges_target_record",
            vec!["tenant_id", "target_database_id", "target_data_id"],
        ),
    ] {
        assert_eq!(
            foreign_key_columns(pool.as_ref(), constraint).await?,
            expected
        );
        let delete_rule = sqlx::query_scalar::<_, String>(
            r#"
            SELECT CAST(DELETE_RULE AS CHAR)
            FROM information_schema.REFERENTIAL_CONSTRAINTS
            WHERE CONSTRAINT_SCHEMA = DATABASE()
              AND TABLE_NAME = 'relation_edges'
              AND CONSTRAINT_NAME = ?
            "#,
        )
        .bind(constraint)
        .fetch_one(pool.as_ref())
        .await?;
        assert_eq!(delete_rule, "RESTRICT");
    }

    let tenant_a = TenantId::default();
    let tenant_b = TenantId::default();
    let source_database = DatabaseId::default();
    let target_database = DatabaseId::default();
    let other_database = DatabaseId::default();
    let tenant_b_database = DatabaseId::default();
    let source_property = PropertyId::default();
    let self_property = PropertyId::default();
    let relation = RelationId::default();
    let self_relation = RelationId::default();
    let source_data = DataId::default();
    let target_data = DataId::default();
    let other_data = DataId::default();
    let tenant_b_data = DataId::default();
    let self_data = DataId::default();

    let mut transaction = pool.begin().await?;
    insert_object(
        &mut transaction,
        &tenant_a,
        &source_database,
        "edge-source",
    )
    .await?;
    insert_object(
        &mut transaction,
        &tenant_a,
        &target_database,
        "edge-target",
    )
    .await?;
    insert_object(
        &mut transaction,
        &tenant_a,
        &other_database,
        "edge-other",
    )
    .await?;
    insert_object(
        &mut transaction,
        &tenant_b,
        &tenant_b_database,
        "edge-cross-tenant",
    )
    .await?;
    insert_field(
        &mut transaction,
        &tenant_a,
        &source_database,
        &source_property,
        1,
    )
    .await?;
    insert_field(
        &mut transaction,
        &tenant_a,
        &source_database,
        &self_property,
        2,
    )
    .await?;
    insert_data(
        &mut transaction,
        &tenant_a,
        &source_database,
        &source_data,
        "source",
    )
    .await?;
    insert_data(
        &mut transaction,
        &tenant_a,
        &target_database,
        &target_data,
        "target",
    )
    .await?;
    insert_data(
        &mut transaction,
        &tenant_a,
        &other_database,
        &other_data,
        "wrong-target",
    )
    .await?;
    insert_data(
        &mut transaction,
        &tenant_b,
        &tenant_b_database,
        &tenant_b_data,
        "cross-tenant-target",
    )
    .await?;
    insert_data(
        &mut transaction,
        &tenant_a,
        &source_database,
        &self_data,
        "self-loop",
    )
    .await?;
    insert_definition(
        &mut transaction,
        &tenant_a,
        &source_database,
        &source_property,
        &relation,
        &target_database,
        1,
    )
    .await?;
    insert_definition(
        &mut transaction,
        &tenant_a,
        &source_database,
        &self_property,
        &self_relation,
        &source_database,
        2,
    )
    .await?;
    transaction.commit().await?;

    insert_edge(
        pool.as_ref(),
        &tenant_a,
        &source_database,
        &source_data,
        &relation,
        &target_database,
        &target_data,
    )
    .await?;
    insert_edge(
        pool.as_ref(),
        &tenant_a,
        &source_database,
        &self_data,
        &self_relation,
        &source_database,
        &self_data,
    )
    .await?;

    assert!(
        insert_edge(
            pool.as_ref(),
            &tenant_a,
            &source_database,
            &source_data,
            &relation,
            &target_database,
            &target_data,
        )
        .await
        .is_err(),
        "duplicate logical identity must be rejected"
    );
    assert!(
        insert_edge(
            pool.as_ref(),
            &tenant_a,
            &source_database,
            &other_data,
            &relation,
            &target_database,
            &target_data,
        )
        .await
        .is_err(),
        "source record must belong to the declared source Database"
    );
    assert!(
        insert_edge(
            pool.as_ref(),
            &tenant_a,
            &other_database,
            &source_data,
            &relation,
            &target_database,
            &target_data,
        )
        .await
        .is_err(),
        "source Data must belong to the declared source Database"
    );
    assert!(
        insert_edge(
            pool.as_ref(),
            &tenant_a,
            &source_database,
            &source_data,
            &relation,
            &target_database,
            &other_data,
        )
        .await
        .is_err(),
        "target record must belong to the declared target Database"
    );
    assert!(
        insert_edge(
            pool.as_ref(),
            &tenant_a,
            &source_database,
            &source_data,
            &relation,
            &other_database,
            &other_data,
        )
        .await
        .is_err(),
        "target Database must match the RelationDefinition"
    );
    assert!(
        insert_edge(
            pool.as_ref(),
            &tenant_b,
            &source_database,
            &source_data,
            &relation,
            &target_database,
            &target_data,
        )
        .await
        .is_err(),
        "cross-tenant source scope must be rejected"
    );
    assert!(
        insert_edge(
            pool.as_ref(),
            &tenant_a,
            &source_database,
            &source_data,
            &relation,
            &target_database,
            &tenant_b_data,
        )
        .await
        .is_err(),
        "cross-tenant target record must be rejected"
    );

    for (sql, id) in [
        ("DELETE FROM data WHERE id = ?", source_data.to_string()),
        ("DELETE FROM data WHERE id = ?", target_data.to_string()),
        (
            "DELETE FROM relationships WHERE id = ?",
            relation.to_string(),
        ),
    ] {
        assert!(
            sqlx::query(sql)
                .bind(id)
                .execute(pool.as_ref())
                .await
                .is_err(),
            "edge endpoint and definition deletes must fail closed"
        );
    }

    Ok(())
}
