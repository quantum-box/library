use std::sync::Arc;

use crate::domain::{Database, DatabaseId};
use crate::usecase::{
    DatabaseDeletionUnitOfWork, DeleteDatabaseInputData,
    DeleteDatabaseInputPort,
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct DeleteDatabaseInteractor {
    deletion_uow: Arc<dyn DatabaseDeletionUnitOfWork>,
}

impl DeleteDatabaseInteractor {
    pub fn new(
        deletion_uow: Arc<dyn DatabaseDeletionUnitOfWork>,
    ) -> Arc<Self> {
        Arc::new(Self { deletion_uow })
    }
}

#[async_trait::async_trait]
impl DeleteDatabaseInputPort for DeleteDatabaseInteractor {
    async fn execute(
        &self,
        input: &DeleteDatabaseInputData<'_>,
    ) -> errors::Result<Database> {
        let tenant_id = input.tenant_id.parse()?;
        let database_id = DatabaseId::from_str(input.database_id)?;
        self.deletion_uow
            .delete_atomically(&tenant_id, &database_id)
            .await
    }
}
