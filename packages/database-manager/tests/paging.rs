use std::collections::HashSet;

use database_manager::{
    domain::{Data, DatabaseId, PropertyType, PropertyValueCommand},
    AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
    PropertyDataInputData, SearchDataInputData,
};
use tachyon_sdk::auth::{self, ExecutorAction, MultiTenancyAction};
use value_object::{DatabaseUrl, OffsetPaginator, TenantId, MAX_PAGE_SIZE};

#[allow(clippy::too_many_arguments)]
async fn search(
    app: &database_manager::App,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    query: &str,
    page: u32,
    page_size: u32,
) -> errors::Result<(Vec<Data>, OffsetPaginator)> {
    app.search_data()
        .execute(&SearchDataInputData {
            executor,
            multi_tenancy,
            tenant_id,
            database_id: Some(database_id.clone()),
            query,
            page: Some(page),
            page_size: Some(page_size),
        })
        .await
}

#[tokio::test]
#[ignore]
async fn paging_is_one_origin_stable_and_filtered() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let tenant_id = TenantId::default();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager");
    let app = database_manager::factory_client(&dsn).await?;
    let pool = sqlx::MySqlPool::connect(&dsn.to_string()).await?;

    let executor = &auth::Executor::SystemUser;
    let multi_tenancy =
        &auth::MultiTenancy::new(None, Some(tenant_id.clone()));

    let database = app
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: None,
            name: "paging regression",
        })
        .await?;
    let property = app
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            display_name: None,
            name: "value",
            property_type: PropertyType::String,
        })
        .await?;

    let (empty, paginator) = search(
        &app,
        executor,
        multi_tenancy,
        &tenant_id,
        database.id(),
        "",
        1,
        7,
    )
    .await?;
    assert!(empty.is_empty());
    assert_eq!(paginator.total_items, 0);
    assert_eq!(paginator.total_pages, 0);
    assert_eq!(paginator.current_page, 1);

    let page_zero = search(
        &app,
        executor,
        multi_tenancy,
        &tenant_id,
        database.id(),
        "",
        0,
        7,
    )
    .await
    .expect_err("page zero must be rejected");
    assert!(page_zero.is_bad_request());

    for invalid_page_size in [0, MAX_PAGE_SIZE + 1] {
        let error = search(
            &app,
            executor,
            multi_tenancy,
            &tenant_id,
            database.id(),
            "",
            1,
            invalid_page_size,
        )
        .await
        .expect_err("an invalid page size must be rejected");
        assert!(error.is_bad_request());
    }

    let mut all_expected_ids = Vec::new();
    let mut matching_expected_ids = Vec::new();
    for index in 0..23 {
        let data = app
            .add_data_usecase()
            .execute(AddDataInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                database_id: database.id(),
                name: "matching",
                property_data: vec![PropertyDataInputData {
                    property_id: property.id().clone(),
                    value: PropertyValueCommand::String(format!(
                        "matching-{index}"
                    )),
                }],
            })
            .await?;
        let id = data.id().to_string();
        all_expected_ids.push(id.clone());
        matching_expected_ids.push(id);
    }
    for index in 0..8 {
        let data = app
            .add_data_usecase()
            .execute(AddDataInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                database_id: database.id(),
                name: &format!("other-{index}"),
                property_data: vec![PropertyDataInputData {
                    property_id: property.id().clone(),
                    value: PropertyValueCommand::String(format!(
                        "other-{index}"
                    )),
                }],
            })
            .await?;
        all_expected_ids.push(data.id().to_string());
    }

    // Give every filtered row the same primary sort value so the id
    // tie-breaker is exercised rather than merely present in the SQL text.
    sqlx::query(
        r#"
        UPDATE tachyon_apps_database_manager.data
        SET created_at = '2026-01-01 00:00:00'
        WHERE tenant_id = ? AND object_id = ? AND name = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind("matching")
    .execute(&pool)
    .await?;

    let indexed_columns = sqlx::query_scalar::<_, String>(
        r#"
        SELECT column_name
        FROM information_schema.statistics
        WHERE table_schema = ? AND table_name = ? AND index_name = ?
        ORDER BY seq_in_index
        "#,
    )
    .bind("tachyon_apps_database_manager")
    .bind("data")
    .bind("idx_data_tenant_object_name_id")
    .fetch_all(&pool)
    .await?;
    assert_eq!(indexed_columns, ["tenant_id", "object_id", "name", "id"]);

    let mut all_actual_ids = Vec::new();
    for page in 1..=5 {
        let (data, paginator) = search(
            &app,
            executor,
            multi_tenancy,
            &tenant_id,
            database.id(),
            "",
            page,
            7,
        )
        .await?;
        assert_eq!(paginator.current_page, page);
        assert_eq!(paginator.items_per_page, 7);
        assert_eq!(paginator.total_items, 31);
        assert_eq!(paginator.total_pages, 5);
        assert_eq!(data.len(), if page == 5 { 3 } else { 7 });
        all_actual_ids
            .extend(data.into_iter().map(|data| data.id().to_string()));
    }
    assert_eq!(
        all_actual_ids.iter().collect::<HashSet<_>>().len(),
        all_actual_ids.len(),
        "pages must not contain duplicate records"
    );
    all_expected_ids.sort();
    assert_eq!(all_actual_ids, all_expected_ids);

    let (out_of_range, paginator) = search(
        &app,
        executor,
        multi_tenancy,
        &tenant_id,
        database.id(),
        "",
        6,
        7,
    )
    .await?;
    assert!(out_of_range.is_empty());
    assert_eq!(paginator.current_page, 6);
    assert_eq!(paginator.total_pages, 5);

    let mut matching_actual_ids = Vec::new();
    for page in 1..=5 {
        let (data, paginator) = search(
            &app,
            executor,
            multi_tenancy,
            &tenant_id,
            database.id(),
            "matching",
            page,
            5,
        )
        .await?;
        assert_eq!(paginator.current_page, page);
        assert_eq!(paginator.items_per_page, 5);
        assert_eq!(paginator.total_items, 23);
        assert_eq!(paginator.total_pages, 5);
        assert_eq!(data.len(), if page == 5 { 3 } else { 5 });
        matching_actual_ids
            .extend(data.into_iter().map(|data| data.id().to_string()));
    }
    assert_eq!(
        matching_actual_ids.iter().collect::<HashSet<_>>().len(),
        matching_actual_ids.len(),
        "filtered pages must not contain duplicate records"
    );
    matching_expected_ids.sort();
    assert_eq!(matching_actual_ids, matching_expected_ids);

    let (missing, paginator) = search(
        &app,
        executor,
        multi_tenancy,
        &tenant_id,
        database.id(),
        "missing",
        1,
        5,
    )
    .await?;
    assert!(missing.is_empty());
    assert_eq!(paginator.total_items, 0);
    assert_eq!(paginator.total_pages, 0);

    pool.close().await;
    Ok(())
}

/// The concurrent list read loads canonical values for the page by
/// re-selecting the page in SQL instead of by the fetched ids. Every mode must
/// still hydrate each listed record exactly as the single-record read does,
/// and canonical-first reads must pick the canonical row of the right record.
#[tokio::test]
#[ignore]
async fn paged_list_hydrates_like_single_reads_in_every_storage_mode(
) -> anyhow::Result<()> {
    use database_manager::{
        property_definition_rollout::PropertyDefinitionStorageMode,
        property_value_rollout::PropertyValueStorageMode, GetDataInputData,
    };

    dotenvy::dotenv().ok();
    let tenant_id = TenantId::default();
    let dsn: DatabaseUrl = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager");
    let writer = database_manager::factory_client_with_property_value_mode(
        &dsn,
        PropertyValueStorageMode::DualWriteLegacyRead,
    )
    .await?;
    let pool = sqlx::MySqlPool::connect(&dsn.to_string()).await?;
    let executor = &auth::Executor::SystemUser;
    let multi_tenancy =
        &auth::MultiTenancy::new(None, Some(tenant_id.clone()));

    let database = writer
        .create_database()
        .execute(CreateDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: None,
            name: "paged hydration",
        })
        .await?;
    let property = writer
        .add_property()
        .execute(AddPropertyInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            display_name: None,
            name: "value",
            property_type: PropertyType::String,
        })
        .await?;
    let mut ids = Vec::new();
    for index in 0..12 {
        let data = writer
            .add_data_usecase()
            .execute(AddDataInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                database_id: database.id(),
                name: &format!("record-{index}"),
                property_data: vec![PropertyDataInputData {
                    property_id: property.id().clone(),
                    value: PropertyValueCommand::String(format!(
                        "legacy-{index}"
                    )),
                }],
            })
            .await?;
        ids.push(data.id().clone());
    }
    ids.sort_by_key(|id| id.to_string());

    // Diverge the canonical copy of the first record on page 2 only.
    let diverged = ids[5].clone();
    let updated = sqlx::query(
        r#"
        UPDATE tachyon_apps_database_manager.property_values
        SET value = JSON_QUOTE('canonical-only')
        WHERE tenant_id = ? AND database_id = ? AND data_id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(diverged.to_string())
    .execute(&pool)
    .await?;
    assert_eq!(updated.rows_affected(), 1);

    for mode in [
        PropertyValueStorageMode::LegacyOnly,
        PropertyValueStorageMode::DualWriteLegacyRead,
        PropertyValueStorageMode::DualWriteCanonicalRead,
    ] {
        let app = database_manager::factory_client_with_storage_modes(
            &dsn,
            mode,
            PropertyDefinitionStorageMode::DualWriteLegacyRead,
        )
        .await?;
        let mut listed = Vec::new();
        for page in 1..=3 {
            let (data, paginator) = search(
                &app,
                executor,
                multi_tenancy,
                &tenant_id,
                database.id(),
                "",
                page,
                5,
            )
            .await?;
            assert_eq!(paginator.total_items, 12);
            listed.extend(data);
        }
        assert_eq!(
            listed
                .iter()
                .map(|data| data.id().clone())
                .collect::<Vec<_>>(),
            ids,
            "{mode:?}: pages must list every record once, in id order"
        );
        for data in &listed {
            let single = app
                .get_data_usecase()
                .execute(&GetDataInputData {
                    executor,
                    multi_tenancy,
                    tenant_id: &tenant_id,
                    database_id: database.id(),
                    data_id: data.id(),
                })
                .await?;
            let value = |data: &Data| {
                data.get_property_data(property.id())
                    .map(|value| value.string_value())
            };
            assert_eq!(value(data), value(&single), "{mode:?}");
            let expected_canonical = data.id() == &diverged
                && mode == PropertyValueStorageMode::DualWriteCanonicalRead;
            assert_eq!(
                value(data).as_deref() == Some("canonical-only"),
                expected_canonical,
                "{mode:?}: only canonical-first reads see the canonical copy, \
                 and only for its own record"
            );
        }
    }

    pool.close().await;
    Ok(())
}
