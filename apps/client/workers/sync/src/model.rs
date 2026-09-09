use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use library_worker_common::hash;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MAX_BODY: usize = 4 * 1024 * 1024;
pub const MAX_UPDATE: usize = 128 * 1024 - 4096;
pub const MAX_API: usize = 8 * 1024 * 1024;
pub const MAX_TEXT: usize = MAX_BODY + 32 * 1024;
pub const MAX_SAFE: u64 = 9_007_199_254_740_991;
pub const TTL: i64 = 60_000;
pub const RESULT_TTL: i64 = 15 * 60_000;
pub const BODY_CHUNK: usize = 64 * 1024;
pub const META: &str = "live:room:meta:v1";
pub const POINTER: &str = "live:room:current-generation:v1";
pub const INIT: &str = "live:room:init-pending:v1";
pub const PENDING: &str = "live:checkpoint:pending:v1";
pub const RESULT_PREFIX: &str = "live:checkpoint:result:";
pub const RESULT_INDEX: &str = "live:checkpoint:result-index:v1";
pub const RESULT_CURSOR: &str = "live:checkpoint:result-cleanup-cursor:v1";
pub const INTERNAL: &str = "x-photon-live-internal";
pub const SESSION: &str = "x-photon-live-session";
pub const TARGET: &str = "x-photon-live-target";
pub const GENERATION: &str = "x-photon-live-generation";
pub const GENERATION_PREFIX: &str = "live-generation:";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub room_id: String,
    pub tenant: String,
    pub database: String,
    pub data: String,
    pub property: String,
    pub format: String,
}
impl Identity {
    pub fn valid(&self) -> bool {
        [
            &self.room_id,
            &self.tenant,
            &self.database,
            &self.data,
            &self.property,
            &self.format,
        ]
        .iter()
        .all(|s| bounded(s, 512))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
    #[serde(flatten)]
    pub identity: Identity,
    pub actor_id: String,
    pub body: String,
    pub record_version: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    #[serde(flatten)]
    pub identity: Identity,
    pub actor_id: String,
    pub record_version: String,
    pub body_hash: String,
    pub org: String,
    pub repo: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub authorization: String,
    pub expires_at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,
}
impl Session {
    pub fn valid(&self, reference: bool) -> bool {
        self.identity.valid()
            && [&self.actor_id, &self.record_version, &self.org, &self.repo]
                .iter()
                .all(|s| bounded(s, 512))
            && bounded(&self.body_hash, 128)
            && self.expires_at > 0
            && self.expires_at <= MAX_SAFE as i64
            && (if reference {
                bounded(&self.session_id, 512)
            } else {
                bearer(&self.authorization).is_some()
            })
            && self
                .platform_id
                .iter()
                .chain(self.operator_id.iter())
                .all(|s| bounded(s, 512))
    }
    pub fn reference(&self) -> Self {
        let mut s = self.clone();
        s.authorization.clear();
        s
    }
    pub fn header(&self) -> String {
        URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&self.reference()).expect("session JSON"),
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    #[serde(flatten)]
    pub identity: Identity,
    pub initialized: bool,
    pub version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_version: Option<u64>,
    pub body_hash: String,
}
impl Metadata {
    pub fn valid(&self) -> bool {
        self.identity.valid()
            && self.version <= MAX_SAFE
            && self.saved_version.is_none_or(|v| v <= self.version)
            && bounded(&self.body_hash, 128)
            && self.record_version.iter().all(|s| bounded(s, 512))
    }
    pub fn saved(&self) -> u64 {
        self.saved_version.unwrap_or(if self.initialized {
            self.version
        } else {
            0
        })
    }
    pub fn dirty(&self) -> bool {
        self.initialized && self.version > self.saved()
    }
    pub fn normalize(mut self) -> Self {
        self.saved_version = Some(self.saved());
        self
    }
    pub fn ready(&self, session: &Session, generation: &str) -> Value {
        json!({"type":"live-ready", "initialized": self.initialized, "version":self.version, "record_version":self.record_version.as_deref().unwrap_or(&session.record_version), "room_generation":generation})
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pointer {
    pub room_id: String,
    pub body_hash: String,
    pub record_version: String,
}
impl Pointer {
    pub fn valid(&self) -> bool {
        generated(&self.room_id)
            && bounded(&self.body_hash, 128)
            && bounded(&self.record_version, 512)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub version: u64,
    pub operation_id: String,
    pub expected_record_version: String,
    pub body_hash: String,
    pub fingerprint: String,
    pub body_byte_length: usize,
    pub chunk_count: usize,
}
impl Pending {
    pub fn valid(&self) -> bool {
        self.version <= MAX_SAFE
            && [
                &self.operation_id,
                &self.expected_record_version,
                &self.fingerprint,
            ]
            .iter()
            .all(|s| bounded(s, 512))
            && bounded(&self.body_hash, 128)
            && self.body_byte_length <= MAX_BODY
            && self.chunk_count <= MAX_BODY.div_ceil(BODY_CHUNK)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub version: u64,
    pub operation_id: String,
    pub record_version: String,
    pub body_hash: String,
    pub fingerprint: String,
    #[serde(default)]
    pub expires_at: i64,
}
impl Saved {
    pub fn valid(&self) -> bool {
        self.version <= MAX_SAFE
            && [&self.operation_id, &self.record_version, &self.fingerprint]
                .iter()
                .all(|s| bounded(s, 512))
            && bounded(&self.body_hash, 128)
            && self.expires_at >= 0
            && self.expires_at <= MAX_SAFE as i64
    }
    pub fn frame(&self) -> Value {
        json!({"type":"live-saved", "version":self.version, "record_version":self.record_version, "operation_id":self.operation_id})
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultEntry {
    pub operation_id: String,
    pub result_key: String,
    pub expires_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub kind: String,
    pub session_id: String,
    pub expires_at: i64,
}

pub fn bounded(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && !value.contains(['\0', '\r', '\n'])
}
pub fn field(value: &Value, key: &str) -> Option<String> {
    value[key]
        .as_str()
        .filter(|s| bounded(s, 512))
        .map(str::to_owned)
}
pub fn canonical(value: &Value, key: &str, legacy: &str) -> Option<String> {
    field(value, key).or_else(|| field(value, legacy))
}
pub fn bearer(value: &str) -> Option<String> {
    if value.len() > 8192 {
        return None;
    }
    let value = value.trim();
    let mut parts = value.split_whitespace();
    if !parts.next()?.eq_ignore_ascii_case("bearer")
        || parts.next().is_none()
        || parts.next().is_some()
    {
        return None;
    }
    Some(value.into())
}
pub fn token(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len())
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
pub fn decode64(value: &str, max: usize) -> Option<Vec<u8>> {
    if value.is_empty() || value.len() > (max * 4).div_ceil(3) + 8 {
        return None;
    }
    let normalized = value.replace('-', "+").replace('_', "/");
    let normalized = normalized.trim_end_matches('=');
    if value.len() - normalized.len() > 2 || normalized.len() % 4 == 1 {
        return None;
    }
    let padded = format!(
        "{}{}",
        normalized,
        "=".repeat((4 - normalized.len() % 4) % 4)
    );
    STANDARD.decode(padded).ok().filter(|b| b.len() <= max)
}
pub fn generated(value: &str) -> bool {
    bounded(value, 512)
        && value
            .strip_prefix(GENERATION_PREFIX)
            .is_some_and(|v| decode64(v, 1024).is_some())
}
pub fn generation(session: &Session) -> String {
    format!(
        "{GENERATION_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(format!(
            "{}\0{}\0{}",
            session.identity.room_id,
            session.record_version,
            session.body_hash
        ))
    )
}
pub fn newer(candidate: &str, current: Option<&str>) -> bool {
    let Some(current) = current else {
        return true;
    };
    if candidate == current {
        return false;
    }
    // API versions are non-negative decimal integers and can exceed u64.
    let normalize = |s: &str| -> Option<String> {
        let s = s.trim().strip_prefix('+').unwrap_or(s.trim());
        if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(s.trim_start_matches('0').into())
    };
    match (normalize(candidate), normalize(current)) {
        (Some(a), Some(b)) => {
            a.len() > b.len() || (a.len() == b.len() && a > b)
        }
        _ => false,
    }
}
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by(|(a, _), (b, _)| {
                a.encode_utf16().cmp(b.encode_utf16())
            });
            format!(
                "{{{}}}",
                entries
                    .into_iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("JSON key"),
                        canonical_json(value)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => value.to_string(),
    }
}

pub fn body_hash(format: &str, body: &str) -> String {
    let normalized = if format == "richText" {
        serde_json::from_str::<Value>(body)
            .ok()
            .and_then(|value| {
                // Match JavaScript's UTF-16 key ordering, numeric property order,
                // and number formatting. JSON.parse keeps __proto__ a data key.
                js_sys::JSON::parse(&canonical_json(&value))
                    .ok()
                    .and_then(|v| js_sys::JSON::stringify(&v).ok())
                    .and_then(|v| v.as_string())
            })
            .unwrap_or_else(|| body.into())
    } else {
        body.into()
    };
    hash(&format!("{format}\0{normalized}"))
}
pub fn result_key(id: &str) -> String {
    format!("{RESULT_PREFIX}{}", URL_SAFE_NO_PAD.encode(id))
}
pub fn body_key(index: usize) -> String {
    format!("live:checkpoint:pending:body:{index:06}")
}
