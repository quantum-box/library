use super::*;
use chrono::Utc;
use sqlx::types::Json;
use sqlx::{MySql, Transaction};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

const RELATION_DEFINITION_COLUMNS: &str = r#"
    id, tenant_id, object_id, field_id, target_object_id,
    forward_cardinality, reverse_cardinality, inverse_field_id,
    inverse_owned, on_target_delete, definition_version, generation
"#;

#[derive(Debug, sqlx::FromRow)]
struct OperationRow {
    tenant_id: String,
    database_id: String,
    data_id: String,
    mutation_kind: String,
    actor_kind: String,
    actor_id: String,
    expected_version: u64,
    fingerprint_version: u16,
    request_fingerprint: Vec<u8>,
    decision_kind: String,
    decision_version: Option<u16>,
    decision_payload: Option<Json<RecordMutationDecision>>,
}

#[derive(Debug, sqlx::FromRow)]
struct IndexProjectionGuardRow {
    property_id: Option<String>,
    relation_id: Option<String>,
    policy: String,
}

#[derive(Debug, sqlx::FromRow)]
struct RelationEdgeMutationRow {
    tenant_id: String,
    source_database_id: String,
    source_data_id: String,
    relation_id: String,
    target_database_id: String,
    target_data_id: String,
}

impl RelationEdgeMutationRow {
    fn restore(
        self,
        definition: &RelationDefinition,
    ) -> errors::Result<RelationEdge> {
        RelationEdge::restore(
            &self.tenant_id.parse()?,
            &self.relation_id.parse()?,
            RecordReference::new(
                &self.source_database_id.parse()?,
                &self.source_data_id.parse()?,
            ),
            RecordReference::new(
                &self.target_database_id.parse()?,
                &self.target_data_id.parse()?,
            ),
            definition,
        )
    }
}

enum OperationClaim {
    New,
    Replay(RecordMutationDecision),
    Reused,
}

#[derive(Debug)]
struct PlannedPropertyChange {
    property: Property,
    data: PropertyData,
    change: PropertyValueChange,
    delta: RecordPropertyDelta,
}

#[derive(Debug)]
struct LockedRelationSchemaScope {
    fields: Vec<FieldRow>,
    definitions: BTreeMap<String, RelationDefinition>,
    inverse_property_ids: BTreeSet<String>,
}

#[derive(Debug)]
struct RelationPatchRequest {
    definition: RelationDefinition,
    requested_target_ids: Vec<DataId>,
}

#[derive(Debug)]
struct LockedRelationEdgeScope {
    definition: RelationDefinition,
    source: RecordReference,
    requested_target_ids: Vec<DataId>,
    current_forward: Vec<RelationEdge>,
    requested_target_backlinks: Vec<RelationEdge>,
}

impl DataRepositoryImpl {
    async fn claim_operation(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
    ) -> errors::Result<OperationClaim> {
        let insert = sqlx::query(
            r#"
            INSERT INTO record_mutation_operations (
                operation_id, tenant_id, database_id, data_id,
                mutation_kind, actor_kind, actor_id, expected_version,
                fingerprint_version, request_fingerprint
            )
            VALUES (?, ?, ?, ?, 'PATCH', ?, ?, ?, ?, ?)
            "#,
        )
        .bind(command.operation_id().to_string())
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.data_id().to_string())
        .bind(command.actor().kind().to_string())
        .bind(command.actor().id())
        .bind(command.expected_version().get())
        .bind(command.fingerprint().version())
        .bind(command.fingerprint().digest().as_slice())
        .execute(&mut **transaction)
        .await;

        match insert {
            Ok(_) => return Ok(OperationClaim::New),
            Err(sqlx::Error::Database(error))
                if error.is_unique_violation() => {}
            Err(error) => return Err(error.into()),
        }

        // A concurrent insert of the same operation_id waits on its unique-key
        // lock. Once visible, the row is either a complete decision or schema
        // corruption: PENDING can never commit independently of the mutation.
        let existing = sqlx::query_as::<_, OperationRow>(
            r#"
            SELECT tenant_id, database_id, data_id,
                   CAST(mutation_kind AS CHAR) AS mutation_kind,
                   CAST(actor_kind AS CHAR) AS actor_kind,
                   CAST(actor_id AS CHAR) AS actor_id, expected_version,
                   fingerprint_version, request_fingerprint,
                   CAST(decision_kind AS CHAR) AS decision_kind,
                   decision_version, decision_payload
            FROM record_mutation_operations
            WHERE operation_id = ?
            FOR UPDATE
            "#,
        )
        .bind(command.operation_id().to_string())
        .fetch_one(&mut **transaction)
        .await?;

        let same_request = existing.tenant_id.as_str()
            == command.tenant_id().as_str()
            && existing.database_id.as_str()
                == command.database_id().as_str()
            && existing.data_id.as_str() == command.data_id().as_str()
            && existing.mutation_kind == "PATCH"
            && existing.actor_kind == command.actor().kind().to_string()
            && existing.actor_id.as_str() == command.actor().id()
            && existing.expected_version
                == command.expected_version().get()
            && existing.fingerprint_version
                == command.fingerprint().version()
            && existing.request_fingerprint.as_slice()
                == command.fingerprint().digest().as_slice();
        if !same_request {
            return Ok(OperationClaim::Reused);
        }
        if existing.decision_kind == "PENDING" {
            return Err(errors::Error::internal_server_error(
                "committed Record operation is still pending",
            ));
        }
        let decision_version =
            existing.decision_version.ok_or_else(|| {
                errors::Error::internal_server_error(
                    "final Record operation has no decision version",
                )
            })?;
        if decision_version != RECORD_DECISION_VERSION_V1 {
            return Err(errors::Error::internal_server_error(
                "unsupported Record operation decision version",
            ));
        }
        let decision = existing.decision_payload.ok_or_else(|| {
            errors::Error::internal_server_error(
                "final Record operation has no decision payload",
            )
        })?;
        if decision.0.kind() != existing.decision_kind
            || decision.0.decision_version() != decision_version
            || decision.0.operation_id() != command.operation_id()
        {
            return Err(errors::Error::internal_server_error(
                "Record operation decision metadata does not match its payload",
            ));
        }
        Ok(OperationClaim::Replay(decision.0))
    }

    async fn persist_decision(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
        decision: &RecordMutationDecision,
    ) -> errors::Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE record_mutation_operations
            SET decision_kind = ?, decision_version = ?,
                decision_payload = ?, decided_at = CURRENT_TIMESTAMP(6)
            WHERE operation_id = ? AND decision_kind = 'PENDING'
            "#,
        )
        .bind(decision.kind())
        .bind(RECORD_DECISION_VERSION_V1)
        .bind(Json(decision.clone()))
        .bind(command.operation_id().to_string())
        .execute(&mut **transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(errors::Error::internal_server_error(
                "Record operation decision was not finalized exactly once",
            ));
        }
        Ok(())
    }

    async fn commit_decision(
        mut transaction: Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
        decision: RecordMutationDecision,
    ) -> errors::Result<RecordMutationDecision> {
        Self::persist_decision(&mut transaction, command, &decision)
            .await?;
        transaction.commit().await?;
        Ok(decision)
    }

    async fn reject(
        transaction: Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
        code: RecordRejectionCode,
    ) -> errors::Result<RecordMutationDecision> {
        Self::commit_decision(
            transaction,
            command,
            RecordMutationDecision::rejected(command.operation_id(), code),
        )
        .await
    }

    async fn relation_definitions_for_patch(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
        lock: bool,
    ) -> errors::Result<Vec<RelationDefinition>> {
        let property_ids = command
            .patch()
            .properties()
            .iter()
            .map(|patch| patch.property_id().to_string())
            .collect::<BTreeSet<_>>();
        if property_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", property_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let lock_clause = if lock { " FOR UPDATE" } else { "" };
        let sql = format!(
            r#"
            SELECT {RELATION_DEFINITION_COLUMNS}
            FROM relationships
            WHERE tenant_id = ? AND (
                (object_id = ? AND field_id IN ({placeholders}))
                OR
                (target_object_id = ? AND inverse_field_id IN ({placeholders}))
            )
            ORDER BY id{lock_clause}
            "#,
        );
        let mut query = sqlx::query_as::<_, RelationDefinitionRow>(&sql)
            .bind(command.tenant_id().to_string())
            .bind(command.database_id().to_string());
        for property_id in &property_ids {
            query = query.bind(property_id);
        }
        query = query.bind(command.database_id().to_string());
        for property_id in &property_ids {
            query = query.bind(property_id);
        }
        query
            .fetch_all(&mut **transaction)
            .await?
            .into_iter()
            .map(RelationDefinition::try_from)
            .collect()
    }

    async fn lock_relation_schema_scope(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
    ) -> errors::Result<LockedRelationSchemaScope> {
        // Discovery is intentionally non-locking. It exists only to compute
        // the endpoint set that must be locked in primary-key order.
        let discovered = Self::relation_definitions_for_patch(
            transaction,
            command,
            false,
        )
        .await?;
        let mut endpoint_ids =
            BTreeSet::from([command.database_id().to_string()]);
        endpoint_ids.extend(discovered.iter().flat_map(|definition| {
            [
                definition.source_database_id().to_string(),
                definition.target_database_id().to_string(),
            ]
        }));

        for endpoint_id in &endpoint_ids {
            let locked = sqlx::query_scalar::<_, String>(
                r#"
                SELECT id
                FROM objects
                WHERE tenant_id = ? AND id = ?
                FOR UPDATE
                "#,
            )
            .bind(command.tenant_id().to_string())
            .bind(endpoint_id)
            .fetch_optional(&mut **transaction)
            .await?;
            if locked.is_none() {
                return Err(errors::Error::not_found("resource not found"));
            }
        }

        // Re-read after endpoint locking. A definition may have appeared or
        // changed after discovery. Never append a newly discovered endpoint
        // lock here: it could sort before one already held and invert the
        // Relation schema writer's lock order. Roll back and let the same
        // operation retry from discovery instead.
        let definitions = Self::relation_definitions_for_patch(
            transaction,
            command,
            true,
        )
        .await?;
        if definitions.iter().any(|definition| {
            !endpoint_ids
                .contains(&definition.source_database_id().to_string())
                || !endpoint_ids
                    .contains(&definition.target_database_id().to_string())
        }) {
            return Err(errors::Error::internal_server_error(
                "RelationDefinition endpoint scope changed; retry the Record operation",
            ));
        }

        let fields = sqlx::query_as::<_, FieldRow>(
            r#"
            SELECT id, tenant_id, object_id, field_name, datatype,
                   datatype_meta, is_indexed, field_num, meta_json,
                   type_key, type_version, type_config
            FROM fields
            WHERE tenant_id = ? AND object_id = ?
            ORDER BY field_num ASC, id ASC
            FOR UPDATE
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .fetch_all(&mut **transaction)
        .await?;

        let changed = command
            .patch()
            .properties()
            .iter()
            .map(|patch| patch.property_id().to_string())
            .collect::<HashSet<_>>();
        let mut source_definitions = BTreeMap::new();
        let mut inverse_property_ids = BTreeSet::new();
        for definition in definitions {
            if definition.source_database_id() == command.database_id()
                && changed
                    .contains(&definition.source_property_id().to_string())
            {
                source_definitions.insert(
                    definition.source_property_id().to_string(),
                    definition.clone(),
                );
            }
            if definition.target_database_id() == command.database_id() {
                if let Some(inverse_property_id) =
                    definition.inverse_property_id()
                {
                    if changed.contains(&inverse_property_id.to_string()) {
                        inverse_property_ids
                            .insert(inverse_property_id.to_string());
                    }
                }
            }
        }

        Ok(LockedRelationSchemaScope {
            fields,
            definitions: source_definitions,
            inverse_property_ids,
        })
    }

    async fn lock_index_projection_guards(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
        relation_ids: &HashSet<String>,
    ) -> errors::Result<bool> {
        let changed = command
            .patch()
            .properties()
            .iter()
            .map(|patch| patch.property_id().to_string())
            .collect::<HashSet<_>>();
        if changed.is_empty() {
            return Ok(false);
        }
        let definitions = sqlx::query_as::<_, IndexProjectionGuardRow>(
            r#"
            SELECT property_id, relation_id, policy
            FROM index_definitions
            WHERE tenant_id = ? AND database_id = ?
            ORDER BY id
            FOR UPDATE
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .fetch_all(&mut **transaction)
        .await?;
        Ok(definitions.iter().any(|definition| {
            definition.policy != "NONE"
                && (definition
                    .property_id
                    .as_ref()
                    .is_some_and(|id| changed.contains(id))
                    || definition
                        .relation_id
                        .as_ref()
                        .is_some_and(|id| relation_ids.contains(id)))
        }))
    }

    async fn lock_relation_edge_scopes(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
        mut requests: Vec<RelationPatchRequest>,
    ) -> errors::Result<Vec<LockedRelationEdgeScope>> {
        requests.sort_by(|left, right| {
            left.definition.id().cmp(right.definition.id())
        });
        let source =
            RecordReference::new(command.database_id(), command.data_id());
        let mut scopes = Vec::with_capacity(requests.len());

        for request in requests {
            let forward_rows =
                sqlx::query_as::<_, RelationEdgeMutationRow>(
                    r#"
                    SELECT tenant_id, source_database_id, source_data_id,
                           relation_id, target_database_id, target_data_id
                    FROM relation_edges
                    WHERE tenant_id = ?
                      AND source_database_id = ?
                      AND source_data_id = ?
                      AND relation_id = ?
                      AND target_database_id = ?
                    ORDER BY target_data_id
                    FOR UPDATE
                    "#,
                )
                .bind(command.tenant_id().to_string())
                .bind(command.database_id().to_string())
                .bind(command.data_id().to_string())
                .bind(request.definition.id().to_string())
                .bind(request.definition.target_database_id().to_string())
                .fetch_all(&mut **transaction)
                .await?;
            let current_forward = forward_rows
                .into_iter()
                .map(|row| row.restore(&request.definition))
                .collect::<errors::Result<Vec<_>>>()?;

            let mut backlink_edges = BTreeSet::new();
            let requested_targets = request
                .requested_target_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            for target_data_id in requested_targets {
                let rows = sqlx::query_as::<_, RelationEdgeMutationRow>(
                    r#"
                    SELECT tenant_id, source_database_id, source_data_id,
                           relation_id, target_database_id, target_data_id
                    FROM relation_edges
                    WHERE tenant_id = ?
                      AND target_database_id = ?
                      AND target_data_id = ?
                      AND relation_id = ?
                      AND source_database_id = ?
                    ORDER BY source_data_id
                    FOR UPDATE
                    "#,
                )
                .bind(command.tenant_id().to_string())
                .bind(request.definition.target_database_id().to_string())
                .bind(target_data_id.to_string())
                .bind(request.definition.id().to_string())
                .bind(command.database_id().to_string())
                .fetch_all(&mut **transaction)
                .await?;
                for row in rows {
                    backlink_edges
                        .insert(row.restore(&request.definition)?);
                }
            }
            let requested_target_backlinks =
                backlink_edges.into_iter().collect();

            scopes.push(LockedRelationEdgeScope {
                definition: request.definition,
                source: source.clone(),
                requested_target_ids: request.requested_target_ids,
                current_forward,
                requested_target_backlinks,
            });
        }
        Ok(scopes)
    }

    async fn lock_record_and_relation_targets(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
        scopes: &[LockedRelationEdgeScope],
    ) -> errors::Result<(Option<DataRow>, bool)> {
        let source = (
            command.database_id().to_string(),
            command.data_id().to_string(),
        );
        let mut records = BTreeSet::from([source.clone()]);
        for scope in scopes {
            records.extend(scope.requested_target_ids.iter().map(
                |target_data_id| {
                    (
                        scope.definition.target_database_id().to_string(),
                        target_data_id.to_string(),
                    )
                },
            ));
        }

        let mut source_row = None;
        let mut target_missing = false;
        for record in records {
            let row = sqlx::query_as::<_, DataRow>(
                r#"
                SELECT * FROM data
                WHERE tenant_id = ? AND object_id = ? AND id = ?
                FOR UPDATE
                "#,
            )
            .bind(command.tenant_id().to_string())
            .bind(&record.0)
            .bind(&record.1)
            .fetch_optional(&mut **transaction)
            .await?;
            if record == source {
                source_row = row;
            } else if row.is_none() {
                target_missing = true;
            }
        }
        Ok((source_row, target_missing))
    }

    async fn apply_relation_edge_plans(
        transaction: &mut Transaction<'_, MySql>,
        plans: &[RelationEdgeMutationPlan],
    ) -> errors::Result<()> {
        for plan in plans {
            for edge in plan.deletions() {
                let result = sqlx::query(
                    r#"
                    DELETE FROM relation_edges
                    WHERE tenant_id = ? AND source_database_id = ?
                      AND source_data_id = ? AND relation_id = ?
                      AND target_database_id = ? AND target_data_id = ?
                    "#,
                )
                .bind(edge.tenant_id().to_string())
                .bind(edge.source().database_id().to_string())
                .bind(edge.source().data_id().to_string())
                .bind(edge.relation_id().to_string())
                .bind(edge.target().database_id().to_string())
                .bind(edge.target().data_id().to_string())
                .execute(&mut **transaction)
                .await?;
                if result.rows_affected() != 1 {
                    return Err(errors::Error::internal_server_error(
                        "locked RelationEdge deletion affected an unexpected row count",
                    ));
                }
            }
            for edge in plan.insertions() {
                sqlx::query(
                    r#"
                    INSERT INTO relation_edges (
                        tenant_id, source_database_id, source_data_id,
                        relation_id, target_database_id, target_data_id
                    )
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(edge.tenant_id().to_string())
                .bind(edge.source().database_id().to_string())
                .bind(edge.source().data_id().to_string())
                .bind(edge.relation_id().to_string())
                .bind(edge.target().database_id().to_string())
                .bind(edge.target().data_id().to_string())
                .execute(&mut **transaction)
                .await?;
            }
        }
        Ok(())
    }

    async fn load_locked_canonical_values(
        &self,
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
    ) -> errors::Result<HashMap<(String, String), PropertyValueRow>> {
        if !self.property_value_mode.reads_or_shadows_canonical() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query_as::<_, PropertyValueRow>(
            r#"
            SELECT tenant_id, database_id, data_id, property_id, type_key,
                   type_version, value_encoding_version, value
            FROM property_values
            WHERE tenant_id = ? AND database_id = ? AND data_id = ?
            ORDER BY property_id
            FOR UPDATE
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.data_id().to_string())
        .fetch_all(&mut **transaction)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                ((row.data_id.clone(), row.property_id.clone()), row)
            })
            .collect())
    }

    fn encoded_snapshot_value(
        &self,
        fields: &[FieldRow],
        property_data: &PropertyData,
    ) -> errors::Result<Option<EncodedPropertyValue>> {
        let Some(value) = property_data.envelope() else {
            return Ok(None);
        };
        match value {
            PropertyValue::Opaque(value) => {
                Ok(Some(EncodedPropertyValue {
                    type_ref: value.type_ref.clone(),
                    encoding_version: value.encoding_version,
                    raw_value: value.raw_value.clone(),
                }))
            }
            PropertyValue::Known(value) => {
                let field = fields
                    .iter()
                    .find(|field| field.id == property_data.property_id().as_ref())
                    .ok_or_else(|| {
                        errors::Error::internal_server_error(
                            "Record snapshot Property definition is missing",
                        )
                    })?;
                let definition =
                    field.definition(self.property_definition_mode)?;
                let config = match definition.config() {
                    ResolvedPropertyConfig::Known(config) => config,
                    ResolvedPropertyConfig::Opaque(_) => {
                        return Err(errors::Error::internal_server_error(
                            "known Record value has an opaque Property definition",
                        ));
                    }
                };
                Ok(Some(
                    BUILTIN_PROPERTY_TYPE_REGISTRY
                        .encode_envelope(config, value.value())?,
                ))
            }
        }
    }

    fn snapshot(
        &self,
        record: &Data,
        fields: &[FieldRow],
    ) -> errors::Result<RecordSnapshot> {
        let properties = record
            .property_data()
            .iter()
            .map(|property_data| {
                Ok(RecordPropertySnapshot::new(
                    property_data.property_id(),
                    self.encoded_snapshot_value(fields, property_data)?,
                ))
            })
            .collect::<errors::Result<Vec<_>>>()?;
        Ok(RecordSnapshot::new(
            record.tenant_id(),
            record.database_id(),
            record.id(),
            record.name().to_string(),
            *record.record_version(),
            properties,
            record.created_at().to_owned(),
            record.updated_at().to_owned(),
        ))
    }

    fn validate_auto_generated_id(
        property: &Property,
        record: &Data,
        command: &PropertyValueCommand,
    ) -> errors::Result<()> {
        if !matches!(
            property.property_type(),
            PropertyType::Id(type_id) if type_id.auto_generate
        ) {
            return Ok(());
        }
        let PropertyValueCommand::Id(value) = command else {
            return Err(errors::Error::business_logic(
                "Auto-generated Id property is immutable",
            ));
        };
        let Some(current) = record
            .get_property_data(property.id())
            .map(PropertyData::string_value)
        else {
            return Err(errors::Error::business_logic(
                "Auto-generated Id property is immutable",
            ));
        };
        // Some pre-policy rows contain an external legacy ID instead of the
        // canonical DataId. Preserve that visible value until an explicit
        // repair migration; this mutation boundary must never silently
        // rewrite it merely because the requested value is the DataId.
        if value != &current {
            return Err(errors::Error::business_logic(
                "Auto-generated Id property is immutable",
            ));
        }
        Ok(())
    }

    fn plan_property_changes(
        &self,
        command: &DecideRecordPatchCommand,
        fields: &[FieldRow],
        current: &Data,
    ) -> errors::Result<Vec<PlannedPropertyChange>> {
        command
            .patch()
            .properties()
            .iter()
            .map(|patch| {
                let field = fields
                    .iter()
                    .find(|field| field.id == patch.property_id().as_ref())
                    .ok_or_else(|| {
                        errors::Error::not_found("resource not found")
                    })?;
                field.ensure_canonical_definition_writable()?;
                let property = field
                    .definition(self.property_definition_mode)?
                    .to_property()?;
                let value_command = match patch.value() {
                    PropertyValueCommand::Relation(targets) => {
                        let mut targets = targets.clone();
                        targets.sort();
                        PropertyValueCommand::Relation(targets)
                    }
                    value => value.clone(),
                };
                Self::validate_auto_generated_id(
                    &property,
                    current,
                    &value_command,
                )?;
                let data =
                    PropertyData::from_command(&property, value_command)?;
                let change = PropertyValueChange::from_property_data(
                    &property, &data,
                )?;
                let delta = match &change {
                    PropertyValueChange::Set { value, .. } => {
                        RecordPropertyDelta::Set {
                            property_id: property.id().clone(),
                            value: BUILTIN_PROPERTY_TYPE_REGISTRY
                                .encode_envelope(
                                    &property
                                        .property_type()
                                        .canonical_config(),
                                    value.value(),
                                )?,
                        }
                    }
                    PropertyValueChange::Clear { .. } => {
                        RecordPropertyDelta::Clear {
                            property_id: property.id().clone(),
                        }
                    }
                };
                Ok(PlannedPropertyChange {
                    property,
                    data,
                    change,
                    delta,
                })
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl VersionedRecordMutationUnitOfWork for DataRepositoryImpl {
    async fn decide_patch_atomically(
        &self,
        command: &DecideRecordPatchCommand,
    ) -> errors::Result<RecordMutationDecision> {
        if self.relation_edge_mode.writes_edges()
            && !self.property_value_mode.writes_canonical()
        {
            return Err(errors::Error::internal_server_error(
                "RelationEdge dual-write requires canonical PropertyValue dual-write",
            ));
        }
        let mut transaction = self.db.pool().begin().await?;
        match Self::claim_operation(&mut transaction, command).await? {
            OperationClaim::Replay(decision) => {
                transaction.commit().await?;
                return Ok(decision);
            }
            OperationClaim::Reused => {
                transaction.rollback().await?;
                return Ok(RecordMutationDecision::rejected(
                    command.operation_id(),
                    RecordRejectionCode::IdempotencyKeyReuse,
                ));
            }
            OperationClaim::New => {}
        }

        if command.patch().is_empty() {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::EmptyPatch,
            )
            .await;
        }
        if command.patch().has_duplicate_properties() {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::DuplicateProperty,
            )
            .await;
        }

        let (fields, relation_definitions, inverse_property_ids) =
            match if self.relation_edge_mode.writes_edges() {
                Self::lock_relation_schema_scope(&mut transaction, command)
                    .await
                    .map(|scope| {
                        (
                            scope.fields,
                            scope.definitions,
                            scope.inverse_property_ids,
                        )
                    })
            } else {
                self.lock_database_and_fields(
                    &mut transaction,
                    command.tenant_id(),
                    command.database_id(),
                )
                .await
                .map(|fields| (fields, BTreeMap::new(), BTreeSet::new()))
            } {
                Ok(scope) => scope,
                Err(error) if error.is_not_found() => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::ResourceNotFound,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            };

        // Resolve definitions before any Record value changes. When the
        // dormant writer is enabled by an internal test constructor, source
        // Relation definitions are already locked after their sorted endpoint
        // objects and before fields.
        let mut relation_projection_required = false;
        let mut invalid_property_value = false;
        let mut relation_requests = Vec::new();
        for patch in command.patch().properties() {
            let Some(field) = fields
                .iter()
                .find(|field| field.id == patch.property_id().as_ref())
            else {
                return Self::reject(
                    transaction,
                    command,
                    RecordRejectionCode::ResourceNotFound,
                )
                .await;
            };
            let property = match field
                .definition(self.property_definition_mode)
                .and_then(|definition| definition.to_property())
            {
                Ok(property) => property,
                Err(
                    errors::Error::BadRequest { .. }
                    | errors::Error::Conflict { .. },
                ) => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::InvalidPropertyValue,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            };
            match property.property_type() {
                PropertyType::Relation(relation_type) => {
                    if !self.relation_edge_mode.writes_edges()
                        || inverse_property_ids
                            .contains(&patch.property_id().to_string())
                    {
                        relation_projection_required = true;
                        continue;
                    }
                    let Some(definition) = relation_definitions
                        .get(&patch.property_id().to_string())
                    else {
                        relation_projection_required = true;
                        continue;
                    };
                    if definition.ensure_writable().is_err()
                        || definition.target_database_id()
                            != &relation_type.database_id
                    {
                        relation_projection_required = true;
                        continue;
                    }
                    let requested_target_ids = match patch.value() {
                        PropertyValueCommand::Clear => Vec::new(),
                        PropertyValueCommand::Relation(targets) => {
                            targets.clone()
                        }
                        _ => {
                            invalid_property_value = true;
                            continue;
                        }
                    };
                    relation_requests.push(RelationPatchRequest {
                        definition: definition.clone(),
                        requested_target_ids,
                    });
                }
                _ if matches!(
                    patch.value(),
                    PropertyValueCommand::Relation(_)
                ) =>
                {
                    invalid_property_value = true;
                }
                _ => {}
            }
        }
        let relation_ids = relation_requests
            .iter()
            .map(|request| request.definition.id().to_string())
            .collect::<HashSet<_>>();
        let index_projection_required = Self::lock_index_projection_guards(
            &mut transaction,
            command,
            &relation_ids,
        )
        .await?;

        let locked_relation_edges =
            if self.relation_edge_mode.writes_edges() {
                Self::lock_relation_edge_scopes(
                    &mut transaction,
                    command,
                    relation_requests,
                )
                .await?
            } else {
                Vec::new()
            };

        let (row, relation_target_missing) =
            if self.relation_edge_mode.writes_edges() {
                Self::lock_record_and_relation_targets(
                    &mut transaction,
                    command,
                    &locked_relation_edges,
                )
                .await?
            } else {
                (
                    sqlx::query_as::<_, DataRow>(
                        r#"
                        SELECT * FROM data
                        WHERE tenant_id = ? AND object_id = ? AND id = ?
                        FOR UPDATE
                        "#,
                    )
                    .bind(command.tenant_id().to_string())
                    .bind(command.database_id().to_string())
                    .bind(command.data_id().to_string())
                    .fetch_optional(&mut *transaction)
                    .await?,
                    false,
                )
            };
        let Some(row) = row else {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::ResourceNotFound,
            )
            .await;
        };
        let canonical = self
            .load_locked_canonical_values(&mut transaction, command)
            .await?;
        let current = hydrate_data_row(
            row,
            &fields,
            &canonical,
            self.property_value_mode,
            self.property_definition_mode,
        )?;

        if current.record_version() != command.expected_version() {
            let snapshot = self.snapshot(&current, &fields)?;
            return Self::commit_decision(
                transaction,
                command,
                RecordMutationDecision::conflict(
                    command.operation_id(),
                    snapshot,
                ),
            )
            .await;
        }
        // Schema/projection guards are locked before the Record to keep one
        // global lock order, but CAS conflict wins when the caller is stale.
        // A fresh retry can then receive the capability-specific rejection.
        if relation_projection_required {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::RelationProjectionRequired,
            )
            .await;
        }
        if invalid_property_value {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::InvalidPropertyValue,
            )
            .await;
        }
        if index_projection_required {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::IndexProjectionRequired,
            )
            .await;
        }
        if relation_target_missing {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::ResourceNotFound,
            )
            .await;
        }

        let mut relation_edge_plans = Vec::new();
        for scope in locked_relation_edges {
            let current_forward = match RelationEdgeSet::new(
                &scope.definition,
                scope.current_forward,
            ) {
                Ok(edges) => edges,
                Err(errors::Error::Conflict { .. }) => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::RelationCardinalityExceeded,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            };
            let requested_target_backlinks = match RelationEdgeSet::new(
                &scope.definition,
                scope.requested_target_backlinks,
            ) {
                Ok(edges) => edges,
                Err(errors::Error::Conflict { .. }) => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::RelationCardinalityExceeded,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            };
            match RelationEdgeMutationPlan::replace_forward(
                &scope.definition,
                scope.source,
                &current_forward,
                &requested_target_backlinks,
                scope.requested_target_ids,
            ) {
                Ok(plan) => relation_edge_plans.push(plan),
                Err(errors::Error::Conflict { .. }) => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::RelationCardinalityExceeded,
                    )
                    .await;
                }
                Err(errors::Error::BadRequest { .. }) => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::InvalidPropertyValue,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            }
        }
        let next_version =
            match current.record_version().checked_increment() {
                Ok(version) => version,
                Err(
                    errors::Error::BadRequest { .. }
                    | errors::Error::Conflict { .. },
                ) => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::VersionExhausted,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            };

        let planned =
            match self.plan_property_changes(command, &fields, &current) {
                Ok(planned) => planned,
                Err(error) if error.is_not_found() => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::ResourceNotFound,
                    )
                    .await;
                }
                Err(
                    errors::Error::BadRequest { .. }
                    | errors::Error::Conflict { .. },
                ) => {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::InvalidPropertyValue,
                    )
                    .await;
                }
                Err(error) => return Err(error),
            };

        // Validate every existing canonical value before the first write so a
        // domain rejection can be committed without a savepoint or partial
        // legacy/canonical mutation.
        for planned in &planned {
            if let Err(error) = Self::ensure_existing_canonical_is_writable(
                &mut transaction,
                &current,
                &planned.property,
            )
            .await
            {
                if matches!(
                    error,
                    errors::Error::BadRequest { .. }
                        | errors::Error::Conflict { .. }
                ) {
                    return Self::reject(
                        transaction,
                        command,
                        RecordRejectionCode::InvalidPropertyValue,
                    )
                    .await;
                }
                return Err(error);
            }
        }

        let mut updated = current.clone();
        let name_delta =
            command.patch().name().as_ref().map(|name| RecordNameDelta {
                previous: current.name().to_string(),
                current: name.to_string(),
            });
        if let Some(name) = command.patch().name() {
            updated.update_name(name);
        }
        for planned in &planned {
            updated.update_property_data(&planned.data)?;
        }
        for planned in &planned {
            self.apply_change(
                &mut transaction,
                &updated,
                &fields,
                &planned.change,
            )
            .await?;
        }
        Self::apply_relation_edge_plans(
            &mut transaction,
            &relation_edge_plans,
        )
        .await?;

        let update = sqlx::query(
            r#"
            UPDATE data
            SET name = ?, updated_at = ?, record_version = ?
            WHERE tenant_id = ? AND object_id = ? AND id = ?
              AND record_version = ?
            "#,
        )
        .bind(updated.name().to_string())
        .bind(updated.updated_at())
        .bind(next_version.get())
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.data_id().to_string())
        .bind(command.expected_version().get())
        .execute(&mut *transaction)
        .await?;
        if update.rows_affected() != 1 {
            return Err(errors::Error::internal_server_error(
                "locked Record CAS update affected an unexpected row count",
            ));
        }

        let event_id = RecordEventId::default();
        let occurred_at = Utc::now();
        let event = RecordPatchedEventV1 {
            event_id: event_id.clone(),
            event_type: "database.record.patched.v1".to_string(),
            operation_id: command.operation_id().clone(),
            tenant_id: command.tenant_id().clone(),
            database_id: command.database_id().clone(),
            data_id: command.data_id().clone(),
            previous_version: command.expected_version().to_string(),
            record_version: next_version.to_string(),
            actor: command.actor().clone(),
            name: name_delta,
            properties: planned
                .into_iter()
                .map(|planned| planned.delta)
                .collect(),
            occurred_at,
        };
        sqlx::query(
            r#"
            INSERT INTO domain_outbox_events (
                event_id, operation_id, event_sequence, tenant_id,
                database_id, aggregate_type, aggregate_id,
                aggregate_version, event_type, payload, occurred_at
            )
            VALUES (?, ?, 1, ?, ?, 'RECORD', ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event_id.to_string())
        .bind(command.operation_id().to_string())
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.data_id().to_string())
        .bind(next_version.get())
        .bind(&event.event_type)
        .bind(Json(event.clone()))
        .bind(occurred_at)
        .execute(&mut *transaction)
        .await?;

        Self::commit_decision(
            transaction,
            command,
            RecordMutationDecision::accepted(
                command.operation_id(),
                next_version,
                vec![event_id],
            ),
        )
        .await
    }
}
