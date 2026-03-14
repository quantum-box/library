use std::sync::Arc;

use crate::domain::{
    Data, DataRepository, Database, DatabaseId, PropertyData, PropertyId,
    PropertyRepository,
};
use crate::usecase::{UpdateDataInputData, UpdateDataInputPort};
use value_object::RepositoryV1;

#[derive(Debug)]
pub struct UpdateDataInteractorImpl {
    database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
    property_repo: Arc<dyn PropertyRepository>,
    data_repo: Arc<dyn DataRepository>,
}

impl UpdateDataInteractorImpl {
    pub fn new(
        database_repo: Arc<dyn RepositoryV1<DatabaseId, Database>>,
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            database_repo,
            property_repo,
            data_repo,
        })
    }
}

#[async_trait::async_trait]
impl UpdateDataInputPort for UpdateDataInteractorImpl {
    #[tracing::instrument(skip(self))]
    async fn execute(
        &self,
        input: UpdateDataInputData<'_>,
    ) -> errors::Result<Data> {
        let database = self
            .database_repo
            .get_by_id(input.tenant_id, input.database_id)
            .await?
            .ok_or(errors::not_found!(
                "database is not found in update data"
            ))?;
        let property = self
            .property_repo
            .find_all(database.id(), database.tenant_id())
            .await?;
        let mut data = self
            .data_repo
            .find_by_id(input.data_id, database.id(), database.tenant_id())
            .await?
            .ok_or(errors::not_found!(
                "data is not found in update data"
            ))?;

        data.update_name(&input.name.parse()?);
        for d in input.data {
            let property = property
                .iter()
                .find(|p| {
                    PropertyId::new(&d.property_id).unwrap().eq(p.id())
                })
                .ok_or(errors::not_found!(
                    "property is not found in update data"
                ))?;
            data.update_property_data(&PropertyData::new(
                property,
                d.value.to_string(),
            )?)?;
        }
        self.data_repo.update(&data).await?;

        Ok(data)
    }
}
