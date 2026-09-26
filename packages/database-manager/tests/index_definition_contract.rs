use database_manager::domain::{
    IndexPolicy, IndexProjectionState, IndexTarget, PropertyType,
    RelationDefinitionRepository, TypeRelation,
};
use database_manager::interface_adapter::gateway::{
    DatabaseRepositoryImpl, IndexDefinitionRepositoryImpl,
    PropertyRepositoryImpl, RelationDefinitionRepositoryImpl,
};
use database_manager::usecase::IndexDefinitionInteractor;
use database_manager::{
    AddPropertyInputData, CreateDatabaseInputData,
    DeclareIndexDefinitionInputData, FindIndexDefinitionByIdInputData,
    FindIndexDefinitionByTargetInputData, FindIndexDefinitionsInputData,
    IndexDefinitionInputPort, ReconfigureIndexDefinitionInputData,
    TransitionIndexProjectionInputData,
};
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

#[derive(Debug)]
struct PermissiveAnonymousExecutor;

impl auth::ExecutorAction for PermissiveAnonymousExecutor {
    fn get_id(&self) -> &str {
        ""
    }

    fn has_tenant_id(&self, _tenant_id: &TenantId) -> bool {
        true
    }

    fn is_system_user(&self) -> bool {
        true
    }

    fn is_user(&self) -> bool {
        false
    }

    fn is_service_account(&self) -> bool {
        false
    }

    fn is_none(&self) -> bool {
        true
    }
}

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

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn index_definition_control_plane_is_scoped_and_cas_ready(
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

    let property_repository = PropertyRepositoryImpl::new(db.clone());
    let database_repository = DatabaseRepositoryImpl::new(db.clone());
    let relation_repository =
        RelationDefinitionRepositoryImpl::new(db.clone());
    let index_repository = IndexDefinitionRepositoryImpl::new(db.clone());
    let control_plane = IndexDefinitionInteractor::new(
        database_repository,
        property_repository,
        relation_repository.clone(),
        index_repository,
    );

    let tenant_id = TenantId::default();
    let foreign_tenant_id = TenantId::default();
    let multi_tenancy = auth::MultiTenancy::new_operator(tenant_id.clone());
    let foreign_multi_tenancy =
        auth::MultiTenancy::new_operator(foreign_tenant_id.clone());
    let source_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "index-control-plane-source",
        })
        .await?;
    let target_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "index-control-plane-target",
        })
        .await?;
    let string_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            name: "slug",
            property_type: PropertyType::String,
        })
        .await?;

    let exact = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Property(string_property.id().clone()),
            policy: IndexPolicy::Exact,
            unique: true,
        })
        .await?;
    assert_eq!(*exact.policy(), IndexPolicy::Exact);
    assert!(*exact.unique());
    assert_eq!(exact.definition_version().get(), 1);
    assert_eq!(exact.generation().get(), 1);
    assert_eq!(*exact.projection_state(), IndexProjectionState::Pending);

    let legacy_field = sqlx::query(
        r#"
        SELECT is_indexed
        FROM fields
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(string_property.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert!(
        !legacy_field.try_get::<bool, _>("is_indexed")?,
        "declaration must not dual-write the legacy field flag"
    );
    let legacy_index_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM indexes WHERE tenant_id = ?",
    )
    .bind(tenant_id.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(
        legacy_index_count, 0,
        "control-plane declarations must not create physical legacy indexes"
    );

    let duplicate_error = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Property(string_property.id().clone()),
            policy: IndexPolicy::Exact,
            unique: false,
        })
        .await
        .expect_err("one target must have one canonical definition");
    assert!(matches!(duplicate_error, errors::Error::Conflict { .. }));

    assert_eq!(
        control_plane
            .find_by_id(FindIndexDefinitionByIdInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &multi_tenancy,
                tenant_id: &tenant_id,
                database_id: source_database.id(),
                index_definition_id: exact.id(),
            })
            .await?,
        Some(exact.clone())
    );
    assert_eq!(
        control_plane
            .find_by_target(FindIndexDefinitionByTargetInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &multi_tenancy,
                tenant_id: &tenant_id,
                database_id: source_database.id(),
                target: exact.target(),
            })
            .await?,
        Some(exact.clone())
    );
    assert!(control_plane
        .find_by_id(FindIndexDefinitionByIdInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: target_database.id(),
            index_definition_id: exact.id(),
        })
        .await?
        .is_none());

    let foreign_database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &foreign_multi_tenancy,
            database_id: None,
            tenant_id: &foreign_tenant_id,
            name: "index-control-plane-foreign",
        })
        .await?;
    let foreign_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &foreign_multi_tenancy,
            tenant_id: &foreign_tenant_id,
            database_id: foreign_database.id(),
            name: "foreign slug",
            property_type: PropertyType::String,
        })
        .await?;
    let cross_tenant_declare = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &foreign_tenant_id,
            database_id: foreign_database.id(),
            target: IndexTarget::Property(foreign_property.id().clone()),
            policy: IndexPolicy::Exact,
            unique: false,
        })
        .await
        .expect_err(
            "an authenticated tenant must not declare in another scope",
        );
    assert!(cross_tenant_declare.is_not_found());
    let foreign_definition = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &foreign_multi_tenancy,
            tenant_id: &foreign_tenant_id,
            database_id: foreign_database.id(),
            target: IndexTarget::Property(foreign_property.id().clone()),
            policy: IndexPolicy::Exact,
            unique: false,
        })
        .await?;
    let cross_tenant_read = control_plane
        .find_by_id(FindIndexDefinitionByIdInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &foreign_tenant_id,
            database_id: foreign_database.id(),
            index_definition_id: foreign_definition.id(),
        })
        .await
        .expect_err("an authenticated tenant must not read another scope");
    assert!(cross_tenant_read.is_not_found());
    let cross_tenant_update = control_plane
        .reconfigure(ReconfigureIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &foreign_tenant_id,
            database_id: foreign_database.id(),
            index_definition_id: foreign_definition.id(),
            expected_generation: *foreign_definition.generation(),
            policy: IndexPolicy::FullText,
            unique: false,
        })
        .await
        .expect_err(
            "an authenticated tenant must not mutate another scope",
        );
    assert!(cross_tenant_update.is_not_found());
    let missing_executor = control_plane
        .find_by_id(FindIndexDefinitionByIdInputData {
            executor: &PermissiveAnonymousExecutor,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: exact.id(),
        })
        .await
        .expect_err("an unauthenticated executor must not read the scope");
    assert!(missing_executor.is_not_found());
    assert_eq!(
        control_plane
            .find_by_id(FindIndexDefinitionByIdInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &foreign_multi_tenancy,
                tenant_id: &foreign_tenant_id,
                database_id: foreign_database.id(),
                index_definition_id: foreign_definition.id(),
            })
            .await?,
        Some(foreign_definition),
        "denied requests must not mutate the foreign definition"
    );

    let full_text = control_plane
        .reconfigure(ReconfigureIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: exact.id(),
            expected_generation: *exact.generation(),
            policy: IndexPolicy::FullText,
            unique: false,
        })
        .await?;
    assert_eq!(*full_text.policy(), IndexPolicy::FullText);
    assert!(!*full_text.unique());
    assert_eq!(full_text.generation().get(), 2);
    assert_eq!(
        *full_text.projection_state(),
        IndexProjectionState::Pending
    );

    let stale_error = control_plane
        .reconfigure(ReconfigureIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: exact.id(),
            expected_generation: *exact.generation(),
            policy: IndexPolicy::None,
            unique: false,
        })
        .await
        .expect_err("a stale generation must not overwrite a newer policy");
    assert!(matches!(stale_error, errors::Error::Conflict { .. }));

    let building = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: full_text.id(),
            expected_generation: *full_text.generation(),
            next_state: IndexProjectionState::Building,
        })
        .await?;
    assert_eq!(building.generation().get(), 3);
    assert_eq!(
        *building.projection_state(),
        IndexProjectionState::Building
    );
    let ready = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: building.id(),
            expected_generation: *building.generation(),
            next_state: IndexProjectionState::Ready,
        })
        .await?;
    assert_eq!(ready.generation(), building.generation());
    assert_eq!(*ready.projection_state(), IndexProjectionState::Ready);
    let invalid_transition = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: ready.id(),
            expected_generation: *ready.generation(),
            next_state: IndexProjectionState::Building,
        })
        .await
        .expect_err("READY must not jump directly back to BUILDING");
    assert!(matches!(invalid_transition, errors::Error::Conflict { .. }));

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
    let relation = relation_repository
        .find_by_source_property(
            &tenant_id,
            source_database.id(),
            relation_property.id(),
        )
        .await?
        .expect("Relation Property must own a RelationDefinition");

    let invalid_relation_unique = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Relation(relation.id().clone()),
            policy: IndexPolicy::Exact,
            unique: true,
        })
        .await
        .expect_err("Relation reverse lookup cannot be unique");
    assert!(invalid_relation_unique.is_bad_request());
    let invalid_relation_full_text = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Relation(relation.id().clone()),
            policy: IndexPolicy::FullText,
            unique: false,
        })
        .await
        .expect_err("Relation reverse lookup only supports EXACT or NONE");
    assert!(invalid_relation_full_text.is_bad_request());

    let relation_exact = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Relation(relation.id().clone()),
            policy: IndexPolicy::Exact,
            unique: false,
        })
        .await?;
    assert_eq!(*relation_exact.policy(), IndexPolicy::Exact);
    assert!(!*relation_exact.unique());
    assert_eq!(
        *relation_exact.projection_state(),
        IndexProjectionState::Pending
    );

    let relation_building = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: relation_exact.id(),
            expected_generation: *relation_exact.generation(),
            next_state: IndexProjectionState::Building,
        })
        .await?;
    assert_eq!(relation_building.generation().get(), 2);
    let duplicate_start = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: relation_exact.id(),
            expected_generation: *relation_exact.generation(),
            next_state: IndexProjectionState::Building,
        })
        .await
        .expect_err("a second builder must lose the generation CAS");
    assert!(matches!(duplicate_start, errors::Error::Conflict { .. }));
    let relation_failed = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: relation_building.id(),
            expected_generation: *relation_building.generation(),
            next_state: IndexProjectionState::Failed,
        })
        .await?;
    assert_eq!(
        relation_failed.generation(),
        relation_building.generation()
    );
    let relation_retry = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: relation_failed.id(),
            expected_generation: *relation_failed.generation(),
            next_state: IndexProjectionState::Building,
        })
        .await?;
    assert_eq!(relation_retry.generation().get(), 3);
    let stale_completion = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: relation_building.id(),
            expected_generation: *relation_building.generation(),
            next_state: IndexProjectionState::Ready,
        })
        .await
        .expect_err("a failed builder must not complete a later retry");
    assert!(matches!(stale_completion, errors::Error::Conflict { .. }));
    assert_eq!(
        control_plane
            .find_by_id(FindIndexDefinitionByIdInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &multi_tenancy,
                tenant_id: &tenant_id,
                database_id: source_database.id(),
                index_definition_id: relation_retry.id(),
            })
            .await?,
        Some(relation_retry.clone()),
        "stale completion must leave the retry building"
    );
    let relation_ready = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: relation_retry.id(),
            expected_generation: *relation_retry.generation(),
            next_state: IndexProjectionState::Ready,
        })
        .await?;

    let definitions = control_plane
        .find_all_by_database(FindIndexDefinitionsInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
        })
        .await?;
    assert_eq!(definitions.len(), 2);
    assert!(definitions.contains(&ready));
    assert!(definitions.contains(&relation_ready));

    // Projection lifecycle is a worker-owned boundary and must revalidate the
    // canonical target on every transition. A tenant user cannot self-report
    // READY, and an opaque definition cannot advance an older declaration.
    let guarded_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            name: "guarded projection",
            property_type: PropertyType::String,
        })
        .await?;
    let guarded_definition = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Property(guarded_property.id().clone()),
            policy: IndexPolicy::Exact,
            unique: false,
        })
        .await?;
    let tenant_user = TenantUserExecutor {
        id: "tenant-user".to_string(),
        tenant_id: tenant_id.clone(),
    };
    let unauthorized_transition = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &tenant_user,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: guarded_definition.id(),
            expected_generation: *guarded_definition.generation(),
            next_state: IndexProjectionState::Building,
        })
        .await
        .expect_err("tenant users must not advance projection lifecycle");
    assert!(unauthorized_transition.is_not_found());
    assert_eq!(
        control_plane
            .find_by_id(FindIndexDefinitionByIdInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &multi_tenancy,
                tenant_id: &tenant_id,
                database_id: source_database.id(),
                index_definition_id: guarded_definition.id(),
            })
            .await?,
        Some(guarded_definition.clone())
    );

    sqlx::query(
        r#"
        UPDATE fields
        SET type_key = 'future_scalar', type_version = 9,
            type_config = '{"future":true}'
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(guarded_property.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let opaque_transition = control_plane
        .transition_projection(TransitionIndexProjectionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            index_definition_id: guarded_definition.id(),
            expected_generation: *guarded_definition.generation(),
            next_state: IndexProjectionState::Building,
        })
        .await
        .expect_err(
            "opaque canonical targets must stop projection transitions",
        );
    assert!(opaque_transition.to_string().contains("NotSupported"));
    assert_eq!(
        control_plane
            .find_by_id(FindIndexDefinitionByIdInputData {
                executor: &auth::Executor::SystemUser,
                multi_tenancy: &multi_tenancy,
                tenant_id: &tenant_id,
                database_id: source_database.id(),
                index_definition_id: guarded_definition.id(),
            })
            .await?,
        Some(guarded_definition)
    );

    // Capability decisions require parity between the canonical definition
    // and its compatibility projection. Neither side of a mismatch wins.
    let mismatched_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            name: "canonical integer with stale legacy string",
            property_type: PropertyType::String,
        })
        .await?;
    sqlx::query(
        r#"
        UPDATE fields
        SET type_key = 'integer', type_version = 1, type_config = 'null'
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(mismatched_property.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let mismatch_error = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Property(mismatched_property.id().clone()),
            policy: IndexPolicy::Range,
            unique: true,
        })
        .await
        .expect_err(
            "mismatched PropertyDefinition projections fail closed",
        );
    assert!(mismatch_error.is_bad_request());
    assert!(mismatch_error.to_string().contains("does not match"));
    let mismatched_definition_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED)
        FROM index_definitions
        WHERE tenant_id = ? AND database_id = ? AND property_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(mismatched_property.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(mismatched_definition_count, 0);

    // A future canonical definition is readable as opaque, but must never
    // inherit STRING capabilities from its stale legacy projection.
    let opaque_property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            name: "future canonical type",
            property_type: PropertyType::String,
        })
        .await?;
    sqlx::query(
        r#"
        UPDATE fields
        SET type_key = 'future_scalar', type_version = 9,
            type_config = '{"future":true}'
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(opaque_property.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let opaque_error = control_plane
        .declare(DeclareIndexDefinitionInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source_database.id(),
            target: IndexTarget::Property(opaque_property.id().clone()),
            policy: IndexPolicy::Exact,
            unique: false,
        })
        .await
        .expect_err(
            "opaque canonical definitions must not use legacy capabilities",
        );
    assert!(opaque_error.is_bad_request());
    assert!(opaque_error.to_string().contains("NotSupported"));
    let opaque_definition_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED)
        FROM index_definitions
        WHERE tenant_id = ? AND database_id = ? AND property_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .bind(opaque_property.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(opaque_definition_count, 0);

    let final_legacy_index_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM indexes WHERE tenant_id = ?",
    )
    .bind(tenant_id.to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(final_legacy_index_count, 0);
    let indexed_field_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT CAST(COUNT(*) AS SIGNED)
        FROM fields
        WHERE tenant_id = ? AND object_id = ? AND is_indexed = TRUE
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(source_database.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(indexed_field_count, 0);

    Ok(())
}
