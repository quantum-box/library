# 利用ユースケース シーケンス図（簡易）

この資料は [use-cases.md](/Users/takanorifukuyama/git/github.com/quantum-box/library/docs/specs/use-cases/use-cases.md) の UC-01〜UC-08 に対応します。

## UC-01 記事台帳構築（REST）

```mermaid
sequenceDiagram
  autonumber
  participant Author as コンテンツ担当
  participant API as REST API
  participant Store as Repository Store
  participant Docs as Public Docs
  Author->>API: POST /v1/repos (repo 作成)
  API->>Store: create repo
  Store-->>API: repo_id
  API-->>Author: 201 Created
  Author->>API: POST /v1/repos/{repo_id}/properties
  API->>Store: save property schema
  Author->>API: POST /v1/repos/{repo_id}/data
  API->>Store: save draft data
  Author->>API: PATCH /v1/data/{data_id} (is_public=true)
  API->>Store: publish
  Author->>Docs: GET /docs/{org}/{repo}
  Docs-->>Author: list visible data
```

## UC-02 多言語記事管理（REST）

```mermaid
sequenceDiagram
  autonumber
  participant Localizer as 翻訳担当
  participant API as REST API
  participant Search as Search/Index
  Localizer->>API: GET /v1/repos/{repo_id}/property-data
  API-->>Localizer: property list
  Localizer->>API: POST /v1/repos/{repo_id}/data (ja)
  Localizer->>API: POST /v1/repos/{repo_id}/data (en)
  API-->>Search: publish update event
  Search-->>API: indexed
  API->>Localizer: GET /v1/docs?search=language:ja
```

## UC-03 更新履歴運用（REST）

```mermaid
sequenceDiagram
  autonumber
  participant Writer as 執筆者
  participant API as REST API
  participant Store as Data Store
  Writer->>API: GET /v1/repos/{repo_id}/data/{data_id}
  API-->>Writer: current payload + updated_at
  Writer->>API: PATCH /v1/data/{data_id}
  API->>Store: update with optimistic check
  Store-->>API: diff + version
  API->>Writer: 200 + updated version
  Writer->>API: GET /docs/{org}/{repo}/{data_id}/md
  API-->>Writer: published body
```

## UC-04 API仕様ナレッジ化（REST + Docs）

```mermaid
sequenceDiagram
  autonumber
  participant Editor as ドキュメント編集者
  participant API as REST API
  participant Docs as Docs Router
  Editor->>API: POST /v1/repos (カテゴリ作成)
  API-->>Editor: repo_id
  Editor->>API: POST /v1/repos/{repo_id}/data
  API->>Docs: sync content
  Docs-->>API: publish result
  Editor->>API: POST /v1/repos/{repo_id}/search-index
  API->>Docs: refresh index
  Docs-->>Editor: index updated
```

## UC-05 エディタ横断統合（REST + GraphQL）

```mermaid
sequenceDiagram
  autonumber
  participant Operator as 運用担当
  participant GraphQL as GraphQL API
  participant REST as REST API
  participant Vault as Metadata Store
  Operator->>GraphQL: query repos + properties
  GraphQL->>Vault: fetch metadata
  GraphQL-->>Operator: plan
  Operator->>REST: POST /v1/import/github
  REST->>Vault: upsert property_data
  REST-->>Operator: import result
  Operator->>REST: GET /v1/repos/{repo_id}/data-list
  REST-->>Operator: total_count + list
```

## UC-06 外部イベント自動更新（Webhook）

```mermaid
sequenceDiagram
  autonumber
  participant Source as 外部システム
  participant Callback as Webhook受信
  participant API as Webhook API
  participant Queue as Retry Queue
  Source->>Callback: POST /webhooks/events
  Callback->>API: verify signature
  API-->>Callback: ok / queued_unverified
  alt success
    Callback->>Callback: idempotency check
    Callback->>Queue: enqueue apply job
    Queue->>API: apply update Data
    API-->>Queue: done
  else duplicate or invalid
    Callback->>Queue: record failure and skip
  end
```

## UC-07 接続管理一元化（GraphQL）

```mermaid
sequenceDiagram
  autonumber
  participant Admin as 管理者
  participant GQL as GraphQL API
  participant Auth as OAuth Provider
  participant Sync as Sync Engine
  Admin->>GQL: mutation init_oauth
  GQL-->>Admin: oauth_url
  Admin->>Auth: authorize
  Auth-->>GQL: auth_code
  GQL->>GQL: mutation exchange_oauth_code
  GQL->>Sync: create sync_operation
  Sync-->>GQL: connection status
  Admin->>GQL: query connections
  GQL-->>Admin: status dashboard
```

## UC-08 同時編集（WebSocket）

```mermaid
sequenceDiagram
  autonumber
  participant EditorA as 編集者A
  participant EditorB as 編集者B
  participant WS as WebSocket
  participant Collab as Collaboration Service
  EditorA->>WS: connect /ws/collab/{document_key}
  WS->>Collab: join session
  EditorB->>WS: connect /ws/collab/{document_key}
  WS->>Collab: join session
  EditorA->>WS: binary patch delta
  WS->>EditorB: broadcast delta
  EditorB->>WS: binary patch delta
  WS->>EditorA: broadcast delta
  Collab->>EditorA: merged state ready
  Collab->>EditorB: merged state ready
```
