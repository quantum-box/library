//! Latency probe for the data list read path used by library-api's
//! `ViewDataList::execute` (`find_all_properties` + `search_data`).
//!
//! Run against a disposable MySQL/TiDB:
//!
//! ```sh
//! DEV_DATABASE_URL=mysql://root:@127.0.0.1:4000 \
//!   cargo test -p database_manager --test data_list_latency -- --ignored --nocapture
//! ```
//!
//! `BENCH_READ_DATABASE_URL` may point the timed reads at a latency-injecting
//! proxy in front of the same server, so round trips cost what they cost in
//! production instead of the ~0.2 ms of a local socket.

use std::time::{Duration, Instant};

use database_manager::{
    domain::{PropertyType, PropertyValueCommand},
    property_definition_rollout::PropertyDefinitionStorageMode,
    property_value_rollout::PropertyValueStorageMode,
    usecase::FindAllPropertiesInputData,
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    PropertyDataInputData, SearchDataInputData,
};
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

const PROPERTIES: usize = 10;
const RECORDS: usize = 100;
const ITERATIONS: usize = 30;

fn url(var: &str) -> anyhow::Result<DatabaseUrl> {
    dotenvy::dotenv().ok();
    let fallback = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string());
    Ok(std::env::var(var)
        .unwrap_or(fallback)
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager"))
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

async fn statement_count(pool: &sqlx::MySqlPool) -> Option<i64> {
    // TiDB only; returns None on MySQL so the probe still runs there.
    sqlx::query(
        "SELECT CAST(COALESCE(SUM(EXEC_COUNT), 0) AS SIGNED) AS n \
         FROM information_schema.statements_summary \
         WHERE SCHEMA_NAME = 'tachyon_apps_database_manager' \
           AND STMT_TYPE = 'Select'",
    )
    .fetch_one(pool)
    .await
    .ok()
    .map(|row| row.get::<i64, _>("n"))
}

#[tokio::test]
#[ignore = "latency probe; requires a MySQL/TiDB configured by DEV_DATABASE_URL"]
async fn data_list_latency_probe() -> anyhow::Result<()> {
    let dsn = url("DEV_DATABASE_URL")?;
    let server = sqlx::MySqlPool::connect(
        &std::env::var("DEV_DATABASE_URL").unwrap_or_else(|_| {
            "mysql://root:@localhost:15000".to_string()
        }),
    )
    .await?;
    sqlx::query(
        "CREATE DATABASE IF NOT EXISTS tachyon_apps_database_manager",
    )
    .execute(&server)
    .await?;
    let read_dsn = url("BENCH_READ_DATABASE_URL")?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let pool = db.pool();

    let writer = database_manager::factory_client_with_property_value_mode(
        dsn.to_string(),
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let tenant_id = TenantId::default();
    let executor = &auth::Executor::SystemUser;
    let multi_tenancy =
        &auth::MultiTenancy::new_operator(tenant_id.clone());
    let database = writer
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            database_id: None,
            tenant_id: &tenant_id,
            name: "data-list-latency",
        })
        .await?;
    let mut properties = Vec::new();
    for index in 0..PROPERTIES {
        let property_type = if index % 2 == 0 {
            PropertyType::String
        } else {
            PropertyType::Markdown
        };
        properties.push(
            writer
                .add_property()
                .execute(AddPropertyInputData {
                    executor,
                    multi_tenancy,
                    tenant_id: &tenant_id,
                    database_id: database.id(),
                    display_name: None,
                    name: &format!("p{index}"),
                    property_type,
                })
                .await?,
        );
    }
    for record in 0..RECORDS {
        writer
            .add_data_usecase()
            .execute(AddDataInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                database_id: database.id(),
                name: &format!("record-{record}"),
                property_data: properties
                    .iter()
                    .enumerate()
                    .map(|(index, property)| PropertyDataInputData {
                        property_id: property.id().clone(),
                        value: if index % 2 == 0 {
                            PropertyValueCommand::String(format!(
                                "value {record}-{index}"
                            ))
                        } else {
                            PropertyValueCommand::Markdown(
                                format!("# heading {record}\n\n")
                                    + "body text ".repeat(50).as_str(),
                            )
                        },
                    })
                    .collect(),
            })
            .await?;
    }

    breakdown(&dsn, &tenant_id, database.id()).await?;

    for mode in [
        PropertyValueStorageMode::LegacyOnly,
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyValueStorageMode::DualWriteCanonicalRead,
    ] {
        let app = database_manager::factory_client_with_storage_modes(
            read_dsn.to_string(),
            mode,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        )
        .await?;
        let properties_input = || FindAllPropertiesInputData {
            tenant_id: tenant_id.clone(),
            database_id: database.id().clone(),
        };
        let search_input = SearchDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: Some(database.id().clone()),
            page: Some(1),
            page_size: Some(RECORDS as u32),
            query: "",
        };
        // `sequential` is ViewDataList::execute before it overlapped the
        // two calls; `concurrent` is the current shape.
        for shape in ["sequential", "concurrent"] {
            let list = || async {
                if shape == "sequential" {
                    let properties = app
                        .find_all_properties()
                        .execute(properties_input())
                        .await?;
                    let (data, paginator) =
                        app.search_data().execute(&search_input).await?;
                    return errors::Result::Ok((
                        properties, data, paginator,
                    ));
                }
                let (properties, data) = tokio::join!(
                    app.find_all_properties().execute(properties_input()),
                    app.search_data().execute(&search_input),
                );
                let (data, paginator) = data?;
                errors::Result::Ok((properties?, data, paginator))
            };

            // Warm the pool so connection setup is not measured.
            list().await?;
            list().await?;
            let before = statement_count(pool.as_ref()).await;
            let mut samples = Vec::with_capacity(ITERATIONS);
            for _ in 0..ITERATIONS {
                let started = Instant::now();
                let (properties, data, paginator) = list().await?;
                samples.push(started.elapsed());
                assert_eq!(properties.len(), PROPERTIES);
                assert_eq!(data.len(), RECORDS);
                assert_eq!(paginator.total_items, RECORDS as u32);
            }
            let after = statement_count(pool.as_ref()).await;
            let selects_per_list =
                before.zip(after).map(|(before, after)| {
                    (after - before) as f64 / ITERATIONS as f64
                });
            println!(
                "mode={mode:?} shape={shape} records={RECORDS} \
                 properties={PROPERTIES} median={:?} min={:?} max={:?} \
                 selects_per_list={selects_per_list:?}",
                median(samples.clone()),
                samples.iter().min().unwrap(),
                samples.iter().max().unwrap(),
            );
        }
    }
    Ok(())
}

/// Time the legacy and canonical page reads on their own, then hydration.
async fn breakdown(
    dsn: &DatabaseUrl,
    tenant_id: &TenantId,
    database_id: &database_manager::domain::DatabaseId,
) -> anyhow::Result<()> {
    use database_manager::interface_adapter::gateway::{
        hydrate_data_row, load_canonical_values_for_page, DataRow, FieldRow,
    };
    let db = persistence::Db::new(dsn.to_string()).await;
    let page = value_object::OffsetPage::new(1, RECORDS as u32)?;
    let fields = || async {
        sqlx::query_as::<_, FieldRow>(
            "SELECT * FROM fields WHERE object_id = ? and tenant_id = ?",
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .fetch_all(db.pool().as_ref())
        .await
    };
    let rows = || async {
        sqlx::query_as::<_, DataRow>(
            "SELECT * FROM data WHERE object_id = ? and tenant_id = ? \
             ORDER BY id ASC LIMIT ? OFFSET ?",
        )
        .bind(database_id.to_string())
        .bind(tenant_id.to_string())
        .bind(page.items_per_page())
        .bind(page.offset())
        .fetch_all(db.pool().as_ref())
        .await
    };
    let canonical = || {
        load_canonical_values_for_page(
            &db,
            tenant_id,
            database_id,
            page,
            PropertyValueStorageMode::DualWriteLegacyRead,
        )
    };
    rows().await?;
    canonical().await?;
    let (mut legacy_read, mut canonical_read) = (vec![], vec![]);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        rows().await?;
        legacy_read.push(started.elapsed());
        let started = Instant::now();
        canonical().await?;
        canonical_read.push(started.elapsed());
    }
    let fields = fields().await?;
    let canonical_values = canonical().await?;
    println!(
        "breakdown: legacy page read median={:?}, canonical page read \
         median={:?} ({} values)",
        median(legacy_read),
        median(canonical_read),
        canonical_values.len(),
    );
    for mode in [
        PropertyValueStorageMode::LegacyOnly,
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyValueStorageMode::DualWriteCanonicalRead,
    ] {
        let mut samples = vec![];
        for _ in 0..ITERATIONS {
            let rows = rows().await?;
            let started = Instant::now();
            for row in rows {
                hydrate_data_row(
                    row,
                    &fields,
                    &canonical_values,
                    mode,
                    PropertyDefinitionStorageMode::DualWriteLegacyRead,
                )?;
            }
            samples.push(started.elapsed());
        }
        println!(
            "breakdown: hydrate {mode:?} median={:?}",
            median(samples)
        );
    }
    Ok(())
}
