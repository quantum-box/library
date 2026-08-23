use database_manager::domain::{
    PropertyData, PropertyType, PropertyValue, PropertyValueCommand,
};
use database_manager::property_value_rollout::PropertyValueStorageMode;
use database_manager::{
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    DeletePropertyInputData, GetDataInputData, PropertyDataInputData,
    UpdateDataInputData,
};
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

fn value_for<'a>(
    data: &'a database_manager::domain::Data,
    property: &database_manager::domain::Property,
) -> &'a PropertyData {
    data.get_property_data(property.id())
        .expect("record must contain the Property projection")
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn dual_write_is_atomic_patch_safe_and_mode_aware(
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
    let app = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let canonical_app =
        database_manager::factory_client_with_property_value_mode(
            dsn.to_string(),
            PropertyValueStorageMode::DualWriteCanonicalRead,
        )
        .await?;
    let legacy_app =
        database_manager::factory_client_with_property_value_mode(
            dsn.to_string(),
            PropertyValueStorageMode::LegacyOnly,
        )
        .await?;
    let pool = db.pool();
    let tenant_id = TenantId::default();
    let executor = &auth::Executor::SystemUser;
    let multi_tenancy =
        &auth::MultiTenancy::new_operator(tenant_id.clone());

    let database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "PropertyValue dual-write regression",
        })
        .await?;
    let primary = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "primary",
            property_type: PropertyType::String,
        })
        .await?;
    let future = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "future",
            property_type: PropertyType::String,
        })
        .await?;
    let integer = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "integer",
            property_type: PropertyType::Integer,
        })
        .await?;
    let record = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "record",
            property_data: vec![
                PropertyDataInputData {
                    property_id: primary.id().clone(),
                    value: PropertyValueCommand::String(
                        "initial".to_string(),
                    ),
                },
                PropertyDataInputData {
                    property_id: future.id().clone(),
                    value: PropertyValueCommand::String(
                        "future-initial".to_string(),
                    ),
                },
                PropertyDataInputData {
                    property_id: integer.id().clone(),
                    value: PropertyValueCommand::Integer(7),
                },
            ],
        })
        .await?;

    let canonical_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM property_values \
         WHERE tenant_id = ? AND database_id = ? AND data_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(canonical_count, 3);

    // A rollback to legacy-only must not mutate a canonical row. In
    // particular, Clear is a legacy-column operation in this mode rather
    // than a request to delete canonical state that the old writer does not
    // own.
    legacy_app
        .update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            name: "legacy-only-clear",
            data: vec![PropertyDataInputData {
                property_id: primary.id().clone(),
                value: PropertyValueCommand::Clear,
            }],
        })
        .await?;
    let canonical_after_legacy_clear = sqlx::query_scalar::<_, String>(
        "SELECT value FROM property_values \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(primary.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(canonical_after_legacy_clear, "\"initial\"");
    let primary_column = format!("value{}", primary.property_num());
    let legacy_after_legacy_clear =
        sqlx::query_scalar::<_, Option<String>>(&format!(
            "SELECT {primary_column} FROM data \
             WHERE tenant_id = ? AND object_id = ? AND id = ?"
        ))
        .bind(tenant_id.to_string())
        .bind(database.id().to_string())
        .bind(record.id().to_string())
        .fetch_one(pool.as_ref())
        .await?;
    assert!(legacy_after_legacy_clear.is_none());

    // Restore parity through the dual writer before the remaining cases.
    app.update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            name: "record",
            data: vec![PropertyDataInputData {
                property_id: primary.id().clone(),
                value: PropertyValueCommand::String("initial".to_string()),
            }],
        })
        .await?;

    sqlx::query(
        "UPDATE property_values SET type_key = 'future_string', \
         value = '{\"future\":true}' \
         WHERE tenant_id = ? AND database_id = ? \
           AND data_id = ? AND property_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .bind(future.id().to_string())
    .execute(pool.as_ref())
    .await?;

    app.update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            name: "record-updated",
            data: vec![PropertyDataInputData {
                property_id: primary.id().clone(),
                value: PropertyValueCommand::String(
                    "primary-updated".to_string(),
                ),
            }],
        })
        .await?;

    let future_row: (String, String) = sqlx::query_as(
        "SELECT type_key, value FROM property_values \
         WHERE tenant_id = ? AND database_id = ? \
           AND data_id = ? AND property_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .bind(future.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(future_row.0, "future_string");
    assert_eq!(future_row.1, "{\"future\":true}");
    let future_legacy = sqlx::query_scalar::<_, Option<String>>(&format!(
        "SELECT value{} FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?",
        future.property_num()
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(future_legacy.as_deref(), Some("future-initial"));

    let get_input = GetDataInputData {
        executor,
        multi_tenancy,
        tenant_id: &tenant_id,
        database_id: database.id(),
        data_id: record.id(),
    };
    let integer_column = format!("value{}", integer.property_num());
    sqlx::query(&format!(
        "UPDATE data SET {integer_column} = 'not-an-integer' \
         WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let canonical_with_corrupt_legacy =
        canonical_app.get_data_usecase().execute(&get_input).await?;
    assert_eq!(
        value_for(&canonical_with_corrupt_legacy, &integer).string_value(),
        "7"
    );
    app.get_data_usecase()
        .execute(&get_input)
        .await
        .expect_err("legacy-first must still fail on corrupt legacy data");
    sqlx::query(
        "DELETE FROM property_values WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(integer.id().to_string())
    .execute(pool.as_ref())
    .await?;
    canonical_app
        .get_data_usecase()
        .execute(&get_input)
        .await
        .expect_err(
            "canonical-first must fail when the row is missing and legacy is corrupt",
        );
    sqlx::query(&format!(
        "UPDATE data SET {integer_column} = '7' \
         WHERE tenant_id = ? AND object_id = ? AND id = ?"
    ))
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(record.id().to_string())
    .execute(pool.as_ref())
    .await?;

    app.update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            name: "record-cleared",
            data: vec![PropertyDataInputData {
                property_id: primary.id().clone(),
                value: PropertyValueCommand::Clear,
            }],
        })
        .await?;
    let cleared_count = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM property_values \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(primary.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(cleared_count, 0);

    app.update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            name: "record-precedence",
            data: vec![PropertyDataInputData {
                property_id: primary.id().clone(),
                value: PropertyValueCommand::String(
                    "legacy-served".to_string(),
                ),
            }],
        })
        .await?;
    sqlx::query(
        "UPDATE property_values SET value = '\"canonical-served\"' \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(primary.id().to_string())
    .execute(pool.as_ref())
    .await?;

    let legacy_read = app.get_data_usecase().execute(&get_input).await?;
    assert_eq!(
        value_for(&legacy_read, &primary).string_value(),
        "legacy-served"
    );
    let canonical_read =
        canonical_app.get_data_usecase().execute(&get_input).await?;
    assert_eq!(
        value_for(&canonical_read, &primary).string_value(),
        "canonical-served"
    );

    sqlx::query(
        "UPDATE property_values SET value = '123' \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(primary.id().to_string())
    .execute(pool.as_ref())
    .await?;
    canonical_app
        .get_data_usecase()
        .execute(&get_input)
        .await
        .expect_err("known canonical decode corruption must fail closed");
    assert_eq!(
        value_for(
            &app.get_data_usecase().execute(&get_input).await?,
            &primary,
        )
        .string_value(),
        "legacy-served"
    );

    sqlx::query(
        "UPDATE property_values SET type_key = 'future_string', \
         value = '{\"future\":true}' \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(primary.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let opaque_read =
        canonical_app.get_data_usecase().execute(&get_input).await?;
    let opaque_value = value_for(&opaque_read, &primary);
    assert!(opaque_value.value().is_none());
    assert!(matches!(
        opaque_value.envelope(),
        Some(PropertyValue::Opaque(_))
    ));

    let opaque_error = app
        .update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            name: "must-roll-back",
            data: vec![PropertyDataInputData {
                property_id: primary.id().clone(),
                value: PropertyValueCommand::String(
                    "must-not-write".to_string(),
                ),
            }],
        })
        .await
        .expect_err("opaque target values are read-only");
    assert!(opaque_error.to_string().contains("unsupported"));
    let record_after_opaque_error: (String, Option<String>) =
        sqlx::query_as(&format!(
            "SELECT name, {primary_column} FROM data \
             WHERE tenant_id = ? AND object_id = ? AND id = ?"
        ))
        .bind(tenant_id.to_string())
        .bind(database.id().to_string())
        .bind(record.id().to_string())
        .fetch_one(pool.as_ref())
        .await?;
    assert_eq!(record_after_opaque_error.0, "record-precedence");
    assert_eq!(
        record_after_opaque_error.1.as_deref(),
        Some("legacy-served")
    );
    let canonical_after_opaque_error: (String, String) = sqlx::query_as(
        "SELECT type_key, value FROM property_values \
         WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(primary.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(
        canonical_after_opaque_error,
        ("future_string".to_string(), "{\"future\":true}".to_string())
    );

    // A rich text document -- including the empty paragraph the type
    // exists to preserve -- must come back byte-identical from the legacy
    // reader (JSON text in a LONGTEXT cell) and the canonical reader (a
    // real JSON array in property_values).
    let body = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            name: "body",
            property_type: PropertyType::RichText,
        })
        .await?;
    let document = serde_json::json!([
        {
            "id": "block-1",
            "type": "paragraph",
            "props": {},
            "content": [
                { "type": "text", "text": "line1", "styles": {} }
            ],
            "children": [],
        },
        { "id": "block-2", "type": "paragraph", "props": {},
          "content": [], "children": [] },
        {
            "id": "block-3",
            "type": "paragraph",
            "props": {},
            "content": [
                { "type": "text", "text": "line2", "styles": {} }
            ],
            "children": [],
        },
    ]);
    app.update_data_usecase()
        .execute(UpdateDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: record.id(),
            name: "record-rich-text",
            data: vec![PropertyDataInputData {
                property_id: body.id().clone(),
                value: PropertyValueCommand::RichText(document.clone()),
            }],
        })
        .await?;
    for reader in [&legacy_app, &app, &canonical_app] {
        let record_read = reader
            .get_data_usecase()
            .execute(&GetDataInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                database_id: database.id(),
                data_id: record.id(),
            })
            .await?;
        let value = value_for(&record_read, &body);
        assert_eq!(
            value.value(),
            &Some(
                database_manager::domain::PropertyDataValue::RichText(
                    document.clone()
                )
            ),
            "every storage mode must return the identical document"
        );
    }
    let canonical_rich_text: (String, String) = sqlx::query_as(
        "SELECT type_key, value FROM property_values          WHERE data_id = ? AND property_id = ?",
    )
    .bind(record.id().to_string())
    .bind(body.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(canonical_rich_text.0, "rich_text");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&canonical_rich_text.1)?,
        document
    );

    let tenant = tenant_id.to_string();
    let database_id = database.id().to_string();
    let property_id = primary.id().to_string();
    app.delete_property_usecase()
        .execute(&DeletePropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant,
            database_id: &database_id,
            property_id: &property_id,
        })
        .await?;
    let remaining = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM fields \
         WHERE tenant_id = ? AND object_id = ? AND id = ?",
    )
    .bind(&tenant)
    .bind(&database_id)
    .bind(&property_id)
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(remaining, 0);
    let legacy_after_delete =
        sqlx::query_scalar::<_, Option<String>>(&format!(
            "SELECT value{} FROM data WHERE tenant_id = ? AND object_id = ? AND id = ?",
            primary.property_num()
        ))
        .bind(&tenant)
        .bind(&database_id)
        .bind(record.id().to_string())
        .fetch_one(pool.as_ref())
        .await?;
    assert!(legacy_after_delete.is_none());

    Ok(())
}
