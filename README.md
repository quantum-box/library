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

前提: Rust は `rust-toolchain.toml` の `nightly-2026-06-04` に固定。Node は 20 以上、パッケージマネージャは Yarn 4（`apps/client` のみ npm）。

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
