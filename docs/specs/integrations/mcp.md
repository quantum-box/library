# MCP 連携仕様

対象: Library の public repo 内の Data を MCP client から参照するための HTTP JSON-RPC endpoint。

## 1. Endpoint

- `POST /mcp`
- Transport: HTTP JSON-RPC 2.0
- 認証: Bearer token optional
- 匿名アクセス範囲: public repo の Data のみ
- 認証後アクセス範囲: 既存 usecase / policy check が許可する Data 操作

private repo は既存の `ViewDataList` / `SearchData` / `ViewData` usecase 側の権限判定で拒否する。MCP endpoint 独自の bypass は持たない。

## 2. OAuth / Protected Resource Metadata

MCP client が認証へ進めるように、次の discovery endpoint を公開する。

- `GET /.well-known/oauth-protected-resource`
- `GET /.well-known/oauth-protected-resource/mcp`

未認証で保護された tool を呼んだ場合、または `MCP_AUTH_REQUIRED=true` の場合、`POST /mcp` は `401 Unauthorized` と次の `WWW-Authenticate` challenge を返す。

```http
WWW-Authenticate: Bearer resource_metadata="https://{host}/.well-known/oauth-protected-resource"
```

設定:

| 環境変数 | 用途 |
| --- | --- |
| `MCP_AUTH_REQUIRED` | `true` / `1` の場合、`initialize` / `tools/list` を含む MCP endpoint 全体で認証を要求する |
| `MCP_RESOURCE_URL` | protected resource metadata の `resource`。未指定時は `{LIBRARY_API_BASE_URL}/mcp` |
| `MCP_RESOURCE_METADATA_URL` | `WWW-Authenticate` に載せる metadata URL。未指定時は `{LIBRARY_API_BASE_URL}/.well-known/oauth-protected-resource` |
| `MCP_AUTHORIZATION_SERVER` | OAuth authorization server issuer。未指定時は Library MCP OAuth facade (`{LIBRARY_API_BASE_URL}/mcp/oauth`) |
| `MCP_AUTHORIZATION_SERVERS` | 複数 issuer を comma-separated で指定する場合 |
| `MCP_SCOPES_SUPPORTED` | protected resource metadata / authorization server metadata に載せる scopes。未指定時は `openid,email,profile` |
| `MCP_OAUTH_ISSUER` | Library MCP OAuth facade の issuer。未指定時は `{LIBRARY_API_BASE_URL}/mcp/oauth` |
| `MCP_COGNITO_CLIENT_ID` | MCP OAuth facade が Cognito `USER_PASSWORD_AUTH` に使う client id。`COGNITO_CLIENT_ID` / `VITE_COGNITO_CLIENT_ID` も fallback として読む |
| `MCP_COGNITO_CLIENT_SECRET` | Cognito client secret。未指定時は `SECRET_HASH` を送らない。`COGNITO_CLIENT_SECRET` も fallback として読む。frontend に公開される `VITE_*` からは読まない |
| `MCP_COGNITO_REGION` | Cognito region。未指定時は `ap-northeast-1` |

Library MCP OAuth facade は Dynamic Client Registration を受け付け、`/mcp/oauth/authorize` で Library login form を出す。入力された credential は Cognito `USER_PASSWORD_AUTH` で検証し、token endpoint は Cognito の実 access token を MCP client に返す。

## 3. 対応メソッド

### `initialize`

MCP server 情報と tools capability を返す。

### `tools/list`

匿名でも利用できる tools:

- `search_repos`
- `get_repo`
- `list_data`
- `search_data`
- `get_data`
- `list_properties`
- `list_sources`
- `get_source`

認証後に追加される write tools:

- `create_repo`
- `update_repo`
- `delete_repo`
- `create_data`
- `update_data`
- `delete_data`
- `create_property`
- `update_property`
- `delete_property`
- `create_source`
- `update_source`
- `delete_source`

### `tools/call`

`params.name` で tool 名を指定し、`params.arguments` に tool ごとの入力を渡す。

## 4. Tools

### `list_data`

public repo の Data 一覧を返す。

入力:

```json
{
  "org": "org-slug",
  "repo": "repo-slug",
  "page": 1,
  "page_size": 20
}
```

### `search_data`

public repo 内の Data を検索する。

入力:

```json
{
  "org": "org-slug",
  "repo": "repo-slug",
  "query": "keyword",
  "page": 1,
  "page_size": 20
}
```

### `get_data`

Data を Markdown として取得する。

入力:

```json
{
  "org": "org-slug",
  "repo": "repo-slug",
  "data_id": "data_xxx"
}
```

### `create_data`

Data を作成する。認証必須。

入力:

```json
{
  "org": "org-slug",
  "repo": "repo-slug",
  "name": "New data",
  "property_data": [
    {
      "property_id": "prop_xxx",
      "value": "body",
      "value_type": "markdown"
    }
  ]
}
```

`value_type` は省略時 `string`。対応値は `string`, `integer`, `html`, `markdown`, `relation`, `select`, `multi_select`, `date`, `image`。

### Repository tools

- `search_repos`: repo を検索する。`org`, `query`, `limit` を指定可能。
- `get_repo`: `org`, `repo` で repo 詳細を取得する。
- `create_repo`: repo を作成する。`org`, `name`, `username`, `is_public`, `description`, `skip_sample_data`。
- `update_repo`: repo 設定を更新する。`name`, `description`, `is_public`, `tags` を変更可能。
- `delete_repo`: repo を削除する。

### Property tools

- `list_properties`: repo の properties を取得する。
- `create_property`: property を作成する。`name`, `property_type`, `meta` を指定可能。
- `update_property`: property を更新する。`name`, `property_type`, `meta` を指定可能。
- `delete_property`: property を削除する。

`property_type` は `string`, `integer`, `html`, `markdown`, `relation`, `select`, `multi_select`, `id`, `location`, `date`, `image`。

### Source tools

- `list_sources`: repo の sources を取得する。
- `get_source`: source を取得する。
- `create_source`: source を作成する。`name`, `url` を指定可能。
- `update_source`: source を更新する。`url: null` で URL を解除できる。
- `delete_source`: source を削除する。

## 5. curl / CLI 検証例

```bash
curl -sS http://localhost:50055/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list"
  }'
```

```bash
curl -sS http://localhost:50055/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "list_data",
      "arguments": {
        "org": "org-slug",
        "repo": "repo-slug"
      }
    }
  }'
```

OAuth protected resource metadata:

```bash
curl -sS http://localhost:50055/.well-known/oauth-protected-resource
```

OAuth authorization server metadata:

```bash
curl -sS http://localhost:50055/.well-known/oauth-authorization-server/mcp/oauth
```

認証 challenge:

```bash
MCP_AUTH_REQUIRED=true curl -i -sS http://localhost:50055/mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list"
  }'
```

Inspector CLI:

```bash
npx @modelcontextprotocol/inspector --cli \
  http://localhost:50055/mcp \
  --transport http \
  --method tools/list \
  --header "Authorization: Bearer ${TOKEN}"
```

## 6. 実装参照

- Handler: `apps/api/src/handler/mcp.rs`
- Route: `apps/api/src/router.rs`
- Markdown 生成: `apps/api/src/usecase/markdown_composer.rs`
- Public docs endpoint: `apps/api/src/handler/docs.rs`
