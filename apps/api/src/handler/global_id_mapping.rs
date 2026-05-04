//! PLT-942: REST handler for the bakuure SDK lookup endpoint.
//!
//! `GET /v1beta/global-id-mapping?system=bakuure&code=BWS-001` returns the
//! Library-issued `global_id` for a (system, system_code) pair within the
//! caller's tenant. Tenant scope: header `x-operator-id` decides which row
//! is visible — cross-tenant lookups silently miss (no information leak).
//!
//! Policy gate: `library:ReadGlobalIdMapping` (bakuure SA Phase 1 read-only
//! per ADR-0001 / IaC commit ba8a0ac3).

use std::sync::Arc;

use axum::{extract::Query, Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::app::LibraryApp;
use crate::handler::library_executor_extractor::{
    LibraryExecutor, LibraryMultiTenancy,
};
use crate::usecase::GetGlobalIdMappingInputData;

#[derive(Debug, Deserialize)]
pub struct GetGlobalIdMappingQuery {
    pub system: String,
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GlobalIdMappingResponse {
    pub id: String,
    pub global_id: String,
    pub system: String,
    pub system_code: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[utoipa::path(
    get,
    path = "/v1beta/global-id-mapping",
    params(
        ("system" = String, Query, description = "Source system name (e.g. `bakuure`)"),
        ("code" = String, Query, description = "Code in the source system (e.g. `BWS-001`)"),
    ),
    responses(
        (status = 200, description = "Mapping found", body = GlobalIdMappingResponse),
        (status = 404, description = "No mapping for (system, code) in caller's tenant"),
    ),
    tag = "global_id_mapping",
)]
#[axum::debug_handler]
pub async fn get_global_id_mapping(
    Query(query): Query<GetGlobalIdMappingQuery>,
    Extension(library_app): Extension<Arc<LibraryApp>>,
    executor: LibraryExecutor,
    LibraryMultiTenancy(multi_tenancy): LibraryMultiTenancy,
) -> errors::Result<Json<GlobalIdMappingResponse>> {
    let mapping = library_app
        .get_global_id_mapping
        .execute(GetGlobalIdMappingInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            system: query.system,
            system_code: query.code,
        })
        .await?;

    let m = mapping
        .ok_or_else(|| errors::Error::not_found("global_id_mapping"))?;

    Ok(Json(GlobalIdMappingResponse {
        id: m.id().to_string(),
        global_id: m.global_id().to_string(),
        system: m.system().to_string(),
        system_code: m.system_code().to_string(),
        name: m.name().to_string(),
        created_at: m.created_at().to_rfc3339(),
        updated_at: m.updated_at().to_rfc3339(),
    }))
}
