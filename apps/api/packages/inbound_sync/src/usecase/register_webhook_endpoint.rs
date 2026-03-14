//! Register a new webhook endpoint use case.

use std::sync::Arc;

use derive_new::new;
use inbound_sync_domain::{
    EndpointStatus, PropertyMapping, Provider, ProviderConfig,
    WebhookEndpoint, WebhookEndpointId, WebhookEndpointRepository,
};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};

// =============================================================================
// RegisterWebhookEndpoint
// =============================================================================

/// Input data for registering a webhook endpoint.
#[derive(Debug)]
pub struct RegisterWebhookEndpointInputData<'a> {
    /// Executor (user or service account)
    pub executor: &'a dyn ExecutorAction,
    /// Multi-tenancy context
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Display name
    pub name: String,
    /// Provider type
    pub provider: Provider,
    /// Provider-specific configuration
    pub config: ProviderConfig,
    /// Events to listen for (empty = all events)
    pub events: Vec<String>,
    /// Target repository ID (optional)
    pub repository_id: Option<String>,
    /// Property mapping configuration (optional)
    pub mapping: Option<PropertyMapping>,
}

/// Output data for register webhook endpoint.
#[derive(Debug)]
pub struct RegisterWebhookEndpointOutputData {
    /// Created endpoint
    pub endpoint: WebhookEndpoint,
    /// Generated webhook URL
    pub webhook_url: String,
    /// Plain text secret (only returned on creation)
    pub secret: String,
}

/// Input port for register webhook endpoint use case.
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait RegisterWebhookEndpointInputPort:
    std::fmt::Debug + Send + Sync
{
    fn policy(&self) -> &'static str {
        "inbound_sync:RegisterWebhookEndpoint"
    }

    async fn execute<'a>(
        &self,
        input: RegisterWebhookEndpointInputData<'a>,
    ) -> errors::Result<RegisterWebhookEndpointOutputData>;
}

/// Register webhook endpoint use case.
#[derive(Debug, Clone, new)]
pub struct RegisterWebhookEndpoint {
    auth: Arc<dyn AuthApp>,
    repository: Arc<dyn WebhookEndpointRepository>,
    base_url: String,
}

#[async_trait::async_trait]
impl RegisterWebhookEndpointInputPort for RegisterWebhookEndpoint {
    #[tracing::instrument(
        name = "inbound_sync::RegisterWebhookEndpoint::execute",
        skip(self, input),
        fields(provider = %input.provider)
    )]
    async fn execute<'a>(
        &self,
        input: RegisterWebhookEndpointInputData<'a>,
    ) -> errors::Result<RegisterWebhookEndpointOutputData> {
        // 1. Policy check
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: self.policy(),
            })
            .await?;

        // 2. Get tenant ID
        let tenant_id = input.multi_tenancy.get_operator_id()?;

        // 3. Validate provider config matches provider type
        if input.config.provider() != input.provider {
            return Err(errors::Error::bad_request(format!(
                "Provider config type {:?} does not match provider {:?}",
                input.config.provider(),
                input.provider
            )));
        }

        // 4. Generate a secure secret (Linear can use provider secret)
        let mut config = input.config;
        let mut secret_value: Option<String> = None;
        let mut secret_hash: Option<String> = None;
        let mut return_secret = true;

        if input.provider == Provider::Linear {
            let provided_secret = match &config {
                ProviderConfig::Linear { webhook_secret, .. } => {
                    webhook_secret
                        .as_ref()
                        .filter(|s| !s.trim().is_empty())
                        .cloned()
                }
                _ => None,
            };

            if let Some(secret) = provided_secret {
                secret_value = Some(secret);
                return_secret = false;
            } else {
                let existing = self
                    .repository
                    .find_by_tenant_and_provider(
                        &tenant_id,
                        Provider::Linear,
                    )
                    .await?;
                if let Some(endpoint) = existing.first() {
                    if let Some(existing_secret) = endpoint.webhook_secret()
                    {
                        secret_value = Some(existing_secret.to_string());
                    } else {
                        secret_hash =
                            Some(endpoint.secret_hash().to_string());
                    }
                    return_secret = false;
                } else {
                    secret_value = Some(generate_secret());
                }
            }

            if let Some(secret) = secret_value.as_ref() {
                if let ProviderConfig::Linear { webhook_secret, .. } =
                    &mut config
                {
                    *webhook_secret = Some(secret.clone());
                }
                secret_hash = Some(hash_secret(secret));
            }
        } else {
            let secret = generate_secret();
            secret_hash = Some(hash_secret(&secret));
            secret_value = Some(secret);
        }

        let secret_hash = secret_hash.unwrap_or_else(|| {
            let secret =
                secret_value.as_ref().expect("secret should be available");
            hash_secret(secret)
        });

        // 5. Create the endpoint
        let mut endpoint = WebhookEndpoint::create(
            tenant_id,
            input.name,
            input.provider,
            config,
            input.events,
            secret_hash,
        );

        // Set optional fields
        if input.repository_id.is_some() {
            endpoint.set_repository_id(input.repository_id);
        }
        if input.mapping.is_some() {
            endpoint.set_mapping(input.mapping);
        }

        // Generate webhook URL
        let webhook_url = endpoint.webhook_url(&self.base_url);

        // Save to repository
        self.repository.save(&endpoint).await?;

        tracing::info!(
            endpoint_id = %endpoint.id(),
            provider = %endpoint.provider(),
            "Webhook endpoint registered"
        );

        Ok(RegisterWebhookEndpointOutputData {
            endpoint,
            webhook_url,
            secret: if return_secret {
                secret_value.unwrap_or_default()
            } else {
                String::new()
            },
        })
    }
}

// =============================================================================
// UpdateWebhookEndpoint
// =============================================================================

/// Update field specification for webhook endpoint.
#[derive(Debug, Clone)]
pub enum WebhookEndpointUpdate {
    /// Update status
    Status(EndpointStatus),
    /// Update events filter
    Events(Vec<String>),
    /// Update property mapping
    Mapping(Option<PropertyMapping>),
    /// Update provider configuration
    Config(ProviderConfig),
}

/// Input data for updating a webhook endpoint.
#[derive(Debug)]
pub struct UpdateWebhookEndpointInputData<'a> {
    /// Executor (user or service account)
    pub executor: &'a dyn ExecutorAction,
    /// Multi-tenancy context
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Endpoint ID to update
    pub endpoint_id: WebhookEndpointId,
    /// Update to apply
    pub update: WebhookEndpointUpdate,
}

/// Output data for update webhook endpoint.
#[derive(Debug)]
pub struct UpdateWebhookEndpointOutputData {
    /// Updated endpoint
    pub endpoint: WebhookEndpoint,
}

/// Input port for update webhook endpoint use case.
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait UpdateWebhookEndpointInputPort:
    std::fmt::Debug + Send + Sync
{
    fn policy(&self) -> &'static str {
        "inbound_sync:UpdateWebhookEndpoint"
    }

    async fn execute<'a>(
        &self,
        input: UpdateWebhookEndpointInputData<'a>,
    ) -> errors::Result<UpdateWebhookEndpointOutputData>;
}

/// Update webhook endpoint use case.
#[derive(Debug, Clone, new)]
pub struct UpdateWebhookEndpoint {
    auth: Arc<dyn AuthApp>,
    repository: Arc<dyn WebhookEndpointRepository>,
}

#[async_trait::async_trait]
impl UpdateWebhookEndpointInputPort for UpdateWebhookEndpoint {
    #[tracing::instrument(
        name = "inbound_sync::UpdateWebhookEndpoint::execute",
        skip(self, input),
        fields(endpoint_id = %input.endpoint_id, update = ?input.update)
    )]
    async fn execute<'a>(
        &self,
        input: UpdateWebhookEndpointInputData<'a>,
    ) -> errors::Result<UpdateWebhookEndpointOutputData> {
        // 1. Policy check
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: self.policy(),
            })
            .await?;

        // 2. Find endpoint
        let mut endpoint = self
            .repository
            .find_by_id(&input.endpoint_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook endpoint"))?;

        // 3. Verify tenant ownership
        let tenant_id = input.multi_tenancy.get_operator_id()?;
        if endpoint.tenant_id() != &tenant_id {
            return Err(errors::Error::forbidden(
                "Cannot update endpoint owned by another tenant",
            ));
        }

        // 4. Apply update
        match input.update {
            WebhookEndpointUpdate::Status(status) => {
                endpoint.set_status(status);
                tracing::info!(
                    endpoint_id = %input.endpoint_id,
                    status = %status,
                    "Webhook endpoint status updated"
                );
            }
            WebhookEndpointUpdate::Events(events) => {
                endpoint.set_events(events);
                tracing::info!(
                    endpoint_id = %input.endpoint_id,
                    "Webhook endpoint events updated"
                );
            }
            WebhookEndpointUpdate::Mapping(mapping) => {
                endpoint.set_mapping(mapping);
                tracing::info!(
                    endpoint_id = %input.endpoint_id,
                    "Webhook endpoint mapping updated"
                );
            }
            WebhookEndpointUpdate::Config(config) => {
                if let ProviderConfig::Linear {
                    webhook_secret: Some(secret),
                    ..
                } = &config
                {
                    if !secret.trim().is_empty() {
                        endpoint.set_secret_hash(hash_secret(secret));
                    }
                }
                endpoint.set_config(config);
                tracing::info!(
                    endpoint_id = %input.endpoint_id,
                    "Webhook endpoint config updated"
                );
            }
        }

        // 5. Save
        self.repository.save(&endpoint).await?;

        Ok(UpdateWebhookEndpointOutputData { endpoint })
    }
}

// =============================================================================
// DeleteWebhookEndpoint
// =============================================================================

/// Input data for deleting a webhook endpoint.
#[derive(Debug)]
pub struct DeleteWebhookEndpointInputData<'a> {
    /// Executor (user or service account)
    pub executor: &'a dyn ExecutorAction,
    /// Multi-tenancy context
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    /// Endpoint ID to delete
    pub endpoint_id: WebhookEndpointId,
}

/// Input port for delete webhook endpoint use case.
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait DeleteWebhookEndpointInputPort:
    std::fmt::Debug + Send + Sync
{
    fn policy(&self) -> &'static str {
        "inbound_sync:DeleteWebhookEndpoint"
    }

    async fn execute<'a>(
        &self,
        input: DeleteWebhookEndpointInputData<'a>,
    ) -> errors::Result<()>;
}

/// Delete webhook endpoint use case.
#[derive(Debug, Clone, new)]
pub struct DeleteWebhookEndpoint {
    auth: Arc<dyn AuthApp>,
    repository: Arc<dyn WebhookEndpointRepository>,
}

#[async_trait::async_trait]
impl DeleteWebhookEndpointInputPort for DeleteWebhookEndpoint {
    #[tracing::instrument(
        name = "inbound_sync::DeleteWebhookEndpoint::execute",
        skip(self, input),
        fields(endpoint_id = %input.endpoint_id)
    )]
    async fn execute<'a>(
        &self,
        input: DeleteWebhookEndpointInputData<'a>,
    ) -> errors::Result<()> {
        // 1. Policy check
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: self.policy(),
            })
            .await?;

        // 2. Find endpoint
        let endpoint = self
            .repository
            .find_by_id(&input.endpoint_id)
            .await?
            .ok_or_else(|| errors::Error::not_found("Webhook endpoint"))?;

        // 3. Verify tenant ownership
        let tenant_id = input.multi_tenancy.get_operator_id()?;
        if endpoint.tenant_id() != &tenant_id {
            return Err(errors::Error::forbidden(
                "Cannot delete endpoint owned by another tenant",
            ));
        }

        // 4. Delete
        self.repository.delete(&input.endpoint_id).await?;

        tracing::info!(
            endpoint_id = %input.endpoint_id,
            "Webhook endpoint deleted"
        );

        Ok(())
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Generate a cryptographically secure random secret.
fn generate_secret() -> String {
    use rand::Rng;
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const SECRET_LEN: usize = 32;

    let mut rng = rand::thread_rng();
    let secret: String = (0..SECRET_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    format!("whsec_{secret}")
}

/// Hash a secret for storage.
fn hash_secret(secret: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let secret = generate_secret();
        assert!(secret.starts_with("whsec_"));
        assert_eq!(secret.len(), 6 + 32); // "whsec_" + 32 chars
    }

    #[test]
    fn test_hash_secret() {
        let secret = "whsec_test123";
        let hash1 = hash_secret(secret);
        let hash2 = hash_secret(secret);
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, secret);
        assert_eq!(hash1.len(), 64); // SHA256 hex = 64 chars
    }
}
