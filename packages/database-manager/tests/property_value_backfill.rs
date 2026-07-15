use database_manager::domain::{
    Data, Database, Property, PropertyType, PropertyValueCommand,
};
use database_manager::interface_adapter::gateway::PropertyValueBackfillGateway;
use database_manager::property_value_rollout::PropertyValueStorageMode;
use database_manager::usecase::PropertyValueBackfillInteractor;
use database_manager::{
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    PropertyDataInputData, PropertyValueBackfillInputData,
    PropertyValueBackfillInputPort,
};
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

async fn create_database(
    app: &database_manager::App,
    tenant_id: &TenantId,
    name: &str,
) -> errors::Result<Database> {
    app.create_database()
        .execute(CreateDatabaseInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &auth::MultiTenancy::new_operator(
                tenant_id.clone(),
            ),
            database_id: None,
            tenant_id,
            name,
        })
        .await
}

async fn add_property(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database: &Database,
    name: &str,
    property_type: PropertyType,
) -> errors::Result<Property> {
    app.add_property()
        .execute(AddPropertyInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &auth::MultiTenancy::new_operator(
                tenant_id.clone(),
            ),
            tenant_id,
            database_id: database.id(),
            name,
            property_type,
        })
        .await
}

async fn add_record(
    app: &database_manager::App,
    tenant_id: &TenantId,
    database: &Database,
    name: &str,
    values: Vec<(&Property, PropertyValueCommand)>,
) -> errors::Result<Data> {
    app.add_data_usecase()
        .execute(AddDataInputData {
            executor: &auth::Executor::SystemUser,
            multi_tenancy: &auth::MultiTenancy::new_operator(
                tenant_id.clone(),
            ),
            tenant_id,
            database_id: database.id(),
            name,
            property_data: values
                .into_iter()
                .map(|(property, value)| PropertyDataInputData {
                    property_id: property.id().clone(),
                    value,
                })
                .collect(),
        })
        .await
}

fn checksum_bytes(value: &str) -> [u8; 32] {
    assert_eq!(value.len(), 64);
    let mut checksum = [0; 32];
    for (index, byte) in checksum.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .expect("checksum hex");
    }
    checksum
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn backfill_is_resumable_idempotent_opaque_safe_and_atomic(
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
    let legacy_app =
        database_manager::factory_client_with_property_value_mode(
            dsn.to_string(),
            PropertyValueStorageMode::LegacyOnly,
        )
        .await?;
    let backfill = PropertyValueBackfillInteractor::new(
        PropertyValueBackfillGateway::new(db.clone()),
    );
    let pool = db.pool();
    let tenant_id = TenantId::default();

    // Resume and idempotency: dry-run never writes, an apply resumes after an
    // exclusive DataId cursor, and reruns report only matches.
    let database =
        create_database(&legacy_app, &tenant_id, "backfill resume").await?;
    let property = add_property(
        &legacy_app,
        &tenant_id,
        &database,
        "value",
        PropertyType::String,
    )
    .await?;
    for value in ["alpha", "beta", "gamma"] {
        add_record(
            &legacy_app,
            &tenant_id,
            &database,
            value,
            vec![(
                &property,
                PropertyValueCommand::String(value.to_string()),
            )],
        )
        .await?;
    }

    let dry_run = backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: database.id(),
            after_data_id: None,
            batch_size: 2,
            dry_run: true,
            checksum_seed: [0; 32],
        })
        .await?;
    assert_eq!(dry_run.scanned_records, 2);
    assert_eq!(dry_run.missing_values, 2);
    assert_eq!(dry_run.written_values, 0);
    assert!(!dry_run.complete);
    let canonical_after_dry_run = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM property_values \
         WHERE tenant_id = ? AND database_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(canonical_after_dry_run, 0);

    let first_apply = backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: database.id(),
            after_data_id: None,
            batch_size: 2,
            dry_run: false,
            checksum_seed: [0; 32],
        })
        .await?;
    assert_eq!(first_apply.written_values, 2);
    let resume_cursor = first_apply
        .next_cursor
        .as_ref()
        .expect("non-final chunk cursor");
    let resumed = backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: database.id(),
            after_data_id: Some(resume_cursor),
            batch_size: 2,
            dry_run: false,
            checksum_seed: checksum_bytes(&first_apply.parity_checksum),
        })
        .await?;
    assert_eq!(resumed.scanned_records, 1);
    assert_eq!(resumed.written_values, 1);
    assert!(resumed.complete);

    let full_rerun = backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: database.id(),
            after_data_id: None,
            batch_size: 3,
            dry_run: false,
            checksum_seed: [0; 32],
        })
        .await?;
    assert_eq!(full_rerun.matched_values, 3);
    assert_eq!(full_rerun.missing_values, 0);
    assert_eq!(full_rerun.written_values, 0);
    assert_eq!(
        resumed.parity_checksum, full_rerun.parity_checksum,
        "checksum must be independent of chunk boundaries"
    );
    let second_rerun = backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: database.id(),
            after_data_id: None,
            batch_size: 3,
            dry_run: false,
            checksum_seed: [0; 32],
        })
        .await?;
    assert_eq!(second_rerun.parity_checksum, full_rerun.parity_checksum);

    // Opaque/future rows remain byte-for-byte untouched and are included in
    // the value-free parity evidence.
    let opaque_database =
        create_database(&legacy_app, &tenant_id, "backfill opaque").await?;
    let opaque_property = add_property(
        &legacy_app,
        &tenant_id,
        &opaque_database,
        "future",
        PropertyType::String,
    )
    .await?;
    let opaque_record = add_record(
        &legacy_app,
        &tenant_id,
        &opaque_database,
        "opaque",
        vec![(
            &opaque_property,
            PropertyValueCommand::String("legacy".into()),
        )],
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO property_values
            (tenant_id, database_id, data_id, property_id, type_key,
             type_version, value_encoding_version, value)
        VALUES (?, ?, ?, ?, 'future_string', 1, 1, '{"future":true}')
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(opaque_database.id().to_string())
    .bind(opaque_record.id().to_string())
    .bind(opaque_property.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let opaque_report = backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: opaque_database.id(),
            after_data_id: None,
            batch_size: 10,
            dry_run: false,
            checksum_seed: [0; 32],
        })
        .await?;
    assert_eq!(opaque_report.opaque_values, 1);
    assert_eq!(opaque_report.written_values, 0);
    let opaque_after = sqlx::query_as::<_, (String, String)>(
        "SELECT type_key, value FROM property_values \
         WHERE tenant_id = ? AND database_id = ? \
           AND data_id = ? AND property_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(opaque_database.id().to_string())
    .bind(opaque_record.id().to_string())
    .bind(opaque_property.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(opaque_after.0, "future_string");
    assert_eq!(opaque_after.1, r#"{"future":true}"#);

    // A known mismatch fails closed. The missing first Property is inserted
    // before the mismatch is detected, proving the whole chunk rolls back.
    let mismatch_database =
        create_database(&legacy_app, &tenant_id, "backfill mismatch")
            .await?;
    let before_mismatch = add_property(
        &legacy_app,
        &tenant_id,
        &mismatch_database,
        "before",
        PropertyType::String,
    )
    .await?;
    let mismatched = add_property(
        &legacy_app,
        &tenant_id,
        &mismatch_database,
        "mismatch",
        PropertyType::String,
    )
    .await?;
    let mismatch_record = add_record(
        &legacy_app,
        &tenant_id,
        &mismatch_database,
        "mismatch",
        vec![
            (
                &before_mismatch,
                PropertyValueCommand::String("first".into()),
            ),
            (&mismatched, PropertyValueCommand::String("legacy".into())),
        ],
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO property_values
            (tenant_id, database_id, data_id, property_id, type_key,
             type_version, value_encoding_version, value)
        VALUES (?, ?, ?, ?, 'string', 1, 1, '"different"')
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(mismatch_database.id().to_string())
    .bind(mismatch_record.id().to_string())
    .bind(mismatched.id().to_string())
    .execute(pool.as_ref())
    .await?;
    let mismatch_error = backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: mismatch_database.id(),
            after_data_id: None,
            batch_size: 10,
            dry_run: false,
            checksum_seed: [0; 32],
        })
        .await
        .expect_err("known mismatch must stop the chunk");
    assert!(mismatch_error.to_string().contains("parity mismatch"));
    let rolled_back_insert = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM property_values \
         WHERE tenant_id = ? AND database_id = ? \
           AND data_id = ? AND property_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(mismatch_database.id().to_string())
    .bind(mismatch_record.id().to_string())
    .bind(before_mismatch.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(rolled_back_insert, 0);

    // Corrupt legacy storage also fails closed and rolls back earlier work in
    // the same chunk.
    let corrupt_database =
        create_database(&legacy_app, &tenant_id, "backfill corrupt")
            .await?;
    let before_corrupt = add_property(
        &legacy_app,
        &tenant_id,
        &corrupt_database,
        "before",
        PropertyType::String,
    )
    .await?;
    let corrupt_integer = add_property(
        &legacy_app,
        &tenant_id,
        &corrupt_database,
        "integer",
        PropertyType::Integer,
    )
    .await?;
    let corrupt_record = add_record(
        &legacy_app,
        &tenant_id,
        &corrupt_database,
        "corrupt",
        vec![
            (
                &before_corrupt,
                PropertyValueCommand::String("first".into()),
            ),
            (&corrupt_integer, PropertyValueCommand::Integer(7)),
        ],
    )
    .await?;
    sqlx::query(&format!(
        "UPDATE data SET value{} = 'not-an-integer' \
         WHERE tenant_id = ? AND object_id = ? AND id = ?",
        corrupt_integer.property_num()
    ))
    .bind(tenant_id.to_string())
    .bind(corrupt_database.id().to_string())
    .bind(corrupt_record.id().to_string())
    .execute(pool.as_ref())
    .await?;
    backfill
        .execute(&PropertyValueBackfillInputData {
            tenant_id: &tenant_id,
            database_id: corrupt_database.id(),
            after_data_id: None,
            batch_size: 10,
            dry_run: false,
            checksum_seed: [0; 32],
        })
        .await
        .expect_err("corrupt legacy value must stop the chunk");
    let corrupt_rollback = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(COUNT(*) AS SIGNED) FROM property_values \
         WHERE tenant_id = ? AND database_id = ? \
           AND data_id = ? AND property_id = ?",
    )
    .bind(tenant_id.to_string())
    .bind(corrupt_database.id().to_string())
    .bind(corrupt_record.id().to_string())
    .bind(before_corrupt.id().to_string())
    .fetch_one(pool.as_ref())
    .await?;
    assert_eq!(corrupt_rollback, 0);

    Ok(())
}
