use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::types::{
    Identifier, OperatorId, PlatformId, PolicyId, PublicApiKeyId,
    PublicApiKeyValue, ServiceAccountId, TenantId, UserId,
};

// ───────────────────── DefaultRole ─────────────────────

/// Default role assigned to users within an operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
pub enum DefaultRole {
    Owner,
    Manager,
    General,
    Store,
}

impl fmt::Display for DefaultRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owner => write!(f, "Owner"),
            Self::Manager => write!(f, "Manager"),
            Self::General => write!(f, "General"),
            Self::Store => write!(f, "Store"),
        }
    }
}

impl FromStr for DefaultRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Owner" => Ok(Self::Owner),
            "Manager" => Ok(Self::Manager),
            "General" => Ok(Self::General),
            "Store" => Ok(Self::Store),
            other => Err(format!("unknown DefaultRole: `{other}`")),
        }
    }
}

// ───────────────────── User ────────────────────────────

/// Represents an authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub tenants: Vec<TenantId>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub email_verified: Option<DateTime<Utc>>,
    pub image: Option<String>,
    pub role: DefaultRole,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn tenants(&self) -> &[TenantId] {
        &self.tenants
    }

    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn image(&self) -> Option<&str> {
        self.image.as_deref()
    }

    pub fn email_verified(&self) -> &Option<DateTime<Utc>> {
        &self.email_verified
    }

    pub fn metadata(&self) -> Option<&HashMap<String, String>> {
        self.metadata.as_ref()
    }

    pub fn role(&self) -> &DefaultRole {
        &self.role
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }
}

// ───────────────────── Operator ────────────────────────

/// Represents a tenant operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operator {
    pub id: TenantId,
    pub name: String,
    pub operator_name: Identifier,
    pub platform_id: TenantId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Operator {
    pub fn id(&self) -> &TenantId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn operator_name(&self) -> &Identifier {
        &self.operator_name
    }

    pub fn platform_id(&self) -> &TenantId {
        &self.platform_id
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }
}

// ───────────────────── TenantHierarchy ─────────────────

/// Resolved tenant hierarchy (Host → Platform → Operator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantHierarchy {
    pub host_id: TenantId,
    pub platform_id: Option<PlatformId>,
    pub operator_id: Option<OperatorId>,
}

// ───────────────────── ServiceAccount ──────────────────

/// Machine user for API access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccount {
    pub id: ServiceAccountId,
    pub tenant_id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl ServiceAccount {
    pub fn id(&self) -> &ServiceAccountId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }
}

// ───────────────────── PublicApiKey ─────────────────────

/// API key associated with a service account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicApiKey {
    pub id: PublicApiKeyId,
    pub tenant_id: TenantId,
    pub service_account_id: ServiceAccountId,
    pub name: String,
    pub value: PublicApiKeyValue,
    pub created_at: DateTime<Utc>,
}

impl PublicApiKey {
    pub fn id(&self) -> &PublicApiKeyId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn service_account_id(&self) -> &ServiceAccountId {
        &self.service_account_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &PublicApiKeyValue {
        &self.value
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }
}

// ───────────────────── Policy ──────────────────────────

/// Represents an authorization policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub name: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub tenant_id: Option<TenantId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Policy {
    pub fn id(&self) -> &PolicyId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn is_system(&self) -> bool {
        self.is_system
    }

    pub fn tenant_id(&self) -> Option<&TenantId> {
        self.tenant_id.as_ref()
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }
}

// ───────────────────── UserPolicy ──────────────────────

/// Mapping between a user, a policy, and a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPolicy {
    pub user_id: UserId,
    pub policy_id: PolicyId,
    pub tenant_id: TenantId,
    pub resource_scope: Option<String>,
    pub assigned_at: DateTime<Utc>,
}

impl UserPolicy {
    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn policy_id(&self) -> &PolicyId {
        &self.policy_id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn resource_scope(&self) -> Option<&str> {
        self.resource_scope.as_deref()
    }

    pub fn assigned_at(&self) -> &DateTime<Utc> {
        &self.assigned_at
    }
}

// ───────────────────── OAuth ───────────────────────────

/// Simplified OAuth token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub provider: String,
    pub access_token: String,
}

/// OAuth token with detailed information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenDetail {
    pub provider: String,
    pub provider_user_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
}

impl OAuthTokenDetail {
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }
}

// ───────────────────── OAuth2 Client ───────────────────

/// Result of creating an OAuth2 client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ClientCreated {
    pub client_id: String,
    pub client_secret: String,
}

// ─────────────────── NewOperatorOwnerMethod ────────────

/// How the owner of a new operator is determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "graphql", derive(async_graphql::Enum))]
pub enum NewOperatorOwnerMethod {
    Inherit,
    Create,
}

// ─────────────── PolicyActionRequest ───────────────────

/// Action entry for policy registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyActionRequest {
    pub action_id: String,
    pub effect: String,
}

// ───────────── EvaluatePoliciesBatchOutcome ─────────────

/// Result of evaluating a single action in a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatePoliciesBatchOutcome {
    pub action: String,
    pub allowed: bool,
    pub error: Option<String>,
}

// ───────────────────── UserQuery ────────────────────────

/// Trait for querying user information.
#[async_trait::async_trait]
pub trait UserQuery: std::fmt::Debug + Send + Sync {
    async fn find_by_id(&self, id: &UserId)
        -> errors::Result<Option<User>>;

    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<User>>;

    async fn get_by_user_id(
        &self,
        tenant_id: &TenantId,
        id: &str,
    ) -> errors::Result<Option<User>>;

    async fn get_by_email(
        &self,
        tenant_id: &TenantId,
        email: &str,
    ) -> errors::Result<Option<User>>;

    async fn get_by_username(
        &self,
        username: &value_object::Username,
    ) -> errors::Result<Option<User>>;

    async fn search_by_username_prefix(
        &self,
        prefix: &str,
        limit: u32,
    ) -> errors::Result<Vec<User>>;
}

// ─────────── UserPolicyMappingRepository ────────────────

/// Repository for managing user-policy mappings.
#[async_trait::async_trait]
pub trait UserPolicyMappingRepository:
    std::fmt::Debug + Send + Sync
{
    async fn create_mapping(
        &self,
        user_id: &UserId,
        policy_id: &PolicyId,
        tenant_id: &TenantId,
    ) -> errors::Result<()>;

    async fn delete_mapping(
        &self,
        user_id: &UserId,
        policy_id: &PolicyId,
        tenant_id: &TenantId,
    ) -> errors::Result<()>;

    async fn find_policies_by_user(
        &self,
        user_id: &UserId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<PolicyId>>;

    async fn find_users_by_policy(
        &self,
        policy_id: &PolicyId,
        tenant_id: &TenantId,
    ) -> errors::Result<Vec<UserId>>;

    async fn exists_mapping(
        &self,
        user_id: &UserId,
        policy_id: &PolicyId,
        tenant_id: &TenantId,
    ) -> errors::Result<bool>;

    async fn create_mapping_with_scope(
        &self,
        user_id: &UserId,
        policy_id: &PolicyId,
        tenant_id: &TenantId,
        resource_scope: &str,
    ) -> errors::Result<()>;

    async fn delete_mapping_with_scope(
        &self,
        user_id: &UserId,
        policy_id: &PolicyId,
        tenant_id: &TenantId,
        resource_scope: &str,
    ) -> errors::Result<()>;

    async fn find_by_resource_scope(
        &self,
        tenant_id: &TenantId,
        resource_scope: &str,
    ) -> errors::Result<Vec<UserPolicy>>;
}
