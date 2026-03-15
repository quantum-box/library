//! Authentication and authorization contract for the
//! Tachyon platform.
//!
//! This module provides the [`AuthApp`] trait and all
//! associated types needed to integrate with Tachyon's
//! auth system. The SDK ships a REST-based implementation
//! that delegates to a running Tachyon API server.

pub mod domain;
pub mod error;
pub mod executor;
pub mod inputs;
pub mod traits;
pub mod types;

// Re-export everything at the auth module level for
// convenient access.

// ID types
pub use types::{
    Identifier, OperatorId, PlatformId, PolicyId, PublicApiKeyId,
    PublicApiKeyValue, ServiceAccountId, TenantId, UserId,
};

// Domain types
pub use domain::{
    DefaultRole, EvaluatePoliciesBatchOutcome, NewOperatorOwnerMethod,
    OAuth2ClientCreated, OAuthToken, OAuthTokenDetail, Operator, Policy,
    PolicyActionRequest, PublicApiKey, ServiceAccount, TenantHierarchy,
    User, UserPolicy, UserPolicyMappingRepository, UserQuery,
};

// Executor / Multi-tenancy
pub use executor::{
    Executor, ExecutorAction, MultiTenancy, MultiTenancyAction,
};

// Input types
pub use inputs::{
    AddUserToTenantInput, AttachSaPolicyInput, AttachUserPolicyInput,
    AttachUserPolicyWithScopeInput, CheckPolicyForResourceInput,
    CheckPolicyInput, CreateOAuth2ClientInput, CreateOperatorInput,
    CreatePublicApiKeyInput, CreateServiceAccountInput,
    DeleteOAuthTokenInput, DeleteOperatorInput, DeleteServiceAccountInput,
    DetachUserPolicyInput, DetachUserPolicyWithScopeInput,
    EvaluatePoliciesBatchInput, FindAllPublicApiKeyInput,
    FindOAuth2ClientByNameInput, FindPolicyByNameInput,
    FindUsersByTenantInput, GetOAuthTokenByProviderInput,
    GetOperatorByIdInput, GetOperatorByIdentifierInput, GetPolicyByIdInput,
    GetServiceAccountByNameInput, GetUserByIdInput,
    GetUserIdByUserProviderIdInput, OAuthTokenInput, RegisterPolicyInput,
    SaveOAuthTokenInput, UpdateServiceAccountInput,
};

// Main trait
pub use traits::AuthApp;

// Mock (test only)
#[cfg(feature = "test")]
pub use traits::MockAuthApp;

#[cfg(feature = "test")]
pub use executor::{MockExecutorAction, MockMultiTenancyAction};
