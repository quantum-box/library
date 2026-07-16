use database_manager::domain::{
    PropertyDefinitionRepository, PropertyType, PropertyValueCommand,
    ResolvedPropertyConfig, TypeRelation,
};
use database_manager::interface_adapter::gateway::PropertyRepositoryImpl;
use database_manager::property_definition_rollout::PropertyDefinitionStorageMode;
use database_manager::property_value_rollout::PropertyValueStorageMode;
use database_manager::{
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    DeletePropertyInputData, PropertyDataInputData, UpdateDataInputData,
    UpdatePropertyInputData,
};
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn property_definitions_dual_write_and_read_without_downgrading_unknown_types(
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
    let legacy_app = database_manager::factory_client_with_storage_modes(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyDefinitionStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let canonical_app =
        database_manager::factory_client_with_storage_modes(
            dsn.to_string(),
            PropertyValueStorageMode::DualWriteCanonicalRead,
            PropertyDefinitionStorageMode::DualWriteCanonicalRead,
        )
        .await?;
    let canonical_definitions =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteCanonicalRead,
        );
    let legacy_definitions =
        PropertyRepositoryImpl::new_with_definition_mode(
            db.clone(),
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        );
    let tenant_id = TenantId::default();
    let executor = &auth::Executor::SystemUser;
    let multi_tenancy =
        &auth::MultiTenancy::new_operator(tenant_id.clone());

    let source = legacy_app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "definition-source",
        })
        .await?;
    let target = legacy_app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "definition-target",
        })
        .await?;
    let title = legacy_app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            name: "title",
            property_type: PropertyType::String,
        })
        .await?;
    let canonical_only_probe = legacy_app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            name: "canonical probe",
            property_type: PropertyType::String,
        })
        .await?;
    let relation = legacy_app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            name: "relation",
            property_type: PropertyType::Relation(TypeRelation::new(
                target.id().clone(),
            )),
        })
        .await?;

    let field = sqlx::query(
        "SELECT datatype, datatype_meta, type_key, type_version, type_config \
         FROM fields WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(title.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(field.try_get::<String, _>("datatype")?, "STRING");
    assert_eq!(field.try_get::<String, _>("type_key")?, "string");
    assert_eq!(field.try_get::<u16, _>("type_version")?, 1);
    assert_eq!(field.try_get::<String, _>("type_config")?, "null");
    assert!(canonical_definitions
        .find_definition_by_id(
            title.id(),
            source.id(),
            &TenantId::default(),
        )
        .await?
        .is_none());
    assert!(canonical_definitions
        .find_definition_by_id(title.id(), target.id(), &tenant_id)
        .await?
        .is_none());

    legacy_app
        .update_property()
        .execute(UpdatePropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            property_id: title.id(),
            name: Some("renamed title"),
            property_type: None,
            meta_json: Some(Some(
                r#"{"integration":"github"}"#.to_string(),
            )),
        })
        .await?;
    let updated = sqlx::query(
        "SELECT field_name, meta_json, datatype, type_key, type_config \
         FROM fields WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(title.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(
        updated.try_get::<String, _>("field_name")?,
        "renamed title"
    );
    assert_eq!(updated.try_get::<String, _>("datatype")?, "STRING");
    assert_eq!(updated.try_get::<String, _>("type_key")?, "string");
    assert_eq!(updated.try_get::<String, _>("type_config")?, "null");

    let relation_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM relationships \
         WHERE tenant_id = ? AND object_id = ? AND field_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(relation.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(relation_count, 1, "RelationDefinition stays in the UoW");

    let record = legacy_app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            name: "definition record",
            property_data: vec![
                PropertyDataInputData {
                    property_id: title.id().clone(),
                    value: PropertyValueCommand::String(
                        "opaque original".to_string(),
                    ),
                },
                PropertyDataInputData {
                    property_id: canonical_only_probe.id().clone(),
                    value: PropertyValueCommand::String(
                        "known original".to_string(),
                    ),
                },
            ],
            database_id: source.id(),
        })
        .await?;

    // A present canonical envelope is authoritative. The canonical reader
    // must not touch a malformed legacy type until the envelope is absent.
    sqlx::query(
        "UPDATE fields SET datatype = 'UNREADABLE_LEGACY_TYPE' \
         WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(canonical_only_probe.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    canonical_definitions
        .find_definition_by_id(
            canonical_only_probe.id(),
            source.id(),
            &tenant_id,
        )
        .await?
        .expect("present canonical envelope");
    sqlx::query(
        "UPDATE fields SET type_key = NULL, type_version = NULL, \
         type_config = NULL WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(canonical_only_probe.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    canonical_definitions
        .find_definition_by_id(
            canonical_only_probe.id(),
            source.id(),
            &tenant_id,
        )
        .await
        .expect_err("absent canonical envelope must fall back to legacy");
    sqlx::query(
        "UPDATE fields SET datatype = 'STRING', type_key = 'string', \
         type_version = 1, type_config = 'null' \
         WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(canonical_only_probe.id().to_string())
    .execute(db.pool().as_ref())
    .await?;

    // A malformed known canonical config fails closed for canonical reads,
    // while legacy-read remains available during the rollout.
    sqlx::query(
        "UPDATE fields SET type_key = 'select', type_version = 1, \
         type_config = 'null' WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(title.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    canonical_definitions
        .find_definition_by_id(title.id(), source.id(), &tenant_id)
        .await
        .expect_err("known malformed canonical config must fail closed");
    legacy_definitions
        .find_definition_by_id(title.id(), source.id(), &tenant_id)
        .await?
        .expect("legacy-read ignores malformed canonical shadow");

    // An unknown type/version stays lossless and read-only. Updating it through
    // either rollout mode must not overwrite either representation.
    let opaque_config = r#"{"feature":{"enabled":true},"items":[1,2]}"#;
    sqlx::query(
        "UPDATE fields SET type_key = 'future_type', type_version = 7, \
         type_config = ? WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(opaque_config)
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(title.id().to_string())
    .execute(db.pool().as_ref())
    .await?;
    let opaque = canonical_definitions
        .find_definition_by_id(title.id(), source.id(), &tenant_id)
        .await?
        .expect("opaque definition");
    assert!(matches!(opaque.config(), ResolvedPropertyConfig::Opaque(_)));
    assert_eq!(
        opaque.raw_config()?,
        serde_json::from_str::<serde_json::Value>(opaque_config)?
    );
    assert!(opaque.to_property().is_err());

    let opaque_value = r#"{"payload":[1,2,3]}"#;
    sqlx::query(
        "UPDATE property_values SET type_key = 'future_type', \
         type_version = 7, value_encoding_version = 1, value = ? \
         WHERE tenant_id = ? AND database_id = ? AND data_id = ? \
         AND property_id = ?",
    )
    .bind(opaque_value)
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(record.id().to_string())
    .bind(title.id().to_string())
    .execute(db.pool().as_ref())
    .await?;

    canonical_app
        .update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            data_id: record.id(),
            name: "definition record updated",
            data: vec![PropertyDataInputData {
                property_id: canonical_only_probe.id().clone(),
                value: PropertyValueCommand::String(
                    "known updated".to_string(),
                ),
            }],
        })
        .await?;
    let opaque_after_unrelated_patch: (String, u16, String) =
        sqlx::query_as(
            "SELECT type_key, type_version, value FROM property_values \
             WHERE tenant_id = ? AND database_id = ? AND data_id = ? \
             AND property_id = ?",
        )
        .bind(tenant_id.to_string())
        .bind(source.id().to_string())
        .bind(record.id().to_string())
        .bind(title.id().to_string())
        .fetch_one(db.pool().as_ref())
        .await?;
    assert_eq!(
        opaque_after_unrelated_patch,
        ("future_type".to_string(), 7, opaque_value.to_string())
    );

    legacy_app
        .update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            data_id: record.id(),
            name: "must roll back",
            data: vec![PropertyDataInputData {
                property_id: title.id().clone(),
                value: PropertyValueCommand::String(
                    "must not overwrite opaque".to_string(),
                ),
            }],
        })
        .await
        .expect_err(
            "legacy-first value writers must preserve opaque definitions",
        );

    legacy_app
        .update_property()
        .execute(UpdatePropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            property_id: title.id(),
            name: Some("legacy must not overwrite"),
            property_type: None,
            meta_json: None,
        })
        .await
        .expect_err(
            "legacy-first writers must also preserve opaque definitions",
        );
    let tenant_id_string = tenant_id.to_string();
    let source_id_string = source.id().to_string();
    let title_id_string = title.id().to_string();
    legacy_app
        .delete_property_usecase()
        .execute(&DeletePropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id_string,
            database_id: &source_id_string,
            property_id: &title_id_string,
        })
        .await
        .expect_err(
            "legacy-first deletes must preserve opaque definitions",
        );
    canonical_app
        .update_property()
        .execute(UpdatePropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: source.id(),
            property_id: title.id(),
            name: Some("must not overwrite"),
            property_type: None,
            meta_json: None,
        })
        .await
        .expect_err("opaque definitions are read-only");
    let after_rejection: (String, String, u16, String) = sqlx::query_as(
        "SELECT field_name, type_key, type_version, type_config FROM fields \
         WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(source.id().to_string())
    .bind(title.id().to_string())
    .fetch_one(db.pool().as_ref())
    .await?;
    assert_eq!(
        after_rejection,
        (
            "renamed title".to_string(),
            "future_type".to_string(),
            7,
            opaque_config.to_string(),
        )
    );

    Ok(())
}
