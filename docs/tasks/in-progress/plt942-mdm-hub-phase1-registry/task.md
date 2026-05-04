# PLT-942 Library MDM Hub Phase 1 (Registry): bakuure SKU → Global ID

- Status: in-progress
- Owner: leader-plt942-lib (PdM-Platform)
- Branch: `feature/plt-942-mdm-hub-phase1`
- Linear: PLT-942
- Deadline: 2026-05-07 (GW明け)

## 背景・価値
ADR-0001 (Library MDM Hub) Phase 1 = Registry スタイル。bakuure / 将来の会計・倉庫
システム等が持つ独自ID (SKU 等) を、Library が発行する `global_id` で名寄せする
最初の橋頭堡。Phase 2 (Co-existence) への足場でもある。

primary-source:
- `~/knowledge/src/projects/library/decisions/ADR-0001-library-as-mdm-hub.md`
- `~/knowledge/src/projects/library/overview.md` §MDM Hub 構想

## スコープ (Phase 1)
1. `library.global_id_mapping` table 追加 (sqlx migration)
2. CRUD API (GraphQL + REST)
3. tenant scope 分離 (`tenant_id` column + 全クエリ scope 制限)
4. (Phase 1.5 / 余裕あれば) Library Web UI MDM ページ + DuckDB-WASM 重複検知

非スコープ: Yjs CRDT 同期 / Survivorship Rule / bakuure からの双方向書き込み (Phase 2 以降)

## 設計

### Schema (MariaDB / MySQL)

```sql
CREATE TABLE `global_id_mapping` (
    `id`            VARCHAR(32)  NOT NULL COMMENT 'Mapping ID (ULID, prefix gim_)',
    `tenant_id`     VARCHAR(29)  NOT NULL COMMENT 'Tenant ID (tn_) — scope isolation',
    `global_id`     VARCHAR(64)  NOT NULL COMMENT 'Library-issued global ID (prefix gid_)',
    `system`        VARCHAR(64)  NOT NULL COMMENT 'Source system name (bakuure / tws / ...)',
    `system_code`   VARCHAR(255) NOT NULL COMMENT 'Code in source system (e.g. BWS-001)',
    `name`          VARCHAR(255) NOT NULL COMMENT 'Display name',
    `created_at`    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    `updated_at`    TIMESTAMP    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (`id`),
    UNIQUE KEY `uk_tenant_global_id`   (`tenant_id`, `global_id`),
    UNIQUE KEY `uk_tenant_system_code` (`tenant_id`, `system`, `system_code`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='Phase 1 Registry: external system code → Library global_id (PLT-942 / ADR-0001)';
```

unique 設計の根拠:
- `(tenant_id, global_id)` → Library 内で global_id は tenant scope で一意
- `(tenant_id, system, system_code)` → 同一 system の同一 code が二重登録されない
  (= 名寄せの一貫性保証)
- 単一 column INDEX は付けない: 全クエリは `WHERE tenant_id = ?` 固定で、上記 2 つの
  unique key (どちらも leading column が `tenant_id`) で覆える
- tenant 単位 scope なので、別 tenant が同じ `system_code` を持つことは許可

注意 (advisor 指摘):
- **`global_id` uniqueness は tenant-scoped** に意図的に設計。ADR の「Library 内で
  名寄せ」を Phase 1 では tenant 内一意と解釈する (異 tenant 間で同じ `gid_xxx` が
  別エンティティを指す可能性あり)。Phase 2 で再評価する余地あり。bakuure / PdM-Product
  には schema 共有時にこの解釈を明示する。
- **`system` は MariaDB 予約語**。SQL は backtick quoting で逃げる。sqlx の
  `query_as!` macro が field 名で衝突したら domain 側 field を `source_system` に rename
  する fallback あり (ただし Rust 側の field 名は `system` でも OK、衝突は SQL 側)。

### ID 戦略
- `id` (PK): ULID with prefix `gim_`
- `global_id`: ULID with prefix `gid_` (input 省略時は server auto-generate)
- 既存 def_id! macro 利用 (`SourceId` と同じパターン)

### GraphQL schema 追加

```graphql
type GlobalIdMapping {
  id: String!
  globalId: String!
  system: String!
  systemCode: String!
  name: String!
  tenantId: String!
  createdAt: String!
  updatedAt: String!
}

input CreateGlobalIdMappingInput {
  globalId: String       # optional; server-generated if omitted
  system: String!
  systemCode: String!
  name: String!
}

input UpdateGlobalIdMappingInput {
  id: String!
  name: String           # only mutable field in Phase 1
}
# Note: `system` / `system_code` / `global_id` は post-create immutable.
# 訂正は「delete + recreate」で行う。これは名寄せの一貫性を保つための制約で、
# Phase 2 で再評価する。

extend type Query {
  globalIdMapping(system: String!, systemCode: String!): GlobalIdMapping
  globalIdMappings(system: String): [GlobalIdMapping!]!
}

extend type Mutation {
  createGlobalIdMapping(input: CreateGlobalIdMappingInput!): GlobalIdMapping!
  updateGlobalIdMapping(input: UpdateGlobalIdMappingInput!): GlobalIdMapping!
}
```

### REST endpoint (bakuure SDK 用、Phase 1 はこの 1 本のみ)

```
GET  /v1beta/global-id-mapping?system=bakuure&code=BWS-001
     → 200 GlobalIdMappingResponse | 404
```

tenant scope は `x-operator-id` header (= TenantId) から決定。
auth は既存 `LibraryExecutor` extractor に従い、未認証は permission_denied。

REST は **bakuure SDK lookup 用の最小 1 本だけ** に絞る (advisor 指摘)。
Create / Update / 一覧は全て GraphQL 経由。bakuure 側で別 REST が必要になったら
Phase 1.5 以降で追加する。

### tenant scope 分離方針
- 全 SQL に `WHERE tenant_id = ?` を強制
- `LibraryMultiTenancy.get_operator_id()` を usecase に必ず渡す
- Repository trait で `tenant_id: &TenantId` を全 method の必須引数にする
- 異 tenant の id を直接 GET しても `WHERE tenant_id = ? AND id = ?` で
  hit せず `not_found` 返却 (information leak しない)

### File layout (mirror existing source CRUD)
- `apps/api/migrations/20260427000000_create_global_id_mapping.{up,down}.sql`
- `apps/api/src/domain/global_id_mapping.rs`
- `apps/api/src/domain/mod.rs` に `pub mod global_id_mapping; pub use ...;`
- `apps/api/src/interface_adapter/gateway/global_id_mapping_repository.rs`
- `apps/api/src/usecase/{create,update,get,find}_global_id_mapping.rs`
- `apps/api/src/handler/graphql/input/global_id_mapping.rs`
- `apps/api/src/handler/graphql/model/global_id_mapping.rs`
- `apps/api/src/handler/graphql/resolver.rs` (Query 拡張)
- `apps/api/src/handler/graphql/mutation.rs` (Mutation 拡張)
- `apps/api/src/handler/global_id_mapping.rs` (REST)
- `apps/api/src/handler/types.rs` (Request/Response 追加)
- `apps/api/src/handler/openapi.rs` (登録)
- `apps/api/src/app.rs` (DI wiring)

## マイルストーン
- M1 (4/28): primary-source 読解 + schema 設計確定 + taskdoc commit ← **当タスク**
- M2 (4/30): migration + GraphQL CRUD 実装 + PR draft
- M3 (5/2):  REST endpoint + tenant scope 分離 verify + PR ready
- M4 (5/7):  CI green + admin merge + prod deploy + bakuure 側 integration

## 完了条件
1. migration 適用 → `global_id_mapping` table 存在確認
2. GraphQL + REST endpoint curl + playwright 動作確認
3. tenant scope 分離 verify (異 tenant のデータが見えないこと)
4. PR 起票 + CI 全 SUCCESS + admin merge
5. Library API 本番 deploy 成功
6. bakuure 側 (PdM-Product 担当) との integration 動作確認
7. Linear comment + state/coo-state.md 報告

## bakuure 側との連携
PdM-Product が `quantum-box/bakuure` repo で同 issue PLT-942 並行進行
(REST endpoint client + UI)。schema 確定 (= M1 完了) 時点で Linear comment 共有。
