//! Provision a GitHub hook for an authorized Library sync binding.

use std::sync::Arc;

use inbound_sync_domain::{
    EndpointStatus, Provider, ProviderConfig, WebhookEndpoint,
    WebhookEndpointId, WebhookEndpointRepository,
};
use integration_domain::{
    ExternalSyncBindingId, ExternalSyncBindingRepository,
    ExternalSyncBindingStatus, ExternalSyncPolicy, OAuthProvider,
};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};

use super::{
    GitHubManagedWebhook, GitHubWebhookControl, GitHubWebhookDelivery,
    RegisterWebhookEndpoint, RegisterWebhookEndpointInputData,
    RegisterWebhookEndpointInputPort,
};

pub struct ProvisionGitHubBindingWebhook {
    auth: Arc<dyn AuthApp>,
    bindings: Arc<dyn ExternalSyncBindingRepository>,
    endpoints: Arc<dyn WebhookEndpointRepository>,
    register: Arc<RegisterWebhookEndpoint>,
    broker: Arc<dyn GitHubWebhookControl>,
    base_url: String,
}

pub struct ProvisionGitHubBindingWebhookInput<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub binding_id: ExternalSyncBindingId,
}

pub struct ProvisionedGitHubWebhook {
    pub endpoint_id: WebhookEndpointId,
    pub webhook_url: String,
    pub hook: GitHubManagedWebhook,
}

pub struct GitHubBindingWebhookStatus {
    pub endpoint_id: WebhookEndpointId,
    pub webhook_url: String,
    pub hook: Option<GitHubManagedWebhook>,
    pub deliveries: Vec<GitHubWebhookDelivery>,
}

impl ProvisionGitHubBindingWebhook {
    pub fn new(
        auth: Arc<dyn AuthApp>,
        bindings: Arc<dyn ExternalSyncBindingRepository>,
        endpoints: Arc<dyn WebhookEndpointRepository>,
        register: Arc<RegisterWebhookEndpoint>,
        broker: Arc<dyn GitHubWebhookControl>,
        base_url: String,
    ) -> Self {
        Self {
            auth,
            bindings,
            endpoints,
            register,
            broker,
            base_url,
        }
    }

    pub async fn execute(
        &self,
        input: ProvisionGitHubBindingWebhookInput<'_>,
    ) -> errors::Result<ProvisionedGitHubWebhook> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:InboundSyncRegisterWebhookEndpoint",
            })
            .await?;
        Provider::Github.ensure_runtime_available()?;
        let tenant = input.multi_tenancy.get_operator_id()?;
        let binding = self
            .bindings
            .find_by_id(&tenant, &input.binding_id)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("GitHub sync binding")
            })?;
        if binding.provider() != OAuthProvider::Github
            || binding.status() != ExternalSyncBindingStatus::Active
            || binding.inbound_policy() == ExternalSyncPolicy::Disabled
        {
            return Err(errors::Error::bad_request(
                "GitHub inbound sync is inactive",
            ));
        }
        let scope = binding.external_scope().value();
        let repository = scope["repository"].as_str().ok_or_else(|| {
            errors::Error::bad_request("GitHub repository is missing")
        })?;
        let branch = scope["ref"].as_str().ok_or_else(|| {
            errors::Error::bad_request("GitHub branch is missing")
        })?;
        let pattern = scope["path_pattern"].as_str();
        let candidates = self
            .endpoints
            .find_by_tenant_and_provider(&tenant, Provider::Github)
            .await?
            .into_iter()
            .filter(|endpoint| {
                endpoint.repository_id() == Some(binding.library_repo_id().as_str())
                    && matches!(endpoint.config(), ProviderConfig::Github {
                        repository: configured_repo,
                        branch: configured_branch,
                        path_pattern: configured_pattern,
                    } if configured_repo == repository && configured_branch == branch
                        && configured_pattern.as_deref() == pattern)
            })
            .collect::<Vec<_>>();
        let endpoint = match candidates.as_slice() {
            [endpoint] => endpoint.clone(),
            [] => {
                let stable_id = WebhookEndpointId::from(format!(
                    "whe_{}",
                    input.binding_id.as_str().trim_start_matches("esb_")
                ));
                match self
                    .register
                    .execute(RegisterWebhookEndpointInputData {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        name: format!("GitHub sync {}", binding.id()),
                        provider: Provider::Github,
                        config: ProviderConfig::Github {
                            repository: repository.to_string(),
                            branch: branch.to_string(),
                            path_pattern: pattern.map(str::to_string),
                        },
                        events: vec!["push".into()],
                        repository_id: Some(
                            binding.library_repo_id().as_str().into(),
                        ),
                        mapping: None,
                        endpoint_id: Some(stable_id.clone()),
                    })
                    .await
                {
                    Ok(output) => output.endpoint,
                    Err(error) => {
                        // A concurrent or timed-out registration may have
                        // committed the same deterministic endpoint ID.
                        let Some(saved) =
                            self.endpoints.find_by_id(&stable_id).await?
                        else {
                            return Err(error);
                        };
                        if !matches_binding(
                            &saved,
                            &tenant,
                            binding.library_repo_id().as_str(),
                            repository,
                            branch,
                            pattern,
                        ) {
                            return Err(error);
                        }
                        saved
                    }
                }
            }
            _ => {
                return Err(errors::Error::conflict(
                    "Multiple GitHub endpoints match this binding",
                ));
            }
        };
        if endpoint.status() != &EndpointStatus::Active
            || !endpoint.events().iter().any(|event| event == "push")
        {
            return Err(errors::Error::conflict(
                "GitHub endpoint must be active and receive push",
            ));
        }
        let webhook_url = endpoint.webhook_url(&self.base_url);
        let hook = self
            .broker
            .ensure(
                &tenant,
                repository,
                &webhook_url,
                endpoint.secret_hash(),
            )
            .await?;
        Ok(ProvisionedGitHubWebhook {
            endpoint_id: endpoint.id().clone(),
            webhook_url,
            hook,
        })
    }

    pub async fn status(
        &self,
        input: ProvisionGitHubBindingWebhookInput<'_>,
    ) -> errors::Result<Option<GitHubBindingWebhookStatus>> {
        let (tenant, repository, endpoint) =
            self.resolve_existing(input, false).await?;
        let Some(endpoint) = endpoint else {
            return Ok(None);
        };
        let webhook_url = endpoint.webhook_url(&self.base_url);
        let hook =
            self.broker.find(&tenant, &repository, &webhook_url).await?;
        let deliveries = if let Some(hook) = &hook {
            self.broker
                .deliveries(&tenant, &repository, &webhook_url, hook.id)
                .await?
        } else {
            Vec::new()
        };
        Ok(Some(GitHubBindingWebhookStatus {
            endpoint_id: endpoint.id().clone(),
            webhook_url,
            hook,
            deliveries,
        }))
    }

    pub async fn remove(
        &self,
        input: ProvisionGitHubBindingWebhookInput<'_>,
    ) -> errors::Result<bool> {
        let (tenant, repository, endpoint) =
            self.resolve_existing(input, true).await?;
        let Some(endpoint) = endpoint else {
            return Ok(false);
        };
        let webhook_url = endpoint.webhook_url(&self.base_url);
        let Some(hook) =
            self.broker.find(&tenant, &repository, &webhook_url).await?
        else {
            return Ok(false);
        };
        self.broker
            .remove(&tenant, &repository, &webhook_url, hook.id)
            .await?;
        Ok(true)
    }

    pub async fn redeliver(
        &self,
        input: ProvisionGitHubBindingWebhookInput<'_>,
        delivery_id: u64,
    ) -> errors::Result<()> {
        let (tenant, repository, endpoint) =
            self.resolve_existing(input, true).await?;
        let endpoint = endpoint.ok_or_else(|| {
            errors::Error::not_found("GitHub webhook endpoint")
        })?;
        let webhook_url = endpoint.webhook_url(&self.base_url);
        let hook = self
            .broker
            .find(&tenant, &repository, &webhook_url)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("GitHub repository webhook")
            })?;
        let deliveries = self
            .broker
            .deliveries(&tenant, &repository, &webhook_url, hook.id)
            .await?;
        if !deliveries.iter().any(|delivery| delivery.id == delivery_id) {
            return Err(errors::Error::not_found(
                "GitHub webhook delivery",
            ));
        }
        self.broker
            .redeliver(
                &tenant,
                &repository,
                &webhook_url,
                hook.id,
                delivery_id,
            )
            .await
    }

    async fn resolve_existing(
        &self,
        input: ProvisionGitHubBindingWebhookInput<'_>,
        write: bool,
    ) -> errors::Result<(
        value_object::TenantId,
        String,
        Option<WebhookEndpoint>,
    )> {
        self.auth
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: if write {
                    "library:InboundSyncRegisterWebhookEndpoint"
                } else {
                    "library:InboundSyncListConnections"
                },
            })
            .await?;
        Provider::Github.ensure_runtime_available()?;
        let tenant = input.multi_tenancy.get_operator_id()?;
        let binding = self
            .bindings
            .find_by_id(&tenant, &input.binding_id)
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("GitHub sync binding")
            })?;
        if binding.provider() != OAuthProvider::Github {
            return Err(errors::Error::bad_request(
                "Binding is not GitHub",
            ));
        }
        let scope = binding.external_scope().value();
        let repository = scope["repository"].as_str().ok_or_else(|| {
            errors::Error::bad_request("GitHub repository is missing")
        })?;
        let branch = scope["ref"].as_str().ok_or_else(|| {
            errors::Error::bad_request("GitHub branch is missing")
        })?;
        let pattern = scope["path_pattern"].as_str();
        let candidates = self
            .endpoints
            .find_by_tenant_and_provider(&tenant, Provider::Github)
            .await?
            .into_iter()
            .filter(|endpoint| {
                matches_binding(
                    endpoint,
                    &tenant,
                    binding.library_repo_id().as_str(),
                    repository,
                    branch,
                    pattern,
                )
            })
            .collect::<Vec<_>>();
        let endpoint = match candidates.len() {
            0 => None,
            1 => candidates.into_iter().next(),
            _ => {
                return Err(errors::Error::conflict(
                    "Multiple GitHub endpoints match this binding",
                ))
            }
        };
        Ok((tenant, repository.to_string(), endpoint))
    }
}

fn matches_binding(
    endpoint: &WebhookEndpoint,
    tenant: &value_object::TenantId,
    library_repo_id: &str,
    repository: &str,
    branch: &str,
    pattern: Option<&str>,
) -> bool {
    endpoint.tenant_id() == tenant
        && endpoint.repository_id() == Some(library_repo_id)
        && matches!(endpoint.config(), ProviderConfig::Github {
            repository: configured_repo,
            branch: configured_branch,
            path_pattern: configured_pattern,
        } if configured_repo == repository && configured_branch == branch
            && configured_pattern.as_deref() == pattern)
}
