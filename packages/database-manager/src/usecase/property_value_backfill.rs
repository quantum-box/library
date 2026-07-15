use std::sync::Arc;

use super::{
    PropertyValueBackfillInputData, PropertyValueBackfillInputPort,
    PropertyValueBackfillOutputPort, PropertyValueBackfillReport,
};

pub const MAX_PROPERTY_VALUE_BACKFILL_BATCH_SIZE: u16 = 1_000;

#[derive(Debug, Clone)]
pub struct PropertyValueBackfillInteractor<O>
where
    O: PropertyValueBackfillOutputPort,
{
    output: Arc<O>,
}

impl<O> PropertyValueBackfillInteractor<O>
where
    O: PropertyValueBackfillOutputPort,
{
    pub fn new(output: Arc<O>) -> Arc<Self> {
        Arc::new(Self { output })
    }
}

#[async_trait::async_trait]
impl<O> PropertyValueBackfillInputPort
    for PropertyValueBackfillInteractor<O>
where
    O: PropertyValueBackfillOutputPort,
{
    async fn execute(
        &self,
        input: &PropertyValueBackfillInputData<'_>,
    ) -> errors::Result<PropertyValueBackfillReport> {
        if input.batch_size == 0
            || input.batch_size > MAX_PROPERTY_VALUE_BACKFILL_BATCH_SIZE
        {
            return Err(errors::Error::invalid(format!(
                "PropertyValue backfill batch size must be between 1 and {MAX_PROPERTY_VALUE_BACKFILL_BATCH_SIZE}"
            )));
        }
        self.output.execute_chunk(input).await
    }
}
