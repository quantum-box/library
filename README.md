# Library

GitHub のようにデータを持ち、Notion のように編集する CMS / ドキュメント OS です。

データは organization / repository で区切り、repository の中に schema (Property) と実体 (Data) を置きます。public にした repository はサインインなしで読めます。

```text
Organization
└── Repository
    ├── Data      (page / structured row)
    ├── Property  (schema)
    └── Source    (外部参照)
```

## 構成

| ディレクトリ | 呼称 | 内容 | デプロイ先 |
| --- | --- | --- | --- |
| [`apps/api`](apps/api) | library-api | Rust (axum) API。REST / GraphQL / MCP を提供する v1 / v2 共通バックエンド | AWS Lambda |
| [`apps/client`](apps/client/README.md) | library v2 | React 19 + Tauri v2 のクライアント。desktop / web / mobile シェルはすべてこれ（コード内の別名は "Photon"） | Cloudflare Pages |
| [`apps/cli`](apps/cli/README.md) | library CLI | 端末と自動化から Library を操作する Rust CLI。バイナリ名は `library` | ローカルビルド |
| [`apps/web`](apps/web/README.md) | library web (v1) | Vite + React 18 の旧クライアント | Cloudflare Pages |
| [`apps/user-guide`](apps/user-guide/README.md) | — | 利用者向けドキュメントサイト。API の OpenAPI / GraphQL schema を読んで生成する | Cloudflare Pages |
| [`packages/`](packages) | — | database-manager をはじめとする Rust の共有クレート群 | — |
| [`sdk/rust`](sdk/rust/README.md) | — | tachyon-sdk の Rust API client | — |
| [`docs/specs`](docs/specs/index.md) | — | 仕様書 | — |

公開 URL は v2 クライアント (`planetlibrary`) に寄せています。API が返す `url` もそちらを指します。

## インターフェース

library-api は 4 つの入口を持ちます。

| | 用途 |
| --- | --- |
| REST (`/v1beta/…`) | 一般的な CRUD。OpenAPI を公開 |
| GraphQL (`/v1/graphql`) | クライアントが使う主経路 |
| MCP (`POST /mcp`) | LLM / agent から tool として呼ぶ |
| CLI (`library`) | 端末・CI・agent の shell から |

MCP と CLI はどちらも既存の usecase / policy check を通ります。認可の抜け道は持ちません。

## はじめに

前提: Rust は `rust-toolchain.toml` の `nightly-2026-06-04` に固定。Node は 24.22.1、パッケージマネージャは Yarn 4（`apps/client` のみ npm）。

```bash
# API
cargo build -p library-api

# CLI
cargo build -p library-cli --release

# v2 クライアント
cd apps/client && npm ci && npm run dev
```

API のローカル実行には MySQL が要ります。接続文字列は [`apps/api/.env.sample`](apps/api/.env.sample) を参照してください。

CLI をすぐ使う場合:

```bash
export LIBRARY_API_BASE_URL=https://library-api.txcloud.app
export LIBRARY_API_KEY=pk_xxx
library --json repo list <org>
```

詳細は [apps/cli/README.md](apps/cli/README.md)。

## Claude Code / Codex プラグイン

公開データの検索・要約や認証済みの更新操作を、HTTP MCPと共通スキルで利用できます。このリポジトリに両クライアントのmarketplace定義を同梱しています。インストール、認証、現行APIの制限は [Library plugin](plugins/library/README.md) を参照してください。

## 開発

ローカルでは変更した package に絞って確認し、workspace 全体は CI に任せます。

```bash
cargo fmt -p <package>
cargo clippy -p <package> --all-targets
cargo test -p <package>
```

GitHub Actions が `cargo fmt --all` / `cargo clippy --workspace` / `cargo check --workspace` / `cargo nextest run --workspace`、MySQL を立てた DB 回帰、クライアントの lint / test / build / E2E を回します。定義は [`.github/workflows/ci.yml`](.github/workflows/ci.yml)。

## ドキュメント

- [仕様インデックス](docs/specs/index.md)
- [REST API](docs/specs/apis/rest-api.md) / [GraphQL API](docs/specs/apis/graphql-api.md)
- [MCP 連携](docs/specs/integrations/mcp.md) / [CLI](docs/specs/integrations/cli.md)
- [利用ガイド](docs/specs/guides/usage-guide.md)
- [運用手順](docs/specs/integrations/operations.md) / [スモークテスト](docs/runbook-smoke-test.md)

## ライセンス

[LICENSE](LICENSE) を参照してください。

### Exact numeric properties

The v1 Property Type kernel also supports `decimal`, `quantity`, `formula`, and
`rollup`. All decimal values travel as **strings**, including REST, GraphQL,
MCP, and the client adapters. `precision` defaults to 28 and `scale` to 12;
exponents, floating JSON numbers, overflow, and excess scale are rejected.
`quantity` stores `{ "value": "150.5", "unit": "g" }` and preserves that unit.
Unit dictionary v1 supports `g`/`kg`, `mL`/`L`, and `count`; each property allows
units of one dimension only. No density or per-item mass is inferred.

Formula config contains a bounded, tagged expression AST and decimal result
config. The editors translate property keys, arithmetic, comparisons, `if`, and
`round` into that AST; references are persisted as stable Property IDs. Round
uses midpoint-away-from-zero. Results are read-only and distinguish
`ready`, `null`, `pending`, and `error`. Arithmetic never substitutes zero for
missing inputs. Cycles and nonnumeric dependencies are rejected. Results are
computed per request and are not persisted in the record or a shared cache.

Rollup config identifies a Relation Property, target Property, and
`count`/`sum`/`avg`/`min`/`max`. It uses the existing configured Data reader,
including its canonical/legacy rollout mode, rather than treating an empty
`relation_edges` table as an empty relationship. The target Repo is resolved in
the same tenant and authorized before reading values. Target IDs are deduplicated
and batch-read (maximum 256 per target database per response). A revision or
schema change during the read returns `pending`; forbidden or missing targets
return an explicit error. Count includes related records with unset values;
other aggregations skip unset values and return `null` for an empty input.
Nonterminating/excess-scale averages fail explicitly. Rollups can target numeric
inputs and same-record formulas; nested rollups are not supported.

Decimal and Quantity filters use exact numeric comparison. The search endpoint
accepts `sort=property:<key>` and `sort=property:<key>:desc`; unset values sort
last. Physical index projections and computed-value sorting/filtering remain
unsupported and their kernel capabilities explicitly reject declarations.

Existing properties are never silently converted. To preview a String/Integer
copy into a **new** Decimal property:

```sh
node scripts/library-decimal-migration.mjs --base-url=https://YOUR_API \
  --org=YOUR_ORG --repo=YOUR_REPO --source=PROPERTY_ID --target=cost_decimal
```

Provide `LIBRARY_TOKEN` through the environment. Review the preview and its
errors, then repeat with `--apply --expected-digest=PREVIEW_DIGEST`. The script
rechecks source values, preserves the original property, and deletes its new
property if a copy fails. Rollback is deletion of the new property; the source
is untouched. Numeric config narrowing is rejected without an explicit
migration. No schema DDL or RelationEdge/Index cutover is performed by this
feature. Operators must still complete the existing rollout gates before
changing those production read/write modes.
