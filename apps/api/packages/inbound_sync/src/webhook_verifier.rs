//! Webhook signature verification for various providers.
//!
//! Each provider has a different method of signing webhook payloads.
//! This module provides implementations for each supported provider.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use inbound_sync_domain::Provider;

type HmacSha256 = Hmac<Sha256>;

/// Trait for webhook signature verification.
pub trait WebhookVerifier: Send + Sync + Debug {
    /// Get the provider this verifier handles.
    fn provider(&self) -> Provider;

    /// Verify the webhook signature.
    ///
    /// # Arguments
    ///
    /// * `payload` - Raw webhook payload bytes
    /// * `signature` - Signature header value from the webhook request
    /// * `secret` - The shared secret for this endpoint
    /// * `webhook_url` - The notification URL registered with the
    ///   provider. Some providers (e.g. Square) include this in the
    ///   HMAC input.
    ///
    /// # Returns
    ///
    /// `true` if the signature is valid, `false` otherwise.
    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        webhook_url: Option<&str>,
    ) -> errors::Result<bool>;
}

/// Registry of webhook verifiers for all providers.
#[derive(Debug)]
pub struct WebhookVerifierRegistry {
    verifiers: HashMap<Provider, Arc<dyn WebhookVerifier>>,
}

impl WebhookVerifierRegistry {
    /// Create a new registry with all default verifiers.
    pub fn new() -> Self {
        let mut verifiers: HashMap<Provider, Arc<dyn WebhookVerifier>> =
            HashMap::new();

        verifiers.insert(Provider::Github, Arc::new(GitHubVerifier));
        verifiers.insert(Provider::Linear, Arc::new(LinearVerifier));
        verifiers.insert(Provider::Hubspot, Arc::new(HubSpotVerifier));
        verifiers.insert(Provider::Stripe, Arc::new(StripeVerifier));
        verifiers.insert(Provider::Square, Arc::new(SquareVerifier));
        verifiers.insert(Provider::Notion, Arc::new(NotionVerifier));
        verifiers.insert(Provider::Airtable, Arc::new(GenericHmacVerifier));
        verifiers.insert(Provider::Generic, Arc::new(GenericHmacVerifier));

        Self { verifiers }
    }

    /// Get the verifier for a provider.
    pub fn get(
        &self,
        provider: Provider,
    ) -> Option<Arc<dyn WebhookVerifier>> {
        self.verifiers.get(&provider).cloned()
    }

    /// Verify a webhook using the appropriate verifier.
    pub fn verify(
        &self,
        provider: Provider,
        payload: &[u8],
        signature: &str,
        secret: &str,
        webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        let verifier = self.get(provider).ok_or_else(|| {
            errors::Error::invalid(format!(
                "No verifier registered for provider: {provider}"
            ))
        })?;

        verifier.verify(payload, signature, secret, webhook_url)
    }
}

impl Default for WebhookVerifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// GitHub webhook signature verifier.
///
/// GitHub uses HMAC-SHA256 with the signature in format: `sha256=<hex>`
#[derive(Debug)]
pub struct GitHubVerifier;

impl WebhookVerifier for GitHubVerifier {
    fn provider(&self) -> Provider {
        Provider::Github
    }

    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        _webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        // GitHub signature format: sha256=<hex>
        let expected_sig =
            signature.strip_prefix("sha256=").unwrap_or(signature);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;
        mac.update(payload);

        let computed = hex::encode(mac.finalize().into_bytes());

        Ok(constant_time_compare(&computed, expected_sig))
    }
}

/// Linear webhook signature verifier.
///
/// Linear uses HMAC-SHA256 with the signature as raw hex.
#[derive(Debug)]
pub struct LinearVerifier;

impl WebhookVerifier for LinearVerifier {
    fn provider(&self) -> Provider {
        Provider::Linear
    }

    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        _webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;
        mac.update(payload);

        let computed = hex::encode(mac.finalize().into_bytes());

        Ok(constant_time_compare(&computed, signature))
    }
}

/// HubSpot webhook signature verifier.
///
/// HubSpot v3 uses HMAC-SHA256 with format: `v3:<timestamp>:<signature>`
/// The signature is computed over: `requestMethod + requestUri + requestBody + timestamp`
#[derive(Debug)]
pub struct HubSpotVerifier;

impl WebhookVerifier for HubSpotVerifier {
    fn provider(&self) -> Provider {
        Provider::Hubspot
    }

    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        _webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        // HubSpot signature format varies by version
        // For v3: the header contains the signature directly
        // We compute HMAC-SHA256 over the request body

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;
        mac.update(payload);

        let computed = hex::encode(mac.finalize().into_bytes());

        // HubSpot may send base64 or hex encoded signatures
        if constant_time_compare(&computed, signature) {
            return Ok(true);
        }

        // Try base64 decoding the signature
        if let Ok(decoded) = base64_decode(signature) {
            let sig_hex = hex::encode(&decoded);
            if constant_time_compare(&computed, &sig_hex) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Stripe webhook signature verifier.
///
/// Stripe uses HMAC-SHA256 with timestamp protection.
/// Format: `t=<timestamp>,v1=<signature>`
#[derive(Debug)]
pub struct StripeVerifier;

impl StripeVerifier {
    /// Maximum allowed timestamp difference (5 minutes)
    const MAX_TIMESTAMP_DIFF: i64 = 300;
}

impl WebhookVerifier for StripeVerifier {
    fn provider(&self) -> Provider {
        Provider::Stripe
    }

    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        _webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        // Parse Stripe signature format: t=<timestamp>,v1=<signature>
        let mut timestamp: Option<&str> = None;
        let mut signatures: Vec<&str> = Vec::new();

        for part in signature.split(',') {
            if let Some(ts) = part.strip_prefix("t=") {
                timestamp = Some(ts);
            } else if let Some(sig) = part.strip_prefix("v1=") {
                signatures.push(sig);
            }
        }

        let timestamp = timestamp.ok_or_else(|| {
            errors::Error::invalid("Missing timestamp in Stripe signature")
        })?;

        if signatures.is_empty() {
            return Err(errors::Error::invalid(
                "Missing v1 signature in Stripe header",
            ));
        }

        // Verify timestamp is within acceptable range
        let ts: i64 = timestamp
            .parse()
            .map_err(|_| errors::Error::invalid("Invalid timestamp"))?;
        let now = chrono::Utc::now().timestamp();
        if (now - ts).abs() > Self::MAX_TIMESTAMP_DIFF {
            return Err(errors::Error::invalid(
                "Timestamp too old or in future",
            ));
        }

        // Compute expected signature
        let signed_payload =
            format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;
        mac.update(signed_payload.as_bytes());

        let expected = hex::encode(mac.finalize().into_bytes());

        // Check if any of the provided signatures match
        for sig in signatures {
            if constant_time_compare(&expected, sig) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Square webhook signature verifier.
///
/// Square uses HMAC-SHA256 with base64 encoded signature.
/// The HMAC input is `notification_url + request_body` (the
/// registered webhook URL concatenated with the raw body).
/// The signature is sent in the `x-square-hmacsha256-signature`
/// header.
#[derive(Debug)]
pub struct SquareVerifier;

impl WebhookVerifier for SquareVerifier {
    fn provider(&self) -> Provider {
        Provider::Square
    }

    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        // Square computes HMAC-SHA256 over notification_url + body.
        // https://developer.squareup.com/docs/webhooks/step3validate
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;

        if let Some(url) = webhook_url {
            mac.update(url.as_bytes());
        }
        mac.update(payload);

        let expected = base64_encode(&mac.finalize().into_bytes());

        Ok(constant_time_compare(&expected, signature))
    }
}

/// Notion webhook signature verifier.
#[derive(Debug)]
pub struct NotionVerifier;

impl WebhookVerifier for NotionVerifier {
    fn provider(&self) -> Provider {
        Provider::Notion
    }

    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        _webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        // Notion uses standard HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;
        mac.update(payload);

        let computed = hex::encode(mac.finalize().into_bytes());

        Ok(constant_time_compare(&computed, signature))
    }
}

/// Generic HMAC-SHA256 verifier for custom webhooks.
#[derive(Debug)]
pub struct GenericHmacVerifier;

impl WebhookVerifier for GenericHmacVerifier {
    fn provider(&self) -> Provider {
        Provider::Generic
    }

    fn verify(
        &self,
        payload: &[u8],
        signature: &str,
        secret: &str,
        _webhook_url: Option<&str>,
    ) -> errors::Result<bool> {
        // Try sha256= prefix first
        let sig = signature.strip_prefix("sha256=").unwrap_or(signature);

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| errors::Error::invalid(e.to_string()))?;
        mac.update(payload);

        let computed = hex::encode(mac.finalize().into_bytes());

        Ok(constant_time_compare(&computed, sig))
    }
}

/// Constant-time string comparison to prevent timing attacks.
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

/// Simple base64 encode helper.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for &byte in input {
        buffer = (buffer << 8) | byte as u32;
        bits += 8;

        while bits >= 6 {
            bits -= 6;
            let index = ((buffer >> bits) & 0x3F) as usize;
            result.push(ALPHABET[index] as char);
        }
    }

    if bits > 0 {
        buffer <<= 6 - bits;
        let index = (buffer & 0x3F) as usize;
        result.push(ALPHABET[index] as char);
    }

    // Pad with '=' to make length multiple of 4
    while result.len() % 4 != 0 {
        result.push('=');
    }

    result
}

/// Simple base64 decode helper.
fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    use std::collections::HashMap;

    let alphabet: HashMap<char, u8> =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
            .chars()
            .enumerate()
            .map(|(i, c)| (c, i as u8))
            .collect();

    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for c in input.chars() {
        let val = *alphabet.get(&c).ok_or(())?;
        buffer = (buffer << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_verifier() {
        let verifier = GitHubVerifier;
        let payload = b"test payload";
        let secret = "test_secret";

        // Compute expected signature
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let expected =
            format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(verifier.verify(payload, &expected, secret, None).unwrap());
        assert!(!verifier
            .verify(payload, "sha256=invalid", secret, None)
            .unwrap());
    }

    #[test]
    fn test_stripe_verifier() {
        let verifier = StripeVerifier;
        let payload = b"test payload";
        let secret = "test_secret";
        let timestamp = chrono::Utc::now().timestamp();

        // Compute expected signature
        let signed =
            format!("{}.{}", timestamp, String::from_utf8_lossy(payload));
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signed.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        let header = format!("t={},v1={}", timestamp, sig);

        assert!(verifier.verify(payload, &header, secret, None).unwrap());
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc123", "abc123"));
        assert!(!constant_time_compare("abc123", "abc124"));
        assert!(!constant_time_compare("abc123", "abc12"));
    }

    #[test]
    fn test_square_verifier_with_url() {
        let verifier = SquareVerifier;
        let payload = b"test payload";
        let secret = "test_secret";
        let url = "https://example.com/webhooks/square/whe_123";

        // Square signs over notification_url + body
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(url.as_bytes());
        mac.update(payload);
        let expected = base64_encode(&mac.finalize().into_bytes());

        assert!(verifier
            .verify(payload, &expected, secret, Some(url))
            .unwrap());
        assert!(!verifier
            .verify(payload, "invalid", secret, Some(url))
            .unwrap());
    }

    #[test]
    fn test_square_verifier_without_url() {
        let verifier = SquareVerifier;
        let payload = b"test payload";
        let secret = "test_secret";

        // Without URL, falls back to body-only
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload);
        let expected = base64_encode(&mac.finalize().into_bytes());

        assert!(verifier.verify(payload, &expected, secret, None).unwrap());
    }

    #[test]
    fn test_verifier_registry() {
        let registry = WebhookVerifierRegistry::new();

        assert!(registry.get(Provider::Github).is_some());
        assert!(registry.get(Provider::Linear).is_some());
        assert!(registry.get(Provider::Hubspot).is_some());
        assert!(registry.get(Provider::Stripe).is_some());
        assert!(registry.get(Provider::Square).is_some());
    }
}
