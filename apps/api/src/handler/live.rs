//! Authorization and checkpoint adapters for Photon Live document rooms.
//!
//! Photon Live owns the ephemeral Yjs room. Library remains the authority for
//! the tenant, record, Property definition, and durable checkpoint. This
//! module therefore resolves every room from canonical ids and never accepts a
//! client supplied room id or actor id.

use std::sync::Arc;

use axum::{
    extract::{Extension, Path as AxumPath},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use database_manager::{
    domain::{
        Data, DataId, DatabaseId, Property, PropertyDataValue, PropertyId,
        PropertyType, PropertyValueCommand, RecordMutationDecision,
        RecordOperationId, RecordVersion,
    },
    usecase::PatchRecordInputData,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tachyon_sdk::auth::{
    CheckPolicyInput, ExecutorAction, MultiTenancyAction,
};
use utoipa::ToSchema;
use value_object::TenantId;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::LibraryExecutor;
use crate::sdk_auth::SdkAuthApp;
use crate::usecase::{LibraryOrg, ViewDataInputData, ViewRepoInputData};

const UPDATE_REPO_ACTION: &str = "library:UpdateRepo";

/// The two Property types that Photon Live can open as a document.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema,
)]
pub enum LiveBodyFormat {
    #[serde(rename = "richText")]
    #[schema(rename = "richText")]
    RichText,
    #[serde(rename = "markdown")]
    #[schema(rename = "markdown")]
    Markdown,
}

impl LiveBodyFormat {
    fn for_property(property: &Property) -> errors::Result<Self> {
        match property.property_type() {
            PropertyType::RichText => Ok(Self::RichText),
            PropertyType::Markdown => Ok(Self::Markdown),
            _ => Err(errors::Error::not_found(
                "Live document Property not found",
            )),
        }
    }
}

/// Request for the read-only room authorization exchange.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LiveAuthorizeRequest {
    #[serde(alias = "propertyId")]
    pub property_id: String,
}

/// Canonical scope and current body returned to the Photon Live worker.
///
/// Field names intentionally use the snake_case protocol used by the
/// Database mutation decision DTOs. `format` is the only editor-facing name
/// and is `richText` or `markdown`.
#[derive(Debug, Serialize, ToSchema)]
pub struct LiveAuthorizationResponse {
    pub tenant_id: String,
    pub database_id: String,
    pub repo_id: String,
    pub data_id: String,
    pub property_id: String,
    pub actor_id: String,
    pub record_version: String,
    pub format: LiveBodyFormat,
    /// RichText is encoded as the editor's serialized JSON document string;
    /// Markdown is returned as its source text.
    pub body: String,
    pub room_id: String,
}

/// Request that checkpoints one Photon Live body through the Database BC's
/// versioned patch port. The client cannot provide a title or another
/// Property, so a checkpoint can never overwrite those fields accidentally.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LiveCheckpointRequest {
    #[serde(alias = "propertyId")]
    pub property_id: String,
    #[serde(alias = "operationId")]
    pub operation_id: String,
    /// Decimal RecordVersion. The string form is canonical on the wire.
    #[serde(
        alias = "expected_version",
        alias = "expectedVersion",
        alias = "expectedRecordVersion"
    )]
    pub expected_record_version: String,
    pub format: LiveBodyFormat,
    /// RichText is the serialized JSON document string; Markdown is source
    /// text. Only this one Property is sent to the Database patch port.
    pub body: String,
}

/// Durable decision plus the same canonical room scope used at authorize.
#[derive(Debug, Serialize, ToSchema)]
pub struct LiveCheckpointResponse {
    pub tenant_id: String,
    pub database_id: String,
    pub repo_id: String,
    pub data_id: String,
    pub property_id: String,
    pub actor_id: String,
    pub record_version: String,
    pub format: LiveBodyFormat,
    pub room_id: String,
    pub decision: Value,
}

#[derive(Debug, Clone, Copy)]
struct LiveTarget<'a> {
    org: &'a str,
    repo: &'a str,
    data_id: &'a str,
    property_id: &'a str,
}

#[derive(Debug)]
struct LiveScope {
    tenant_id: TenantId,
    database_id: DatabaseId,
    data_id: DataId,
    repo_id: String,
    property: Property,
    data: Data,
    actor_id: String,
    format: LiveBodyFormat,
    room_id: String,
}

impl LiveScope {
    fn authorization(&self) -> errors::Result<LiveAuthorizationResponse> {
        Ok(LiveAuthorizationResponse {
            tenant_id: self.tenant_id.to_string(),
            database_id: self.database_id.to_string(),
            repo_id: self.repo_id.clone(),
            data_id: self.data_id.to_string(),
            property_id: self.property.id().to_string(),
            actor_id: self.actor_id.clone(),
            record_version: self.data.record_version().to_string(),
            format: self.format,
            body: current_body(&self.data, &self.property, self.format)?,
            room_id: self.room_id.clone(),
        })
    }
}

/// POST /v1beta/repos/{org}/{repo}/data/{data_id}/live/authorize
///
/// This endpoint authorizes a room but never mints a token. Photon Live must
/// forward the caller's Bearer token to this boundary on every authorization
/// exchange and derive its room from the response's canonical ids.
#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/data/{data_id}/live/authorize",
    request_body = LiveAuthorizeRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID")
    ),
    responses(
        (status = 200, description = "Photon Live room authorized", body = LiveAuthorizationResponse),
        (status = 400, description = "Invalid Property or request"),
        (status = 401, description = "Caller is not authenticated"),
        (status = 403, description = "Caller cannot edit the repository"),
        (status = 404, description = "Repository, record, or Property not found"),
        (status = 503, description = "Record patching is disabled during storage rollout")
    )
)]
#[axum::debug_handler]
pub async fn authorize_live(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Extension(base_sdk): Extension<Arc<SdkAuthApp>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    Json(request): Json<LiveAuthorizeRequest>,
) -> errors::Result<Json<LiveAuthorizationResponse>> {
    let scope = load_live_scope(
        &library_app,
        &base_sdk,
        &executor,
        &library_org,
        LiveTarget {
            org: &org,
            repo: &repo,
            data_id: &data_id,
            property_id: &request.property_id,
        },
    )
    .await?;

    if !record_patch_enabled() {
        return Err(errors::Error::service_unavailable(
            "Photon Live authorization requires dual-write PropertyValue storage",
        ));
    }

    Ok(Json(scope.authorization()?))
}

/// POST /v1beta/repos/{org}/{repo}/data/{data_id}/live/checkpoint
///
/// This is deliberately dormant until PropertyValue backfill/parity has
/// enabled a dual-write storage mode. The Database patch port itself keeps
/// the CAS, idempotency, actor derivation, and outbox transaction boundary.
#[utoipa::path(
    post,
    path = "/v1beta/repos/{org}/{repo}/data/{data_id}/live/checkpoint",
    request_body = LiveCheckpointRequest,
    params(
        ("org" = String, Path, description = "Organization username"),
        ("repo" = String, Path, description = "Repository username"),
        ("data_id" = String, Path, description = "Data ID")
    ),
    responses(
        (status = 200, description = "Photon Live checkpoint decision", body = LiveCheckpointResponse),
        (status = 400, description = "Invalid checkpoint"),
        (status = 401, description = "Caller is not authenticated"),
        (status = 403, description = "Caller cannot edit the repository"),
        (status = 404, description = "Repository, record, or Property not found"),
        (status = 409, description = "Record version conflict"),
        (status = 422, description = "Record patch rejected"),
        (status = 503, description = "Record patching is disabled during storage rollout")
    )
)]
#[axum::debug_handler]
pub async fn checkpoint_live(
    AxumPath((org, repo, data_id)): AxumPath<(String, String, String)>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    Extension(base_sdk): Extension<Arc<SdkAuthApp>>,
    Extension(database_app): Extension<Arc<database_manager::App>>,
    executor: LibraryExecutor,
    library_org: LibraryOrg,
    Json(request): Json<LiveCheckpointRequest>,
) -> errors::Result<impl IntoResponse> {
    let scope = load_live_scope(
        &library_app,
        &base_sdk,
        &executor,
        &library_org,
        LiveTarget {
            org: &org,
            repo: &repo,
            data_id: &data_id,
            property_id: &request.property_id,
        },
    )
    .await?;

    if !record_patch_enabled() {
        return Err(errors::Error::service_unavailable(
            "Photon Live checkpoints require dual-write PropertyValue storage",
        ));
    }

    let expected_version =
        parse_record_version(&request.expected_record_version)?;
    let operation_id = RecordOperationId::new(request.operation_id)?;
    let value =
        command_for_body(&scope.property, request.format, &request.body)?;

    let decision = database_app
        .patch_record_usecase()
        .execute(PatchRecordInputData {
            executor: &executor,
            multi_tenancy: &library_org,
            tenant_id: &scope.tenant_id,
            database_id: &scope.database_id,
            data_id: &scope.data_id,
            operation_id: &operation_id,
            expected_version,
            name: None,
            properties: vec![
                database_manager::usecase::PropertyDataInputData {
                    property_id: scope.property.id().clone(),
                    value,
                },
            ],
        })
        .await?;
    let status = match &decision {
        RecordMutationDecision::Accepted { .. } => StatusCode::OK,
        RecordMutationDecision::Conflict { .. } => StatusCode::CONFLICT,
        RecordMutationDecision::Rejected { .. } => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
    };
    let decision_json = serde_json::to_value(&decision)
        .map_err(errors::Error::internal_server_error)?;
    let record_version = decision_record_version(
        &decision_json,
        scope.data.record_version().to_string(),
    );

    Ok((
        status,
        Json(LiveCheckpointResponse {
            tenant_id: scope.tenant_id.to_string(),
            database_id: scope.database_id.to_string(),
            repo_id: scope.repo_id,
            data_id: scope.data_id.to_string(),
            property_id: scope.property.id().to_string(),
            actor_id: scope.actor_id,
            record_version: record_version.to_string(),
            format: scope.format,
            room_id: scope.room_id,
            decision: decision_json,
        }),
    ))
}

async fn load_live_scope(
    library_app: &LibraryApp,
    base_sdk: &SdkAuthApp,
    executor: &LibraryExecutor,
    library_org: &LibraryOrg,
    target: LiveTarget<'_>,
) -> errors::Result<LiveScope> {
    // A public repository can be read by an anonymous executor. Live always
    // requires a caller credential because this response grants a write room.
    if executor.is_none() {
        return Err(errors::Error::unauthorized(
            "Photon Live requires an authenticated caller",
        ));
    }

    let (data, properties) = library_app
        .view_data
        .execute(&ViewDataInputData {
            executor,
            multi_tenancy: library_org,
            org_username: target.org.to_string(),
            repo_username: target.repo.to_string(),
            data_id: target.data_id.to_string(),
        })
        .await?;

    let repo_output = library_app
        .view_repo
        .execute(&ViewRepoInputData {
            executor,
            multi_tenancy: library_org,
            organization_username: target.org.to_string(),
            repo_username: target.repo.to_string(),
        })
        .await?;

    let tenant_id = library_org.get_operator_id()?;
    if tenant_id != *data.tenant_id()
        || repo_output.repo.organization_id() != &tenant_id
        || !executor.has_tenant_id(&tenant_id)
    {
        // Keep tenant mismatches indistinguishable from a missing document.
        return Err(errors::Error::not_found("resource not found"));
    }

    let property_id = target.property_id.parse::<PropertyId>()?;
    let property = properties
        .into_iter()
        .find(|property| property.id() == &property_id)
        .ok_or_else(|| {
            errors::Error::not_found("Live document Property")
        })?;
    if property.tenant_id() != data.tenant_id()
        || property.database_id() != data.database_id()
    {
        return Err(errors::Error::not_found("Live document Property"));
    }
    let format = LiveBodyFormat::for_property(&property)?;

    // Use the request credential for the write policy. The process credential
    // may only be used for service-owned work and must never grant a public
    // reader a Live editing room.
    let caller_auth = executor.caller_auth_app(base_sdk)?.auth_app();
    caller_auth
        .check_policy(&CheckPolicyInput {
            executor,
            multi_tenancy: library_org,
            action: UPDATE_REPO_ACTION,
        })
        .await?;

    let tenant_id = data.tenant_id().clone();
    let database_id = data.database_id().clone();
    let data_id = data.id().clone();
    let room_id =
        canonical_room_id(&tenant_id, &database_id, &data_id, &property_id);

    Ok(LiveScope {
        tenant_id,
        database_id,
        data_id,
        repo_id: repo_output.repo.id().to_string(),
        property,
        data,
        actor_id: executor.get_id().to_string(),
        format,
        room_id,
    })
}

fn current_body(
    data: &Data,
    property: &Property,
    format: LiveBodyFormat,
) -> errors::Result<String> {
    let value = data
        .get_property_data(property.id())
        .and_then(|property_data| property_data.value().as_ref());

    match (format, value) {
        (_, None) => Ok(String::new()),
        (
            LiveBodyFormat::RichText,
            Some(PropertyDataValue::RichText(body)),
        ) => serde_json::to_string(body)
            .map_err(errors::Error::internal_server_error),
        (
            LiveBodyFormat::Markdown,
            Some(PropertyDataValue::Markdown(body)),
        ) => Ok(body.clone()),
        _ => Err(errors::Error::internal_server_error(
            "stored Live body does not match its Property definition",
        )),
    }
}

fn command_for_body(
    property: &Property,
    format: LiveBodyFormat,
    body: &str,
) -> errors::Result<PropertyValueCommand> {
    match (format, property.property_type()) {
        (LiveBodyFormat::RichText, PropertyType::RichText) => {
            let body = serde_json::from_str(body).map_err(|_| {
                errors::Error::invalid(
                    "richText Live body must be a serialized JSON document",
                )
            })?;
            Ok(PropertyValueCommand::RichText(body))
        }
        (LiveBodyFormat::Markdown, PropertyType::Markdown) => {
            Ok(PropertyValueCommand::Markdown(body.to_string()))
        }
        _ => Err(errors::Error::invalid(
            "checkpoint format does not match the Property definition",
        )),
    }
}

fn parse_record_version(value: &str) -> errors::Result<RecordVersion> {
    let number = value.parse::<u64>().map_err(|_| {
        errors::Error::invalid(
            "expected_record_version must be a decimal string",
        )
    })?;
    RecordVersion::new(number)
}

fn decision_record_version(decision: &Value, fallback: String) -> String {
    decision
        .get("record_version")
        .and_then(Value::as_str)
        .or_else(|| {
            decision
                .get("current")
                .and_then(|current| current.get("record_version"))
                .and_then(Value::as_str)
        })
        .map(ToOwned::to_owned)
        .unwrap_or(fallback)
}

/// Keep the room identity entirely server-derived from canonical scope ids.
pub fn canonical_room_id(
    tenant_id: &TenantId,
    database_id: &DatabaseId,
    data_id: &DataId,
    property_id: &PropertyId,
) -> String {
    format!(
        "tenant:{}:database:{}:data:{}:property:{}",
        tenant_id, database_id, data_id, property_id
    )
}

fn record_patch_enabled() -> bool {
    // Keep the public route deployed but inert until both rollout controls
    // are explicitly enabled. This prevents an accidental Live room from
    // becoming writable during the legacy-only storage phase.
    if !matches!(
        std::env::var("LIBRARY_PHOTON_LIVE_ENABLED").as_deref(),
        Ok("true")
    ) {
        return false;
    }

    std::env::var("PROPERTY_VALUE_STORAGE_MODE")
        .ok()
        .and_then(|value| value.parse().ok())
        .is_some_and(
            database_manager::property_value_rollout::PropertyValueStorageMode::writes_canonical,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_id_uses_only_canonical_scope_ids() {
        let tenant_id = "tn_01hmp05xtq6fs5mmk8fg125cy7".parse().unwrap();
        let database_id = "db_01hmp05xtq6fs5mmk8fg125cy7".parse().unwrap();
        let data_id = "data_01hmp05xtq6fs5mmk8fg125cy7".parse().unwrap();
        let property_id =
            "prop_01hmp05xtq6fs5mmk8fg125cy7".parse().unwrap();

        assert_eq!(
            canonical_room_id(
                &tenant_id,
                &database_id,
                &data_id,
                &property_id,
            ),
            "tenant:tn_01hmp05xtq6fs5mmk8fg125cy7:database:db_01hmp05xtq6fs5mmk8fg125cy7:data:data_01hmp05xtq6fs5mmk8fg125cy7:property:prop_01hmp05xtq6fs5mmk8fg125cy7"
        );
    }

    #[test]
    fn checkpoint_body_requires_the_declared_format() {
        let property = Property::new(
            &"prop_01hmp05xtq6fs5mmk8fg125cy7".parse().unwrap(),
            &"tn_01hmp05xtq6fs5mmk8fg125cy7".parse().unwrap(),
            &"db_01hmp05xtq6fs5mmk8fg125cy7".parse().unwrap(),
            "body",
            &PropertyType::Markdown,
            false,
            0,
        );

        assert!(command_for_body(
            &property,
            LiveBodyFormat::Markdown,
            "hello"
        )
        .is_ok());
        assert!(command_for_body(
            &property,
            LiveBodyFormat::RichText,
            "[]",
        )
        .is_err());
        assert!(command_for_body(
            &property,
            LiveBodyFormat::Markdown,
            "{\"text\":\"hello\"}",
        )
        .is_ok());
    }

    #[test]
    fn record_version_parser_accepts_decimal_wire_forms_only() {
        assert_eq!(parse_record_version("9").unwrap().get(), 9);
        assert!(parse_record_version("0").is_err());
        assert!(parse_record_version("9.0").is_err());
    }

    #[test]
    fn patch_activation_requires_a_dual_write_mode() {
        use database_manager::property_value_rollout::PropertyValueStorageMode;

        assert!(!PropertyValueStorageMode::LegacyOnly.writes_canonical());
        assert!(PropertyValueStorageMode::DualWriteLegacyRead
            .writes_canonical());
        assert!(PropertyValueStorageMode::DualWriteCanonicalRead
            .writes_canonical());
    }
}
