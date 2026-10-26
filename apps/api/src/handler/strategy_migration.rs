//! Reviewed, resumable legacy adoption. Provider content is not backed up.
use super::*;
use database_manager::domain::{
    PropertyType, PropertyValueCommand, RecordMutationDecision,
    RecordOperationId,
};
use database_manager::usecase::{
    PatchRecordInputData, PropertyDataInputData,
};
use strategy_projection::migration::{
    validate_classifications, Plan, PropertyClassification, Request,
    ReviewedEntry,
};

fn classifications(
    properties: &[database_manager::domain::Property],
) -> Vec<PropertyClassification> {
    let mut result = properties
        .iter()
        .map(|p| PropertyClassification {
            id: p.id().to_string(),
            relation: matches!(
                p.property_type(),
                PropertyType::Relation(_)
            ),
            type_key: format!("{:?}", p.property_type()),
        })
        .collect::<Vec<_>>();
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}
pub(super) async fn dry_run(
    Path((org, repo)): Path<(String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
    Json(request): Json<Request>,
) -> errors::Result<Json<Plan>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    request.validate().map_err(map_error)?;
    let binding = state.store.binding(&repo_id).await.map_err(map_error)?;
    validate_subscription(
        &state
            .upstream(
                &executor,
                &binding.tenant_id,
                &binding.subscription_id,
                "",
                None,
            )
            .await?,
        &binding,
    )?;
    let mut entries = Vec::new();
    for entry in &request.entries {
        let (data, properties) = app
            .view_data
            .execute(&ViewDataInputData {
                executor: &executor,
                multi_tenancy: &scope,
                org_username: org.clone(),
                repo_username: repo.clone(),
                data_id: entry.legacy_data_id.clone(),
            })
            .await?;
        if data.record_version().to_string()
            != entry.expected_record_version
        {
            return Err(errors::Error::conflict(
                "Legacy record version differs from review",
            ));
        }
        let schema_hash =
            validate_classifications(entry, &classifications(&properties))
                .map_err(map_error)?;
        entries.push(ReviewedEntry {
            entry: entry.clone(),
            schema_hash,
            operation_id: format!(
                "pm-migrate-{}",
                uuid::Uuid::new_v4().simple()
            ),
            original_projection_id: String::new(),
        });
    }
    Ok(Json(
        state
            .store
            .review_adoption(&repo_id, executor.get_id(), request, entries)
            .await
            .map_err(map_error)?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Approval {
    digest: String,
}
pub(super) async fn apply(
    Path((org, repo, id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(database): Extension<Arc<database_manager::App>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
    Json(approval): Json<Approval>,
) -> errors::Result<Json<Plan>> {
    let (repo_id, tenant) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    let before = state
        .store
        .adoption_plan(&repo_id, &id)
        .await
        .map_err(map_error)?;
    if before.reviewed_by != executor.get_id()
        || before.digest != approval.digest
    {
        return Err(errors::Error::permission_denied(
            "Reviewed actor and digest must match",
        ));
    }
    let binding = state.store.binding(&repo_id).await.map_err(map_error)?;
    validate_subscription(
        &state
            .upstream(
                &executor,
                &binding.tenant_id,
                &binding.subscription_id,
                "",
                None,
            )
            .await?,
        &binding,
    )?;
    // Preflight all records before the first mutation. On resume the stable
    // operation IDs let the existing CAS journal return its prior decision.
    if before.status == "reviewed" {
        for e in &before.entries {
            let (data, properties) = app
                .view_data
                .execute(&ViewDataInputData {
                    executor: &executor,
                    multi_tenancy: &scope,
                    org_username: org.clone(),
                    repo_username: repo.clone(),
                    data_id: e.entry.legacy_data_id.clone(),
                })
                .await?;
            if data.record_version().to_string()
                != e.entry.expected_record_version
                || validate_classifications(
                    &e.entry,
                    &classifications(&properties),
                )
                .map_err(map_error)?
                    != e.schema_hash
            {
                return Err(errors::Error::conflict(
                    "Legacy record or schema changed; review again",
                ));
            }
        }
    }
    let plan = state
        .store
        .begin_adoption(&repo_id, &id, executor.get_id(), &approval.digest)
        .await
        .map_err(map_error)?;
    if plan.status != "applied" {
        let repo_entity = app
            .view_repo
            .execute(&ViewRepoInputData {
                executor: &executor,
                multi_tenancy: &scope,
                organization_username: org.clone(),
                repo_username: repo.clone(),
            })
            .await?
            .repo;
        let database_id =
            repo_entity.databases().first().ok_or_else(|| {
                errors::Error::not_found("Repository database")
            })?;
        let tenant_id = tenant.parse()?;
        for e in &plan.entries {
            let (_, properties) = app
                .view_data
                .execute(&ViewDataInputData {
                    executor: &executor,
                    multi_tenancy: &scope,
                    org_username: org.clone(),
                    repo_username: repo.clone(),
                    data_id: e.entry.legacy_data_id.clone(),
                })
                .await?;
            if validate_classifications(
                &e.entry,
                &classifications(&properties),
            )
            .map_err(map_error)?
                != e.schema_hash
            {
                return Err(errors::Error::conflict(
                    "Property schema changed; migration remains paused",
                ));
            }
            let version = scrub_entry(
                &database,
                &executor,
                &scope,
                &tenant_id,
                database_id,
                e,
            )
            .await?;
            state
                .store
                .adopt_scrubbed_record(
                    &repo_id,
                    &id,
                    &e.entry.resource_ref,
                    &version,
                )
                .await
                .map_err(map_error)?;
        }
        state
            .store
            .finish_adoption(&repo_id, &id)
            .await
            .map_err(map_error)?;
    }
    // Source credentials are not retained; caller explicitly authorized this
    // apply. Recreate only the provider projection, never the generic fields.
    state
        .restore_snapshot(&repo_id, &executor, &binding)
        .await?;
    state
        .store
        .activate_tachyon_writer(&repo_id)
        .await
        .map_err(map_error)?;
    Ok(Json(
        state
            .store
            .adoption_plan(&repo_id, &id)
            .await
            .map_err(map_error)?,
    ))
}
pub(super) async fn view(
    Path((org, repo, id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
) -> errors::Result<Json<Plan>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    Ok(Json(
        state
            .store
            .adoption_plan(&repo_id, &id)
            .await
            .map_err(map_error)?,
    ))
}
pub(super) async fn rollback(
    Path((org, repo, id)): Path<(String, String, String)>,
    Extension(app): Extension<Arc<LibraryApp>>,
    Extension(state): Extension<StrategyState>,
    executor: LibraryExecutor,
    scope: LibraryOrg,
    Json(approval): Json<Approval>,
) -> errors::Result<Json<Plan>> {
    let (repo_id, _) =
        authorize(&app, &executor, &scope, &org, &repo, true, true).await?;
    let plan = state
        .store
        .adoption_plan(&repo_id, &id)
        .await
        .map_err(map_error)?;
    if plan.digest != approval.digest {
        return Err(errors::Error::conflict("Review digest differs"));
    }
    // Rollback never restores stale provider content. Generic local fields and
    // relations were never removed. Projection IDs return to their prior value.
    Ok(Json(
        state
            .store
            .rollback_adoption(&repo_id, &id)
            .await
            .map_err(map_error)?,
    ))
}

async fn scrub_entry(
    database: &database_manager::App,
    executor: &dyn tachyon_sdk::auth::ExecutorAction,
    scope: &dyn tachyon_sdk::auth::MultiTenancyAction,
    tenant_id: &value_object::TenantId,
    database_id: &database_manager::domain::DatabaseId,
    e: &ReviewedEntry,
) -> errors::Result<String> {
    let operation_id = RecordOperationId::new(&e.operation_id)?;
    let data_id = e.entry.legacy_data_id.parse()?;
    let decision = database
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor,
            multi_tenancy: scope,
            tenant_id,
            database_id,
            data_id: &data_id,
            operation_id: &operation_id,
            expected_version: database_manager::domain::RecordVersion::new(
                e.entry.expected_record_version.parse::<u64>().map_err(
                    |_| errors::Error::invalid("Invalid record version"),
                )?,
            )?,
            name: Some(&e.entry.local_name),
            properties: e
                .entry
                .clear_property_ids
                .iter()
                .map(|id| {
                    Ok(PropertyDataInputData {
                        property_id: id.parse()?,
                        value: PropertyValueCommand::Clear,
                    })
                })
                .collect::<errors::Result<Vec<_>>>()?,
        })
        .await?;
    let version = match decision {
        RecordMutationDecision::Accepted { record_version, .. } => {
            record_version.to_string()
        }
        RecordMutationDecision::Conflict { .. } => {
            return Err(errors::Error::conflict(
                "Legacy record changed; migration remains paused",
            ))
        }
        RecordMutationDecision::Rejected { .. } => {
            return Err(errors::Error::conflict(
                "Legacy scrub rejected; migration remains paused",
            ))
        }
    };
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use database_manager::domain::{PropertyDataValue, TypeRelation};
    use database_manager::{
        AddDataInputData, AddPropertyInputData, CreateDatabaseInputData,
        GetDataInputData,
    };
    use strategy_projection::migration::Entry;
    use strategy_projection::{
        Change, ChangePage, Operation, Projection, ResourceRef,
    };
    #[tokio::test]
    #[ignore = "requires disposable loopback strategy_migration_test database"]
    async fn reviewed_adoption_scrubs_both_stores_preserves_relations_and_rolls_back(
    ) {
        let url =
            std::env::var("LIBRARY_MIGRATION_TEST_DATABASE_URL").unwrap();
        let target = url::Url::parse(&url).unwrap();
        assert_eq!(target.host_str(), Some("127.0.0.1"));
        assert_eq!(target.path(), "/strategy_migration_test");
        let pool = sqlx::MySqlPool::connect(&url).await.unwrap();
        // One historical migration hard-codes its production schema. Rewrite
        // only that qualifier so every statement stays inside this test DB.
        for migration in
            sqlx::migrate!("../../packages/database-manager/migrations")
                .iter()
        {
            let sql = migration.sql.replace(
                "tachyon_apps_database_manager.",
                "strategy_migration_test.",
            );
            sqlx::raw_sql(&sql).execute(&pool).await.unwrap();
        }
        for migration in [include_str!("../../migrations/20261024000000_strategy_projection.up.sql"),include_str!("../../migrations/20261024010000_pm_reviewed_adoption.up.sql")] {
            for statement in migration.split(';').filter(|s|!s.trim().is_empty()){sqlx::query(statement).execute(&pool).await.unwrap();}
        }
        sqlx::query("CREATE TABLE IF NOT EXISTS webhook_endpoints(id VARCHAR(64) PRIMARY KEY,repository_id VARCHAR(64),tenant_id VARCHAR(64),provider VARCHAR(32),status VARCHAR(32))").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS sync_states(endpoint_id VARCHAR(64),data_id VARCHAR(64),external_id VARCHAR(255),metadata JSON)").execute(&pool).await.unwrap();
        let database=database_manager::factory_client_with_property_value_mode(url.clone(),database_manager::property_value_rollout::PropertyValueStorageMode::DualWriteCanonicalRead).await.unwrap();
        let tenant = value_object::TenantId::default();
        let scope =
            tachyon_sdk::auth::MultiTenancy::new_operator(tenant.clone());
        let executor = tachyon_sdk::auth::Executor::SystemUser;
        let db = database
            .create_database()
            .execute(CreateDatabaseInputData {
                executor: &executor,
                multi_tenancy: &scope,
                database_id: None,
                tenant_id: &tenant,
                name: "legacy migration",
            })
            .await
            .unwrap();
        let provider = database
            .add_property()
            .execute(AddPropertyInputData {
                executor: &executor,
                multi_tenancy: &scope,
                tenant_id: &tenant,
                database_id: db.id(),
                display_name: None,
                name: "provider_text",
                property_type: PropertyType::String,
            })
            .await
            .unwrap();
        let local = database
            .add_property()
            .execute(AddPropertyInputData {
                executor: &executor,
                multi_tenancy: &scope,
                tenant_id: &tenant,
                database_id: db.id(),
                display_name: None,
                name: "local_notes",
                property_type: PropertyType::Markdown,
            })
            .await
            .unwrap();
        let related = database
            .add_property()
            .execute(AddPropertyInputData {
                executor: &executor,
                multi_tenancy: &scope,
                tenant_id: &tenant,
                database_id: db.id(),
                display_name: None,
                name: "related",
                property_type: PropertyType::Relation(TypeRelation::new(
                    db.id().clone(),
                )),
            })
            .await
            .unwrap();
        let target_record = database
            .add_data_usecase()
            .execute(AddDataInputData {
                executor: &executor,
                multi_tenancy: &scope,
                tenant_id: &tenant,
                database_id: db.id(),
                name: "local document",
                property_data: vec![],
            })
            .await
            .unwrap();
        let legacy = database
            .add_data_usecase()
            .execute(AddDataInputData {
                executor: &executor,
                multi_tenancy: &scope,
                tenant_id: &tenant,
                database_id: db.id(),
                name: "provider secret name",
                property_data: vec![
                    PropertyDataInputData {
                        property_id: provider.id().clone(),
                        value: PropertyValueCommand::String(
                            "provider secret".into(),
                        ),
                    },
                    PropertyDataInputData {
                        property_id: local.id().clone(),
                        value: PropertyValueCommand::Markdown(
                            "independent notes".into(),
                        ),
                    },
                    PropertyDataInputData {
                        property_id: related.id().clone(),
                        value: PropertyValueCommand::Relation(vec![
                            target_record.id().clone(),
                        ]),
                    },
                ],
            })
            .await
            .unwrap();
        let repo = format!("repo_{}", uuid::Uuid::new_v4().simple());
        let sub = format!("sub_{}", uuid::Uuid::new_v4().simple());
        let endpoint = format!("old_{}", uuid::Uuid::new_v4().simple());
        let store = Store::new(pool.clone());
        store
            .connect(&repo, tenant.as_ref(), &sub, "workspace", 1)
            .await
            .unwrap();
        store.activate_tachyon_writer(&repo).await.unwrap();
        let reference = ResourceRef {
            provider: "linear".into(),
            workspace_id: "workspace".into(),
            kind: "project".into(),
            external_id: "project".into(),
        };
        let projection = Projection {
            resource_ref: reference.clone(),
            name: "current source".into(),
            url: None,
            archived: false,
            status: None,
            owner: None,
            target_date: None,
            source_updated_at: None,
        };
        store
            .apply_page(
                &repo,
                &ChangePage {
                    generation: 1,
                    changes: vec![Change {
                        subscription_id: sub,
                        generation: 1,
                        sequence: 1,
                        event_id: "first".into(),
                        resource_ref: reference.clone(),
                        observed_revision: 1,
                        operation: Operation::Upsert,
                        projection: Some(projection.clone()),
                        relation: None,
                        schema_version: 1,
                        observed_at: "2026-10-24".into(),
                    }],
                    next_cursor: None,
                },
            )
            .await
            .unwrap();
        let original =
            store.resolve(&repo, &reference).await.unwrap().unwrap();
        store
            .record_local_context(&repo, &original, "projection note", &[])
            .await
            .unwrap();
        sqlx::query("INSERT INTO webhook_endpoints(id,repository_id,tenant_id,provider,status) VALUES(?,?,?,'linear','paused')").bind(&endpoint).bind(&repo).bind(tenant.to_string()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO sync_states(endpoint_id,data_id,external_id,metadata) VALUES(?,?,?,?)").bind(&endpoint).bind(legacy.id().to_string()).bind("linear:project:project").bind(json!({"workspace_id":"workspace"})).execute(&pool).await.unwrap();
        let entry = Entry {
            retire_only: false,
            resource_ref: reference.clone(),
            endpoint_id: endpoint,
            legacy_data_id: legacy.id().to_string(),
            expected_record_version: legacy.record_version().to_string(),
            local_name: "Local notes".into(),
            clear_property_ids: vec![provider.id().to_string()],
            preserve_property_ids: vec![
                local.id().to_string(),
                related.id().to_string(),
            ],
            workspace_attestation: None,
        };
        let properties =
            vec![provider.clone(), local.clone(), related.clone()];
        let reviewed = ReviewedEntry {
            schema_hash: validate_classifications(
                &entry,
                &classifications(&properties),
            )
            .unwrap(),
            entry: entry.clone(),
            operation_id: format!(
                "migration-{}",
                uuid::Uuid::new_v4().simple()
            ),
            original_projection_id: String::new(),
        };
        let request = Request {
            workspace_id: "workspace".into(),
            review_note:
                "Reviewed against exact legacy workspace and object IDs"
                    .into(),
            entries: vec![entry],
        };
        let plan = store
            .review_adoption(&repo, "reviewer", request, vec![reviewed])
            .await
            .unwrap();
        assert!(store
            .begin_adoption(&repo, &plan.id, "other", &plan.digest)
            .await
            .is_err());
        // A legacy write arriving after review must force another review.
        sqlx::query("INSERT INTO sync_states(endpoint_id,data_id,external_id,metadata) VALUES(?,'unexpected-after-review','linear:issue:new',NULL)").bind(&plan.entries[0].entry.endpoint_id).execute(&pool).await.unwrap();
        assert!(store
            .begin_adoption(&repo, &plan.id, "reviewer", &plan.digest)
            .await
            .is_err());
        sqlx::query("DELETE FROM sync_states WHERE endpoint_id=? AND data_id='unexpected-after-review'").bind(&plan.entries[0].entry.endpoint_id).execute(&pool).await.unwrap();
        let plan = store
            .begin_adoption(&repo, &plan.id, "reviewer", &plan.digest)
            .await
            .unwrap();
        assert!(store.activate_tachyon_writer(&repo).await.is_err());
        let version = scrub_entry(
            &database,
            &executor,
            &scope,
            &tenant,
            db.id(),
            &plan.entries[0],
        )
        .await
        .unwrap();
        // Resume after a crash between the two databases: same operation is a replay.
        assert_eq!(
            scrub_entry(
                &database,
                &executor,
                &scope,
                &tenant,
                db.id(),
                &plan.entries[0]
            )
            .await
            .unwrap(),
            version
        );
        sqlx::query("INSERT INTO library_document_releases(repo_id,id,data_id,source_version,title,body,published_by,published_at) VALUES(?,'legacy-release',?,'1','old provider title','old provider text','reviewer','2026-10-24')").bind(&repo).bind(legacy.id().to_string()).execute(&pool).await.unwrap();
        store
            .adopt_scrubbed_record(&repo, &plan.id, &reference, &version)
            .await
            .unwrap();
        store.finish_adoption(&repo, &plan.id).await.unwrap();
        let revoked:bool=sqlx::query_scalar("SELECT revoked FROM library_document_releases WHERE repo_id=? AND id='legacy-release'").bind(&repo).fetch_one(&pool).await.unwrap();
        assert!(revoked);
        let fresh = database
            .get_data_usecase()
            .execute(&GetDataInputData {
                executor: &executor,
                multi_tenancy: &scope,
                tenant_id: &tenant,
                database_id: db.id(),
                data_id: legacy.id(),
            })
            .await
            .unwrap();
        assert_eq!(fresh.name().to_string(), "Local notes");
        assert!(fresh
            .get_property_data(provider.id())
            .is_none_or(|p| p.value().is_none()));
        assert!(
            matches!(fresh.get_property_data(local.id()).unwrap().value(),Some(PropertyDataValue::Markdown(s)) if s=="independent notes")
        );
        assert_eq!(
            fresh.get_property_data(related.id()).unwrap().value(),
            legacy.get_property_data(related.id()).unwrap().value()
        );
        let old_slot = format!(
            "SELECT value{} FROM data WHERE id=?",
            provider.property_num()
        );
        let raw: Option<String> = sqlx::query_scalar(&old_slot)
            .bind(legacy.id().to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(raw.is_none_or(|s| s.is_empty()));
        let canonical:i64=sqlx::query_scalar("SELECT COUNT(*) FROM property_values WHERE data_id=? AND property_id=?").bind(legacy.id().to_string()).bind(provider.id().to_string()).fetch_one(&pool).await.unwrap();
        assert_eq!(canonical, 0);
        store
            .replace_snapshot(&repo, 1, 1, vec![projection], vec![])
            .await
            .unwrap();
        store.activate_tachyon_writer(&repo).await.unwrap();
        assert_eq!(
            store.resolve(&repo, &reference).await.unwrap(),
            Some(legacy.id().to_string())
        );
        assert_eq!(
            store
                .record(&repo, &legacy.id().to_string())
                .await
                .unwrap()
                .note,
            "projection note"
        );
        store.rollback_adoption(&repo, &plan.id).await.unwrap();
        // Rollback changes projection identity only, not the scrubbed generic values.
        assert_eq!(store.binding(&repo).await.unwrap().writer, "paused");
        let restored = database
            .get_data_usecase()
            .execute(&GetDataInputData {
                executor: &executor,
                multi_tenancy: &scope,
                tenant_id: &tenant,
                database_id: db.id(),
                data_id: legacy.id(),
            })
            .await
            .unwrap();
        assert_eq!(restored.name().to_string(), "Local notes");
        let mapped:String=sqlx::query_scalar("SELECT id FROM library_pm_records WHERE repo_id=? AND resource_key=?").bind(&repo).bind(reference.key()).fetch_one(&pool).await.unwrap();
        assert_eq!(mapped, original);
    }
}
