use database_manager::domain::{
    DataId, DatabaseId, PropertyId, RelationDefinitionRepository,
    RelationEdgeRepository, RelationId,
};
use database_manager::interface_adapter::gateway::{
    RelationDefinitionRepositoryImpl, RelationEdgeRepositoryImpl,
};
use sqlx::{MySql, Transaction};
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

#[allow(clippy::too_many_arguments)]
async fn insert_definition(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    source_property_id: &PropertyId,
    relation_id: &RelationId,
    target_database_id: &DatabaseId,
    legacy_relation_id: u32,
    definition_version: u16,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO relationships (
            id, tenant_id, object_id, field_id, relation_id,
            target_object_id, definition_version
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(relation_id.to_string())
    .bind(tenant_id.to_string())
    .bind(source_database_id.to_string())
    .bind(source_property_id.to_string())
    .bind(legacy_relation_id)
    .bind(target_database_id.to_string())
    .bind(definition_version)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_edge(
    transaction: &mut Transaction<'_, MySql>,
    tenant_id: &TenantId,
    source_database_id: &DatabaseId,
    source_data_id: &DataId,
    relation_id: &RelationId,
    target_database_id: &DatabaseId,
    target_data_id: &DataId,
) -> anyhow::Result<()> {
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
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn read_repository_is_tenant_scoped_and_uses_one_edge_for_backlinks(
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

    let tenant_a = TenantId::default();
    let tenant_b = TenantId::default();
    let source_database_a = DatabaseId::default();
    let target_database_a = DatabaseId::default();
    let source_database_b = DatabaseId::default();
    let target_database_b = DatabaseId::default();
    let source_property_a = PropertyId::default();
    let future_property_a = PropertyId::default();
    let source_property_b = PropertyId::default();
    let relation_a = RelationId::default();
    let future_relation_a = RelationId::default();
    let relation_b = RelationId::default();
    let source_a1 = DataId::default();
    let source_a2 = DataId::default();
    let target_a1 = DataId::default();
    let target_a2 = DataId::default();
    let source_b1 = DataId::default();
    let target_b1 = DataId::default();

    let mut transaction = db.pool().begin().await?;
    for (tenant_id, database_id, name) in [
        (&tenant_a, &source_database_a, "contract-source-a"),
        (&tenant_a, &target_database_a, "contract-target-a"),
        (&tenant_b, &source_database_b, "contract-source-b"),
        (&tenant_b, &target_database_b, "contract-target-b"),
    ] {
        insert_object(&mut transaction, tenant_id, database_id, name)
            .await?;
    }
    insert_field(
        &mut transaction,
        &tenant_a,
        &source_database_a,
        &source_property_a,
        1,
    )
    .await?;
    insert_field(
        &mut transaction,
        &tenant_a,
        &source_database_a,
        &future_property_a,
        2,
    )
    .await?;
    insert_field(
        &mut transaction,
        &tenant_b,
        &source_database_b,
        &source_property_b,
        1,
    )
    .await?;
    for (tenant_id, database_id, data_id, name) in [
        (&tenant_a, &source_database_a, &source_a1, "source-a1"),
        (&tenant_a, &source_database_a, &source_a2, "source-a2"),
        (&tenant_a, &target_database_a, &target_a1, "target-a1"),
        (&tenant_a, &target_database_a, &target_a2, "target-a2"),
        (&tenant_b, &source_database_b, &source_b1, "source-b1"),
        (&tenant_b, &target_database_b, &target_b1, "target-b1"),
    ] {
        insert_data(
            &mut transaction,
            tenant_id,
            database_id,
            data_id,
            name,
        )
        .await?;
    }
    insert_definition(
        &mut transaction,
        &tenant_a,
        &source_database_a,
        &source_property_a,
        &relation_a,
        &target_database_a,
        1,
        1,
    )
    .await?;
    insert_definition(
        &mut transaction,
        &tenant_a,
        &source_database_a,
        &future_property_a,
        &future_relation_a,
        &target_database_a,
        2,
        2,
    )
    .await?;
    insert_definition(
        &mut transaction,
        &tenant_b,
        &source_database_b,
        &source_property_b,
        &relation_b,
        &target_database_b,
        1,
        1,
    )
    .await?;
    for (
        tenant_id,
        source_database_id,
        source_data_id,
        relation_id,
        target_database_id,
        target_data_id,
    ) in [
        (
            &tenant_a,
            &source_database_a,
            &source_a1,
            &relation_a,
            &target_database_a,
            &target_a1,
        ),
        (
            &tenant_a,
            &source_database_a,
            &source_a1,
            &relation_a,
            &target_database_a,
            &target_a2,
        ),
        (
            &tenant_a,
            &source_database_a,
            &source_a2,
            &relation_a,
            &target_database_a,
            &target_a1,
        ),
        (
            &tenant_a,
            &source_database_a,
            &source_a2,
            &future_relation_a,
            &target_database_a,
            &target_a2,
        ),
        (
            &tenant_b,
            &source_database_b,
            &source_b1,
            &relation_b,
            &target_database_b,
            &target_b1,
        ),
    ] {
        insert_edge(
            &mut transaction,
            tenant_id,
            source_database_id,
            source_data_id,
            relation_id,
            target_database_id,
            target_data_id,
        )
        .await?;
    }
    transaction.commit().await?;

    let definitions = RelationDefinitionRepositoryImpl::new(db.clone());
    let edges = RelationEdgeRepositoryImpl::new(db);
    let definition_a = definitions
        .find_by_id(&tenant_a, &source_database_a, &relation_a)
        .await?
        .expect("tenant A definition");

    let forward = edges
        .find_forward(&tenant_a, &definition_a, &source_a1)
        .await?;
    assert_eq!(forward.edges().len(), 2);
    let mut expected_targets = vec![target_a1.clone(), target_a2.clone()];
    expected_targets.sort();
    assert_eq!(
        forward
            .edges()
            .iter()
            .map(|edge| edge.target().data_id().clone())
            .collect::<Vec<_>>(),
        expected_targets
    );
    assert!(forward
        .edges()
        .iter()
        .all(|edge| edge.tenant_id() == &tenant_a));

    let backlinks = edges
        .find_backlinks(&tenant_a, &definition_a, &target_a1)
        .await?;
    assert_eq!(backlinks.edges().len(), 2);
    let mut expected_sources = vec![source_a1.clone(), source_a2.clone()];
    expected_sources.sort();
    assert_eq!(
        backlinks
            .edges()
            .iter()
            .map(|edge| edge.source().data_id().clone())
            .collect::<Vec<_>>(),
        expected_sources
    );

    assert!(
        edges
            .find_forward(&tenant_b, &definition_a, &source_a1)
            .await
            .is_err(),
        "an authenticated tenant cannot query another tenant's definition"
    );
    let definition_b = definitions
        .find_by_id(&tenant_b, &source_database_b, &relation_b)
        .await?
        .expect("tenant B definition");
    let tenant_b_forward = edges
        .find_forward(&tenant_b, &definition_b, &source_b1)
        .await?;
    assert_eq!(tenant_b_forward.edges().len(), 1);
    assert_eq!(tenant_b_forward.edges()[0].tenant_id(), &tenant_b);

    let future_definition = definitions
        .find_by_id(&tenant_a, &source_database_a, &future_relation_a)
        .await?
        .expect("future definition remains readable");
    assert_eq!(future_definition.definition_version().get(), 2);
    let future_edges = edges
        .find_forward(&tenant_a, &future_definition, &source_a2)
        .await?;
    assert_eq!(future_edges.edges().len(), 1);
    assert_eq!(future_edges.edges()[0].target().data_id(), &target_a2);

    Ok(())
}
