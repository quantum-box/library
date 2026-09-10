# MCP 連携仕様

対象: Library の org / repo / Data / Property / Source を MCP client から読み書きするための JSON-RPC endpoint。

Claude Code / Codex 向けの接続設定とスキルは [Library plugin](../../../plugins/library/README.md) として同梱している。

基本ワークフローの対応範囲と監査結果は [MCPツール対応範囲](mcp-coverage.md) を参照。所属org discovery、private Dataの認可、型付き編集を同じ接続で扱う。

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
| `MCP_AUTHORIZATION_SERVER` | 設定するとTachyon OAuth検証を有効化する。信頼するHTTPS issuerを一つ指定。未設定時は従来のLibrary OAuth facade |
| `MCP_AUTHORIZATION_SERVERS` | 上記の別名（優先）。現在は一つのissuerのみ対応。複数・空値は認証を拒否する |
| `MCP_OAUTH_JWKS_URL` | 外部モードで必須。信頼するTachyon discoveryの `jwks_uri` を管理者が確認して指定するHTTPS URL |
| `MCP_SCOPES_SUPPORTED` | 旧モードのscopes。外部モードは `openid,profile,email,mcp:read,mcp:write` を固定で案内する |
| `MCP_OAUTH_ISSUER` | Library MCP OAuth facade の issuer。未指定時は `{LIBRARY_API_BASE_URL}/mcp/oauth` |
| `MCP_COGNITO_CLIENT_ID` | MCP OAuth facade が Cognito `USER_PASSWORD_AUTH` に使う client id。`COGNITO_CLIENT_ID` / `VITE_COGNITO_CLIENT_ID` も fallback として読む |
| `MCP_COGNITO_CLIENT_SECRET` | Cognito client secret。未指定時は `SECRET_HASH` を送らない。`COGNITO_CLIENT_SECRET` も fallback として読む。frontend に公開される `VITE_*` からは読まない |
| `MCP_COGNITO_REGION` | Cognito region。未指定時は `ap-northeast-1` |
| `LIBRARY_MCP_SSE_ENABLED` | `true` の場合のみ SSE transport の route を登録する。既定 off |
| `MCP_SSE_MESSAGE_ENDPOINT` | SSE transport の `endpoint` event が返す post 先。未指定時は相対パス `/messages` |

### Tachyonへの切替

外部モードではTachyon発行access tokenのRS256署名、kid、issuer、exp、nbf、Libraryのresource URLと一致するaudienceを検証する。同じtokenをSDKでも検証し、返されたユーザーIDとsubが一致することを確認する。その後は従来の組織所属とデータアクセス権を適用する。APIキーの組織指定・ポリシー検証は維持する。

`mcp:read` は読取ツール、`mcp:write` は変更ツールに対応する。一方のscopeからもう一方を推定しない。scope不足のツールは一覧から除外し、直接呼出しは `403` と `insufficient_scope` challengeを返す。`openid` / `profile` / `email` は Tachyon 側の同意・ID 用途であり、Library MCP のツール認可には使わない。

JWKSは最大5分キャッシュする。未知のkidや期限切れキャッシュの取得失敗は認証を拒否するため、鍵ローテーション時は新しい公開鍵を5分以上前に公開する。tokenヘッダーのjku/x5uやJWKS HTTPリダイレクトは使用しない。

切替手順:

1. Tachyonのform/public client対応をデプロイし、Library resource向けaudienceの発行・refresh時の維持を実装・検証する。現在確認済みのTachyonコードはaudにclient_idを設定しており、そのtokenはLibraryの外部モードでは拒否される。
2. Previewで `MCP_AUTHORIZATION_SERVER`、`MCP_OAUTH_JWKS_URL`、正規の `MCP_RESOURCE_URL` とmetadata URLを設定する。issuer/JWKS/resourceはHTTPSを必須とする。
3. 実MCPクライアントで再登録・再認可し、読取・変更・scope不足・別resource・権限外組織の動作を確認する。Tachyon側の複数インスタンスをまたぐ認可コード・callback/consent replay防止も別途確認する。
4. 本番設定を切り替える。外部モードではLibraryの旧discoveryと `/mcp/oauth/register`・`authorize`・`token` は410になる。旧Cognito tokenは新しい検証条件を満たさず拒否されるため、再認可が必要。
5. 本番の接続確認後に旧OAuth実装を削除する。切戻しは外部モードの環境変数を解除するが、旧プロセス内登録情報は復元されないため再登録する。

以下は未切替環境にのみ残す従来経路である。

Library MCP OAuth facade は Dynamic Client Registration を受け付け、`/mcp/oauth/authorize` で Library login form を出す。入力された credential は Cognito `USER_PASSWORD_AUTH` で検証し、token endpoint は Cognito の実 access token を MCP client に返す。

## 3. 対応メソッド

### `initialize`

MCP server 情報と tools capability を返す。

### `tools/list`

匿名接続のカタログにも表示されるread tools（`get_me` / `list_orgs` / `search_repos`の実行は認証必須）:

- `get_me`
- `list_orgs`
- `get_org`
- `list_repos`
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
- `rename_repo`
- `delete_repo`
- `create_data`
- `update_data`
- `upsert_data`
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

public repo、または呼び出し元にread権限があるprivate repoのData一覧を返す。

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

read権限のあるrepo内でData名の完全一致検索を行う。空の`query`は一覧取得。全文検索ではない。

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

read権限のあるDataをMarkdownと型付き`property_data`で取得する。`url`と文字列`record_version`も返す。既存の`id` / `title` / `markdown`は維持する。

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

`value_type` は省略時 `string`。対応値は `string`, `integer`, `html`, `markdown`, `rich_text`, `relation`, `select`, `multi_select`, `date`, `image`, `boolean`, `id`, `location`。locationは`{"latitude":35.0,"longitude":139.0}`形式。自動生成Idは変更不可。

### Data update / upsert

- `update_data`: 必須`org`, `repo`, `data_id`, `name`。`property_data`に指定したPropertyだけを更新し、他の値は保持する。
- `upsert_data`: 同じ引数で、指定した有効な`data_id`のレコードを作成または更新する。戻り値の`outcome`は`created` / `updated`。同じIDへの再試行で別レコードを作らないが、再書き込みや同時更新の競合を防ぐものではない。
- write後のData結果にも型付き値・URL・revisionを含む。`record_version`は保存済みの版番号を参考情報として返す。現行MCP CRUDはlegacy経路で、この番号を増加させない。変更の検知・競合確認には使えず、条件付き更新の引数もない。変更内容は再取得して確認する。

### Organization tools

- `get_me`: 認証されたuser / service accountのID・種別・名前。引数不要。認証必須。
- `list_orgs`: 検証済み所属先のうちLibraryに登録済みのorg一覧。任意`page` / `page_size`。API keyは発行orgのみ。認証必須。
- `get_org`: organization と配下repoを取得する。必須`org`。匿名・非メンバーには公開repoのみ。
- `create_org`: organization を作成する。必須 `name`, `username`。任意 `description`, `website`。
- `update_org`: organizationをpatchする。必須`org`。省略した`name` / `description` / `website`は保持する。説明・URLの明示nullはクリア。

### Repository tools

- `list_repos`: orgのrepo一覧。必須`org`、任意`page` / `page_size`。匿名・非メンバーには公開repoのみ。
- `search_repos`: 所属org内でrepo名を部分一致検索する。認証と`org`必須。任意`query`, `limit`（1〜100）。
- `get_repo`: `org`, `repo` で repo 詳細を取得する。
- `create_repo`: repo を作成する。`org`, `name`, `username`, `is_public`, `description`, `skip_sample_data`。
- `update_repo`: repo 設定を更新する。`name`, `description`, `is_public`, `tags` を変更可能。
- `rename_repo`: repoのslugを変更する。必須`org`, `repo`, `new_username`。
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
