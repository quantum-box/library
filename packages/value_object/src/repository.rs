use crate::TenantId;
use errors::Result;
use std::fmt::Debug;

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait RepositoryV1<ID, E, C = Vec<E>>:
    Send + Sync + Debug + 'static
where
    ID: Debug + Send + Sync + 'static,
    E: Debug + Send + Sync + 'static,
    C: Debug + Send + Sync + 'static,
{
    async fn save(&self, entity: &E) -> Result<()>;
    async fn delete(&self, tenant_id: &TenantId, id: &ID) -> Result<()>;
    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &ID,
    ) -> Result<Option<E>>;
    async fn find_all(&self, tenant_id: &TenantId) -> Result<C>;
}
