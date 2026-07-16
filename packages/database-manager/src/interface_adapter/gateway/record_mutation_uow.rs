use super::*;
use chrono::Utc;
use sqlx::types::Json;
use sqlx::{Acquire, MySql, Transaction};
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

struct OperationRequest<'a> {
    operation_id: &'a RecordOperationId,
    tenant_id: &'a TenantId,
    database_id: &'a DatabaseId,
    data_id: &'a DataId,
    mutation_kind: &'static str,
    actor: &'a RecordActor,
    expected_version: RecordVersion,
    fingerprint: &'a RecordRequestFingerprint,
}

impl<'a> OperationRequest<'a> {
    fn patch(command: &'a DecideRecordPatchCommand) -> Self {
        Self {
            operation_id: command.operation_id(),
            tenant_id: command.tenant_id(),
            database_id: command.database_id(),
            data_id: command.data_id(),
            mutation_kind: "PATCH",
            actor: command.actor(),
            expected_version: *command.expected_version(),
            fingerprint: command.fingerprint(),
        }
    }

    fn delete(command: &'a DecideRecordDeleteCommand) -> Self {
        Self {
            operation_id: command.operation_id(),
            tenant_id: command.tenant_id(),
            database_id: command.database_id(),
            data_id: command.data_id(),
            mutation_kind: "DELETE",
            actor: command.actor(),
            expected_version: *command.expected_version(),
            fingerprint: command.fingerprint(),
        }
    }
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

#[derive(Debug, sqlx::FromRow)]
struct LegacyRelationCandidateRow {
    data_id: String,
    relation_value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredRelationValue {
    present: bool,
    target_ids: BTreeSet<DataId>,
}

impl StoredRelationValue {
    fn absent() -> Self {
        Self {
            present: false,
            target_ids: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct DeleteRelationProperty {
    property: Property,
    field_num: u32,
}

#[derive(Debug)]
struct SourceRelationRewrite {
    property: Property,
    field_num: u32,
    remaining_target_ids: Vec<DataId>,
    delta: RecordPropertyDelta,
}

#[derive(Debug)]
struct SourceDeleteUpdate {
    source: RecordReference,
    previous_version: RecordVersion,
    next_version: RecordVersion,
    rewrites: Vec<SourceRelationRewrite>,
}

#[derive(Debug)]
struct LockedDeleteSchemaScope {
    fields_by_database: BTreeMap<String, Vec<FieldRow>>,
    definitions: Vec<RelationDefinition>,
    relation_properties: BTreeMap<String, DeleteRelationProperty>,
    index_guards_by_database:
        BTreeMap<String, Vec<IndexProjectionGuardRow>>,
    capability_rejection_required: bool,
}

struct LockedDeleteRecords {
    target: Option<DataRow>,
    sources: BTreeMap<(String, String), Option<DataRow>>,
}

impl LockedDeleteRecords {
    fn row(&self, record: &RecordReference) -> Option<&DataRow> {
        if self.target.as_ref().is_some_and(|target| {
            target.object_id == record.database_id().as_str()
                && target.id == record.data_id().as_str()
        }) {
            return self.target.as_ref();
        }
        self.sources
            .get(&(
                record.database_id().to_string(),
                record.data_id().to_string(),
            ))
            .and_then(Option::as_ref)
    }

    fn record_keys(&self) -> BTreeSet<(String, String)> {
        let mut keys =
            self.sources.keys().cloned().collect::<BTreeSet<_>>();
        if let Some(target) = &self.target {
            keys.insert((target.object_id.clone(), target.id.clone()));
        }
        keys
    }
}

impl DataRepositoryImpl {
    async fn claim_operation(
        transaction: &mut Transaction<'_, MySql>,
        request: &OperationRequest<'_>,
    ) -> errors::Result<OperationClaim> {
        let insert = sqlx::query(
            r#"
            INSERT INTO record_mutation_operations (
                operation_id, tenant_id, database_id, data_id,
                mutation_kind, actor_kind, actor_id, expected_version,
                fingerprint_version, request_fingerprint
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(request.operation_id.to_string())
        .bind(request.tenant_id.to_string())
        .bind(request.database_id.to_string())
        .bind(request.data_id.to_string())
        .bind(request.mutation_kind)
        .bind(request.actor.kind().to_string())
        .bind(request.actor.id())
        .bind(request.expected_version.get())
        .bind(request.fingerprint.version())
        .bind(request.fingerprint.digest().as_slice())
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
        .bind(request.operation_id.to_string())
        .fetch_one(&mut **transaction)
        .await?;

        let same_request = existing.tenant_id.as_str()
            == request.tenant_id.as_str()
            && existing.database_id.as_str()
                == request.database_id.as_str()
            && existing.data_id.as_str() == request.data_id.as_str()
            && existing.mutation_kind == request.mutation_kind
            && existing.actor_kind == request.actor.kind().to_string()
            && existing.actor_id.as_str() == request.actor.id()
            && existing.expected_version == request.expected_version.get()
            && existing.fingerprint_version
                == request.fingerprint.version()
            && existing.request_fingerprint.as_slice()
                == request.fingerprint.digest().as_slice();
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
            || decision.0.operation_id() != request.operation_id
        {
            return Err(errors::Error::internal_server_error(
                "Record operation decision metadata does not match its payload",
            ));
        }
        Ok(OperationClaim::Replay(decision.0))
    }

    async fn persist_decision(
        transaction: &mut Transaction<'_, MySql>,
        operation_id: &RecordOperationId,
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
        .bind(operation_id.to_string())
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
        operation_id: &RecordOperationId,
        decision: RecordMutationDecision,
    ) -> errors::Result<RecordMutationDecision> {
        Self::persist_decision(&mut transaction, operation_id, &decision)
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
            command.operation_id(),
            RecordMutationDecision::rejected(command.operation_id(), code),
        )
        .await
    }

    async fn reject_delete(
        transaction: Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        code: RecordRejectionCode,
    ) -> errors::Result<RecordMutationDecision> {
        Self::commit_decision(
            transaction,
            command.operation_id(),
            RecordMutationDecision::rejected(command.operation_id(), code),
        )
        .await
    }

    async fn discover_relation_definitions_for_delete(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
    ) -> errors::Result<Vec<RelationDefinition>> {
        let sql = format!(
            r#"
            SELECT {RELATION_DEFINITION_COLUMNS}
            FROM relationships
            WHERE tenant_id = ?
              AND (object_id = ? OR target_object_id = ?)
            ORDER BY object_id, id
            "#,
        );
        sqlx::query_as::<_, RelationDefinitionRow>(&sql)
            .bind(command.tenant_id().to_string())
            .bind(command.database_id().to_string())
            .bind(command.database_id().to_string())
            .fetch_all(&mut **transaction)
            .await?
            .into_iter()
            .map(RelationDefinition::try_from)
            .collect()
    }

    async fn lock_relation_definitions_for_delete(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
    ) -> errors::Result<Vec<RelationDefinition>> {
        let sql = format!(
            r#"
            SELECT {RELATION_DEFINITION_COLUMNS}
            FROM relationships
            WHERE tenant_id = ?
              AND (object_id = ? OR target_object_id = ?)
            ORDER BY object_id, id
            FOR UPDATE
            "#,
        );
        sqlx::query_as::<_, RelationDefinitionRow>(&sql)
            .bind(command.tenant_id().to_string())
            .bind(command.database_id().to_string())
            .bind(command.database_id().to_string())
            .fetch_all(&mut **transaction)
            .await?
            .into_iter()
            .map(RelationDefinition::try_from)
            .collect()
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

    async fn lock_delete_schema_scope(
        &self,
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        discovered: &[RelationDefinition],
    ) -> errors::Result<LockedDeleteSchemaScope> {
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
                SELECT id FROM objects
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

        let definitions = Self::lock_relation_definitions_for_delete(
            transaction,
            command,
        )
        .await?;
        if definitions.iter().any(|definition| {
            !endpoint_ids
                .contains(&definition.source_database_id().to_string())
                || !endpoint_ids
                    .contains(&definition.target_database_id().to_string())
        }) {
            return Err(errors::Error::internal_server_error(
                "RelationDefinition endpoint scope changed; retry the Record deletion",
            ));
        }

        let mut fields_by_database = BTreeMap::new();
        for endpoint_id in &endpoint_ids {
            let fields = sqlx::query_as::<_, FieldRow>(
                r#"
                SELECT id, tenant_id, object_id, field_name, datatype,
                       datatype_meta, is_indexed, field_num, meta_json,
                       type_key, type_version, type_config
                FROM fields
                WHERE tenant_id = ? AND object_id = ?
                ORDER BY field_num, id
                FOR UPDATE
                "#,
            )
            .bind(command.tenant_id().to_string())
            .bind(endpoint_id)
            .fetch_all(&mut **transaction)
            .await?;
            fields_by_database.insert(endpoint_id.clone(), fields);
        }

        let mut relation_properties = BTreeMap::new();
        let mut capability_rejection_required = false;
        for definition in &definitions {
            let property =
                (|| -> errors::Result<DeleteRelationProperty> {
                    definition.ensure_writable()?;
                    let fields = fields_by_database
                    .get(definition.source_database_id().as_str())
                    .ok_or_else(|| {
                        errors::Error::internal_server_error(
                            "locked Relation source Database fields are missing",
                        )
                    })?;
                    let field = fields
                        .iter()
                        .find(|field| {
                            field.id
                                == definition.source_property_id().as_str()
                        })
                        .ok_or_else(|| {
                            errors::Error::conflict(
                            "RelationDefinition source Property is missing",
                        )
                        })?;
                    let property = field
                        .definition_for_schema_write(
                            self.property_definition_mode,
                        )?
                        .to_property()?;
                    match property.property_type() {
                        PropertyType::Relation(relation)
                            if &relation.database_id
                                == definition.target_database_id() => {}
                        _ => {
                            return Err(errors::Error::conflict(
                                "RelationDefinition source Property does not match its target",
                            ));
                        }
                    }
                    Ok(DeleteRelationProperty {
                        property,
                        field_num: field.field_num,
                    })
                })();
            match property {
                Ok(property) => {
                    relation_properties
                        .insert(definition.id().to_string(), property);
                }
                Err(
                    errors::Error::BadRequest { .. }
                    | errors::Error::Conflict { .. },
                ) => capability_rejection_required = true,
                Err(error) => return Err(error),
            }
        }

        let mut index_guards_by_database = BTreeMap::new();
        for endpoint_id in &endpoint_ids {
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
            .bind(endpoint_id)
            .fetch_all(&mut **transaction)
            .await?;
            index_guards_by_database
                .insert(endpoint_id.clone(), definitions);
        }

        Ok(LockedDeleteSchemaScope {
            fields_by_database,
            definitions,
            relation_properties,
            index_guards_by_database,
            capability_rejection_required,
        })
    }

    fn relation_value_from_property_data(
        data: &PropertyData,
    ) -> errors::Result<StoredRelationValue> {
        let Some(value) = data.envelope() else {
            return Ok(StoredRelationValue::absent());
        };
        value.ensure_writable()?;
        let PropertyValue::Known(value) = value else {
            return Err(errors::Error::conflict(
                "opaque Relation PropertyValue is read-only",
            ));
        };
        let PropertyDataValue::Relation(_, target_ids) = value.value()
        else {
            return Err(errors::Error::conflict(
                "Relation PropertyValue has a different type",
            ));
        };
        let targets = target_ids.iter().cloned().collect::<BTreeSet<_>>();
        if targets.len() != target_ids.len() {
            return Err(errors::Error::conflict(
                "Relation PropertyValue contains duplicate targets",
            ));
        }
        Ok(StoredRelationValue {
            present: true,
            target_ids: targets,
        })
    }

    fn legacy_relation_value(
        property: &Property,
        raw: Option<String>,
    ) -> errors::Result<StoredRelationValue> {
        let Some(raw) = raw else {
            return Ok(StoredRelationValue::absent());
        };
        Self::relation_value_from_property_data(
            &PropertyData::from_storage(property, raw)?,
        )
    }

    fn canonical_relation_value(
        property: &Property,
        row: Option<&PropertyValueRow>,
    ) -> errors::Result<StoredRelationValue> {
        let Some(row) = row else {
            return Ok(StoredRelationValue::absent());
        };
        Self::relation_value_from_property_data(
            &PropertyData::from_envelope(property, row.envelope()?)?,
        )
    }

    async fn discover_delete_relation_sources(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        schema: &LockedDeleteSchemaScope,
    ) -> errors::Result<BTreeMap<String, BTreeSet<RecordReference>>> {
        let mut sources_by_relation = BTreeMap::new();
        for definition in &schema.definitions {
            let relation_id = definition.id().to_string();
            let sources = sources_by_relation
                .entry(relation_id.clone())
                .or_insert_with(BTreeSet::new);
            if definition.source_database_id() == command.database_id() {
                sources.insert(RecordReference::new(
                    command.database_id(),
                    command.data_id(),
                ));
            }

            if definition.target_database_id() == command.database_id() {
                if let Some(scope) =
                    schema.relation_properties.get(&relation_id)
                {
                    let sql = format!(
                        "SELECT id AS data_id, value{} AS relation_value \
                         FROM data WHERE tenant_id = ? AND object_id = ? \
                         AND value{} IS NOT NULL ORDER BY id",
                        scope.field_num, scope.field_num,
                    );
                    let rows = sqlx::query_as::<
                        _,
                        LegacyRelationCandidateRow,
                    >(&sql)
                    .bind(command.tenant_id().to_string())
                    .bind(definition.source_database_id().to_string())
                    .fetch_all(&mut **transaction)
                    .await?;
                    for row in rows {
                        let stored = Self::legacy_relation_value(
                            &scope.property,
                            row.relation_value,
                        )?;
                        if stored.target_ids.contains(command.data_id()) {
                            sources.insert(RecordReference::new(
                                definition.source_database_id(),
                                &row.data_id.parse()?,
                            ));
                        }
                    }

                    let rows = sqlx::query_as::<_, PropertyValueRow>(
                        r#"
                        SELECT tenant_id, database_id, data_id, property_id,
                               type_key, type_version,
                               value_encoding_version, value
                        FROM property_values
                        WHERE tenant_id = ? AND database_id = ?
                          AND property_id = ?
                        ORDER BY data_id
                        "#,
                    )
                    .bind(command.tenant_id().to_string())
                    .bind(definition.source_database_id().to_string())
                    .bind(definition.source_property_id().to_string())
                    .fetch_all(&mut **transaction)
                    .await?;
                    for row in rows {
                        let stored = Self::canonical_relation_value(
                            &scope.property,
                            Some(&row),
                        )?;
                        if stored.target_ids.contains(command.data_id()) {
                            sources.insert(RecordReference::new(
                                definition.source_database_id(),
                                &row.data_id.parse()?,
                            ));
                        }
                    }
                }

                let rows = sqlx::query_as::<_, RelationEdgeMutationRow>(
                    r#"
                    SELECT tenant_id, source_database_id, source_data_id,
                           relation_id, target_database_id, target_data_id
                    FROM relation_edges
                    WHERE tenant_id = ? AND target_database_id = ?
                      AND target_data_id = ? AND relation_id = ?
                      AND source_database_id = ?
                    ORDER BY source_database_id, source_data_id, relation_id,
                             target_database_id, target_data_id
                    "#,
                )
                .bind(command.tenant_id().to_string())
                .bind(command.database_id().to_string())
                .bind(command.data_id().to_string())
                .bind(definition.id().to_string())
                .bind(definition.source_database_id().to_string())
                .fetch_all(&mut **transaction)
                .await?;
                for row in rows {
                    let edge = row.restore(definition)?;
                    sources.insert(edge.source().clone());
                }
            }
        }
        Ok(sources_by_relation)
    }

    async fn lock_delete_relation_edges(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        definitions: &[RelationDefinition],
        sources_by_relation: &BTreeMap<String, BTreeSet<RecordReference>>,
    ) -> errors::Result<BTreeMap<String, Vec<RelationEdge>>> {
        let definitions_by_id = definitions
            .iter()
            .map(|definition| (definition.id().to_string(), definition))
            .collect::<BTreeMap<_, _>>();
        let mut lock_scopes = sources_by_relation
            .iter()
            .flat_map(|(relation_id, sources)| {
                sources.iter().map(move |source| {
                    (source.clone(), relation_id.clone())
                })
            })
            .collect::<Vec<_>>();
        lock_scopes.sort_by(|left, right| {
            left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1))
        });

        let mut edges_by_relation =
            BTreeMap::<String, Vec<RelationEdge>>::new();
        for (source, relation_id) in lock_scopes {
            let definition =
                definitions_by_id.get(&relation_id).ok_or_else(|| {
                    errors::Error::internal_server_error(
                        "RelationEdge lock scope has no definition",
                    )
                })?;
            let rows = sqlx::query_as::<_, RelationEdgeMutationRow>(
                r#"
                SELECT tenant_id, source_database_id, source_data_id,
                       relation_id, target_database_id, target_data_id
                FROM relation_edges
                WHERE tenant_id = ? AND source_database_id = ?
                  AND source_data_id = ? AND relation_id = ?
                  AND target_database_id = ?
                ORDER BY tenant_id, source_database_id, source_data_id,
                         relation_id, target_database_id, target_data_id
                FOR UPDATE
                "#,
            )
            .bind(command.tenant_id().to_string())
            .bind(source.database_id().to_string())
            .bind(source.data_id().to_string())
            .bind(&relation_id)
            .bind(definition.target_database_id().to_string())
            .fetch_all(&mut **transaction)
            .await?;
            for row in rows {
                edges_by_relation
                    .entry(relation_id.clone())
                    .or_default()
                    .push(row.restore(definition)?);
            }
        }
        Ok(edges_by_relation)
    }

    async fn lock_delete_records(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        sources_by_relation: &BTreeMap<String, BTreeSet<RecordReference>>,
    ) -> errors::Result<LockedDeleteRecords> {
        let target =
            RecordReference::new(command.database_id(), command.data_id());
        let mut records = BTreeSet::from([RecordReference::new(
            command.database_id(),
            command.data_id(),
        )]);
        records.extend(
            sources_by_relation
                .values()
                .flat_map(|sources| sources.iter().cloned()),
        );
        let mut locked_target = None;
        let mut locked_sources = BTreeMap::new();
        for record in records {
            let row = sqlx::query_as::<_, DataRow>(
                r#"
                SELECT * FROM data
                WHERE tenant_id = ? AND object_id = ? AND id = ?
                FOR UPDATE
                "#,
            )
            .bind(command.tenant_id().to_string())
            .bind(record.database_id().to_string())
            .bind(record.data_id().to_string())
            .fetch_optional(&mut **transaction)
            .await?;
            if record == target {
                locked_target = row;
            } else {
                locked_sources.insert(
                    (
                        record.database_id().to_string(),
                        record.data_id().to_string(),
                    ),
                    row,
                );
            }
        }
        Ok(LockedDeleteRecords {
            target: locked_target,
            sources: locked_sources,
        })
    }

    async fn lock_delete_canonical_values(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        records: &LockedDeleteRecords,
    ) -> errors::Result<BTreeMap<(String, String, String), PropertyValueRow>>
    {
        let mut canonical = BTreeMap::new();
        for (database_id, data_id) in records.record_keys() {
            let rows = sqlx::query_as::<_, PropertyValueRow>(
                r#"
                SELECT tenant_id, database_id, data_id, property_id,
                       type_key, type_version, value_encoding_version, value
                FROM property_values
                WHERE tenant_id = ? AND database_id = ? AND data_id = ?
                ORDER BY property_id
                FOR UPDATE
                "#,
            )
            .bind(command.tenant_id().to_string())
            .bind(&database_id)
            .bind(&data_id)
            .fetch_all(&mut **transaction)
            .await?;
            for row in rows {
                canonical.insert(
                    (
                        row.database_id.clone(),
                        row.data_id.clone(),
                        row.property_id.clone(),
                    ),
                    row,
                );
            }
        }
        Ok(canonical)
    }

    fn delete_index_projection_required(
        command: &DecideRecordDeleteCommand,
        schema: &LockedDeleteSchemaScope,
        sources_by_relation: &BTreeMap<String, BTreeSet<RecordReference>>,
    ) -> bool {
        if schema
            .index_guards_by_database
            .get(command.database_id().as_str())
            .is_some_and(|guards| {
                guards.iter().any(|guard| guard.policy != "NONE")
            })
        {
            return true;
        }

        schema.definitions.iter().any(|definition| {
            if definition.target_database_id() != command.database_id()
                || *definition.on_target_delete()
                    != RelationOnDelete::Nullify
            {
                return false;
            }
            let has_actionable_source = sources_by_relation
                .get(&definition.id().to_string())
                .is_some_and(|sources| {
                    sources.iter().any(|source| {
                        source.database_id() != command.database_id()
                            || source.data_id() != command.data_id()
                    })
                });
            has_actionable_source
                && schema
                    .index_guards_by_database
                    .get(definition.source_database_id().as_str())
                    .is_some_and(|guards| {
                        guards.iter().any(|guard| {
                            guard.policy != "NONE"
                                && (guard.property_id.as_deref()
                                    == Some(
                                        definition
                                            .source_property_id()
                                            .as_str(),
                                    )
                                    || guard.relation_id.as_deref()
                                        == Some(definition.id().as_str()))
                        })
                    })
        })
    }

    fn build_delete_relation_plan(
        command: &DecideRecordDeleteCommand,
        schema: &LockedDeleteSchemaScope,
        sources_by_relation: &BTreeMap<String, BTreeSet<RecordReference>>,
        edges_by_relation: &BTreeMap<String, Vec<RelationEdge>>,
        records: &LockedDeleteRecords,
        canonical: &BTreeMap<(String, String, String), PropertyValueRow>,
    ) -> errors::Result<RelationTargetDeletionPlan> {
        let deleted =
            RecordReference::new(command.database_id(), command.data_id());
        let mut scopes = Vec::with_capacity(schema.definitions.len());

        for definition in &schema.definitions {
            let relation_id = definition.id().to_string();
            let relation_property = schema
                .relation_properties
                .get(&relation_id)
                .ok_or_else(|| {
                    errors::Error::internal_server_error(
                        "writable RelationDefinition has no source Property",
                    )
                })?;
            let edges = edges_by_relation
                .get(&relation_id)
                .cloned()
                .unwrap_or_default();
            let _ = RelationEdgeSet::new(definition, edges.clone())?;
            let edges_by_source = edges.iter().fold(
                BTreeMap::<RecordReference, BTreeSet<DataId>>::new(),
                |mut grouped, edge| {
                    grouped
                        .entry(edge.source().clone())
                        .or_default()
                        .insert(edge.target().data_id().clone());
                    grouped
                },
            );

            let mut logical_edges = Vec::new();
            for source in sources_by_relation
                .get(&relation_id)
                .into_iter()
                .flat_map(|sources| sources.iter())
            {
                let row = records.row(source).ok_or_else(|| {
                    errors::Error::internal_server_error(
                        "discovered Relation source Record disappeared",
                    )
                })?;
                let legacy = Self::legacy_relation_value(
                    &relation_property.property,
                    row.get_field(relation_property.field_num)?,
                )?;
                let canonical_key = (
                    source.database_id().to_string(),
                    source.data_id().to_string(),
                    definition.source_property_id().to_string(),
                );
                if let Some(canonical_row) = canonical.get(&canonical_key) {
                    let canonical_value = Self::canonical_relation_value(
                        &relation_property.property,
                        Some(canonical_row),
                    )?;
                    if legacy != canonical_value {
                        return Err(errors::Error::conflict(
                            "legacy and canonical Relation values do not match during Record deletion",
                        ));
                    }
                }
                let edge_targets = edges_by_source
                    .get(source)
                    .cloned()
                    .unwrap_or_default();
                if !edge_targets.is_subset(&legacy.target_ids) {
                    return Err(errors::Error::conflict(
                        "RelationEdges contain a target absent from the authoritative legacy value during Record deletion",
                    ));
                }
                logical_edges.extend(
                    legacy
                        .target_ids
                        .iter()
                        .map(|target_data_id| {
                            RelationEdge::new(
                                definition,
                                source.clone(),
                                RecordReference::new(
                                    definition.target_database_id(),
                                    target_data_id,
                                ),
                            )
                        })
                        .collect::<errors::Result<Vec<_>>>()?,
                );
            }

            let _ =
                RelationEdgeSet::new(definition, logical_edges.clone())?;
            let incident = logical_edges
                .into_iter()
                .filter(|edge| {
                    edge.source() == &deleted || edge.target() == &deleted
                })
                .collect();
            scopes.push(RelationEdgeSet::new(definition, incident)?);
        }

        RelationTargetDeletionPlan::new(
            command.tenant_id(),
            &deleted,
            scopes,
        )
    }

    fn plan_source_delete_updates(
        command: &DecideRecordDeleteCommand,
        schema: &LockedDeleteSchemaScope,
        plan: &RelationTargetDeletionPlan,
        records: &LockedDeleteRecords,
    ) -> errors::Result<Vec<SourceDeleteUpdate>> {
        let mut updates = Vec::with_capacity(plan.nullify_groups().len());
        for group in plan.nullify_groups() {
            let row = records.row(group.source()).ok_or_else(|| {
                errors::Error::internal_server_error(
                    "Nullify source Record disappeared",
                )
            })?;
            let previous_version = RecordVersion::new(row.record_version)?;
            let next_version = previous_version.checked_increment()?;
            let mut rewrites = BTreeMap::new();

            for nullification in group.nullifications() {
                let relation_id =
                    nullification.definition().id().to_string();
                let scope = schema
                    .relation_properties
                    .get(&relation_id)
                    .ok_or_else(|| {
                        errors::Error::internal_server_error(
                            "Nullify Relation Property is missing",
                        )
                    })?;
                let mut current = Self::legacy_relation_value(
                    &scope.property,
                    row.get_field(scope.field_num)?,
                )?
                .target_ids;
                if !current.remove(command.data_id()) {
                    return Err(errors::Error::internal_server_error(
                        "Nullify target is absent from locked legacy Relation value",
                    ));
                }
                let remaining_target_ids =
                    current.into_iter().collect::<Vec<DataId>>();
                let data = PropertyData::from_command(
                    &scope.property,
                    PropertyValueCommand::Relation(
                        remaining_target_ids.clone(),
                    ),
                )?;
                let value = data.envelope().as_ref().ok_or_else(|| {
                    errors::Error::internal_server_error(
                        "empty Relation rewrite lost its typed value",
                    )
                })?;
                value.ensure_writable()?;
                let PropertyValue::Known(value) = value else {
                    return Err(errors::Error::internal_server_error(
                        "writable Relation rewrite became opaque",
                    ));
                };
                let envelope = BUILTIN_PROPERTY_TYPE_REGISTRY
                    .encode_envelope(
                        &scope.property.property_type().canonical_config(),
                        value.value(),
                    )?;
                let rewrite = SourceRelationRewrite {
                    property: scope.property.clone(),
                    field_num: scope.field_num,
                    remaining_target_ids,
                    delta: RecordPropertyDelta::Set {
                        property_id: scope.property.id().clone(),
                        value: envelope,
                    },
                };
                if rewrites
                    .insert(scope.property.id().to_string(), rewrite)
                    .is_some()
                {
                    return Err(errors::Error::internal_server_error(
                        "duplicate Nullify rewrite for one Relation Property",
                    ));
                }
            }

            updates.push(SourceDeleteUpdate {
                source: group.source().clone(),
                previous_version,
                next_version,
                rewrites: rewrites.into_values().collect(),
            });
        }
        updates.sort_by(|left, right| left.source.cmp(&right.source));
        Ok(updates)
    }

    async fn persist_source_relation_rewrite(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        source: &RecordReference,
        rewrite: &SourceRelationRewrite,
    ) -> errors::Result<()> {
        let data = PropertyData::from_command(
            &rewrite.property,
            PropertyValueCommand::Relation(
                rewrite.remaining_target_ids.clone(),
            ),
        )?;
        let value = data.envelope().as_ref().ok_or_else(|| {
            errors::Error::internal_server_error(
                "empty Relation rewrite lost its typed value",
            )
        })?;
        value.ensure_writable()?;
        let PropertyValue::Known(value) = value else {
            return Err(errors::Error::internal_server_error(
                "writable Relation rewrite became opaque",
            ));
        };
        let envelope = BUILTIN_PROPERTY_TYPE_REGISTRY.encode_envelope(
            &rewrite.property.property_type().canonical_config(),
            value.value(),
        )?;
        sqlx::query(
            r#"
            INSERT INTO property_values (
                tenant_id, database_id, data_id, property_id,
                type_key, type_version, value_encoding_version, value
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                type_key = VALUES(type_key),
                type_version = VALUES(type_version),
                value_encoding_version = VALUES(value_encoding_version),
                value = VALUES(value)
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(source.database_id().to_string())
        .bind(source.data_id().to_string())
        .bind(rewrite.property.id().to_string())
        .bind(envelope.type_ref.key.as_str())
        .bind(envelope.type_ref.version.get())
        .bind(envelope.encoding_version.get())
        .bind(
            serde_json::to_string(&envelope.raw_value)
                .map_err(errors::Error::invalid)?,
        )
        .execute(&mut **transaction)
        .await?;

        let legacy = LegacyPropertyValueCodec::encode(
            value.value(),
            rewrite.property.property_type(),
        )?;
        let update = sqlx::query(&format!(
            "UPDATE data SET value{} = ? \
             WHERE tenant_id = ? AND object_id = ? AND id = ?",
            rewrite.field_num,
        ))
        .bind(legacy)
        .bind(command.tenant_id().to_string())
        .bind(source.database_id().to_string())
        .bind(source.data_id().to_string())
        .execute(&mut **transaction)
        .await?;
        if update.rows_affected() != 1 {
            return Err(errors::Error::internal_server_error(
                "locked Nullify source update affected an unexpected row count",
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn insert_delete_outbox_event(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordDeleteCommand,
        event_id: &RecordEventId,
        event_sequence: u32,
        aggregate: &RecordReference,
        aggregate_version: RecordVersion,
        event_type: &str,
        payload: serde_json::Value,
        occurred_at: chrono::DateTime<Utc>,
    ) -> errors::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO domain_outbox_events (
                event_id, operation_id, event_sequence, tenant_id,
                database_id, aggregate_type, aggregate_id,
                aggregate_version, event_type, payload, occurred_at
            )
            VALUES (?, ?, ?, ?, ?, 'RECORD', ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event_id.to_string())
        .bind(command.operation_id().to_string())
        .bind(event_sequence)
        .bind(command.tenant_id().to_string())
        .bind(aggregate.database_id().to_string())
        .bind(aggregate.data_id().to_string())
        .bind(aggregate_version.get())
        .bind(event_type)
        .bind(Json(payload))
        .bind(occurred_at)
        .execute(&mut **transaction)
        .await?;
        Ok(())
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
        let operation = OperationRequest::patch(command);
        match Self::claim_operation(&mut transaction, &operation).await? {
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
                command.operation_id(),
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
            command.operation_id(),
            RecordMutationDecision::accepted(
                command.operation_id(),
                next_version,
                vec![event_id],
            ),
        )
        .await
    }
}

#[async_trait::async_trait]
impl VersionedRecordDeletionUnitOfWork for DataRepositoryImpl {
    async fn decide_delete_atomically(
        &self,
        command: &DecideRecordDeleteCommand,
    ) -> errors::Result<RecordMutationDecision> {
        // There is deliberately no production constructor or configuration
        // path for this state yet. DELETE may enter the journal only when all
        // three Relation representations can be maintained atomically.
        if !self.relation_edge_mode.writes_edges()
            || !self.property_value_mode.writes_canonical()
        {
            return Err(errors::Error::internal_server_error(
                "versioned Record deletion requires dormant RelationEdge and canonical PropertyValue writes",
            ));
        }

        let mut connection = self.db.pool().acquire().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *connection)
            .await?;
        let mut transaction = connection.begin().await?;
        let operation = OperationRequest::delete(command);
        match Self::claim_operation(&mut transaction, &operation).await? {
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

        // DELETE uses READ COMMITTED on this one connection. Its non-locking
        // discovery reads therefore observe a writer that commits while this
        // transaction waits on the endpoint mutex, without acquiring a second
        // pool connection or weakening other Record mutation transactions.
        let discovered = Self::discover_relation_definitions_for_delete(
            &mut transaction,
            command,
        )
        .await?;
        let schema = match self
            .lock_delete_schema_scope(
                &mut transaction,
                command,
                &discovered,
            )
            .await
        {
            Ok(schema) => schema,
            Err(error) if error.is_not_found() => {
                return Self::reject_delete(
                    transaction,
                    command,
                    RecordRejectionCode::ResourceNotFound,
                )
                .await;
            }
            Err(error) => return Err(error),
        };
        let sources_by_relation = Self::discover_delete_relation_sources(
            &mut transaction,
            command,
            &schema,
        )
        .await?;
        let edges_by_relation = Self::lock_delete_relation_edges(
            &mut transaction,
            command,
            &schema.definitions,
            &sources_by_relation,
        )
        .await?;
        let mut records = Self::lock_delete_records(
            &mut transaction,
            command,
            &sources_by_relation,
        )
        .await?;
        let canonical = Self::lock_delete_canonical_values(
            &mut transaction,
            command,
            &records,
        )
        .await?;

        let Some(target_row) = records.target.as_ref() else {
            return Self::reject_delete(
                transaction,
                command,
                RecordRejectionCode::ResourceNotFound,
            )
            .await;
        };
        let target_version = RecordVersion::new(target_row.record_version)?;

        if &target_version != command.expected_version() {
            let target_fields = schema
                .fields_by_database
                .get(command.database_id().as_str())
                .ok_or_else(|| {
                    errors::Error::internal_server_error(
                        "deleted Record Database fields are missing",
                    )
                })?;
            let target_canonical = canonical
                .iter()
                .filter(|((database_id, data_id, _), _)| {
                    database_id == command.database_id().as_str()
                        && data_id == command.data_id().as_str()
                })
                .map(|((_, data_id, property_id), row)| {
                    ((data_id.clone(), property_id.clone()), row.clone())
                })
                .collect::<HashMap<_, _>>();
            let target_row = records.target.take().ok_or_else(|| {
                errors::Error::internal_server_error(
                    "locked Record disappeared before conflict hydration",
                )
            })?;
            let current = hydrate_data_row(
                target_row,
                target_fields,
                &target_canonical,
                self.property_value_mode,
                self.property_definition_mode,
            )?;
            let snapshot = self.snapshot(&current, target_fields)?;
            return Self::commit_decision(
                transaction,
                command.operation_id(),
                RecordMutationDecision::conflict(
                    command.operation_id(),
                    snapshot,
                ),
            )
            .await;
        }
        // All schema/index/edge locks are acquired before the target Record,
        // but stale CAS wins over every policy/capability rejection.
        if schema.capability_rejection_required {
            return Self::reject_delete(
                transaction,
                command,
                RecordRejectionCode::RelationProjectionRequired,
            )
            .await;
        }
        // Legacy is authoritative during backfill, but every canonical row
        // and normalized edge that does exist must agree with it. Corruption
        // is an infrastructure failure and must roll back the pending journal
        // claim rather than being hidden behind a durable policy decision.
        let deletion_plan = Self::build_delete_relation_plan(
            command,
            &schema,
            &sources_by_relation,
            &edges_by_relation,
            &records,
            &canonical,
        )?;
        if Self::delete_index_projection_required(
            command,
            &schema,
            &sources_by_relation,
        ) {
            return Self::reject_delete(
                transaction,
                command,
                RecordRejectionCode::IndexProjectionRequired,
            )
            .await;
        }
        if deletion_plan.is_restricted() {
            return Self::reject_delete(
                transaction,
                command,
                RecordRejectionCode::RelationDeleteRestricted,
            )
            .await;
        }

        let target_next_version = match target_version.checked_increment() {
            Ok(version) => version,
            Err(errors::Error::Conflict { .. }) => {
                return Self::reject_delete(
                    transaction,
                    command,
                    RecordRejectionCode::VersionExhausted,
                )
                .await;
            }
            Err(error) => return Err(error),
        };
        if deletion_plan.nullify_groups().iter().any(|group| {
            records
                .row(group.source())
                .is_some_and(|row| row.record_version == u64::MAX)
        }) {
            return Self::reject_delete(
                transaction,
                command,
                RecordRejectionCode::VersionExhausted,
            )
            .await;
        }
        let source_updates = Self::plan_source_delete_updates(
            command,
            &schema,
            &deletion_plan,
            &records,
        )?;
        if u32::try_from(source_updates.len() + 1).is_err() {
            return Self::reject_delete(
                transaction,
                command,
                RecordRejectionCode::VersionExhausted,
            )
            .await;
        }

        let occurred_at = Utc::now();
        for update in &source_updates {
            for rewrite in &update.rewrites {
                Self::persist_source_relation_rewrite(
                    &mut transaction,
                    command,
                    &update.source,
                    rewrite,
                )
                .await?;
            }
            let result = sqlx::query(
                r#"
                UPDATE data
                SET updated_at = ?, record_version = ?
                WHERE tenant_id = ? AND object_id = ? AND id = ?
                  AND record_version = ?
                "#,
            )
            .bind(occurred_at)
            .bind(update.next_version.get())
            .bind(command.tenant_id().to_string())
            .bind(update.source.database_id().to_string())
            .bind(update.source.data_id().to_string())
            .bind(update.previous_version.get())
            .execute(&mut *transaction)
            .await?;
            if result.rows_affected() != 1 {
                return Err(errors::Error::internal_server_error(
                    "locked Nullify source CAS affected an unexpected row count",
                ));
            }
        }

        let deleted =
            RecordReference::new(command.database_id(), command.data_id());
        let mut incident_edges = edges_by_relation
            .values()
            .flat_map(|edges| edges.iter())
            .filter(|edge| {
                edge.source() == &deleted || edge.target() == &deleted
            })
            .cloned()
            .collect::<Vec<_>>();
        incident_edges.sort_by(|left, right| {
            (
                left.source().database_id(),
                left.source().data_id(),
                left.relation_id(),
                left.target().database_id(),
                left.target().data_id(),
            )
                .cmp(&(
                    right.source().database_id(),
                    right.source().data_id(),
                    right.relation_id(),
                    right.target().database_id(),
                    right.target().data_id(),
                ))
        });
        incident_edges.dedup();
        for edge in incident_edges {
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
            .execute(&mut *transaction)
            .await?;
            if result.rows_affected() != 1 {
                return Err(errors::Error::internal_server_error(
                    "locked RelationEdge cleanup affected an unexpected row count",
                ));
            }
        }

        sqlx::query(
            r#"
            DELETE FROM indexes
            WHERE tenant_id = ? AND object_id = ?
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.data_id().to_string())
        .execute(&mut *transaction)
        .await?;
        let deleted_row = sqlx::query(
            r#"
            DELETE FROM data
            WHERE tenant_id = ? AND object_id = ? AND id = ?
              AND record_version = ?
            "#,
        )
        .bind(command.tenant_id().to_string())
        .bind(command.database_id().to_string())
        .bind(command.data_id().to_string())
        .bind(command.expected_version().get())
        .execute(&mut *transaction)
        .await?;
        if deleted_row.rows_affected() != 1 {
            return Err(errors::Error::internal_server_error(
                "locked Record CAS deletion affected an unexpected row count",
            ));
        }

        let mut event_ids = Vec::with_capacity(source_updates.len() + 1);
        let mut event_sequence = 1_u32;
        for update in source_updates {
            let event_id = RecordEventId::default();
            let event_type = "database.record.patched.v1".to_string();
            let event = RecordPatchedEventV1 {
                event_id: event_id.clone(),
                event_type: event_type.clone(),
                operation_id: command.operation_id().clone(),
                tenant_id: command.tenant_id().clone(),
                database_id: update.source.database_id().clone(),
                data_id: update.source.data_id().clone(),
                previous_version: update.previous_version.to_string(),
                record_version: update.next_version.to_string(),
                actor: command.actor().clone(),
                name: None,
                properties: update
                    .rewrites
                    .into_iter()
                    .map(|rewrite| rewrite.delta)
                    .collect(),
                occurred_at,
            };
            Self::insert_delete_outbox_event(
                &mut transaction,
                command,
                &event_id,
                event_sequence,
                &update.source,
                update.next_version,
                &event_type,
                serde_json::to_value(&event)
                    .map_err(errors::Error::internal_server_error)?,
                occurred_at,
            )
            .await?;
            event_ids.push(event_id);
            event_sequence += 1;
        }

        let delete_event_id = RecordEventId::default();
        let delete_event_type = "database.record.deleted.v1".to_string();
        let delete_event = RecordDeletedEventV1 {
            event_id: delete_event_id.clone(),
            event_type: delete_event_type.clone(),
            operation_id: command.operation_id().clone(),
            tenant_id: command.tenant_id().clone(),
            database_id: command.database_id().clone(),
            data_id: command.data_id().clone(),
            previous_version: command.expected_version().to_string(),
            record_version: target_next_version.to_string(),
            actor: command.actor().clone(),
            occurred_at,
        };
        Self::insert_delete_outbox_event(
            &mut transaction,
            command,
            &delete_event_id,
            event_sequence,
            &deleted,
            target_next_version,
            &delete_event_type,
            serde_json::to_value(&delete_event)
                .map_err(errors::Error::internal_server_error)?,
            occurred_at,
        )
        .await?;
        event_ids.push(delete_event_id);

        Self::commit_decision(
            transaction,
            command.operation_id(),
            RecordMutationDecision::accepted(
                command.operation_id(),
                target_next_version,
                event_ids,
            ),
        )
        .await
    }
}
