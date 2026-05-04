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
| `MCP_AUTHORIZATION_SERVER` | OAuth authorization server issuer |
| `MCP_AUTHORIZATION_SERVERS` | 複数 issuer を comma-separated で指定する場合 |

## 3. 対応メソッド

### `initialize`

MCP server 情報と tools capability を返す。

### `tools/list`

次の tools を返す。

- `list_data`
- `search_data`
- `get_data`

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

`value_type` は省略時 `string`。対応値は `string`, `integer`, `html`, `markdown`, `select`, `date`, `image`。

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
