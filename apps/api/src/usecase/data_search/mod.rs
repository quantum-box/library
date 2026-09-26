//! Location, filter and text search over one repo's records.
//!
//! The design, and when to replace it, is in
//! `docs/specs/decisions/ADR-0011-data-search-index.md`.

mod cache;
mod index;
mod query;

pub use query::DataSearchQuery;

use cache::SearchIndexCache;
use index::{SearchIndex, Visibility};

use std::fmt::Debug;
use std::sync::Arc;

use database_manager::{
    domain::{Data, Property},
    usecase::FindAllPropertiesInputData,
};
use sha2::{Digest, Sha256};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};
use value_object::{OffsetPage, OffsetPaginator};

use crate::{
    domain::VisibilityService, usecase::authorize_private_repo_read,
};

#[derive(Debug, Clone)]
pub struct DataSearchInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: &'a str,
    pub repo_username: &'a str,
    pub query: DataSearchQuery,
    /// Return drafts too. Only editors of the repo may ask.
    pub include_unpublished: bool,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct DataSearchHit {
    pub data: Data,
    /// Meters from the query point, when the query has one.
    pub distance_meters: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DataSearchOutput {
    pub hits: Vec<DataSearchHit>,
    pub properties: Vec<Property>,
    pub paginator: OffsetPaginator,
    /// Identifies the data the results were computed from and who could
    /// see it. Equal fingerprints and equal queries give equal results, so
    /// it can back an HTTP validator.
    pub fingerprint: String,
    /// Whether anyone at all may read these results: a public repo, drafts
    /// excluded. Only such responses belong in a shared cache.
    pub publicly_cacheable: bool,
}

#[async_trait::async_trait]
pub trait DataSearchInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &DataSearchInputData,
    ) -> errors::Result<DataSearchOutput>;
}

#[derive(Debug)]
pub struct DataSearch {
    database: Arc<database_manager::App>,
    get_org_by_username:
        Arc<dyn crate::usecase::GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn crate::usecase::GetRepoByUsernameQuery>,
    auth_app: Arc<dyn AuthApp>,
    cache: SearchIndexCache,
}

impl DataSearch {
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
            cache: SearchIndexCache::default(),
        })
    }
}

#[async_trait::async_trait]
impl DataSearchInputPort for DataSearch {
    #[tracing::instrument(name = "DataSearch::execute", skip_all)]
    async fn execute(
        &self,
        input: &DataSearchInputData,
    ) -> errors::Result<DataSearchOutput> {
        // Reject a bad page before doing any work.
        let page = OffsetPage::from_options(input.page, input.page_size)?;

        let org = self
            .get_org_by_username
            .execute(&input.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!("organization not found"))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &input.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!("repo not found"))?;

        if VisibilityService::new().check_access(&repo, input.executor)? {
            authorize_private_repo_read(
                self.auth_app.as_ref(),
                input.executor,
                input.multi_tenancy,
                repo.id().as_ref(),
            )
            .await?;
        }
        let visibility = if input.include_unpublished {
            if input.executor.is_none() {
                return Err(errors::Error::permission_denied(
                    "include_unpublished requires signing in as an editor",
                ));
            }
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: "library:UpdateRepo",
                })
                .await?;
            Visibility::All
        } else {
            Visibility::PublishedOnly
        };

        let tenant_id = org.id().clone();
        let database_id = repo
            .databases()
            .first()
            .ok_or_else(|| {
                errors::Error::application_logic_error(
                    "Repository has no associated database",
                )
            })?
            .clone();

        // The fingerprint needs the Property definitions as well as the
        // record revision: a new option or a renamed key changes how
        // records match without touching any record.
        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: tenant_id.clone(),
                database_id: database_id.clone(),
            })
            .await?;
        let revision = self
            .database
            .data_snapshot()
            .revision(&tenant_id, &database_id)
            .await?;
        let data_fingerprint = format!(
            "{}:{}",
            revision.token(),
            properties_digest(&properties)
        );
        let cache_key = format!("{tenant_id}/{database_id}");

        let index = match self.cache.get(&cache_key, &data_fingerprint) {
            Some(index) => index,
            None => {
                let data = self
                    .database
                    .data_snapshot()
                    .load_all(&tenant_id, &database_id)
                    .await?;
                let index = Arc::new(SearchIndex::build(data, properties));
                tracing::info!(
                    records = index.record_count(),
                    %cache_key,
                    "rebuilt data search index"
                );
                self.cache.insert(
                    &cache_key,
                    &data_fingerprint,
                    index.clone(),
                );
                index
            }
        };

        let matches = index.search(&input.query, visibility)?;
        let total = u32::try_from(matches.len()).map_err(|_| {
            errors::Error::internal_server_error(
                "result count exceeds the supported pagination range",
            )
        })?;
        let hits = matches
            .into_iter()
            .skip(page.offset() as usize)
            .take(page.items_per_page() as usize)
            .map(|hit| DataSearchHit {
                data: hit.data.clone(),
                distance_meters: hit.distance_meters,
            })
            .collect();

        Ok(DataSearchOutput {
            hits,
            properties: index.properties().to_vec(),
            paginator: OffsetPaginator::new(page, total),
            fingerprint: format!("{data_fingerprint}:{visibility:?}"),
            publicly_cacheable: *repo.is_public()
                && visibility == Visibility::PublishedOnly,
        })
    }
}

fn properties_digest(properties: &[Property]) -> String {
    let mut hasher = Sha256::new();
    for property in properties {
        hasher.update(property.id().to_string());
        hasher.update([0]);
        hasher.update(property.name());
        hasher.update([0]);
        hasher.update(format!("{:?}", property.property_type()));
        hasher.update([0xff]);
    }
    hex::encode(&hasher.finalize()[..8])
}
