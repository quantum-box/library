use std::fmt::Debug;
use std::sync::Arc;

use crate::{
    domain::VisibilityService, usecase::authorize_private_repo_read,
};
use database_manager::{
    domain::{Data, Property},
    usecase::FindAllPropertiesInputData,
};
use tachyon_sdk::auth::{AuthApp, ExecutorAction, MultiTenancyAction};
use value_object::OffsetPaginator;

#[async_trait::async_trait]
pub trait ViewDataListInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &ViewDataListInputData,
    ) -> errors::Result<(Vec<Data>, Vec<Property>, OffsetPaginator)>;
}

#[derive(Debug, Clone)]
pub struct ViewDataListInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,

    pub org_username: String,
    pub repo_username: String,
    pub page_size: Option<u32>,
    pub page: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ViewDataList {
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    database: Arc<database_manager::App>,
    auth_app: Arc<dyn AuthApp>,
}

impl ViewDataList {
    pub fn new(
        database: Arc<database_manager::App>,
        get_org_by_username: Arc<
            dyn crate::usecase::GetOrganizationByUsernameQuery,
        >,
        get_repo_by_username: Arc<
            dyn crate::usecase::GetRepoByUsernameQuery,
        >,
        auth_app: Arc<dyn AuthApp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database,
            get_org_by_username,
            get_repo_by_username,
            auth_app,
        })
    }
}

#[async_trait::async_trait]
impl ViewDataListInputPort for ViewDataList {
    #[tracing::instrument(name = "ViewDataList::execute", skip_all)]
    async fn execute(
        &self,
        input: &ViewDataListInputData,
    ) -> errors::Result<(Vec<Data>, Vec<Property>, OffsetPaginator)> {
        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::not_found!(
                "organization not found in view data list"
            ))?;

        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await
            .map_err(|e| {
                errors::Error::application_logic_error(e.to_string())
            })?
            .ok_or(errors::not_found!(
                "repo not found in view data list"
            ))?;

        let need_policy_check =
            VisibilityService::new().check_access(&repo, input.executor)?;
        if need_policy_check {
            authorize_private_repo_read(
                self.auth_app.as_ref(),
                input.executor,
                input.multi_tenancy,
                repo.id().as_ref(),
            )
            .await?;
        }

        let database_id = repo.databases().first().unwrap().clone();
        let find_all_properties = self.database.find_all_properties();
        let search_data = self.database.search_data();
        let search_input = database_manager::SearchDataInputData {
            executor: input.executor,
            multi_tenancy: input.multi_tenancy,
            tenant_id: org.id(),
            database_id: Some(database_id.clone()),
            page: input.page,
            page_size: input.page_size,
            query: "",
        };
        let properties_input = || FindAllPropertiesInputData {
            tenant_id: org.id().clone(),
            database_id: database_id.clone(),
        };
        let (data_list, properties, paginator) =
            list_with_matching_properties(
                || find_all_properties.execute(properties_input()),
                || search_data.execute(&search_input),
            )
            .await?;

        let data_list = super::computed_properties::evaluate(
            &self.database,
            self.get_repo_by_username.as_ref(),
            self.auth_app.as_ref(),
            (input.executor, input.multi_tenancy),
            org.id(),
            data_list,
            &properties,
        )
        .await?;
        Ok((data_list, properties, paginator))
    }
}

/// Load a page of data and the property metadata describing it.
///
/// The two reads are independent, so they run concurrently. `join!` (not
/// `try_join!`) waits for both and the results are checked in the original
/// order, so the error a caller sees does not change. Concurrent reads may
/// straddle a schema change, leaving a hydrated property without metadata
/// (callers look metadata up for every property). In that case the pair is
/// read again in the original order, properties first, as it was before the
/// reads overlapped.
async fn list_with_matching_properties<P, PF, D, DF>(
    load_properties: P,
    load_data: D,
) -> errors::Result<(Vec<Data>, Vec<Property>, OffsetPaginator)>
where
    P: Fn() -> PF,
    PF: std::future::Future<Output = errors::Result<Vec<Property>>>,
    D: Fn() -> DF,
    DF: std::future::Future<
        Output = errors::Result<(Vec<Data>, OffsetPaginator)>,
    >,
{
    let (properties, data) = tokio::join!(load_properties(), load_data());
    let properties = properties?;
    let (data_list, paginator) = data?;
    if describes_every_property(&properties, &data_list) {
        return Ok((data_list, properties, paginator));
    }

    tracing::info!(
        "property schema changed during concurrent data list reads; re-reading in order"
    );
    let properties = load_properties().await?;
    let (data_list, paginator) = load_data().await?;
    Ok((data_list, properties, paginator))
}

fn describes_every_property(
    properties: &[Property],
    data: &[Data],
) -> bool {
    let known = properties
        .iter()
        .map(|property| property.id())
        .collect::<std::collections::HashSet<_>>();
    data.iter().all(|data| {
        data.property_data()
            .iter()
            .all(|value| known.contains(value.property_id()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use database_manager::domain::{PropertyData, PropertyType};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use value_object::{OffsetPage, TenantId};

    fn property(name: &str) -> Property {
        Property::new(
            &Default::default(),
            &TenantId::default(),
            &Default::default(),
            name,
            &PropertyType::String,
            false,
            0,
        )
    }

    fn data(properties: &[&Property]) -> Data {
        Data::new(
            &Default::default(),
            &TenantId::default(),
            &Default::default(),
            "record",
            properties
                .iter()
                .map(|property| {
                    PropertyData::new(property, "value".to_string())
                        .unwrap()
                })
                .collect(),
            chrono::Utc::now(),
            chrono::Utc::now(),
        )
        .unwrap()
    }

    fn paginator() -> OffsetPaginator {
        OffsetPaginator::new(OffsetPage::new(1, 20).unwrap(), 1)
    }

    #[tokio::test]
    async fn matching_snapshots_are_read_once() {
        let title = property("title");
        let calls = AtomicUsize::new(0);
        let (data_list, properties, _) = list_with_matching_properties(
            || async {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(vec![title.clone()])
            },
            || async { Ok((vec![data(&[&title])], paginator())) },
        )
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(properties.len(), 1);
        assert_eq!(data_list.len(), 1);
    }

    #[tokio::test]
    async fn a_schema_change_between_the_reads_is_read_again_in_order() {
        // The first concurrent property read misses `body` (it was deleted
        // or added between the two statements); the data still has it.
        let title = property("title");
        let body = property("body");
        let property_calls = AtomicUsize::new(0);
        let data_calls = AtomicUsize::new(0);
        let (data_list, properties, _) = list_with_matching_properties(
            || async {
                if property_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    Ok(vec![title.clone()])
                } else {
                    Ok(vec![title.clone(), body.clone()])
                }
            },
            || async {
                data_calls.fetch_add(1, Ordering::SeqCst);
                Ok((vec![data(&[&title, &body])], paginator()))
            },
        )
        .await
        .unwrap();
        assert_eq!(property_calls.load(Ordering::SeqCst), 2);
        assert_eq!(data_calls.load(Ordering::SeqCst), 2);
        assert!(describes_every_property(&properties, &data_list));
    }

    #[tokio::test]
    async fn errors_keep_their_original_precedence() {
        let error = list_with_matching_properties(
            || async { Err(errors::Error::not_found("properties")) },
            || async {
                Err::<(Vec<Data>, OffsetPaginator), _>(
                    errors::Error::not_found("data"),
                )
            },
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("properties"));
    }
}
