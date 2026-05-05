# ADR-0004: Library/bakuure stub の責務境界

## Status

Accepted (2026-05-05)

## Context

Library は、人類レベルの知識 DB / CMS / Document OS / MDM Hub として育てる。PLT-942 では Phase 1 Registry として `global_id_mapping` を導入し、bakuure の SKU など外部システムの局所 ID を Library の `global_id` に対応付ける。

bakuure は販売領域 ERP として、商品、価格、在庫、受注、出荷、決済、会計連携などの transactional truth を持つ。一方で、Library は商品や業務概念を横断的に名寄せし、文書、関係、外部システム ID の接続点になる。bakuure が Library の内部データ構造へ直接依存すると、Library の schema 変更が ERP の実行時安定性に波及し、両者の bounded context が崩れる。

PLT-990 では、bakuure context が Library を参照する場合の stub 責務、bakuure 側に保持する ID / version / relation、ACL と同期方式を確定する。

## Decision

bakuure は Library を外部 context として扱い、bakuure 内には `LibraryKnowledgeStub` 相当の anti-corruption layer を置く。stub は Library API / event / cache の差異を吸収し、bakuure domain model に Library の内部表現を流入させない。

責務境界は以下とする。

- Library は `global_id`, `system`, `system_code`, `name`, relation metadata, schema/relation version を所有する。
- bakuure は商品、価格、在庫、受注、発注、出荷、決済など業務トランザクションの truth を所有する。
- bakuure が保持してよい Library 由来データは、参照用 DTO / snapshot に限定する。
- bakuure は Library DB に直接接続しない。参照は Library API 経由、または後続 Phase の Library event 経由に限定する。
- Phase 1 では `GET /v1beta/global-id-mapping?system=bakuure&code=...` を read-through lookup として使う。作成/更新は Library 側 GraphQL / 管理導線で行う。
- Phase 1 の ACL は bakuure service account に `library:ReadGlobalIdMapping` のみを付与する。`CreateGlobalIdMapping` / `UpdateGlobalIdMapping` は Library 管理者操作に限定する。
- bakuure 側 cache は optional とし、cache entry には `global_id`, `system`, `system_code`, `display_name`, `schema_version`, `relation_version`, `fetched_at` を保存する。cache は read availability のためであり、Library の truth を上書きしない。
- Library event 受信は Phase 2 以降とする。event contract が確定するまでは、bakuure は API lookup + TTL cache のみで連携する。

Library が一時的に利用できない場合、bakuure は既存 cache を参照表示に使ってよい。ただし、新規 mapping の自動作成、既存 mapping の補正、relation 更新は行わない。

stub interface の draft は以下を基準にする。実装言語や crate 名は bakuure 側で調整してよいが、domain model はこの DTO を越えて Library 内部型を参照しない。

```rust
pub struct LibraryKnowledgeRef {
    pub global_id: String,
    pub source_system: String,
    pub source_code: String,
    pub display_name: String,
    pub schema_version: Option<String>,
    pub relation_version: Option<String>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait::async_trait]
pub trait LibraryKnowledgeStub {
    async fn find_by_source_code(
        &self,
        tenant_id: &TenantId,
        source_system: &str,
        source_code: &str,
    ) -> Result<Option<LibraryKnowledgeRef>, LibraryStubError>;
}
```

## Consequences

### Positive

- Library の内部 schema 変更が bakuure domain model に直接波及しない。
- bakuure は transactional truth を保ちつつ、Library の MDM Hub 化に段階的に乗れる。
- Phase 1 は read-only 連携で開始でき、ACL と障害時挙動を小さく保てる。
- event 連携を急がず、API lookup の実運用を観察してから Phase 2 に進める。

### Negative

- bakuure 側に stub / cache / version handling の実装コストが発生する。
- Phase 1 では Library 側の relation 更新が bakuure に即時反映されない。
- Library unavailable 時の cache 利用により、表示データが一時的に古くなる可能性がある。

### Neutral

- bakuure の商品 ID / SKU / order ID などは Shared Kernel に移さない。
- Library の `global_id` は Phase 1 では tenant-scoped とし、cross-tenant global uniqueness は Phase 2 で再評価する。
- Library の relation model や document body format は、この ADR では固定しない。

## Alternatives Considered

### bakuure から Library DB を直接参照する

最短で実装できるが、Library の schema 変更が bakuure の production path を壊す。tenant scope / ACL / migration 順序も密結合になるため不採用。

### Library の full model を bakuure domain に取り込む

参照時の表現力は高いが、Library の文書・関係・同期モデルが ERP domain に流入する。bakuure の bounded context が肥大化するため不採用。

### 初期から event-driven sync を必須にする

整合性は取りやすいが、event contract、retry、idempotency、dead letter、replay の設計が必要になる。PLT-942 Phase 1 の小ささを失うため、Phase 2 以降に送る。

## Follow-up

- bakuure 側に `LibraryKnowledgeStub` trait と DTO を追加する。
- Library API response に version / relation metadata を含める必要があるかを PLT-942 後続で確認する。
- Library event contract を Phase 2 ADR で定義する。
- cache TTL / stale 許容時間を bakuure の画面別に決める。

## References

- [PLT-942 task](../../tasks/in-progress/plt942-mdm-hub-phase1-registry/task.md)
- [GlobalIdMapping domain](../../../apps/api/src/domain/global_id_mapping.rs)
- [GlobalIdMapping REST handler](../../../apps/api/src/handler/global_id_mapping.rs)
- [GlobalIdMapping migration](../../../apps/api/migrations/20260427000000_create_global_id_mapping.up.sql)
