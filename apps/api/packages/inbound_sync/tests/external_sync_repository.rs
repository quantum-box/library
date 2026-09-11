use std::{str::FromStr, sync::Arc};

use chrono::{TimeZone, Utc};
use inbound_sync::interface_adapter::{
    SqlxExternalObjectLinkRepository, SqlxExternalSyncBindingRepository,
    SqlxExternalSyncDispatchRepository,
    SqlxExternalSyncLifecycleRepository,
};
use integration_domain::{
    ConnectionId, ExternalChangeType, ExternalDeletePolicy,
    ExternalObjectLink, ExternalObjectLinkRepository, ExternalScope,
    ExternalSyncBinding, ExternalSyncBindingId,
    ExternalSyncBindingRepository, ExternalSyncBindingStatus,
    ExternalSyncPolicy, InboundChangeSet, InboundChangeSetRepository,
    LibraryDataId, LibraryRepoId, OAuthProvider, OutboundDelivery,
    OutboundDeliveryRepository, OutboundDeliveryStatus,
};
use serde_json::json;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};

#[tokio::test]
#[ignore = "requires a local MySQL database configured by DEV_DATABASE_URL"]
async fn repositories_round_trip_and_enforce_tenant_scope(
) -> Result<(), Box<dyn std::error::Error>> {
    let database_name = format!(
        "library_external_sync_test_{}",
        ulid::Ulid::new().to_string().to_lowercase()
    );
    let dsn = std::env::var("DEV_DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:@127.0.0.1:15000/library".into());
    let options = MySqlConnectOptions::from_str(&dsn)?;
    let admin = MySqlPoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone().database("mysql"))
        .await?;
    sqlx::query(&format!(
        "CREATE DATABASE `{database_name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci"
    ))
        .execute(&admin)
        .await?;

    let test_result = async {
        let pool = MySqlPoolOptions::new()
            .max_connections(2)
            .connect_with(options.database(&database_name))
            .await?;
        sqlx::raw_sql(
            r#"
            CREATE TABLE repos (
                id VARCHAR(29) NOT NULL PRIMARY KEY,
                org_id VARCHAR(29) NOT NULL
            );
            CREATE TABLE integration_connections (
                id VARCHAR(32) NOT NULL PRIMARY KEY,
                tenant_id VARCHAR(29) NOT NULL,
                provider VARCHAR(32) NOT NULL
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
              COLLATE=utf8mb4_unicode_ci;
            CREATE TABLE webhook_events (
                id VARCHAR(30) NOT NULL PRIMARY KEY
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
              COLLATE=utf8mb4_unicode_ci;
            "#,
        )
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../migrations/20260911000000_create_external_sync_model.up.sql"
        ))
        .execute(&pool)
        .await?;
        sqlx::raw_sql(include_str!(
            "../../../migrations/20260911010000_create_external_sync_lifecycle.up.sql"
        ))
        .execute(&pool)
        .await?;

        let tenant_id: value_object::TenantId =
            "tn_01j91h09tpj5ehwbwfwfxpak2b".parse().unwrap();
        let repo_id =
            LibraryRepoId::parse("rp_01j91h09tpj5ehwbwfwfxpak2b")?;
        let connection_id =
            ConnectionId::new("con_01j91h09tpj5ehwbwfwfxpak2b");
        sqlx::query("INSERT INTO repos (id, org_id) VALUES (?, ?)")
            .bind(repo_id.as_str())
            .bind(tenant_id.to_string())
            .execute(&pool)
            .await?;
        sqlx::query(
            "INSERT INTO integration_connections (id, tenant_id, provider) \
             VALUES (?, ?, 'github')",
        )
        .bind(connection_id.as_str())
        .bind(tenant_id.to_string())
        .execute(&pool)
        .await?;

        let scope = ExternalScope::new(json!({
            "github_repository": "example/docs",
            "ref": "main",
            "path_pattern": "docs/**/*.md"
        }))?;
        let now = Utc.with_ymd_and_hms(2026, 9, 11, 0, 0, 0).unwrap();
        let binding = ExternalSyncBinding::new(
            ExternalSyncBindingId::parse(
                "esb_01j91h09tpj5ehwbwfwfxpak2b",
            )?,
            tenant_id.clone(),
            repo_id.clone(),
            OAuthProvider::Github,
            connection_id,
            scope.clone(),
            "markdown_document",
            json!({"content": "body"}),
            ExternalSyncPolicy::Review,
            ExternalSyncPolicy::Disabled,
            ExternalDeletePolicy::ReviewTombstone,
            ExternalSyncBindingStatus::Active,
            now,
            now,
        )?;
        let bindings =
            SqlxExternalSyncBindingRepository::new(Arc::new(pool.clone()));
        bindings.save(&binding).await?;

        let restored = bindings
            .find_by_scope(
                &tenant_id,
                &repo_id,
                OAuthProvider::Github,
                &ExternalScope::new(json!({
                    "path_pattern": "docs/**/*.md",
                    "ref": "main",
                    "github_repository": "example/docs"
                }))?,
            )
            .await?
            .expect("binding should round-trip");
        assert_eq!(restored.id(), binding.id());
        assert_eq!(restored.external_scope(), &scope);
        assert_eq!(restored.outbound_policy(), ExternalSyncPolicy::Disabled);

        let duplicate_scope = ExternalSyncBinding::new(
            ExternalSyncBindingId::parse(
                "esb_01j91h09tpj5ehwbwfwfxpak2c",
            )?,
            tenant_id.clone(),
            repo_id.clone(),
            OAuthProvider::Github,
            binding.connection_id().clone(),
            scope.clone(),
            "markdown_document",
            json!({"content": "body"}),
            ExternalSyncPolicy::Review,
            ExternalSyncPolicy::Review,
            ExternalDeletePolicy::ReviewTombstone,
            ExternalSyncBindingStatus::Active,
            now,
            now,
        )?;
        assert!(matches!(
            bindings.save(&duplicate_scope).await,
            Err(errors::Error::Conflict { .. })
        ));

        let link = ExternalObjectLink::new(
            binding.id().clone(),
            LibraryDataId::parse("data_01j91h09tpj5ehwbwfwfxpak2b")?,
            "docs/guide.md",
            Some("github-sha".into()),
            Some("library-revision".into()),
            Some("a".repeat(64)),
        )?;
        let links =
            SqlxExternalObjectLinkRepository::new(Arc::new(pool.clone()));
        links.save(&tenant_id, &link).await?;
        let restored_link = links
            .find_by_external_object(
                &tenant_id,
                binding.id(),
                "docs/guide.md",
            )
            .await?
            .expect("object link should round-trip");
        assert_eq!(restored_link, link);

        let duplicate_object = ExternalObjectLink::new(
            binding.id().clone(),
            LibraryDataId::parse("data_01j91h09tpj5ehwbwfwfxpak2c")?,
            "docs/guide.md",
            None,
            None,
            None,
        )?;
        assert!(matches!(
            links.save(&tenant_id, &duplicate_object).await,
            Err(errors::Error::Conflict { .. })
        ));

        let other_tenant: value_object::TenantId =
            "tn_01j91h09tpj5ehwbwfwfxpak2c".parse().unwrap();
        assert!(bindings
            .find_by_id(&other_tenant, binding.id())
            .await?
            .is_none());
        assert!(links
            .find_by_data(&other_tenant, binding.id(), link.data_id())
            .await?
            .is_none());

        let lifecycle =
            SqlxExternalSyncLifecycleRepository::new(Arc::new(pool.clone()));
        let mut change = InboundChangeSet::create(
            tenant_id.clone(),
            binding.id().clone(),
            Some(link.data_id().clone()),
            "docs/guide.md",
            "github-sha-2",
            Some("github-sha".into()),
            ExternalChangeType::Upsert,
            json!({"content": "updated"}),
        )?;
        InboundChangeSetRepository::save(&lifecycle, &change).await?;
        let restored_change = InboundChangeSetRepository::find_by_id(
            &lifecycle,
            &tenant_id,
            change.id(),
        )
            .await?
            .expect("change set should round-trip");
        assert_eq!(restored_change.status().as_str(), "pending");
        change.accept(Some("reviewed".into()))?;
        InboundChangeSetRepository::save(&lifecycle, &change).await?;
        assert_eq!(
            InboundChangeSetRepository::find_by_id(
                &lifecycle,
                &tenant_id,
                change.id(),
            )
                .await?
                .expect("decision should persist")
                .status()
                .as_str(),
            "accepted"
        );

        let mut delivery = OutboundDelivery::create(
            tenant_id.clone(),
            binding.id().clone(),
            link.data_id().clone(),
            "docs/guide.md",
            "outbox-event-1",
            Some("github-sha".into()),
            json!({"content": "updated"}),
        )?;
        OutboundDeliveryRepository::save(&lifecycle, &delivery).await?;
        let attempted_at = Utc::now();
        delivery.mark_attempt(attempted_at);
        delivery.mark_conflict(None, attempted_at);
        OutboundDeliveryRepository::save(&lifecycle, &delivery).await?;
        let restored_delivery = OutboundDeliveryRepository::find_by_id(
            &lifecycle,
            &tenant_id,
            delivery.id(),
        )
            .await?
            .expect("delivery should round-trip");
        assert_eq!(restored_delivery.status(), OutboundDeliveryStatus::Conflict);
        assert_eq!(restored_delivery.attempt_count(), 1);

        let event_id = inbound_sync::WebhookEventId::from(
            "wev_01j91h09tpj5ehwbwfwfxpak2b".to_owned(),
        );
        sqlx::query("INSERT INTO webhook_events (id) VALUES (?)")
            .bind(event_id.to_string())
            .execute(&pool)
            .await?;
        let dispatch =
            SqlxExternalSyncDispatchRepository::new(Arc::new(pool.clone()));
        let job = dispatch.create(&event_id).await?;
        assert!(!dispatch.validate(&event_id, &"0".repeat(64)).await?);
        assert!(dispatch.validate(&event_id, &job.capability).await?);
        assert!(dispatch.claim(&event_id, &job.capability).await?);
        assert!(!dispatch.claim(&event_id, &job.capability).await?);
        dispatch.complete(&event_id).await?;
        assert!(dispatch.completed(&event_id, &job.capability).await?);

        sqlx::raw_sql(include_str!(
            "../../../migrations/20260911010000_create_external_sync_lifecycle.down.sql"
        ))
        .execute(&pool)
        .await?;

        sqlx::raw_sql(include_str!(
            "../../../migrations/20260911000000_create_external_sync_model.down.sql"
        ))
        .execute(&pool)
        .await?;
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM information_schema.tables \
             WHERE table_schema = ? AND table_name IN \
             ('external_sync_bindings', 'external_object_links')",
        )
        .bind(&database_name)
        .fetch_one(&pool)
        .await?;
        assert_eq!(remaining, 0);
        pool.close().await;

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    sqlx::query(&format!("DROP DATABASE `{database_name}`"))
        .execute(&admin)
        .await?;
    admin.close().await;
    test_result
}
