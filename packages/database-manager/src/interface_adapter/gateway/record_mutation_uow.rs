use super::*;
use chrono::Utc;
use sqlx::types::Json;
use sqlx::{MySql, Transaction};
use std::collections::{HashMap, HashSet};

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
    policy: String,
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

    async fn lock_index_projection_guards(
        transaction: &mut Transaction<'_, MySql>,
        command: &DecideRecordPatchCommand,
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
            SELECT property_id, policy
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
                && definition
                    .property_id
                    .as_ref()
                    .is_some_and(|id| changed.contains(id))
        }))
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
                if matches!(
                    property.property_type(),
                    PropertyType::Relation(_)
                ) || matches!(
                    patch.value(),
                    PropertyValueCommand::Relation(_)
                ) {
                    return Err(errors::Error::not_supported(
                        "Relation changes require the RelationEdge writer",
                    ));
                }
                Self::validate_auto_generated_id(
                    &property,
                    current,
                    patch.value(),
                )?;
                let data = PropertyData::from_command(
                    &property,
                    patch.value().clone(),
                )?;
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

        let fields = match self
            .lock_database_and_fields(
                &mut transaction,
                command.tenant_id(),
                command.database_id(),
            )
            .await
        {
            Ok(fields) => fields,
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

        // Resolve definitions before any Record value changes. Relation
        // mutation and active Index projections remain fail-closed until their
        // cleanup/projection-aware writers join this same UoW.
        let mut relation_projection_required = false;
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
            if matches!(property.property_type(), PropertyType::Relation(_))
                || matches!(
                    patch.value(),
                    PropertyValueCommand::Relation(_)
                )
            {
                relation_projection_required = true;
            }
        }
        let index_projection_required =
            Self::lock_index_projection_guards(&mut transaction, command)
                .await?;

        let row = sqlx::query_as::<_, DataRow>(
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
        .await?;
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
        if index_projection_required {
            return Self::reject(
                transaction,
                command,
                RecordRejectionCode::IndexProjectionRequired,
            )
            .await;
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
