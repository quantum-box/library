# CLAUDE.md

## 呼称 / Naming

- **library v2 (v2)** = `apps/client` の Tauri + React クライアント（コード内の別名は "Photon"）。
  desktop / web / mobile シェルはすべてこのアプリを指す。
- **library web (v1)** = `apps/web` の Next.js アプリ。
- **library-api** = `apps/api` の Rust (axum) API。v1 / v2 の共通バックエンド。
- **library CLI** = `apps/cli` の Rust CLI。バイナリ名は `library`。library-api の REST / GraphQL / MCP を叩く。

ユーザーが「v2」「tauri app」と言った場合は `apps/client` のこと。
