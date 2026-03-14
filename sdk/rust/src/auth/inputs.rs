use super::domain::{
    NewOperatorOwnerMethod, PolicyActionRequest,
};
use super::executor::{ExecutorAction, MultiTenancyAction};
use super::types::{
    Identifier, OperatorId, PlatformId, PolicyId,
    ServiceAccountId, TenantId, UserId,
};

// ─────────────── Policy check inputs ──────────────────

#[derive(Debug, Clone)]
pub struct CheckPolicyInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub action: &'a str,
}

#[derive(Debug, Clone)]
pub struct EvaluatePoliciesBatchInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub actions: &'a [&'a str],
}

#[derive(Debug, Clone)]
pub struct CheckPolicyForResourceInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub action: &'a str,
    pub resource_trn: &'a str,
}

// ─────────────── User provider lookup ─────────────────

#[derive(Debug, Clone)]
pub struct GetUserIdByUserProviderIdInput<'a> {
    pub tenant_id: &'a TenantId,
    pub provider_user_id: &'a str,
}

// ──────────────── Operator inputs ──────────────────────

#[derive(Debug, Clone)]
pub struct DeleteOperatorInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub platform_id: &'a PlatformId,
    pub operator_id: &'a OperatorId,
}

#[derive(Debug, Clone)]
pub struct GetOperatorByIdentifierInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub platform_id: &'a PlatformId,
    pub operator_identifier: &'a Identifier,
}

#[derive(Debug, Clone)]
pub struct GetOperatorByIdInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub operator_id: &'a OperatorId,
}

#[derive(Debug, Clone)]
pub struct CreateOperatorInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub platform_id: &'a PlatformId,
    pub operator_alias: &'a Identifier,
    pub operator_name: &'a str,
    pub new_operator_owner_method: NewOperatorOwnerMethod,
    pub new_operator_owner_id: &'a UserId,
    pub new_operator_owner_password: Option<&'a str>,
}

// ──────────────── OAuth inputs ─────────────────────────

#[derive(Debug, Clone)]
pub struct OAuthTokenInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
}

#[derive(Debug, Clone)]
pub struct GetOAuthTokenByProviderInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub provider: &'a str,
}

#[derive(Debug, Clone)]
pub struct SaveOAuthTokenInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub provider: &'a str,
    pub provider_user_id: &'a str,
    pub access_token: &'a str,
    pub refresh_token: Option<&'a str>,
    pub expires_in: i64,
}

#[derive(Debug, Clone)]
pub struct DeleteOAuthTokenInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub provider: &'a str,
}

// ────────── Service account inputs ─────────────────────

#[derive(Debug, Clone)]
pub struct CreateServiceAccountInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub tenant_id: &'a TenantId,
    pub name: &'a str,
}

#[derive(Debug, Clone)]
pub struct UpdateServiceAccountInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub service_account_id: &'a ServiceAccountId,
    pub name: &'a str,
}

#[derive(Debug, Clone)]
pub struct DeleteServiceAccountInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub service_account_id: &'a ServiceAccountId,
}

#[derive(Debug, Clone)]
pub struct GetServiceAccountByNameInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub tenant_id: &'a TenantId,
    pub name: &'a str,
}

// ────────── Public API key inputs ──────────────────────

#[derive(Debug, Clone)]
pub struct FindAllPublicApiKeyInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub operator_id: &'a OperatorId,
    pub service_account_id: &'a ServiceAccountId,
}

#[derive(Debug, Clone)]
pub struct CreatePublicApiKeyInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub operator_id: &'a OperatorId,
    pub service_account_id: &'a ServiceAccountId,
    pub name: &'a str,
}

// ────────── User policy inputs ─────────────────────────

#[derive(Debug, Clone)]
pub struct AttachUserPolicyInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub user_id: &'a UserId,
    pub policy_id: &'a PolicyId,
    pub tenant_id: &'a TenantId,
}

#[derive(Debug, Clone)]
pub struct DetachUserPolicyInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub user_id: &'a UserId,
    pub policy_id: &'a PolicyId,
    pub tenant_id: &'a TenantId,
}

#[derive(Debug, Clone)]
pub struct AttachUserPolicyWithScopeInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub user_id: &'a UserId,
    pub policy_id: &'a PolicyId,
    pub tenant_id: &'a TenantId,
    pub resource_scope: &'a str,
}

#[derive(Debug, Clone)]
pub struct DetachUserPolicyWithScopeInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub user_id: &'a UserId,
    pub policy_id: &'a PolicyId,
    pub tenant_id: &'a TenantId,
    pub resource_scope: &'a str,
}

// ────────── User/tenant inputs ─────────────────────────

#[derive(Debug, Clone)]
pub struct AddUserToTenantInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub user_id: &'a UserId,
    pub tenant_id: &'a TenantId,
}

#[derive(Debug, Clone)]
pub struct GetUserByIdInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub user_id: &'a UserId,
}

#[derive(Debug, Clone)]
pub struct FindUsersByTenantInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub tenant_id: &'a TenantId,
}

// ────────── Policy management inputs ───────────────────

#[derive(Debug, Clone)]
pub struct GetPolicyByIdInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub policy_id: &'a PolicyId,
}

#[derive(Debug, Clone)]
pub struct RegisterPolicyInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub tenant_id: &'a TenantId,
    pub actions: Vec<PolicyActionRequest>,
}

#[derive(Debug, Clone)]
pub struct FindPolicyByNameInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub tenant_id: &'a TenantId,
    pub name: &'a str,
}

// ────────── Service account policy inputs ──────────────

#[derive(Debug, Clone)]
pub struct AttachSaPolicyInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub service_account_id: &'a ServiceAccountId,
    pub policy_id: &'a PolicyId,
}

// ────────── OAuth2 client inputs ───────────────────────

#[derive(Debug, Clone)]
pub struct CreateOAuth2ClientInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub name: &'a str,
    pub redirect_uris: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub grant_types: Vec<String>,
    pub use_tachyon_user_pool: bool,
}

#[derive(Debug, Clone)]
pub struct FindOAuth2ClientByNameInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub tenant_id: &'a TenantId,
    pub name: &'a str,
}
