# PLT-4553 Library MCP 32ツールの outputSchema

[Linear](https://linear.app/issue/PLT-4553) / [親タスク](../../../todo/plt-4532-library-plugin-marketplace-submit/task.md) / [提出資料](../../../../../../plugins/library/submission/README.md)

## 概要

OpenAI Plugins Directory の `Scan Tools` で、Library MCPの全32ツールに `outputSchema` がないという推奨が表示された。各ツールの実際の返却JSONと一致するschemaを `tools/list` に追加し、ChatGPT/Codexが結果を構造として解釈できるようにする。

## スコープ

- 認証済みカタログの全32ツールへMCP `outputSchema`を追加する
- 共通レスポンス構造を再利用し、tool名とschemaの対応漏れをテストする
- 実際の返却値とschemaの整合を代表ケースまたはschema検証で確認する
- OpenAI Platformで修正版を再scanし、推奨表示が消えることをデプロイ後ゲートとして残す

## 非スコープ

- toolの入力schema、操作内容、認可境界、OAuth scopeの変更
- APIレスポンスそのものの破壊的な形式変更
- このPR内での本番デプロイまたはOpenAI審査提出

## 完了条件

- [x] 全32ツールが空でない `outputSchema` を返す
- [x] schemaが各toolの成功時JSONと一致する
- [x] `list_share_links` がread-onlyかつnon-destructiveである
- [x] focused test、format、diff checkが成功する
- [x] デプロイ後のportal再scanを運用todoへ残す

## 実装

- tool名ごとの`outputSchema`対応を網羅matchにし、Organization、Repo、Data、Property、Source、共有リンク、paginatorの共通schemaを再利用する。
- 成功時JSONを`structuredContent`に返し、既存clientとの互換性のため同じ値を整形済みtext contentにも残す。
- 認証済み全32toolのschema対応漏れと、全toolの代表成功値がschemaに一致することを単体テストで固定する。
- portal scanで見つかった`list_share_links`のannotationをread-only / non-destructiveへ訂正する。

## 検証

- `cargo test -p library-api handler::mcp --lib`: 38 passed、0 failed（clean nightly target）
- `cargo check -p library-api --lib`: 成功（clean nightly target）
- `cargo clippy -p library-api --lib -- -D warnings -A clippy::double_must_use -A clippy::redundant_field_names`: 成功（clean nightly target）
- `cargo fmt --all -- --check`: 成功（nightly-2026-06-04）
- `git diff --check`: 成功

## デプロイ後ゲート

- OpenAI Platformの再scanは本番デプロイ前のため未実施。親提出taskで、32toolの`outputSchema`と修正版annotationsを再確認してから審査提出する。
