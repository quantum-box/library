//! Receive webhooks via provider-only endpoint.

use std::sync::Arc;

use inbound_sync_domain::{
    ConnectionRepository, EndpointStatus, OAuthProvider, Provider,
    ProviderConfig, WebhookEndpoint, WebhookEndpointRepository,
    WebhookEvent, WebhookEventId, WebhookEventRepository,
};

use crate::providers::linear::LinearWebhookEvent;
use crate::{WebhookSecretStore, WebhookVerifierRegistry};

use super::receive_webhook::{extract_event_type, resolve_webhook_secret};

/// Input for receiving a provider-only webhook.
pub struct ReceiveProviderWebhookInput {
    /// Provider from URL
    pub provider: Provider,
    /// Raw payload bytes
    pub payload: Vec<u8>,
    /// Signature header value
    pub signature: String,
    /// Event type (from header if available)
    pub event_type: Option<String>,
    /// All headers as JSON
    pub headers: Option<serde_json::Value>,
    /// Full webhook notification URL (used by Square for signature
    /// verification)
    pub webhook_url: Option<String>,
}

/// Output for provider webhook reception.
pub struct ReceiveProviderWebhookOutput {
    /// Created event IDs
    pub event_ids: Vec<WebhookEventId>,
}

/// Receive webhook for provider-only endpoints.
///
/// This use case resolves the tenant from the provider payload and
/// routes the event to matching endpoints for that tenant.
pub struct ReceiveProviderWebhook {
    endpoint_repository: Arc<dyn WebhookEndpointRepository>,
    event_repository: Arc<dyn WebhookEventRepository>,
    verifier_registry: Arc<WebhookVerifierRegistry>,
    connection_repository: Arc<dyn ConnectionRepository>,
    provider_secrets: Arc<WebhookSecretStore>,
}

impl ReceiveProviderWebhook {
    pub fn new(
        endpoint_repository: Arc<dyn WebhookEndpointRepository>,
        event_repository: Arc<dyn WebhookEventRepository>,
        verifier_registry: Arc<WebhookVerifierRegistry>,
        connection_repository: Arc<dyn ConnectionRepository>,
        provider_secrets: Arc<WebhookSecretStore>,
    ) -> Self {
        Self {
            endpoint_repository,
            event_repository,
            verifier_registry,
            connection_repository,
            provider_secrets,
        }
    }

    pub async fn execute(
        &self,
        input: ReceiveProviderWebhookInput,
    ) -> errors::Result<ReceiveProviderWebhookOutput> {
        let payload: serde_json::Value =
            serde_json::from_slice(&input.payload).map_err(|e| {
                errors::Error::invalid(format!("Invalid JSON payload: {e}"))
            })?;

        let external_account_id =
            extract_external_account_id(input.provider, &payload)
                .ok_or_else(|| {
                    errors::Error::bad_request(
                        "Missing external account id in payload",
                    )
                })?;

        let oauth_provider: OAuthProvider = input.provider.into();
        let connection = self
            .connection_repository
            .find_by_provider_and_external_account_id(
                oauth_provider,
                &external_account_id,
            )
            .await?
            .ok_or_else(|| {
                errors::Error::not_found("Integration connection")
            })?;

        let endpoints = self
            .endpoint_repository
            .find_by_tenant_and_provider(
                connection.tenant_id(),
                input.provider,
            )
            .await?;

        let endpoints =
            filter_endpoints(input.provider, &payload, endpoints)?;

        if endpoints.is_empty() {
            return Err(errors::Error::not_found("Webhook endpoint"));
        }

        let event_type = input
            .event_type
            .or_else(|| extract_event_type(&payload, input.provider))
            .unwrap_or_else(|| "unknown".to_string());

        let mut event_ids = Vec::new();

        for endpoint in endpoints {
            if *endpoint.status() == EndpointStatus::Disabled {
                continue;
            }

            let secret =
                resolve_webhook_secret(&endpoint, &self.provider_secrets);
            let signature_valid = self.verifier_registry.verify(
                input.provider,
                &input.payload,
                &input.signature,
                secret,
                input.webhook_url.as_deref(),
            )?;

            let mut event = WebhookEvent::create(
                endpoint.id().clone(),
                input.provider,
                event_type.clone(),
                payload.clone(),
                input.headers.clone(),
                signature_valid,
            );

            if !endpoint.should_process_event(&event_type) {
                event.mark_skipped(
                    "Event type not in configured events list",
                );
            }

            let event_id = event.id().clone();
            self.event_repository.save(&event).await?;
            event_ids.push(event_id);
        }

        Ok(ReceiveProviderWebhookOutput { event_ids })
    }
}

fn extract_external_account_id(
    provider: Provider,
    payload: &serde_json::Value,
) -> Option<String> {
    match provider {
        Provider::Linear => payload
            .get("organizationId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn filter_endpoints(
    provider: Provider,
    payload: &serde_json::Value,
    endpoints: Vec<WebhookEndpoint>,
) -> errors::Result<Vec<WebhookEndpoint>> {
    match provider {
        Provider::Linear => filter_linear_endpoints(payload, endpoints),
        _ => Ok(endpoints),
    }
}

fn filter_linear_endpoints(
    payload: &serde_json::Value,
    endpoints: Vec<WebhookEndpoint>,
) -> errors::Result<Vec<WebhookEndpoint>> {
    let event: LinearWebhookEvent = serde_json::from_value(payload.clone())
        .map_err(|e| {
            errors::Error::invalid(format!(
                "Failed to parse Linear webhook event: {e}"
            ))
        })?;

    match event.event_type.as_str() {
        "Issue" => {
            if let Some(issue) = event.as_issue() {
                let issue_team_id =
                    issue.team.as_ref().map(|team| team.id.as_str());
                let issue_project_id = issue
                    .project
                    .as_ref()
                    .map(|project| project.id.as_str());

                Ok(endpoints
                    .into_iter()
                    .filter(|endpoint| match endpoint.config() {
                        ProviderConfig::Linear {
                            team_id,
                            project_id,
                            ..
                        } => {
                            if let Some(config_team_id) = team_id {
                                if issue_team_id != Some(config_team_id) {
                                    return false;
                                }
                            }
                            if let Some(config_project_id) = project_id {
                                if issue_project_id
                                    != Some(config_project_id)
                                {
                                    return false;
                                }
                            }
                            true
                        }
                        _ => false,
                    })
                    .collect())
            } else {
                Ok(endpoints)
            }
        }
        "Project" => {
            if let Some(project) = event.as_project() {
                Ok(endpoints
                    .into_iter()
                    .filter(|endpoint| match endpoint.config() {
                        ProviderConfig::Linear { project_id, .. } => {
                            if let Some(config_project_id) = project_id {
                                return config_project_id == &project.id;
                            }
                            true
                        }
                        _ => false,
                    })
                    .collect())
            } else {
                Ok(endpoints)
            }
        }
        _ => Ok(endpoints),
    }
}
