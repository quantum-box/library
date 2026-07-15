use crate::usecase::database_scope::DatabaseScope;
use crate::{
    domain::{Data, DataRepository, Database, DatabaseId},
    SearchDataInputData, SearchDataInputPort,
};
use errors;
use value_object::OffsetPaginator;
use value_object::{RepositoryV1, TenantId};

use std::fmt::Debug;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait DataQuery: Debug + Send + Sync + 'static {
    async fn search_by_name(
        &self,
        tenant_id: &TenantId,
        database_id: &DatabaseId,
        name: &str,
        page: u32,
        page_size: u32,
    ) -> anyhow::Result<(Vec<Data>, OffsetPaginator)>;
}

#[derive(Debug, Clone)]
pub struct SearchData {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    data_repo: Arc<dyn DataRepository>,
    data_query: Arc<dyn DataQuery>,
}

impl SearchData {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        data_repo: Arc<dyn DataRepository>,
        data_query: Arc<dyn DataQuery>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            data_repo,
            data_query,
        })
    }
}

#[async_trait::async_trait]
impl SearchDataInputPort for SearchData {
    #[tracing::instrument(skip(self))]
    async fn execute(
        &self,
        input: &SearchDataInputData,
    ) -> errors::Result<(Vec<Data>, OffsetPaginator)> {
        let page = input.page.unwrap_or(1);
        let page_size = input.page_size.unwrap_or(20);
        if let Some(database_id) = input.database_id.clone() {
            let scope = DatabaseScope::new(input.tenant_id, &database_id);
            let database =
                scope.require_database(self.database_repo.as_ref()).await?;
            if input.query.is_empty() {
                let (data, paginator) = self
                    .data_repo
                    .find_all_with_paging(
                        database.tenant_id(),
                        database.id(),
                        page,
                        page_size,
                    )
                    .await?;
                return Ok((data.value().to_vec(), paginator));
            }
            let (data, paginator) = self
                .data_query
                .search_by_name(
                    database.tenant_id(),
                    database.id(),
                    input.query,
                    page,
                    page_size,
                )
                .await?;
            return Ok((data, paginator));
        }

        Err(errors::Error::not_supported(
            "search_data without database_id is not available in Library GA",
        ))
    }
}
