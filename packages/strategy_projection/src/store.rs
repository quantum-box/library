//! All changes, object identities, edges and checkpoint commit in one transaction.
use super::*;
use sqlx::{MySql, MySqlPool, Row, Transaction};

#[derive(Clone, Debug)]
pub struct Store {
    pub(crate) pool: MySqlPool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub tenant_id: String,
    pub subscription_id: String,
    pub workspace_id: String,
    pub generation: u64,
    pub checkpoint: u64,
    pub cursor: Option<String>,
    pub writer: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRelease {
    pub id: String,
    pub data_id: String,
    pub source_version: String,
    pub title: String,
    pub body: String,
    pub published_by: String,
    pub published_at: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseSummary {
    pub id: String,
    pub data_id: String,
    pub title: String,
    pub source_version: String,
    pub published_at: String,
    pub revoked: bool,
}

impl Store {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
    pub async fn binding(&self, repo: &str) -> Result<Binding> {
        let r = sqlx::query(
            "SELECT * FROM library_pm_bindings WHERE repo_id=?",
        )
        .bind(repo)
        .fetch_one(&self.pool)
        .await?;
        Ok(Binding {
            tenant_id: r.try_get("tenant_id")?,
            subscription_id: r.try_get("subscription_id")?,
            workspace_id: r.try_get("workspace_id")?,
            generation: r.try_get("generation")?,
            checkpoint: r.try_get("checkpoint")?,
            cursor: r.try_get("cursor_value")?,
            writer: r.try_get("writer")?,
        })
    }
    /// Registration is idempotent, but does not authorize migration implicitly.
    pub async fn connect(
        &self,
        repo: &str,
        tenant: &str,
        sub: &str,
        workspace: &str,
        generation: u64,
    ) -> Result<Binding> {
        if sub.is_empty()
            || sub.len() > 255
            || workspace.is_empty()
            || workspace.len() > 255
            || generation == 0
        {
            return Err(Error::Invalid("invalid binding"));
        }
        sqlx::query("INSERT IGNORE INTO library_pm_bindings (repo_id,tenant_id,subscription_id,workspace_id,generation) VALUES (?,?,?,?,?)")
            .bind(repo).bind(tenant).bind(sub).bind(workspace).bind(generation).execute(&self.pool).await?;
        let b = self.binding(repo).await?;
        if b.tenant_id != tenant
            || b.subscription_id != sub
            || b.workspace_id != workspace
            || b.generation != generation
        {
            return Err(Error::Conflict(
                "binding already exists with different identity/generation",
            ));
        }
        Ok(b)
    }
    /// Fence is shared with every legacy Linear data-handler write. Acquiring it
    /// drains an in-flight old writer before changing ownership.
    pub async fn activate_tachyon_writer(&self, repo: &str) -> Result<()> {
        self.transition_writer(repo, "tachyon").await
    }
    pub async fn pause_projection(&self, repo: &str) -> Result<()> {
        self.transition_writer(repo, "paused").await
    }
    pub async fn restore_legacy_writer(&self, repo: &str) -> Result<()> {
        self.transition_writer(repo, "legacy").await
    }
    async fn transition_writer(
        &self,
        repo: &str,
        writer: &str,
    ) -> Result<()> {
        if !["tachyon", "paused", "legacy"].contains(&writer) {
            return Err(Error::Invalid("invalid writer"));
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT IGNORE INTO library_pm_writer_fences(repo_id,writer) VALUES (?,'legacy')").bind(repo).execute(&mut *tx).await?;
        sqlx::query("SELECT writer FROM library_pm_writer_fences WHERE repo_id=? FOR UPDATE").bind(repo).fetch_one(&mut *tx).await?;
        sqlx::query("SELECT repo_id FROM library_pm_bindings WHERE repo_id=? FOR UPDATE").bind(repo).fetch_one(&mut *tx).await?;
        if writer != "paused" {
            let applying:i64=sqlx::query_scalar("SELECT COUNT(*) FROM library_pm_migration_plans WHERE repo_id=? AND status='applying'").bind(repo).fetch_one(&mut *tx).await?;
            if applying > 0 {
                return Err(Error::Conflict(
                    "reviewed migration is still applying",
                ));
            }
        }
        sqlx::query(
            "UPDATE library_pm_writer_fences SET writer=? WHERE repo_id=?",
        )
        .bind(writer)
        .bind(repo)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE library_pm_bindings SET writer=? WHERE repo_id=?",
        )
        .bind(writer)
        .bind(repo)
        .execute(&mut *tx)
        .await?;
        if writer != "tachyon" {
            sqlx::query("UPDATE library_pm_records SET projection=NULL,lifecycle='inaccessible' WHERE repo_id=?").bind(repo).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn apply_page(
        &self,
        repo: &str,
        page: &ChangePage,
    ) -> Result<u64> {
        if page.changes.len() > 500 {
            return Err(Error::Invalid("page too large"));
        }
        for c in &page.changes {
            c.validate()?;
        }
        let mut tx = self.pool.begin().await?;
        let r = sqlx::query(
            "SELECT * FROM library_pm_bindings WHERE repo_id=? FOR UPDATE",
        )
        .bind(repo)
        .fetch_one(&mut *tx)
        .await?;
        if r.try_get::<String, _>("writer")? != "tachyon" {
            return Err(Error::Conflict("Tachyon writer is not active"));
        }
        let generation: u64 = r.try_get("generation")?;
        let subscription: String = r.try_get("subscription_id")?;
        let workspace: String = r.try_get("workspace_id")?;
        if page.generation != generation {
            return Err(Error::Conflict(
                "generation mismatch; explicit resync required",
            ));
        }
        let mut checkpoint: u64 = r.try_get("checkpoint")?;
        for c in &page.changes {
            if c.generation != generation
                || c.subscription_id != subscription
                || c.resource_ref.workspace_id != workspace
            {
                return Err(Error::Invalid("change outside binding"));
            }
            let payload_hash = hash(&serde_json::to_vec(c)?);
            if c.sequence <= checkpoint {
                let existing:Option<String>=sqlx::query_scalar("SELECT payload_hash FROM library_pm_events WHERE repo_id=? AND generation=? AND sequence=? AND event_id=?")
                    .bind(repo).bind(generation).bind(c.sequence).bind(&c.event_id).fetch_optional(&mut *tx).await?;
                if existing.as_deref() != Some(&payload_hash) {
                    return Err(Error::Conflict("sequence replay differs"));
                }
                continue;
            }
            if c.sequence != checkpoint + 1 {
                return Err(Error::Conflict("sequence gap"));
            }
            apply_change(&mut tx, repo, c).await?;
            sqlx::query("INSERT INTO library_pm_events(repo_id,generation,sequence,event_id,payload_hash) VALUES(?,?,?,?,?)")
                .bind(repo).bind(generation).bind(c.sequence).bind(&c.event_id).bind(payload_hash).execute(&mut *tx).await?;
            checkpoint = c.sequence;
        }
        sqlx::query("UPDATE library_pm_bindings SET checkpoint=?,cursor_value=? WHERE repo_id=?")
            .bind(checkpoint).bind(&page.next_cursor).bind(repo).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(checkpoint)
    }
    /// Explicit generation rollover retains stable resource IDs and local notes.
    pub async fn begin_resync(
        &self,
        repo: &str,
        generation: u64,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let current:u64=sqlx::query_scalar("SELECT generation FROM library_pm_bindings WHERE repo_id=? FOR UPDATE").bind(repo).fetch_one(&mut *tx).await?;
        if generation <= current {
            return Err(Error::Conflict("resync generation must advance"));
        }
        sqlx::query("UPDATE library_pm_bindings SET generation=?,checkpoint=0,cursor_value=NULL WHERE repo_id=?").bind(generation).bind(repo).execute(&mut *tx).await?;
        sqlx::query("UPDATE library_pm_records SET projection=NULL,lifecycle='inaccessible',observed_revision=0 WHERE repo_id=?").bind(repo).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM library_pm_relations WHERE repo_id=?")
            .bind(repo)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn list(
        &self,
        repo: &str,
        after: Option<&str>,
        limit: u32,
    ) -> Result<ProjectionList> {
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid("limit must be 1..100"));
        }
        let rows=sqlx::query("SELECT * FROM library_pm_records WHERE repo_id=? AND projection IS NOT NULL AND id>? ORDER BY id LIMIT ?")
            .bind(repo).bind(after.unwrap_or("")).bind(limit+1).fetch_all(&self.pool).await?;
        let has_more = rows.len() > limit as usize;
        let mut records = Vec::new();
        for r in rows.into_iter().take(limit as usize) {
            records.push(ProjectionRecord {
                id: r.try_get("id")?,
                resource_ref: serde_json::from_value(
                    r.try_get("resource_ref")?,
                )?,
                projection: serde_json::from_value(
                    r.try_get("projection")?,
                )?,
                observed_at: r.try_get("observed_at")?,
                observed_revision: r.try_get("observed_revision")?,
                lifecycle: r.try_get("lifecycle")?,
                note: r.try_get("local_note")?,
                document_ids: serde_json::from_value(
                    r.try_get("document_ids")?,
                )?,
                legacy_data_id: r.try_get("legacy_data_id")?,
            });
        }
        // Only resolved edges with visible ends are exposed. Missing targets remain
        // durably pending, including when a later page carries the target.
        let edges=sqlx::query("SELECT e.relation_value FROM library_pm_relations e JOIN library_pm_records s ON s.repo_id=e.repo_id AND s.resource_key=e.source_key JOIN library_pm_records t ON t.repo_id=e.repo_id AND t.resource_key=e.target_key WHERE e.repo_id=? AND s.projection IS NOT NULL AND t.projection IS NOT NULL")
            .bind(repo).fetch_all(&self.pool).await?;
        let relations = edges
            .into_iter()
            .map(|r| {
                Ok(serde_json::from_value(r.try_get("relation_value")?)?)
            })
            .collect::<Result<Vec<Relation>>>()?;
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM library_pm_relations WHERE repo_id=?",
        )
        .bind(repo)
        .fetch_one(&self.pool)
        .await?;
        let pending_relations =
            (total as usize).saturating_sub(relations.len());
        let next_after = if has_more {
            records.last().map(|r| r.id.clone())
        } else {
            None
        };
        Ok(ProjectionList {
            sync_pending: false,
            records,
            relations,
            pending_relations,
            next_after,
        })
    }
    /// Complete snapshots replace only provider-owned fields. Run identity and
    /// sequence must be checked by the caller on both sides of snapshot fetch.
    pub async fn replace_snapshot(
        &self,
        repo: &str,
        generation: u64,
        sequence: u64,
        nodes: Vec<Projection>,
        edges: Vec<wire::WireRelation>,
    ) -> Result<()> {
        use std::collections::BTreeSet;
        let mut refs = BTreeSet::new();
        for node in &nodes {
            node.resource_ref.validate()?;
            if !refs.insert(node.resource_ref.clone()) {
                return Err(Error::Invalid("duplicate snapshot node"));
            }
        }
        for edge in &edges {
            if edge.kind != "initiative_project"
                || !refs.contains(&edge.source)
                || !refs.contains(&edge.target)
                || edge.source.kind != "initiative"
                || edge.target.kind != "project"
            {
                return Err(Error::Invalid("invalid snapshot edge"));
            }
        }
        let mut tx = self.pool.begin().await?;
        let binding=sqlx::query("SELECT generation,checkpoint,workspace_id FROM library_pm_bindings WHERE repo_id=? FOR UPDATE").bind(repo).fetch_one(&mut *tx).await?;
        let current_generation = binding.try_get::<u64, _>("generation")?;
        let current_sequence = binding.try_get::<u64, _>("checkpoint")?;
        if generation < current_generation
            || (generation == current_generation
                && sequence < current_sequence)
        {
            return Err(Error::Conflict("stale snapshot"));
        }
        let workspace: String = binding.try_get("workspace_id")?;
        if nodes
            .iter()
            .any(|n| n.resource_ref.workspace_id != workspace)
        {
            return Err(Error::Invalid("snapshot outside workspace"));
        }
        sqlx::query("UPDATE library_pm_records SET projection=NULL,lifecycle='inaccessible',observed_revision=0 WHERE repo_id=?").bind(repo).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM library_pm_relations WHERE repo_id=?")
            .bind(repo)
            .execute(&mut *tx)
            .await?;
        // Snapshot may start at sequence zero; revision 1 only affects local
        // stale-event comparisons until the next observed revision arrives.
        for node in nodes {
            let change = Change {
                subscription_id: String::new(),
                generation,
                sequence,
                event_id: String::new(),
                resource_ref: node.resource_ref.clone(),
                observed_revision: 1,
                operation: if node.archived {
                    Operation::Archived
                } else {
                    Operation::Upsert
                },
                projection: Some(node),
                relation: None,
                schema_version: 1,
                observed_at: chrono::Utc::now().to_rfc3339(),
            };
            apply_change(&mut tx, repo, &change).await?;
        }
        for edge in edges {
            let relation = Relation {
                source: edge.source,
                target: edge.target,
                removed: false,
            };
            sqlx::query("INSERT INTO library_pm_relations(repo_id,source_key,target_key,relation_value) VALUES(?,?,?,?)").bind(repo).bind(relation.source.key()).bind(relation.target.key()).bind(serde_json::to_value(relation)?).execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE library_pm_bindings SET generation=?,checkpoint=?,cursor_value=NULL WHERE repo_id=?").bind(generation).bind(sequence).bind(repo).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    /// Inventory uses explicit external IDs only; unknown workspace provenance
    /// remains quarantined for an operator rather than name/URL matching.
    pub async fn legacy_inventory(
        &self,
        repo: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let rows=sqlx::query("SELECT s.data_id,s.external_id,e.id AS endpoint_id,e.status FROM sync_states s JOIN webhook_endpoints e ON e.id=s.endpoint_id WHERE e.repository_id=? AND e.provider='linear' ORDER BY s.external_id LIMIT 1001").bind(repo).fetch_all(&self.pool).await?;
        if rows.len() > 1000 {
            return Err(Error::Invalid(
                "legacy inventory exceeds limit; narrow repository scope",
            ));
        }
        rows.into_iter().map(|r|Ok(serde_json::json!({"data_id":r.try_get::<String,_>("data_id")?,"external_id":r.try_get::<String,_>("external_id")?,"endpoint_id":r.try_get::<String,_>("endpoint_id")?,"status":r.try_get::<String,_>("status")?,"decision":"quarantined","reason":"workspace provenance requires explicit verification; no automatic merge"}))).collect()
    }
    pub async fn record_local_context(
        &self,
        repo: &str,
        id: &str,
        note: &str,
        documents: &[String],
    ) -> Result<()> {
        if note.len() > 100_000 || documents.len() > 100 {
            return Err(Error::Invalid("local context too large"));
        }
        let result=sqlx::query("UPDATE library_pm_records SET local_note=?,document_ids=? WHERE repo_id=? AND id=?")
            .bind(note).bind(serde_json::to_value(documents)?).bind(repo).bind(id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(Error::Conflict(
                "projection not found or unchanged",
            ));
        }
        Ok(())
    }
    pub async fn record(
        &self,
        repo: &str,
        id: &str,
    ) -> Result<ProjectionRecord> {
        let r=sqlx::query("SELECT * FROM library_pm_records WHERE repo_id=? AND id=? AND projection IS NOT NULL").bind(repo).bind(id).fetch_one(&self.pool).await?;
        Ok(ProjectionRecord {
            id: r.try_get("id")?,
            resource_ref: serde_json::from_value(
                r.try_get("resource_ref")?,
            )?,
            projection: serde_json::from_value(r.try_get("projection")?)?,
            observed_at: r.try_get("observed_at")?,
            observed_revision: r.try_get("observed_revision")?,
            lifecycle: r.try_get("lifecycle")?,
            note: r.try_get("local_note")?,
            document_ids: serde_json::from_value(
                r.try_get("document_ids")?,
            )?,
            legacy_data_id: r.try_get("legacy_data_id")?,
        })
    }
    pub async fn resolve(
        &self,
        repo: &str,
        reference: &ResourceRef,
    ) -> Result<Option<String>> {
        reference.validate()?;
        Ok(sqlx::query_scalar("SELECT id FROM library_pm_records WHERE repo_id=? AND resource_key=? AND projection IS NOT NULL")
            .bind(repo).bind(reference.key()).fetch_optional(&self.pool).await?)
    }
    pub async fn publish(
        &self,
        repo: &str,
        release: DocumentRelease,
    ) -> Result<DocumentRelease> {
        sqlx::query("INSERT IGNORE INTO library_document_releases(repo_id,id,data_id,source_version,title,body,published_by,published_at) VALUES(?,?,?,?,?,?,?,?)")
            .bind(repo).bind(&release.id).bind(&release.data_id).bind(&release.source_version).bind(&release.title).bind(&release.body).bind(&release.published_by).bind(&release.published_at).execute(&self.pool).await?;
        let id:String=sqlx::query_scalar("SELECT id FROM library_document_releases WHERE repo_id=? AND data_id=? AND source_version=?")
            .bind(repo).bind(&release.data_id).bind(&release.source_version).fetch_one(&self.pool).await?;
        let saved = self.release(repo, &id).await?;
        if saved.body != release.body || saved.title != release.title {
            return Err(Error::Conflict(
                "source changed without version advancement",
            ));
        }
        Ok(saved)
    }
    pub async fn release(
        &self,
        repo: &str,
        id: &str,
    ) -> Result<DocumentRelease> {
        let r=sqlx::query("SELECT * FROM library_document_releases WHERE repo_id=? AND id=?").bind(repo).bind(id).fetch_one(&self.pool).await?;
        Ok(DocumentRelease {
            id: r.try_get("id")?,
            data_id: r.try_get("data_id")?,
            source_version: r.try_get("source_version")?,
            title: r.try_get("title")?,
            body: r.try_get("body")?,
            published_by: r.try_get("published_by")?,
            published_at: r.try_get("published_at")?,
            revoked: r.try_get("revoked")?,
        })
    }
    pub async fn release_candidates(
        &self,
        repo: &str,
        after: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ReleaseSummary>> {
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid("limit must be 1..100"));
        }
        let rows=sqlx::query("SELECT id,data_id,title,source_version,published_at,revoked FROM library_document_releases WHERE repo_id=? AND revoked=FALSE AND id>? ORDER BY id LIMIT ?").bind(repo).bind(after.unwrap_or("")).bind(limit+1).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|r| {
                Ok(ReleaseSummary {
                    id: r.try_get("id")?,
                    data_id: r.try_get("data_id")?,
                    title: r.try_get("title")?,
                    source_version: r.try_get("source_version")?,
                    published_at: r.try_get("published_at")?,
                    revoked: r.try_get("revoked")?,
                })
            })
            .collect()
    }
    pub async fn releases_for_document(
        &self,
        repo: &str,
        data_id: &str,
    ) -> Result<Vec<DocumentRelease>> {
        let ids:Vec<String>=sqlx::query_scalar("SELECT id FROM library_document_releases WHERE repo_id=? AND data_id=? ORDER BY published_at DESC LIMIT 100").bind(repo).bind(data_id).fetch_all(&self.pool).await?;
        let mut releases = Vec::new();
        for id in ids {
            releases.push(self.release(repo, &id).await?);
        }
        Ok(releases)
    }
    pub async fn revoke_release(&self, repo: &str, id: &str) -> Result<()> {
        sqlx::query("UPDATE library_document_releases SET revoked=TRUE WHERE repo_id=? AND id=?").bind(repo).bind(id).execute(&self.pool).await?;
        Ok(())
    }
}
async fn apply_change(
    tx: &mut Transaction<'_, MySql>,
    repo: &str,
    c: &Change,
) -> Result<()> {
    if let Operation::AccessRevoked = c.operation {
        sqlx::query("UPDATE library_pm_records SET projection=NULL,lifecycle='inaccessible' WHERE repo_id=?").bind(repo).execute(&mut **tx).await?;
        return Ok(());
    }
    if let Operation::RelationChange = c.operation {
        let relation = c.relation.as_ref().expect("validated relation");
        if relation.removed {
            sqlx::query("DELETE FROM library_pm_relations WHERE repo_id=? AND source_key=? AND target_key=?").bind(repo).bind(relation.source.key()).bind(relation.target.key()).execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO library_pm_relations(repo_id,source_key,target_key,relation_value) VALUES(?,?,?,?) ON DUPLICATE KEY UPDATE relation_value=VALUES(relation_value)")
                .bind(repo).bind(relation.source.key()).bind(relation.target.key()).bind(serde_json::to_value(relation)?).execute(&mut **tx).await?;
        }
        return Ok(());
    }
    let old:Option<u64>=sqlx::query_scalar("SELECT observed_revision FROM library_pm_records WHERE repo_id=? AND resource_key=?").bind(repo).bind(c.resource_ref.key()).fetch_optional(&mut **tx).await?;
    if old.is_some_and(|r| r >= c.observed_revision) {
        return Ok(());
    }
    if let Some(p) = &c.projection {
        if p.resource_ref != c.resource_ref {
            return Err(Error::Invalid("projection identity mismatch"));
        }
    }
    let (projection, state) = match c.operation {
        Operation::Upsert => {
            (Some(serde_json::to_value(&c.projection)?), "active")
        }
        Operation::Archived => {
            (Some(serde_json::to_value(&c.projection)?), "archived")
        }
        Operation::Deleted => (None, "deleted"),
        _ => (None, "inaccessible"),
    };
    sqlx::query("INSERT INTO library_pm_records(repo_id,resource_key,id,resource_ref,projection,observed_at,observed_revision,lifecycle,local_note,document_ids) VALUES(?,?,?,?,?,?,?,?,'',JSON_ARRAY()) ON DUPLICATE KEY UPDATE projection=VALUES(projection),observed_at=VALUES(observed_at),observed_revision=VALUES(observed_revision),lifecycle=VALUES(lifecycle)")
        .bind(repo).bind(c.resource_ref.key()).bind(format!("pm_{}",uuid::Uuid::new_v4().simple())).bind(serde_json::to_value(&c.resource_ref)?).bind(projection).bind(&c.observed_at).bind(c.observed_revision).bind(state).execute(&mut **tx).await?;
    Ok(())
}
