use std::sync::Arc;

use chrono::Utc;

use crate::domain::{
    Data, DataId, DataRepository, PropertyData, PropertyRepository,
};
use crate::usecase::{AddDataInputData, AddDataInputPort};
#[derive(Debug, Clone)]
pub struct AddDataInteractorImpl {
    property_repo: Arc<dyn PropertyRepository>,
    data_repo: Arc<dyn DataRepository>,
}

impl AddDataInteractorImpl {
    pub fn new(
        property_repo: Arc<dyn PropertyRepository>,
        data_repo: Arc<dyn DataRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            property_repo,
            data_repo,
        })
    }
}

#[async_trait::async_trait]
impl AddDataInputPort for AddDataInteractorImpl {
    async fn execute(
        &self,
        input: AddDataInputData<'_>,
    ) -> errors::Result<Data> {
        let properies = self
            .property_repo
            .find_all(input.database_id, input.tenant_id)
            .await?;

        let mut property_data_list = Vec::new();
        for val in input.property_data.into_iter() {
            let property = properies
                .iter()
                .find(|x| x.id() == &val.property_id)
                .ok_or(errors::not_found!("property not found"))?;
            let col = PropertyData::new(property, val.value)?;
            property_data_list.push(col);
        }
        let data = Data::new(
            &DataId::default(),
            input.tenant_id,
            input.database_id,
            input.name,
            property_data_list,
            Utc::now(),
            Utc::now(),
        )?;

        self.data_repo.create(&data).await?;
        Ok(data)
    }
}
