mod currency;
pub use currency::{Currency, JPY};

mod percent;
pub use percent::*;

mod primitives;
pub use primitives::*;

mod repository;
pub use repository::*;

mod address;
pub use address::*;

mod name;
pub use name::*;

mod quantity;
pub use quantity::*;

mod amount;
pub use amount::*;

mod phone_number;
pub use phone_number::*;

mod money;
pub use money::*;

mod id;
pub use id::*;

mod identifier;
pub use identifier::*;

mod database_url;
pub use database_url::*;

mod paginator;
pub use paginator::*;

mod period;
pub use period::*;

mod date_range;
pub use date_range::*;

mod query;
pub use query::*;

mod billing_frequency;
pub use billing_frequency::*;

mod product_code;
pub use product_code::*;

mod file;
pub use file::*;

mod location;
pub use location::*;

mod model_name;
pub use model_name::*;

#[cfg(feature = "async-graphql")]
pub mod graphql;

mod actor_id;
pub use actor_id::*;

pub use std::str::FromStr;
pub use ulid::Ulid;
use util::{def_id, def_id_serde_impls};

// re-export
pub use email_address::EmailAddress;

// re-export
pub use url::Url;

#[macro_use]
extern crate rust_i18n;

i18n!("locales", fallback = "ja");

def_id!(TenantId, "tn_");
def_id!(ContractId, "ct_");
def_id!(UserId, "us_");
def_id!(ServiceAccountId, "sa_");

def_id!(FileId, "file_");

pub type PlatformId = TenantId;
pub type OperatorId = TenantId;

def_id!(HostId, "tn_");

#[async_trait::async_trait]
pub trait Repository<ID, T, C = Vec<T>>:
    std::marker::Send + Sync + std::fmt::Debug
{
    async fn insert(&self, entity: &T) -> anyhow::Result<()>;
    async fn update(&self, _entity: &T) -> anyhow::Result<()> {
        unimplemented!()
    }
    async fn get_by_id(&self, id: ID) -> anyhow::Result<Option<T>>;
    async fn find_all(&self, tenant_id: &TenantId) -> anyhow::Result<C>;
    async fn delete(&self, id: ID) -> anyhow::Result<()>;
}

/// TODO: add English documentation
/// TODO: add English documentation
#[async_trait::async_trait]
pub trait RepositoryForATenant<ID, T, C = Vec<T>>:
    std::marker::Send + Sync + std::fmt::Debug
{
    async fn insert(&self, entity: &T) -> errors::Result<()>;
    async fn update(&self, _entity: &T) -> errors::Result<()> {
        unimplemented!()
    }
    async fn delete(
        &self,
        _tenant_id: &TenantId,
        _id: &ID,
    ) -> errors::Result<()> {
        unimplemented!()
    }

    async fn get_by_id(
        &self,
        tenant_id: &TenantId,
        id: &ID,
    ) -> errors::Result<Option<T>>;
    async fn find_all(&self, tenant_id: &TenantId) -> errors::Result<C>;
}

#[derive(Debug, Clone)]
pub enum IdOrEmail<ID: FromStr> {
    Id(ID),
    Email(EmailAddress),
}

mod presigned_url;
pub use presigned_url::*;

mod username;
pub use username::Username;

mod nano_dollar;
pub use nano_dollar::*;

mod usd_cents;
pub use usd_cents::*;

mod usd;
pub use usd::USD;

mod action_string;
pub use action_string::*;

mod product_id;
pub use product_id::*;
