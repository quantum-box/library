use crate::{
    domain::{Data, DataRepository, DatabaseId},
    SearchDataInputData, SearchDataInputPort,
};
use errors;
use value_object::OffsetPaginator;
use value_object::TenantId;

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
    data_repo: Arc<dyn DataRepository>,
    data_query: Arc<dyn DataQuery>,
}

impl SearchData {
    pub fn new(
        data_repo: Arc<dyn DataRepository>,
        data_query: Arc<dyn DataQuery>,
    ) -> Arc<Self> {
        Arc::new(Self {
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
            if input.query.is_empty() {
                let (data, paginator) = self
                    .data_repo
                    .find_all_with_paging(
                        input.tenant_id,
                        &database_id,
                        page,
                        page_size,
                    )
                    .await?;
                return Ok((data.value().to_vec(), paginator));
            }
            let (data, paginator) = self
                .data_query
                .search_by_name(
                    input.tenant_id,
                    &database_id,
                    input.query,
                    page,
                    page_size,
                )
                .await?;
            return Ok((data, paginator));
        } else {
            // TODO: add English comment
            unimplemented!()
        }
    }
}
