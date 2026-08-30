//! Translator backed by tachyon-api's chat completions endpoint.
//!
//! Routing through Tachyon rather than a provider SDK keeps model
//! selection, billing and credentials in one place: this adapter holds
//! no provider key and names no provider.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;

use crate::domain::translation::{
    build_translation_prompt, parse_translation_response, system_prompt,
    TranslationBatch, Translator,
};
use crate::sdk_auth::SdkAuthApp;

/// Model used for translation, as `provider/model`.
///
/// Unset means the feature is off: rows stay pending and readers are
/// served the source text. Configuring rather than hardcoding it also
/// keeps model changes an operational act -- `translations.model` is
/// part of the ETag, so swapping the model invalidates exactly the
/// cached translations it should.
pub const TRANSLATION_MODEL_ENV: &str = "LIBRARY_TRANSLATION_MODEL";

/// Enough for a batch of schema labels and their JSON envelope.
const MAX_COMPLETION_TOKENS: i32 = 4096;

#[derive(Clone)]
pub struct TachyonTranslator {
    sdk: Arc<SdkAuthApp>,
    model: String,
}

impl std::fmt::Debug for TachyonTranslator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TachyonTranslator")
            .field("model", &self.model)
            .finish_non_exhaustive()
    }
}

impl TachyonTranslator {
    /// Builds a translator when a model is configured.
    ///
    /// Returns `None` rather than failing so that a deployment without
    /// the setting serves source text instead of refusing to start.
    pub fn from_env(sdk: Arc<SdkAuthApp>) -> Option<Self> {
        let model = std::env::var(TRANSLATION_MODEL_ENV)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())?;

        tracing::info!(model = %model, "translation model configured");
        Some(Self { sdk, model })
    }

    /// The model this translator records on the rows it writes.
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// The response is read leniently on purpose.
///
/// The generated SDK types this endpoint's body as a streaming chunk
/// even for a non-streaming request, so the completion text may arrive
/// as `message.content` or as `delta.content` depending on what the
/// server actually sends. Accepting either costs one extra field and
/// avoids a whole class of deserialization failure that would be
/// invisible until the first live call.
#[derive(Debug, Deserialize)]
struct CompletionResponse {
    #[serde(default)]
    choices: Vec<CompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct CompletionChoice {
    #[serde(default)]
    message: Option<CompletionContent>,
    #[serde(default)]
    delta: Option<CompletionContent>,
}

#[derive(Debug, Deserialize)]
struct CompletionContent {
    #[serde(default)]
    content: Option<String>,
}

impl CompletionResponse {
    fn text(&self) -> Option<&str> {
        self.choices.iter().find_map(|choice| {
            choice
                .message
                .as_ref()
                .and_then(|part| part.content.as_deref())
                .or_else(|| {
                    choice
                        .delta
                        .as_ref()
                        .and_then(|part| part.content.as_deref())
                })
        })
    }
}

#[async_trait]
impl Translator for TachyonTranslator {
    #[tracing::instrument(name = "TachyonTranslator::translate", skip_all)]
    async fn translate(
        &self,
        batch: TranslationBatch<'_>,
    ) -> errors::Result<HashMap<String, String>> {
        if batch.items.is_empty() {
            return Ok(HashMap::new());
        }

        let endpoint = self.sdk.service_endpoint();
        let url = format!("{}/v1/llms/chat/completions", endpoint.base_url);

        // No reasoning-effort control is sent. Cheap models in this
        // class are reasoning models, and translating a five-word column
        // heading needs none of it -- the reasoning is paid for in
        // latency and output tokens either way. Tachyon's chat
        // completions request carries no effort field to turn it down,
        // so this waits on that field existing.
        //
        // `temperature` is deliberately absent. Translation would like
        // to be deterministic, but the current generation of cheap
        // models are reasoning models, and several reject any
        // temperature other than their default outright. A rejected
        // request fails the whole run and cannot be fixed without a
        // deploy; non-determinism costs nothing here, because the result
        // is cached against the source hash either way.
        let body = serde_json::json!({
            "model": self.model,
            "max_completion_tokens": MAX_COMPLETION_TOKENS,
            "stream": false,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": system_prompt() },
                {
                    "role": "user",
                    "content": build_translation_prompt(&batch)
                }
            ]
        });

        let response = reqwest::Client::new()
            .post(&url)
            .header("x-operator-id", &endpoint.operator_id)
            .header(
                "Authorization",
                format!("Bearer {}", endpoint.auth_token),
            )
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                errors::Error::internal_server_error(format!(
                    "translation request failed: {error}"
                ))
            })?;

        let status = response.status();
        let raw = response.text().await.map_err(|error| {
            errors::Error::internal_server_error(format!(
                "could not read translation response: {error}"
            ))
        })?;

        if !status.is_success() {
            // Surfaced verbatim because the first failure most teams
            // hit is an unknown model id or a missing service-account
            // grant, and both are only diagnosable from the body.
            return Err(errors::Error::internal_server_error(format!(
                "translation model `{}` returned {}: {}",
                self.model,
                status,
                raw.chars().take(500).collect::<String>()
            )));
        }

        let completion: CompletionResponse = serde_json::from_str(&raw)
            .map_err(|error| {
                errors::Error::internal_server_error(format!(
                    "unexpected translation response shape: {error}"
                ))
            })?;

        let text = completion.text().ok_or_else(|| {
            errors::Error::internal_server_error(
                "translation response carried no completion text",
            )
        })?;

        parse_translation_response(text, batch.items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_completion_text_from_the_message_field() {
        let raw = r#"{"choices":[{"message":{"content":"hello"}}]}"#;
        let parsed: CompletionResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.text(), Some("hello"));
    }

    #[test]
    fn reads_completion_text_from_the_delta_field() {
        // The generated SDK types this endpoint as a stream chunk, so
        // the same call can come back in this shape instead.
        let raw = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        let parsed: CompletionResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.text(), Some("hello"));
    }

    #[test]
    fn tolerates_extra_fields_the_server_may_add() {
        let raw = r#"{"id":"x","model":"m","usage":{"total_tokens":1},
            "choices":[{"index":0,"finish_reason":"stop",
            "message":{"role":"assistant","content":"hello"}}]}"#;
        let parsed: CompletionResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.text(), Some("hello"));
    }

    #[test]
    fn reports_no_text_when_the_response_is_empty() {
        let raw = r#"{"choices":[]}"#;
        let parsed: CompletionResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.text(), None);
    }
}
