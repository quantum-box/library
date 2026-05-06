---
title: "PLT-1139 Library GA docs routes"
type: "feature"
emoji: "📚"
topics: ["library", "docs", "openapi", "auth"]
published: true
targetFiles:
  - "apps/api/src/handler/docs.rs"
  - "apps/api/src/handler/openapi.rs"
  - "apps/api/src/usecase/view_data.rs"
  - "apps/api/src/usecase/view_data_list.rs"
  - "apps/api/tests/scenarios/library_public_docs_routes.scenario.md"
github: "https://linear.app/quantum-box/issue/PLT-1139/"
---

# PLT-1139 Library GA docs routes

## 概要

Library の公開 Docs routes (`GET /docs/{org}/{repo}` など) を GA 品質に揃える。公開 repo は匿名アクセス可能、private repo は認証済み executor の通常権限でのみアクセス可能にし、OpenAPI と scenario を更新する。

## 背景・目的

`/docs/{org}/{repo}` 系は CMS / Document OS の公開入口になる。GA 前に公開/非公開の境界、Markdown/frontmatter の仕様、slug 運用、検索/埋め込み用途の前提を固定する。

## 詳細仕様

- `GET /docs/{org}/{repo}`: docs 一覧 HTML。公開 repo は匿名可、private repo は認証済み権限が必要。
- `GET /docs/{org}/{repo}/{data_id}`: 単一 docs HTML。Markdown から YAML frontmatter を除いた本文を HTML rendering。
- `GET /docs/{org}/{repo}/{data_id}/md`: YAML frontmatter 付き Markdown。検索/埋め込み/外部 indexing の正規入力。
- `data_id` が canonical URL key。`slug` は property として運用できるが routing key にはしない。
- private repo の permission failure は data 本体取得前に判定する。

## 実装方針

- docs handler に `LibraryExecutor` extractor を追加し、匿名/認証済みの両方を usecase に渡す。
- `extract_org_username` が `/docs/{org}/...` から org を抽出できるようにする。
- `ViewData` / `ViewDataList` の private repo 権限チェックを DB data 読み込み前に移動する。
- `#[utoipa::path]` と `openapi.rs` 登録で `/docs` routes を OpenAPI に含める。
- REST scenario で公開/非公開/404/Markdown をカバーする。

## タスク分解

### フェーズ1: 調査 ✅

- [x] 既存 `/docs` handler と router 登録を確認
- [x] `ViewData` / `ViewDataList` の公開判定を確認
- [x] OpenAPI 登録状況を確認

### フェーズ2: 実装 ✅

- [x] docs handler の optional auth handling を修正
- [x] private repo の permission check 順序を修正
- [x] OpenAPI に docs routes を登録
- [x] docs 仕様を更新
- [x] scenario を追加

### フェーズ3: 検証 ✅

- [x] `cargo fmt`
- [x] `cargo fmt --check`
- [x] `cargo check -p library-api`
- [x] `cargo clippy -p library-api -- -D warnings`
- [x] OpenAPI 生成
- [x] scenario 実行可否確認

### フェーズ4: PR 📝

- [ ] commit / push
- [ ] PR 作成
- [ ] Linear 更新

## テスト計画

- `cargo fmt`
- `cargo check -p library-api`
- `cargo run -p library-api --bin library_codegen`
- `cargo test -p library-api run_library_api_scenarios -- --nocapture` または対象 scenario の実行

検証結果:

- `cargo fmt`: 成功
- `RUSTUP_TOOLCHAIN=nightly cargo fmt --check`: 成功
- `cargo check -p library-api`: stable toolchain では既存 `packages/errors` の nightly feature で停止
- `RUSTUP_TOOLCHAIN=nightly cargo check -p library-api`: 成功
- `RUSTUP_TOOLCHAIN=nightly cargo clippy -p library-api -- -D warnings`: 成功
- `RUSTUP_TOOLCHAIN=nightly cargo test -p library-api handler::docs::tests -- --nocapture`: 成功
- `RUSTUP_TOOLCHAIN=nightly cargo run -p library-api --bin library_codegen`: 成功
- Scenario は追加済み。現在の checkout には scenario runner が起動前提にする `target/debug/tachyon-api` binary が存在しないため未実行。

## リスクと対策

- private docs で既存の匿名拒否挙動が変わる可能性がある。認証済み権限がある場合だけ許可し、匿名は引き続き拒否する。
- OpenAPI の `/docs` routes は `/v1beta` prefix 外の root routes なので、spec 上の path と runtime path が一致するよう明示登録する。
- slug property は運用上便利だが routing key にすると既存 data_id URL と衝突するため、このタスクでは data_id canonical に固定する。

## 完了条件

- `/docs` routes の auth/permission 境界が明確になっている。
- OpenAPI に `/docs/{org}/{repo}` / `{data_id}` / `{data_id}/md` が含まれる。
- scenario またはテストで主要ケースをカバーする。
- 仕様 docs に Markdown/frontmatter と slug 方針が反映されている。
- PR が作成されている。
