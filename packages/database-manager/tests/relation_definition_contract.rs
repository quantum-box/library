use database_manager::domain::{
    PropertyType, RelationCardinality, RelationDefinitionRepository,
    RelationOnDelete, RelationRepository, TypeRelation,
};
use database_manager::interface_adapter::gateway::{
    RelationDefinitionRepositoryImpl, RelationRepositoryImpl,
};
use database_manager::{AddPropertyInputData, CreateDatabaseInputData};
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn relation_property_creation_persists_a_queryable_definition(
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
    let app = database_manager::factory_client(dsn.to_string()).await?;
    let definitions = RelationDefinitionRepositoryImpl::new(db.clone());
    let legacy_relations = RelationRepositoryImpl::new(db.clone());
    let tenant_id = TenantId::default();
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());

    let source_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "relation-definition-source",
        })
        .await?;
    let target_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "relation-definition-target",
        })
        .await?;

    let relation_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            name: "related records",
            property_type: PropertyType::Relation(TypeRelation::new(
                target_database.id().clone(),
            )),
        })
        .await?;

    let definition = definitions
        .find_by_source_property(
            &tenant_id,
            source_database.id(),
            relation_property.id(),
        )
        .await?
        .expect("Relation Property must own one definition");
    assert_eq!(definition.tenant_id(), &tenant_id);
    assert_eq!(definition.source_database_id(), source_database.id());
    assert_eq!(definition.source_property_id(), relation_property.id());
    assert_eq!(definition.target_database_id(), target_database.id());
    assert_eq!(
        *definition.forward_cardinality(),
        RelationCardinality::Many
    );
    assert_eq!(
        *definition.reverse_cardinality(),
        RelationCardinality::Many
    );
    assert_eq!(*definition.on_target_delete(), RelationOnDelete::Restrict);
    assert!(definition.inverse_property_id().is_none());

    assert_eq!(
        definitions
            .find_all_by_source_database(&tenant_id, source_database.id())
            .await?,
        [definition.clone()]
    );
    assert_eq!(
        definitions
            .find_all_by_target_database(&tenant_id, target_database.id())
            .await?,
        [definition]
    );

    // The existing public Property shape and legacy physical columns remain
    // available while RelationEdge storage is introduced separately.
    let field = sqlx::query(
        r#"
        SELECT datatype, datatype_meta
        FROM fields
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(relation_property.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(field.try_get::<String, _>("datatype")?, "RELATION");
    let legacy_meta =
        field.try_get::<serde_json::Value, _>("datatype_meta")?;
    assert_eq!(
        legacy_meta
            .get("database_id")
            .and_then(|value| value.as_str()),
        Some(target_database.id().as_str())
    );
    let legacy_relation_id = sqlx::query_scalar::<_, u32>(
        r#"
        SELECT relation_id
        FROM relationships
        WHERE tenant_id = ? AND object_id = ? AND field_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(relation_property.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(legacy_relation_id, 0);
    let legacy_projection = legacy_relations
        .find_all_by_database(source_database.id(), &tenant_id)
        .await?;
    assert_eq!(legacy_projection.len(), 1);
    assert_eq!(legacy_projection[0].property_id(), relation_property.id());
    assert_eq!(
        legacy_projection[0].target_database_id(),
        target_database.id()
    );

    let self_relation = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            name: "self relation",
            property_type: PropertyType::Relation(TypeRelation::new(
                source_database.id().clone(),
            )),
        })
        .await?;
    let self_definition = definitions
        .find_by_source_property(
            &tenant_id,
            source_database.id(),
            self_relation.id(),
        )
        .await?
        .expect("self Relation must be supported");
    assert_eq!(
        self_definition.source_database_id(),
        self_definition.target_database_id()
    );

    Ok(())
}
