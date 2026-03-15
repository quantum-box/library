use std::env;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::RequestPartsExt;
use axum::{http::request::Parts, Extension};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use inbound_sync::sdk::SystemExecutor;
use tachyon_sdk::auth::{ExecutorAction, ServiceAccount, User};
use value_object::TenantId;

use crate::sdk_auth::SdkAuthApp;
use crate::usecase::ViewOrgOutputData;

use super::extract_org_username;

// ---- Conversion: LibraryExecutor → tachyon_sdk::auth::Executor ----

impl From<LibraryExecutor> for tachyon_sdk::auth::Executor {
    fn from(le: LibraryExecutor) -> Self {
        match le.inner {
            LibraryExecutorKind::User(u) => {
                tachyon_sdk::auth::Executor::User(u)
            }
            LibraryExecutorKind::ServiceAccount(sa) => {
                tachyon_sdk::auth::Executor::ServiceAccount(sa)
            }
            LibraryExecutorKind::None => tachyon_sdk::auth::Executor::None,
        }
    }
}

// ---- LibraryMultiTenancy ----
//
// SDK-based replacement for tachyon_sdk::auth::MultiTenancy as an axum
// extractor. The auth crate's FromRequestParts implementation
// requires Extension<Arc<tachyon_sdk::auth::AuthApp>> which library-api does
// not have. This extractor parses x-operator-id / x-platform-id
// headers and optionally validates the operator via SdkAuthApp
// REST calls.

pub struct LibraryMultiTenancy(pub tachyon_sdk::auth::MultiTenancy);

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for LibraryMultiTenancy
where
    S: Send + Sync,
{
    type Rejection = errors::Error;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let operator_id = parts
            .headers
            .get("x-operator-id")
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string());

        let platform_id = parts
            .headers
            .get("x-platform-id")
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string());

        let operator =
            operator_id.map(|id| id.parse::<TenantId>()).transpose()?;
        let platform =
            platform_id.map(|id| id.parse::<TenantId>()).transpose()?;

        Ok(LibraryMultiTenancy(tachyon_sdk::auth::MultiTenancy::new(
            platform, operator,
        )))
    }
}

/// Executor resolved from the incoming request's
/// Authorization header. Optionally carries the original
/// Bearer token so it can be forwarded to tachyon-api for
/// user-scoped operations.
#[derive(Debug, Clone)]
pub struct LibraryExecutor {
    pub inner: LibraryExecutorKind,
    /// The raw Bearer token from the request, if available.
    /// Used by graphql_handler to create a request-scoped
    /// SdkAuthApp that forwards the user's JWT.
    pub original_token: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LibraryExecutorKind {
    User(Box<User>),
    ServiceAccount(Box<ServiceAccount>),
    None,
}

#[async_trait::async_trait]
impl ExecutorAction for LibraryExecutor {
    fn get_id(&self) -> &str {
        self.inner.get_id()
    }

    fn has_tenant_id(&self, tenant_id: &TenantId) -> bool {
        self.inner.has_tenant_id(tenant_id)
    }

    fn is_system_user(&self) -> bool {
        self.inner.is_system_user()
    }

    fn is_user(&self) -> bool {
        self.inner.is_user()
    }

    fn is_service_account(&self) -> bool {
        self.inner.is_service_account()
    }

    fn is_none(&self) -> bool {
        self.inner.is_none()
    }
}

#[async_trait::async_trait]
impl ExecutorAction for LibraryExecutorKind {
    fn get_id(&self) -> &str {
        match self {
            LibraryExecutorKind::User(user) => user.id().as_str(),
            LibraryExecutorKind::ServiceAccount(sa) => sa.id().as_str(),
            LibraryExecutorKind::None => "",
        }
    }

    fn has_tenant_id(&self, tenant_id: &TenantId) -> bool {
        match self {
            LibraryExecutorKind::User(us) => {
                us.tenants().contains(tenant_id)
            }
            LibraryExecutorKind::ServiceAccount(sa) => {
                sa.tenant_id() == tenant_id
            }
            LibraryExecutorKind::None => true,
        }
    }

    fn is_system_user(&self) -> bool {
        matches!(self, LibraryExecutorKind::None)
    }

    fn is_user(&self) -> bool {
        matches!(self, LibraryExecutorKind::User(_))
    }

    fn is_service_account(&self) -> bool {
        matches!(self, LibraryExecutorKind::ServiceAccount(_))
    }

    fn is_none(&self) -> bool {
        matches!(self, LibraryExecutorKind::None)
    }
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for LibraryExecutor
where
    S: Send + Sync,
{
    type Rejection = errors::Error;

    #[tracing::instrument(
        name = "library_executor_from_request_parts",
        level = "trace",
        skip(state)
    )]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Extension(sdk) = parts
            .extract::<Extension<Arc<SdkAuthApp>>>()
            .await
            .map_err(|_| {
                errors::Error::internal_server_error(
                    "SdkAuthApp is missing. not initialized",
                )
            })?;
        let Extension(library_app) = parts
            .extract::<Extension<Arc<crate::app::LibraryApp>>>()
            .await
            .map_err(|_| {
                errors::Error::internal_server_error(
                    "library app is missing. not initialized",
                )
            })?;

        let token =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(
                parts, state,
            )
            .await
            .map(|token| token.token().to_string());

        if token.is_err() {
            tracing::debug!("token is missing");
            return Ok(LibraryExecutor {
                inner: LibraryExecutorKind::None,
                original_token: None,
            });
        }
        let token = token.unwrap();

        let environment =
            env::var("ENVIRONMENT").unwrap_or("development".into());

        // For development / test mode with dummy token
        if token == *"dummy-token"
            && (environment == *"development" || environment == *"test")
        {
            let user_id = parts
                .headers
                .get("x-user-id")
                .and_then(|v| v.to_str().ok())
                .filter(|v| !v.is_empty())
                .unwrap_or("us_01hs2yepy5hw4rz8pdq2wywnwt")
                .to_string();

            let org_username = extract_org_username(parts);
            if let Some(org_username) = org_username {
                let ViewOrgOutputData {
                    organization: org, ..
                } = library_app
                    .view_org
                    .execute(&crate::usecase::ViewOrgInputData {
                        executor: &SystemExecutor,
                        multi_tenancy:
                            &tachyon_sdk::auth::MultiTenancy::default(),
                        organization_username: org_username,
                    })
                    .await?;

                match sdk.get_user_by_id_full(org.id(), &user_id).await {
                    Ok(Some(user)) => {
                        return Ok(LibraryExecutor {
                            inner: LibraryExecutorKind::User(Box::new(
                                user,
                            )),
                            original_token: Some(token),
                        });
                    }
                    _ => {
                        return Ok(LibraryExecutor {
                            inner: LibraryExecutorKind::None,
                            original_token: Some(token),
                        });
                    }
                }
            } else {
                let platform_id = parts
                    .headers
                    .get("x-platform-id")
                    .map(|value| value.to_str().unwrap().to_string())
                    .unwrap_or("tn_01j702qf86pc2j35s0kv0gv3gy".into());

                let tenant_id = TenantId::new(&platform_id)?;

                match sdk.get_user_by_id_full(&tenant_id, &user_id).await {
                    Ok(Some(user)) => {
                        return Ok(LibraryExecutor {
                            inner: LibraryExecutorKind::User(Box::new(
                                user,
                            )),
                            original_token: Some(token),
                        });
                    }
                    _ => {
                        return Ok(LibraryExecutor {
                            inner: LibraryExecutorKind::None,
                            original_token: Some(token),
                        });
                    }
                }
            }
        }

        // For service account authentication
        if token.starts_with("pk_") {
            let org_username = extract_org_username(parts);
            if let Some(org_username) = org_username {
                let ViewOrgOutputData {
                    organization: org, ..
                } = library_app
                    .view_org
                    .execute(&crate::usecase::ViewOrgInputData {
                        executor: &SystemExecutor,
                        multi_tenancy:
                            &tachyon_sdk::auth::MultiTenancy::default(),
                        organization_username: org_username,
                    })
                    .await?;

                let out = sdk
                    .verify_api_key(org.id(), token.as_str())
                    .await
                    .map_err(|err| {
                        tracing::error!("middleware error: {:?}", err);
                        errors::Error::unauthenticated(format!(
                            "verify public api key \
                                 failed error: {err}"
                        ))
                    })?;
                return Ok(LibraryExecutor {
                    inner: LibraryExecutorKind::ServiceAccount(Box::new(
                        out,
                    )),
                    // pk_* token: keep for forwarding
                    original_token: Some(token),
                });
            }
        }

        // JWT token: verify and store for forwarding.
        // If verification fails (e.g. non-JWT token like dummy-token
        // sent from a dev client, or an expired token), treat the
        // request as unauthenticated rather than returning an error.
        // Mutations that require authentication will reject None
        // executor via their own policy checks.
        match sdk.verify_token(&token).await {
            Ok(user) => Ok(LibraryExecutor {
                inner: LibraryExecutorKind::User(Box::new(user)),
                original_token: Some(token),
            }),
            Err(err) => {
                tracing::warn!(
                    "JWT verification failed, \
                     treating request as unauthenticated: {err}"
                );
                Ok(LibraryExecutor {
                    inner: LibraryExecutorKind::None,
                    original_token: None,
                })
            }
        }
    }
}
