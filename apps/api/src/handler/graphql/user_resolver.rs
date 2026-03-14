use super::model::{Operator, User};
use crate::sdk_auth::SdkAuthApp;
use async_graphql::{Context, Result};
use std::collections::HashSet;
use std::sync::Arc;
use tachyon_sdk::auth::MultiTenancyAction;

#[async_graphql::ComplexObject]
impl User {
    /// Resolve operator organizations accessible to this user
    #[tracing::instrument(name = "organizations_by_user", skip_all)]
    async fn organizations(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<Operator>> {
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;
        let _executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let user_id = &self.id;

        let mut operators = Vec::new();
        let mut seen = HashSet::new();

        // If platform_id is available, fetch
        // operators via REST.
        if let Some(platform_id) = multi_tenancy.platform_id() {
            match sdk.find_operators_by_user(&platform_id, user_id).await {
                Ok(resp_operators) => {
                    for op in resp_operators {
                        if seen.insert(op.id.clone()) {
                            match crate::sdk_auth::operator_from_resp(&op) {
                                Ok(operator) => operators.push(operator),
                                Err(err) => {
                                    tracing::warn!(
                                        operator_id = %op.id,
                                        error = ?err,
                                        "Failed to parse operator"
                                    );
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::error!(
                        error = ?err,
                        "Failed to load platform operators"
                    );
                    return Err(async_graphql::Error::new(err.to_string()));
                }
            }
        } else {
            // Fallback: iterate user's tenants and
            // resolve each operator by ID.
            for tenant_id in &self.tenant_id_list {
                if seen.insert(tenant_id.clone()) {
                    match sdk.get_operator(tenant_id).await {
                        Ok(Some(op)) => {
                            match crate::sdk_auth::operator_from_resp(&op) {
                                Ok(operator) => operators.push(operator),
                                Err(err) => {
                                    tracing::warn!(
                                        tenant_id = %tenant_id,
                                        error = ?err,
                                        "Failed to parse operator"
                                    );
                                }
                            }
                        }
                        Ok(None) => {
                            tracing::warn!(
                                tenant_id = %tenant_id,
                                "Operator not found"
                            );
                        }
                        Err(err) => {
                            tracing::warn!(
                                tenant_id = %tenant_id,
                                error = ?err,
                                "Failed to load operator"
                            );
                        }
                    }
                }
            }
        }

        Ok(operators.into_iter().map(Into::into).collect())
    }
}
