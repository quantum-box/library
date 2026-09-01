# MCP 連携仕様

対象: Library の org / repo / Data / Property / Source を MCP client から読み書きするための JSON-RPC endpoint。

## 1. Endpoint

2 種類の transport を提供する。どちらも同じ tool set・同じ認証規則で動く。

| Transport | Endpoint | 状態 | 用途 |
| --- | --- | --- | --- |
| HTTP (Streamable) | `POST /mcp` | GA | 1 往復で完結する client |
| HTTP + SSE | `GET /sse` + `POST /messages` | **Non-GA / 既定 off** | 長寿命の event stream を前提とする旧来の client |

共通の性質:

- Protocol: JSON-RPC 2.0
- 認証: Bearer token optional
- 匿名アクセス範囲: public repo の Data のみ
- 認証後アクセス範囲: 既存 usecase / policy check が許可する Data 操作

private repo は既存の `ViewDataList` / `SearchData` / `ViewData` usecase 側の権限判定で拒否する。MCP endpoint 独自の bypass は持たない。

### SSE transport

#### GA status

1. 判断: **Non-GA / experimental**。
2. 標準環境では `/sse` `/mcp/sse` `/messages` `/mcp/messages` を router に登録しない。
3. 常駐 process を持つ環境でのみ `LIBRARY_MCP_SSE_ENABLED=true` を明示して有効化する。
4. `POST /mcp` は無条件に登録される。1 往復で完結する MCP client はすべてそちらで足りる。

**現行の Lambda 配信では動作しない。** Lambda の実行環境は 1 インスタンスにつき同時 1 リクエストであり、`GET /sse` の stream を保持しているインスタンスはその間占有される。したがって `POST /messages` は必ず別インスタンスに振られ、そこには session table も stream も無い。応答の受け渡しが成立しない。

Lambda の `InvokeMode` を `RESPONSE_STREAM` にすれば byte は流れるようになるが、この配送問題は解決しない。tachyon-apps の ADR-0008 も、常時接続 origin は Lambda に載せず Cloudflare Durable Objects や常駐 compute で扱うと決めている。

GA に上げる場合の完了条件:

1. session と stream が同一 process に着地することが保証される配信形態を用意する（Durable Object のような session id で名前解決できる常駐 origin）。
2. その配信経路で `GET /sse` → `POST /messages` の往復を実機で検証する。
3. 複数 client の同時接続と再接続を検証する。

#### 仕組み

Streamable HTTP より前の MCP client は、event stream を開いてから request を別 endpoint に送る 2 endpoint 構成を期待する。

1. client が `GET /sse` を開く
2. server が `endpoint` event で post 先 URL (`/messages?sessionId=…`) を返す
3. client は JSON-RPC request を `POST /messages?sessionId=…` に送る
4. response は POST の body ではなく `GET /sse` の stream に流れる

route は `/sse` と `/mcp/sse`、`/messages` と `/mcp/messages` の両方を受ける。client によって MCP server の base path の解釈が違うため、どちらの綴りでも到達できる。

`endpoint` event が返す URL は既定で相対パスなので、client は自分が開いた SSE URL を基準に解決する。stream と messages を別ホストで終端する構成では `MCP_SSE_MESSAGE_ENDPOINT` に絶対 URL を設定する。

**認証**: stream を開いたときの `Authorization` header を session に保持する。`GET /sse` でだけ認証し、以降は素の request を post する client がそのまま認証状態を保てる。`POST /messages` 側に header があればそちらが優先される。

同時に開ける session 数は 1024 で頭打ちにしてある。session table は process global で、`MCP_AUTH_REQUIRED` が off なら無認証で到達できるため。stream が切れた session は登録から外れる。

両 transport は `dispatch_rpc` を共有して認証と実行を行う。どの tool に credential が要るかが transport 間でずれることはない。

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
| `LIBRARY_MCP_SSE_ENABLED` | `true` の場合のみ SSE transport の route を登録する。既定 off |
| `MCP_SSE_MESSAGE_ENDPOINT` | SSE transport の `endpoint` event が返す post 先。未指定時は相対パス `/messages` |

Library MCP OAuth facade は Dynamic Client Registration を受け付け、`/mcp/oauth/authorize` で Library login form を出す。入力された credential は Cognito `USER_PASSWORD_AUTH` で検証し、token endpoint は Cognito の実 access token を MCP client に返す。

## 3. 対応メソッド

### `initialize`

MCP server 情報と tools capability を返す。

### `tools/list`

匿名でも利用できる tools:

- `get_org`
- `search_repos`
- `get_repo`
- `list_data`
- `search_data`
- `get_data`
- `list_properties`
- `get_property`
- `list_sources`
- `get_source`

認証後に追加される write tools:

- `create_org`
- `update_org`
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

`value_type` は省略時 `string`。対応値は `string`, `integer`, `html`, `markdown`, `rich_text`, `relation`, `select`, `multi_select`, `date`, `image`, `boolean`。

### Organization tools

- `get_org`: organization と配下 repo を取得する。必須 `org`。
- `create_org`: organization を作成する。必須 `name`, `username`。任意 `description`, `website`。
- `update_org`: organization を更新する。必須 `org`, `name`。`description` / `website` は省略すると空になるので、残す値は毎回送る。

### Repository tools

- `search_repos`: repo を検索する。`org`, `query`, `limit` を指定可能。
- `get_repo`: `org`, `repo` で repo 詳細を取得する。
- `create_repo`: repo を作成する。`org`, `name`, `username`, `is_public`, `description`, `skip_sample_data`。
- `update_repo`: repo 設定を更新する。`name`, `description`, `is_public`, `tags` を変更可能。
- `delete_repo`: repo を削除する。

### Property tools

- `list_properties`: repo の properties を取得する。
- `get_property`: property を 1 件取得する。必須 `org`, `repo`, `property_id`。type と meta を含む。
- `create_property`: property を作成する。`name`, `property_type`, `meta` を指定可能。
- `update_property`: property を更新する。`name`, `property_type`, `meta` を指定可能。
- `delete_property`: property を削除する。

`property_type` は `string`, `integer`, `html`, `markdown`, `relation`, `select`, `multi_select`, `id`, `location`, `date`, `image`, `rich_text`, `boolean`。`html` は `rich_text` に置き換えられた旧型。

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

SSE transport。`LIBRARY_MCP_SSE_ENABLED=true` で起動した環境でのみ到達できる。stream を開くと最初に `endpoint` event が届く。

```bash
curl -N -sS http://localhost:50055/sse
```

```
event: endpoint
data: /messages?sessionId=0d6f…
```

その `sessionId` に request を post する。response は POST の body ではなく、開いたままの stream 側に出る。

```bash
curl -sS 'http://localhost:50055/messages?sessionId=0d6f…' \
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

```bash
npx @modelcontextprotocol/inspector --cli \
  http://localhost:50055/sse \
  --transport sse \
  --method tools/list \
  --header "Authorization: Bearer ${TOKEN}"
```

### Library CLI

`library mcp` は client を用意せずに server を叩ける。詳細は [Library CLI 仕様](cli.md)。

```bash
library mcp tools
library mcp call list_data --arg org=acme --arg repo=docs
```

client 設定はそのまま出力できる。`--transport` で `http` / `sse` を選ぶ。

```bash
library mcp config --transport sse
```

## 6. 実装参照

- Handler: `apps/api/src/handler/mcp.rs`
- SSE transport: `apps/api/src/handler/mcp_sse.rs`
- Route: `apps/api/src/router.rs`
- CLI: `apps/cli/src/commands/mcp.rs`
- Markdown 生成: `apps/api/src/usecase/markdown_composer.rs`
- Public docs endpoint: `apps/api/src/handler/docs.rs`
