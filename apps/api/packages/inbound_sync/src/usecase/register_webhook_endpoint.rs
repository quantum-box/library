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
    /// Stable identity for retryable managed registrations.
    pub endpoint_id: Option<WebhookEndpointId>,
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
        "library:InboundSyncRegisterWebhookEndpoint"
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

        input.provider.ensure_runtime_available()?;

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
        let returned_secret = registration_secret(
            input.provider,
            &secret_hash,
            secret_value.unwrap_or_default(),
            return_secret,
        );

        // 5. Create the endpoint
        let mut endpoint = WebhookEndpoint::create_with_id(
            input.endpoint_id.unwrap_or_default(),
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
            secret: returned_secret,
        })
    }
}

// =============================================================================
// UpdateWebhookEndpoint
// =============================================================================

/// Update field specification for webhook endpoint.
#[derive(Debug, Clone)]
pub enum WebhookEndpointUpdate {
    /// Generate a new endpoint-specific GitHub signing key.
    RotateGitHubSecret,
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
        "library:InboundSyncUpdateWebhookEndpoint"
    }

    async fn execute<'a>(
        &self,
        input: UpdateWebhookEndpointInputData<'a>,
    ) -> errors::Result<UpdateWebhookEndpointOutputData>;
}

/// Extra permission for rotating a GitHub endpoint key.
///
/// The rotated key is the HMAC key GitHub signs with, and the mutation hands
/// it back so it can be pasted into a hook set up by hand. Whoever holds it
/// can sign push events that the endpoint then fetches with the tenant's
/// GitHub token and writes into the bound repository, so the action every
/// member has (`InboundSyncUpdateWebhookEndpoint`) is not enough. This is the
/// owner check the API already uses for handing out repository access
/// (API keys with a role, repository policy changes).
///
/// The key is still returned rather than pushed to GitHub server-side: the
/// endpoint is not tied to a binding, and the hook broker that could update
/// the hook is not enabled in every deployment, so a hook configured by hand
/// has no other way to receive the new key.
pub const ROTATE_GITHUB_SECRET_POLICY: &str = "library:ManageRepoPolicy";

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
        if matches!(input.update, WebhookEndpointUpdate::RotateGitHubSecret)
        {
            self.auth
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: input.multi_tenancy,
                    action: ROTATE_GITHUB_SECRET_POLICY,
                })
                .await?;
        }

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
            WebhookEndpointUpdate::RotateGitHubSecret => {
                if *endpoint.provider() != Provider::Github {
                    return Err(errors::Error::bad_request(
                        "Signing key rotation is supported only for GitHub",
                    ));
                }
                Provider::Github.ensure_runtime_available()?;
                let key = hash_secret(&generate_secret());
                self.repository
                    .rotate_secret(
                        &tenant_id,
                        endpoint.id(),
                        endpoint.secret_hash(),
                        &key,
                    )
                    .await?;
                endpoint.set_secret_hash(key);
                return Ok(UpdateWebhookEndpointOutputData { endpoint });
            }
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
            WebhookEndpointUpdate::Config(mut config) => {
                keep_stored_linear_secret(
                    &mut config,
                    endpoint.webhook_secret(),
                );
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
        "library:InboundSyncDeleteWebhookEndpoint"
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

fn registration_secret(
    provider: Provider,
    verification_key: &str,
    original_secret: String,
    return_secret: bool,
) -> String {
    if !return_secret {
        return String::new();
    }
    if provider == Provider::Github {
        // Endpoint-specific GitHub verification uses the legacy secret_hash
        // column as its HMAC key. Return that same random-derived key, not
        // its discarded preimage: GitHub cannot sign with a different key.
        // This column is secret material, never a public credential digest.
        verification_key.to_owned()
    } else {
        original_secret
    }
}

/// Generate a cryptographically secure random secret.
/// The API never returns the stored Linear signing secret, so clients send
/// it back empty when they save other settings. Keep the stored secret in
/// that case instead of clearing it; a non-empty value still replaces it.
fn keep_stored_linear_secret(
    config: &mut ProviderConfig,
    stored: Option<&str>,
) {
    if let (
        ProviderConfig::Linear {
            webhook_secret: incoming,
            ..
        },
        Some(stored),
    ) = (config, stored)
    {
        if incoming.as_deref().is_none_or(|s| s.trim().is_empty()) {
            *incoming = Some(stored.to_string());
        }
    }
}

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
    fn github_registration_key_verifies_a_provider_signed_payload() {
        use crate::webhook_verifier::{GitHubVerifier, WebhookVerifier};
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let seed = generate_secret();
        let stored_key = hash_secret(&seed);
        let returned_key =
            registration_secret(Provider::Github, &stored_key, seed, true);
        let payload = br#"{"ref":"refs/heads/test"}"#;
        let mut signer =
            Hmac::<Sha256>::new_from_slice(returned_key.as_bytes())
                .unwrap();
        signer.update(payload);
        let signature = format!(
            "sha256={}",
            hex::encode(signer.finalize().into_bytes())
        );
        assert!(GitHubVerifier
            .verify(payload, &signature, &stored_key, None)
            .unwrap());
        assert!(!GitHubVerifier
            .verify(b"tampered", &signature, &stored_key, None)
            .unwrap());
    }

    fn linear(secret: Option<&str>) -> ProviderConfig {
        ProviderConfig::Linear {
            team_id: None,
            project_id: None,
            webhook_secret: secret.map(String::from),
        }
    }

    fn linear_secret(config: &ProviderConfig) -> Option<&str> {
        match config {
            ProviderConfig::Linear { webhook_secret, .. } => {
                webhook_secret.as_deref()
            }
            _ => None,
        }
    }

    #[test]
    fn empty_linear_secret_keeps_the_stored_one() {
        for incoming in [None, Some(""), Some("  ")] {
            let mut config = linear(incoming);
            keep_stored_linear_secret(&mut config, Some("stored"));
            assert_eq!(linear_secret(&config), Some("stored"));
        }
    }

    #[test]
    fn new_linear_secret_replaces_the_stored_one() {
        let mut config = linear(Some("rotated"));
        keep_stored_linear_secret(&mut config, Some("stored"));
        assert_eq!(linear_secret(&config), Some("rotated"));
    }

    mockall::mock! {
        #[derive(Debug)]
        Endpoints {}
        #[async_trait::async_trait]
        impl WebhookEndpointRepository for Endpoints {
            async fn save(
                &self,
                endpoint: &WebhookEndpoint,
            ) -> errors::Result<()>;
            async fn find_by_id(
                &self,
                id: &WebhookEndpointId,
            ) -> errors::Result<Option<WebhookEndpoint>>;
            async fn find_by_tenant(
                &self,
                tenant_id: &value_object::TenantId,
            ) -> errors::Result<Vec<WebhookEndpoint>>;
            async fn find_by_tenant_and_provider(
                &self,
                tenant_id: &value_object::TenantId,
                provider: Provider,
            ) -> errors::Result<Vec<WebhookEndpoint>>;
            async fn delete(
                &self,
                id: &WebhookEndpointId,
            ) -> errors::Result<()>;
        }
    }

    /// Auth that grants what every member has and refuses the owner check.
    fn member_auth() -> Arc<tachyon_sdk::auth::MockAuthApp> {
        let mut auth = tachyon_sdk::auth::MockAuthApp::new();
        auth.expect_check_policy().returning(|input| {
            let refused = input.action == ROTATE_GITHUB_SECRET_POLICY;
            Box::pin(async move {
                if refused {
                    Err(errors::Error::forbidden("not an owner"))
                } else {
                    Ok(())
                }
            })
        });
        Arc::new(auth)
    }

    #[tokio::test]
    async fn member_without_owner_permission_cannot_rotate_github_key() {
        use tachyon_sdk::auth::test_helper::{
            create_test_executor, create_test_multi_tenancy,
        };
        // No repository expectations: touching the endpoint would panic,
        // so the refusal has to happen before the key is read or replaced.
        let usecase = UpdateWebhookEndpoint::new(
            member_auth(),
            Arc::new(MockEndpoints::new()),
        );
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let err = usecase
            .execute(UpdateWebhookEndpointInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                endpoint_id: WebhookEndpointId::from(
                    "whe_test".to_string(),
                ),
                update: WebhookEndpointUpdate::RotateGitHubSecret,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, errors::Error::Forbidden { .. }), "{err:?}");
    }

    #[tokio::test]
    async fn member_can_still_change_other_endpoint_settings() {
        use tachyon_sdk::auth::test_helper::{
            create_test_executor, create_test_multi_tenancy,
        };
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();
        let tenant = multi_tenancy.get_operator_id().unwrap();
        let endpoint = WebhookEndpoint::create(
            tenant,
            "github",
            Provider::Github,
            ProviderConfig::Github {
                repository: "o/r".into(),
                branch: "main".into(),
                path_pattern: None,
            },
            vec!["push".into()],
            "key",
        );
        let mut endpoints = MockEndpoints::new();
        endpoints
            .expect_find_by_id()
            .returning(move |_| Ok(Some(endpoint.clone())));
        endpoints.expect_save().returning(|_| Ok(()));
        let usecase =
            UpdateWebhookEndpoint::new(member_auth(), Arc::new(endpoints));
        usecase
            .execute(UpdateWebhookEndpointInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                endpoint_id: WebhookEndpointId::from(
                    "whe_test".to_string(),
                ),
                update: WebhookEndpointUpdate::Status(
                    EndpointStatus::Paused,
                ),
            })
            .await
            .unwrap();
    }

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
