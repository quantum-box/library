# GitHub Markdown import / sync readiness

最終更新: 2026-08-26

## 結論

one-shot GitHub Markdown import のみを GA とする。**双方向継続同期は
Experimental** として再設計する（2026-09-11 変更）。

2026-08-26 に双方向継続同期を GA としたが、本番 `library-api` は Lambda で
`LIBRARY_WEBHOOK_WORKER_ENABLED` が設定されておらず、受信イベントは DB に
queued された後に処理されない。また、inbound の直接 Data 更新と generic Data
mutation からの inline writeback は ADR-0006 の ChangeSet / transactional outbox
境界に反する。接続 UI や queue 保存を同期成功の根拠にしない。

- **inbound (GitHub → Library)**: Experimental。push webhook を検証して DB queue に保存する実装と、常駐 process で queue を処理する実装はあるが、Lambda 本番で durable consumer invocation を保証していない。現行 processor は ChangeSet を通さず Data を直接 upsert/delete する。
- **outbound (Library → GitHub)**: Experimental。Data 保存時の inline best-effort writeback と手動 mutation はあるが、失敗状態を利用者に返さず、external revision を共通 base とする競合解決もない。
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
| Markdown import | GA (one-shot) | `enableGithubSync=false` のみ。import 後の継続同期は作らない |
| 手動 writeback | Experimental | contents API の SHA conflict 処理はあるが、Library revision と external revision の共通 base / 利用者向け状態がない |
| 自動 writeback | Experimental | inline best-effort であり ADR-0006 の禁止事項に該当する。移行対象 |
| inbound sync runtime | Experimental | handler と processor は配線済みだが、Lambda 本番に durable consumer がない。直接 Data 更新も移行対象 |
| marketplace readiness | Experimental | `LIBRARY_ENABLE_EXPERIMENTAL_INTEGRATIONS=true` の環境だけで表示・操作する |
| GitHub App installation | Non-GA | `completeGitHubInstall` は installation_id を connection metadata に保存するのみ |

## 検証

秘密値、実 GitHub credentials、本番データは使用しない。

1. unit test: `ExtGithubMeta` の default-deny / ref 既定（`ext_github_meta.rs`）。
2. unit test: デコレータの skip 条件（ext_github 無し / enabled=false）と meta 抽出（`github_writeback.rs`）。
3. unit test: エコー抑止 — `external_version == push.after` で upsert されず skipped、SHA 変化で処理（`event_processor.rs`）。
4. unit test: webhook secret が `Provider::Github` として store に入る（`bootstrap.rs`）。
5. unit test: `github_import_metadata` の enable_sync true/false 両ケース（`import_markdown_from_github.rs`）。
6. web unit test: `normalizeExtGithubEditorState` の ref/default-deny（`ext-github-sync-policy.test.ts`）。

## 再設計

正本と移行計画は [PLT-4534 design](../../src/tasks/in-progress/plt-4534-library-github-sync/design.md)
および ADR-0006 を参照する。

## 残リスク

1. 実 GitHub repository に対する end-to-end 同期は利用者 OAuth token が必要なため、この検証では実施していない。本番検証には検証用 GitHub account / repository と OAuth 接続、webhook endpoint 作成、および Tachyon 側 oauth-providers への `github` webhook_secret 登録が必要。
2. エコー抑止は push head SHA 比較のため、自 push 直後に第三者 commit が同一 push に混ざると fail-open で再 import される（安全側・収束する）。
3. 自動 writeback は inline await のため、GitHub API が遅い場合 Data 保存のレイテンシに乗る。問題になる場合は owned-type 化して `tokio::spawn` に移す。
