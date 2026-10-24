use crate::{migration::*, store::Store, *};
use sqlx::Row;
impl Store {
    pub async fn review_adoption(
        &self,
        repo: &str,
        actor: &str,
        request: Request,
        mut entries: Vec<ReviewedEntry>,
    ) -> Result<Plan> {
        request.validate()?;
        if entries.len() != request.entries.len() {
            return Err(Error::Invalid("reviewed entry count mismatch"));
        }
        let binding = self.binding(repo).await?;
        if request.workspace_id != binding.workspace_id {
            return Err(Error::Invalid("binding workspace differs"));
        }
        let legacy_rows=sqlx::query("SELECT s.data_id,s.external_id FROM sync_states s JOIN webhook_endpoints w ON w.id=s.endpoint_id WHERE w.repository_id=? AND w.tenant_id=? AND w.provider='linear'").bind(repo).bind(&binding.tenant_id).fetch_all(&self.pool).await?;
        let mut legacy_ids = std::collections::BTreeSet::new();
        let mut external_ids = std::collections::BTreeSet::new();
        for row in legacy_rows {
            let data: String = row.try_get("data_id")?;
            let external: String = row.try_get("external_id")?;
            if !legacy_ids.insert(data) || !external_ids.insert(external) {
                return Err(Error::Invalid(
                    "duplicate or ambiguous legacy mapping",
                ));
            }
        }
        let reviewed_ids = entries
            .iter()
            .map(|e| e.entry.legacy_data_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        if legacy_ids != reviewed_ids {
            return Err(Error::Invalid("review must cover every legacy Linear record; retire issue copies explicitly"));
        }
        for reviewed in &mut entries {
            let e = &reviewed.entry;
            let proofs=sqlx::query("SELECT s.metadata FROM sync_states s JOIN webhook_endpoints w ON w.id=s.endpoint_id WHERE w.repository_id=? AND w.tenant_id=? AND w.provider='linear' AND w.id=? AND s.data_id=? AND s.external_id=?")
                .bind(repo).bind(&binding.tenant_id).bind(&e.endpoint_id).bind(&e.legacy_data_id).bind(format!("linear:{}:{}",e.resource_ref.kind,e.resource_ref.external_id)).fetch_all(&self.pool).await?;
            if proofs.len() != 1 {
                return Err(Error::Invalid(
                    "legacy identity proof missing or ambiguous",
                ));
            }
            let metadata: Option<serde_json::Value> =
                proofs[0].try_get("metadata")?;
            verify_workspace(
                e,
                metadata
                    .as_ref()
                    .and_then(|m| m.get("workspace_id"))
                    .and_then(serde_json::Value::as_str),
            )?;
            let original:Option<String>=sqlx::query_scalar("SELECT id FROM library_pm_records WHERE repo_id=? AND resource_key=?").bind(repo).bind(e.resource_ref.key()).fetch_optional(&self.pool).await?;
            reviewed.original_projection_id = if e.retire_only {
                String::new()
            } else {
                original.ok_or(Error::Invalid("resource is not in authorized projection; synchronize first"))?
            };
            let collision:Option<String>=sqlx::query_scalar("SELECT resource_key FROM library_pm_records WHERE repo_id=? AND id=?").bind(repo).bind(&e.legacy_data_id).fetch_optional(&self.pool).await?;
            if collision.is_some_and(|key| key != e.resource_ref.key()) {
                return Err(Error::Conflict(
                    "legacy ID belongs to another projection",
                ));
            }
        }
        let id = format!("pmm_{}", uuid::Uuid::new_v4().simple());
        let digest =
            hash(&serde_json::to_vec(&(repo, actor, &request, &entries))?);
        let plan = Plan {
            id,
            repo_id: repo.into(),
            reviewed_by: actor.into(),
            digest,
            status: "reviewed".into(),
            request,
            entries,
        };
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO library_pm_migration_plans(id,repo_id,digest,reviewed_by,plan_json,status) VALUES(?,?,?,?,?,'reviewed')")
            .bind(&plan.id).bind(repo).bind(&plan.digest).bind(actor).bind(serde_json::to_value(&plan)?).execute(&mut *tx).await?;
        for e in &plan.entries {
            sqlx::query("INSERT INTO library_pm_migration_items(plan_id,resource_key,legacy_data_id,original_projection_id,operation_id) VALUES(?,?,?,?,?)")
            .bind(&plan.id).bind(e.entry.resource_ref.key()).bind(&e.entry.legacy_data_id).bind(&e.original_projection_id).bind(&e.operation_id).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(plan)
    }
    pub async fn adoption_plan(
        &self,
        repo: &str,
        id: &str,
    ) -> Result<Plan> {
        let row=sqlx::query("SELECT plan_json,status FROM library_pm_migration_plans WHERE repo_id=? AND id=?").bind(repo).bind(id).fetch_one(&self.pool).await?;
        let mut plan: Plan =
            serde_json::from_value(row.try_get("plan_json")?)?;
        plan.status = row.try_get("status")?;
        Ok(plan)
    }
    pub async fn begin_adoption(
        &self,
        repo: &str,
        id: &str,
        actor: &str,
        digest: &str,
    ) -> Result<Plan> {
        let plan = self.adoption_plan(repo, id).await?;
        if plan.reviewed_by != actor || plan.digest != digest {
            return Err(Error::Invalid("reviewed actor or digest differs"));
        }
        if !["reviewed", "applying", "applied"]
            .contains(&plan.status.as_str())
        {
            return Err(Error::Conflict("plan is not applicable"));
        }
        if plan.status == "applied" {
            return Ok(plan);
        }
        self.pause_projection(repo).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT writer FROM library_pm_writer_fences WHERE repo_id=? FOR UPDATE").bind(repo).fetch_one(&mut *tx).await?;
        sqlx::query("SELECT repo_id FROM library_pm_bindings WHERE repo_id=? FOR UPDATE").bind(repo).fetch_one(&mut *tx).await?;
        let current: String = sqlx::query_scalar("SELECT status FROM library_pm_migration_plans WHERE repo_id=? AND id=? FOR UPDATE").bind(repo).bind(id).fetch_one(&mut *tx).await?;
        if !["reviewed", "applying"].contains(&current.as_str()) {
            return Err(Error::Conflict(
                "migration changed while waiting for fence",
            ));
        }
        let other:i64=sqlx::query_scalar("SELECT COUNT(*) FROM library_pm_migration_plans WHERE repo_id=? AND status='applying' AND id<>?").bind(repo).bind(id).fetch_one(&mut *tx).await?;
        if other > 0 {
            return Err(Error::Conflict("another migration is applying"));
        }
        // Review can precede draining the old worker. Recheck its entire
        // inventory while holding the same fence used by legacy writes.
        let current_ids: Vec<String> = sqlx::query_scalar("SELECT s.data_id FROM sync_states s JOIN webhook_endpoints w ON w.id=s.endpoint_id WHERE w.repository_id=? AND w.tenant_id=(SELECT tenant_id FROM library_pm_bindings WHERE repo_id=?) AND w.provider='linear'").bind(repo).bind(repo).fetch_all(&mut *tx).await?;
        let actual = current_ids
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let reviewed = plan
            .entries
            .iter()
            .map(|e| e.entry.legacy_data_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        if actual.len() != current_ids.len() || actual != reviewed {
            return Err(Error::Conflict(
                "legacy inventory changed after review",
            ));
        }
        sqlx::query("UPDATE library_pm_migration_plans SET status='applying' WHERE id=?").bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE library_pm_writer_fences SET writer='paused' WHERE repo_id=?").bind(repo).execute(&mut *tx).await?;
        sqlx::query("UPDATE library_pm_bindings SET writer='paused' WHERE repo_id=?").bind(repo).execute(&mut *tx).await?;
        tx.commit().await?;
        self.adoption_plan(repo, id).await
    }
    pub async fn adopt_scrubbed_record(
        &self,
        repo: &str,
        plan_id: &str,
        reference: &ResourceRef,
        accepted_version: &str,
    ) -> Result<()> {
        let plan = self.adoption_plan(repo, plan_id).await?;
        if plan.status != "applying" {
            return Err(Error::Conflict("plan is not applying"));
        }
        let mut tx = self.pool.begin().await?;
        let status:String=sqlx::query_scalar("SELECT status FROM library_pm_migration_plans WHERE id=? AND repo_id=? FOR UPDATE").bind(plan_id).bind(repo).fetch_one(&mut *tx).await?;
        if status != "applying" {
            return Err(Error::Conflict("migration has stopped"));
        }
        let item=sqlx::query("SELECT * FROM library_pm_migration_items WHERE plan_id=? AND resource_key=? FOR UPDATE").bind(plan_id).bind(reference.key()).fetch_one(&mut *tx).await?;
        let legacy: String = item.try_get("legacy_data_id")?;
        let original: String = item.try_get("original_projection_id")?;
        if !original.is_empty() {
            let current:String=sqlx::query_scalar("SELECT id FROM library_pm_records WHERE repo_id=? AND resource_key=? FOR UPDATE").bind(repo).bind(reference.key()).fetch_one(&mut *tx).await?;
            if current != legacy && current != original {
                return Err(Error::Conflict(
                    "projection identity changed after review",
                ));
            }
            sqlx::query("UPDATE library_pm_records SET id=?,legacy_data_id=? WHERE repo_id=? AND resource_key=?").bind(&legacy).bind(&legacy).bind(repo).bind(reference.key()).execute(&mut *tx).await?;
        }
        // A frozen document may contain the old provider-owned fields. Do not
        // leave that historical snapshot readable through the release API.
        sqlx::query("UPDATE library_document_releases SET revoked=TRUE WHERE repo_id=? AND data_id=?").bind(repo).bind(&legacy).execute(&mut *tx).await?;
        sqlx::query("UPDATE library_pm_migration_items SET status='adopted',accepted_version=? WHERE plan_id=? AND resource_key=?").bind(accepted_version).bind(plan_id).bind(reference.key()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn finish_adoption(
        &self,
        repo: &str,
        id: &str,
    ) -> Result<Plan> {
        let mut tx = self.pool.begin().await?;
        let status: String = sqlx::query_scalar("SELECT status FROM library_pm_migration_plans WHERE id=? AND repo_id=? FOR UPDATE").bind(id).bind(repo).fetch_one(&mut *tx).await?;
        if status != "applying" && status != "applied" {
            return Err(Error::Conflict("migration is not applying"));
        }
        let remaining:i64=sqlx::query_scalar("SELECT COUNT(*) FROM library_pm_migration_items WHERE plan_id=? AND status<>'adopted'").bind(id).fetch_one(&mut *tx).await?;
        if remaining > 0 {
            return Err(Error::Conflict(
                "migration still has uncommitted records",
            ));
        }
        sqlx::query("UPDATE library_pm_migration_plans SET status='applied' WHERE id=? AND repo_id=?").bind(id).bind(repo).execute(&mut *tx).await?;
        tx.commit().await?;
        self.adoption_plan(repo, id).await
    }
    pub async fn rollback_adoption(
        &self,
        repo: &str,
        id: &str,
    ) -> Result<Plan> {
        let plan = self.adoption_plan(repo, id).await?;
        if !["applying", "applied", "rolled_back"]
            .contains(&plan.status.as_str())
        {
            return Err(Error::Conflict("plan has not been applied"));
        }
        if plan.status == "rolled_back" {
            return Ok(plan);
        }
        self.pause_projection(repo).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT status FROM library_pm_migration_plans WHERE id=? AND repo_id=? FOR UPDATE").bind(id).bind(repo).fetch_one(&mut *tx).await?;
        for e in &plan.entries {
            if e.original_projection_id.is_empty() {
                continue;
            }
            sqlx::query("UPDATE library_pm_records SET id=?,legacy_data_id=NULL WHERE repo_id=? AND resource_key=? AND id=?").bind(&e.original_projection_id).bind(repo).bind(e.entry.resource_ref.key()).bind(&e.entry.legacy_data_id).execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE library_pm_migration_plans SET status='rolled_back' WHERE id=? AND repo_id=?").bind(id).bind(repo).execute(&mut *tx).await?;
        tx.commit().await?;
        self.adoption_plan(repo, id).await
    }
}
