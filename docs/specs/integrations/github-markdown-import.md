# GitHub Markdown import / sync GA verification

最終更新: 2026-08-26

## 結論

GitHub Markdown import と**双方向継続同期**を GA として扱う（2026-08-26 変更。旧判断: one-shot import のみ GA）。

- **inbound (GitHub → Library)**: push webhook のみ。`POST /webhooks/github` を `x-hub-signature-256` で検証 → webhook event queue → `GitHubEventProcessor`（5秒 poll の `WebhookEventWorker`）→ Data upsert。
- **outbound (Library → GitHub)**: Data 保存時の自動 writeback。`ext_github.enabled=true` の Data を `AddData` / `UpdateData` port 経由で保存すると、`GithubWritebackDispatch` デコレータが markdown を合成し `ext_github.ref` ブランチへ push する。best-effort（失敗は warn のみで保存は成功）。手動経路として `syncDataToGithub` / `bulkSyncExtGithub` mutation も利用可。
- GitHub App installation（`completeGitHubInstall`）だけは引き続き Non-GA 相当: connection metadata を保存するのみで、installation token 発行は未実装。認証は OAuth App ベース。

## ループ防止

1. **構造的防止**: inbound webhook の Data upsert は `LibraryDataRepositoryImpl` 経由で `database_manager::App` に直接書き込み、`LibraryApp` の `AddData`/`UpdateData` port（デコレータの位置）を通らない。よって inbound 起点で outbound push は発火しない。
2. **エコー抑止**: outbound push 成功時に commit SHA を inbound `SyncState.external_version` に記録（`GithubWritebackDispatch::record_outbound_commit`）。`GitHubEventProcessor::process_added_or_modified` は `has_external_changed(push.after)` が false の commit を skip する（webhook 再配送の冪等性も兼ねる）。
3. `last_synced_sha` を `ext_github` property に書き戻すことは**しない**（Data 更新→デコレータ再発火の自己ループになるため）。

## ext_github wire 形式

```json
{
  "repo": "owner/repo",
  "path": "docs/article.md",
  "ref": "main",
  "enabled": true,
  "sync_to_github": true
}
```

- Rust 側の parse は `apps/api/src/usecase/ext_github_meta.rs` の `ExtGithubMeta` に集約。`enabled` / `sync_to_github` 欠落は false（default-deny）、`ref` 欠落は `"main"`。
- web 側は `ext-github-sync-policy.ts` の `normalizeExtGithubEditorState` が同じ default-deny / ref 既定を適用する。

## Primary-source 確認

| 対象 | 判定 | 確認元 |
| --- | --- | --- |
| OAuth URL / token exchange | GA | `apps/api/src/handler/graphql/mutation.rs` の `githubAuthUrl` / `githubExchangeToken`。OAuth state は HMAC 検証され、token は provider `github` として保存される |
| Directory / preview / frontmatter analyze | GA | `apps/api/src/usecase/list_github_directory.rs`、`apps/api/src/usecase/get_markdown_previews.rs` |
| Markdown import | GA | `apps/api/src/usecase/import_markdown_from_github.rs`。`enableGithubSync` 入力が `ext_github.enabled` / `sync_to_github` にそのまま反映される（デフォルト false） |
| 手動 writeback | GA | `apps/api/src/usecase/sync_data_to_github.rs`（単一）と `apps/api/src/usecase/bulk_sync_ext_github.rs`（一括）。どちらも `outbound_sync::SyncData` → `GitHubSyncProvider`（contents API、SHA conflict 処理付き） |
| 自動 writeback | GA | `apps/api/src/usecase/github_writeback.rs`。`apps/api/src/app.rs` で `AddData` / `UpdateData` port をラップ |
| inbound sync runtime | GA | `apps/api/src/router.rs` が `OAuthGitHubClient` / `DefaultGitHubDataHandler` を配線。webhook secret は `apps/api/src/sdk_auth.rs`（`github_webhook_secret`）→ `apps/api/src/bootstrap.rs` → `WebhookSecretStore` |
| marketplace readiness | GA | `inbound_sync/domain/src/provider.rs` で `Provider::Github` は `Ga` / runtime available。`builtin_integrations.rs` で `int_github` は `SyncCapability::Bidirectional` |
| GitHub App installation | Non-GA | `completeGitHubInstall` は installation_id を connection metadata に保存するのみ |

## 検証

秘密値、実 GitHub credentials、本番データは使用しない。

1. unit test: `ExtGithubMeta` の default-deny / ref 既定（`ext_github_meta.rs`）。
2. unit test: デコレータの skip 条件（ext_github 無し / enabled=false）と meta 抽出（`github_writeback.rs`）。
3. unit test: エコー抑止 — `external_version == push.after` で upsert されず skipped、SHA 変化で処理（`event_processor.rs`）。
4. unit test: webhook secret が `Provider::Github` として store に入る（`bootstrap.rs`）。
5. unit test: `github_import_metadata` の enable_sync true/false 両ケース（`import_markdown_from_github.rs`）。
6. web unit test: `normalizeExtGithubEditorState` の ref/default-deny（`ext-github-sync-policy.test.ts`）。

## 残リスク

1. 実 GitHub repository に対する end-to-end 同期は利用者 OAuth token が必要なため、この検証では実施していない。本番検証には検証用 GitHub account / repository と OAuth 接続、webhook endpoint 作成、および Tachyon 側 oauth-providers への `github` webhook_secret 登録が必要。
2. エコー抑止は push head SHA 比較のため、自 push 直後に第三者 commit が同一 push に混ざると fail-open で再 import される（安全側・収束する）。
3. 自動 writeback は inline await のため、GitHub API が遅い場合 Data 保存のレイテンシに乗る。問題になる場合は owned-type 化して `tokio::spawn` に移す。
