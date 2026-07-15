use database_manager::{
    AddDataInputData, CreateDatabaseInputData, GetDataInputData,
    GetDatabaseInputData, SearchDataInputData,
};
use sqlx::Row;
use tachyon_sdk::auth;
use value_object::{DatabaseUrl, TenantId};

fn database_url() -> anyhow::Result<DatabaseUrl> {
    dotenvy::dotenv().ok();
    Ok(std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@localhost:15000".to_string())
        .parse::<DatabaseUrl>()?
        .use_database("tachyon_apps_database_manager"))
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn record_version_column_is_nonzero_unsigned_and_defaults_to_one(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;

    let column = sqlx::query(
        r#"
        SELECT
            CAST(DATA_TYPE AS CHAR) AS data_type_text,
            CAST(COLUMN_TYPE AS CHAR) AS column_type_text,
            CAST(IS_NULLABLE AS CHAR) AS nullable_text,
            CAST(COLUMN_DEFAULT AS CHAR) AS default_text
        FROM information_schema.COLUMNS
        WHERE TABLE_SCHEMA = DATABASE()
          AND TABLE_NAME = 'data'
          AND COLUMN_NAME = 'record_version'
        "#,
    )
    .fetch_one(db.pool().as_ref())
    .await?;

    assert_eq!(column.try_get::<String, _>("data_type_text")?, "bigint");
    assert_eq!(
        column.try_get::<String, _>("column_type_text")?,
        "bigint unsigned"
    );
    assert_eq!(column.try_get::<String, _>("nullable_text")?, "NO");
    assert_eq!(column.try_get::<String, _>("default_text")?, "1");

    let check_clause = sqlx::query_scalar::<_, String>(
        r#"
        SELECT CAST(CHECK_CLAUSE AS CHAR)
        FROM information_schema.CHECK_CONSTRAINTS
        WHERE CONSTRAINT_SCHEMA = DATABASE()
          AND CONSTRAINT_NAME = 'chk_data_record_version_nonzero'
        "#,
    )
    .fetch_one(db.pool().as_ref())
    .await?;
    assert!(
        check_clause
            .split_whitespace()
            .collect::<String>()
            .contains("`record_version`>0"),
        "unexpected CHECK clause: {check_clause}"
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires a MySQL database configured by DEV_DATABASE_URL"]
async fn persisted_version_reaches_every_database_read_path(
) -> anyhow::Result<()> {
    let dsn = database_url()?;
    let db = persistence::Db::new(dsn.to_string()).await;
    sqlx::migrate!("./migrations")
        .run(db.pool().as_ref())
        .await?;
    let app = database_manager::factory_client(&dsn).await?;

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
            name: "record-version-read-paths",
        })
        .await?;
    let created = app
        .add_data_usecase()
        .execute(AddDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            name: "versioned-record",
            property_data: vec![],
            database_id: database.id(),
        })
        .await?;
    assert_eq!(created.record_version().get(), 1);

    sqlx::query(
        r#"
        UPDATE data
        SET record_version = 7
        WHERE tenant_id = ? AND object_id = ? AND id = ?
        "#,
    )
    .bind(tenant_id.to_string())
    .bind(database.id().to_string())
    .bind(created.id().to_string())
    .execute(db.pool().as_ref())
    .await?;

    let by_id = app
        .get_data_usecase()
        .execute(&GetDataInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            data_id: created.id(),
        })
        .await?;
    assert_eq!(by_id.record_version().get(), 7);

    let (_, _, paged, _) = app
        .get_database_usecase()
        .execute(GetDatabaseInputData {
            executor,
            multi_tenancy,
            tenant_id: &tenant_id,
            database_id: database.id(),
            page: Some(1),
            page_size: Some(20),
        })
        .await?;
    assert_eq!(paged.len(), 1);
    assert_eq!(paged[0].record_version().get(), 7);

    for query in ["", "versioned-record"] {
        let (results, _) = app
            .search_data()
            .execute(&SearchDataInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                database_id: Some(database.id().clone()),
                query,
                page: Some(1),
                page_size: Some(20),
            })
            .await?;
        assert_eq!(results.len(), 1, "query={query:?}");
        assert_eq!(results[0].record_version().get(), 7);
    }

    Ok(())
}
