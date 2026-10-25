//! Request-local computed values. No shared cache may outlive an authorization decision.
use super::{authorize_private_repo_read, GetRepoByUsernameQuery};
use database_manager::{
    domain::{
        self, ComputedValue, Data, Property, PropertyDataValue,
        PropertyType,
    },
    usecase::FindAllPropertiesInputData,
};
use std::collections::{BTreeMap, BTreeSet};
use tachyon_sdk::auth::{AuthApp, ExecutorAction, MultiTenancyAction};
use value_object::TenantId;

pub(super) async fn evaluate(
    database: &database_manager::App,
    repos: &dyn GetRepoByUsernameQuery,
    auth: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    tenancy: &dyn MultiTenancyAction,
    tenant_id: &TenantId,
    records: Vec<Data>,
    properties: &[Property],
) -> errors::Result<Vec<Data>> {
    if !properties.iter().any(|p| {
        matches!(
            p.property_type(),
            PropertyType::Formula(_) | PropertyType::Rollup(_)
        )
    }) {
        return Ok(records);
    }
    let mut definitions = BTreeMap::new();
    let mut targets = BTreeMap::new();
    let mut failures = BTreeMap::new();
    for property in properties {
        let PropertyType::Rollup(config) = property.property_type() else {
            continue;
        };
        let Some(PropertyType::Relation(relation)) = properties
            .iter()
            .find(|p| p.id().as_str() == config.relation_property_id)
            .map(|p| p.property_type())
        else {
            continue;
        };
        let target_db = &relation.database_id;
        if definitions.contains_key(target_db)
            || failures.contains_key(target_db)
        {
            continue;
        }
        // Resolve within the source tenant before reading any target metadata or values.
        let allowed = authorized_target(
            repos, auth, executor, tenancy, tenant_id, target_db,
        )
        .await;
        if !allowed {
            failures.insert(
                target_db.clone(),
                ComputedValue::Error {
                    code: "access_denied".into(),
                },
            );
            continue;
        }
        let before = database
            .data_snapshot()
            .revision(tenant_id, target_db)
            .await?;
        let props = database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: tenant_id.clone(),
                database_id: target_db.clone(),
            })
            .await?;
        let ids: BTreeSet<_> = records
            .iter()
            .flat_map(|record| {
                properties
                    .iter()
                    .filter_map(|p| match p.property_type() {
                        PropertyType::Relation(r)
                            if &r.database_id == target_db =>
                        {
                            record.get_property_data(p.id()).and_then(|v| {
                                match v.value() {
                                    Some(PropertyDataValue::Relation(
                                        db,
                                        ids,
                                    )) if db == target_db => {
                                        Some(ids.clone())
                                    }
                                    _ => None,
                                }
                            })
                        }
                        _ => None,
                    })
                    .flatten()
            })
            .collect();
        // Bound fan-out and deduplicate across all records in this response.
        // Read only referenced IDs, never scan a target repository.
        if ids.len() > 256 {
            failures.insert(target_db.clone(), ComputedValue::Pending);
            continue;
        }
        let ids = ids.into_iter().collect::<Vec<_>>();
        let rows = database
            .data_snapshot()
            .load_ids(tenant_id, target_db, &ids)
            .await?;
        for id in ids {
            let record = rows
                .iter()
                .find(|row| {
                    row.id() == &id
                        && row.tenant_id() == tenant_id
                        && row.database_id() == target_db
                })
                .cloned();
            targets.insert((target_db.clone(), id), record);
        }
        let after = database
            .data_snapshot()
            .revision(tenant_id, target_db)
            .await?;
        let current_props = database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: tenant_id.clone(),
                database_id: target_db.clone(),
            })
            .await?;
        if before != after
            || format!("{:?}", props) != format!("{:?}", current_props)
        {
            failures.insert(target_db.clone(), ComputedValue::Pending);
        }
        definitions.insert(target_db.clone(), props);
    }
    records
        .into_iter()
        .map(|record| {
            let mut rollups = BTreeMap::new();
            for property in properties {
                let PropertyType::Rollup(config) = property.property_type()
                else {
                    continue;
                };
                let result = (|| {
                    let relation = properties.iter().find(|p| {
                        p.id().as_str() == config.relation_property_id
                    })?;
                    let PropertyType::Relation(relation_type) =
                        relation.property_type()
                    else {
                        return None;
                    };
                    let db = &relation_type.database_id;
                    if let Some(error) = failures.get(db) {
                        return Some(error.clone());
                    };
                    let props = definitions.get(db)?;
                    let target = props.iter().find(|p| {
                        p.id().as_str() == config.target_property_id
                    })?;
                    if !matches!(
                        target.property_type(),
                        PropertyType::Decimal(_)
                            | PropertyType::Integer
                            | PropertyType::Formula(_)
                    ) {
                        return Some(ComputedValue::Error {
                            code: "type_mismatch".into(),
                        });
                    };
                    let ids = match record
                        .get_property_data(relation.id())
                        .and_then(|p| p.value().as_ref())
                    {
                        Some(PropertyDataValue::Relation(
                            stored_db,
                            ids,
                        )) if stored_db == db => {
                            ids.iter().collect::<BTreeSet<_>>()
                        }
                        None => BTreeSet::new(),
                        _ => {
                            return Some(ComputedValue::Error {
                                code: "invalid_relation".into(),
                            })
                        }
                    };
                    let values = ids
                        .into_iter()
                        .map(|id| {
                            let Some(Some(row)) =
                                targets.get(&(db.clone(), id.clone()))
                            else {
                                return ComputedValue::Error {
                                    code: "missing_target".into(),
                                };
                            };
                            let row = match domain::evaluate_record(
                                row,
                                props,
                                &BTreeMap::new(),
                            ) {
                                Ok(row) => row,
                                Err(_) => {
                                    return ComputedValue::Error {
                                        code: "invalid_target".into(),
                                    }
                                }
                            };
                            match row
                                .get_property_data(target.id())
                                .and_then(|p| p.value().as_ref())
                            {
                                Some(PropertyDataValue::Decimal(value)) => {
                                    ComputedValue::Ready {
                                        value: value.clone(),
                                    }
                                }
                                Some(PropertyDataValue::Integer(value)) => {
                                    ComputedValue::Ready {
                                        value: value.to_string(),
                                    }
                                }
                                Some(PropertyDataValue::Formula(value)) => {
                                    value.clone()
                                }
                                None => ComputedValue::Null,
                                _ => ComputedValue::Error {
                                    code: "type_mismatch".into(),
                                },
                            }
                        })
                        .collect::<Vec<_>>();
                    Some(config.evaluate(&values))
                })()
                .unwrap_or(ComputedValue::Error {
                    code: "missing_dependency".into(),
                });
                rollups.insert(property.id().to_string(), result);
            }
            domain::evaluate_record(&record, properties, &rollups)
        })
        .collect()
}

async fn authorized_target(
    repos: &dyn GetRepoByUsernameQuery,
    auth: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    tenancy: &dyn MultiTenancyAction,
    tenant: &TenantId,
    target: &domain::DatabaseId,
) -> bool {
    let Ok(Some(repo)) = repos.find_by_database(tenant, target).await
    else {
        return false;
    };
    match crate::domain::VisibilityService::new()
        .check_access(&repo, executor)
    {
        Ok(false) => true,
        Ok(true) => authorize_private_repo_read(
            auth,
            executor,
            tenancy,
            repo.id().as_ref(),
        )
        .await
        .is_ok(),
        Err(_) => false,
    }
}

pub(super) async fn authorize_definition(
    database: &database_manager::App,
    repos: &dyn GetRepoByUsernameQuery,
    auth: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    tenancy: &dyn MultiTenancyAction,
    tenant: &TenantId,
    source: &domain::DatabaseId,
    typ: &PropertyType,
) -> errors::Result<()> {
    let PropertyType::Rollup(config) = typ else {
        return Ok(());
    };
    let properties = database
        .find_all_properties()
        .execute(FindAllPropertiesInputData {
            tenant_id: tenant.clone(),
            database_id: source.clone(),
        })
        .await?;
    let Some(PropertyType::Relation(relation)) = properties
        .iter()
        .find(|p| p.id().as_str() == config.relation_property_id)
        .map(|p| p.property_type())
    else {
        return Err(errors::Error::invalid("unknown rollup relation"));
    };
    if !authorized_target(
        repos,
        auth,
        executor,
        tenancy,
        tenant,
        &relation.database_id,
    )
    .await
    {
        return Err(errors::Error::permission_denied(
            "rollup target is not readable",
        ));
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp,
    };
    #[derive(Debug)]
    struct Repos {
        visible: bool,
        tenant: TenantId,
        database: domain::DatabaseId,
    }
    #[async_trait::async_trait]
    impl GetRepoByUsernameQuery for Repos {
        async fn execute(
            &self,
            _: &value_object::Identifier,
            _: &value_object::Identifier,
        ) -> errors::Result<Option<crate::domain::Repo>> {
            panic!("must use scoped database lookup")
        }
        async fn find_by_database(
            &self,
            tenant: &TenantId,
            database: &domain::DatabaseId,
        ) -> errors::Result<Option<crate::domain::Repo>> {
            if tenant != &self.tenant
                || database != &self.database
                || !self.visible
            {
                return Ok(None);
            }
            Ok(Some(crate::domain::Repo::new(
                &crate::domain::RepoId::default(),
                &Default::default(),
                &"org".parse().unwrap(),
                &"target".parse().unwrap(),
                &"target".parse().unwrap(),
                false,
                None,
                vec![database.clone()],
                vec![],
            )))
        }
    }
    #[tokio::test]
    async fn rollup_authorization_fails_closed_for_missing_foreign_or_denied_targets(
    ) {
        let tenant = TenantId::default();
        let database = domain::DatabaseId::default();
        let repos = Repos {
            visible: true,
            tenant: tenant.clone(),
            database: database.clone(),
        };
        let executor = create_test_executor();
        let tenancy = create_test_multi_tenancy();
        let mut denied = MockAuthApp::new();
        denied
            .expect_check_policy_for_resource()
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Err(errors::Error::permission_denied("denied"))
                })
            });
        denied.expect_check_policy().times(1).returning(|_| {
            Box::pin(async {
                Err(errors::Error::permission_denied("denied"))
            })
        });
        assert!(
            !authorized_target(
                &repos, &denied, &executor, &tenancy, &tenant, &database
            )
            .await
        );
        let untouched = MockAuthApp::new();
        assert!(
            !authorized_target(
                &repos,
                &untouched,
                &executor,
                &tenancy,
                &TenantId::default(),
                &database
            )
            .await
        );
        assert!(
            !authorized_target(
                &repos,
                &untouched,
                &executor,
                &tenancy,
                &tenant,
                &domain::DatabaseId::default()
            )
            .await
        );
    }
}
