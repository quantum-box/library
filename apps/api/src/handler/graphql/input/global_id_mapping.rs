use async_graphql::InputObject;

#[derive(InputObject, Debug, Clone)]
pub struct CreateGlobalIdMappingInput {
    /// Optional Library-issued global ID (prefix `gid_`).
    /// Server generates one when omitted.
    pub global_id: Option<String>,
    /// Source system name (e.g. `bakuure`).
    pub system: String,
    /// Code in the source system (e.g. `BWS-001`).
    pub system_code: String,
    /// Display name.
    pub name: String,
}

#[derive(InputObject, Debug, Clone)]
pub struct UpdateGlobalIdMappingInput {
    pub id: String,
    /// Only `name` is mutable in Phase 1. To change `system` / `system_code`
    /// / `global_id`, await Phase 1.5+ (deletion is out of scope for Phase 1).
    pub name: String,
}
