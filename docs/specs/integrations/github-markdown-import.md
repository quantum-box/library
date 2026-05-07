# GitHub Markdown import GA verification

最終更新: 2026-05-07

## 結論

GitHub Markdown import は **GA** として扱う。ただし GA 対象は GitHub OAuth token を使った **one-shot import** に限定する。

GitHub sync / writeback / GitHub App installation / inbound GitHub sync は **Non-GA** として扱う。GA UI からは同期有効化導線を出さず、API は `enableGithubSync=true`、`syncDataToGithub`、`bulkSyncExtGithub`、`enableGithubSync` を保存または外部書き戻し前に `bad_request` で拒否する。

## Primary-source 確認

| 対象 | 判定 | 確認元 |
| --- | --- | --- |
| OAuth URL / token exchange | GA import 用 | `apps/api/src/handler/graphql/mutation.rs` の `githubAuthUrl` / `githubExchangeToken`。OAuth state は HMAC 検証され、token は provider `github` として保存される |
| Directory / preview / frontmatter analyze | GA import 用 | `apps/api/src/usecase/list_github_directory.rs`、`apps/api/src/usecase/get_markdown_previews.rs`。OAuth token を要求し、Markdown file preview に閉じる |
| Markdown import | GA one-shot | `apps/api/src/usecase/import_markdown_from_github.rs`。`library:CreateData` 認可後、Library repo/property/data を作成または更新する |
| Import sync flag | Non-GA | `apps/api/src/handler/graphql/mutation.rs` で `enableGithubSync=true` を拒否し、usecase へは `false` のみ渡す |
| GitHub writeback mutations | Non-GA | `syncDataToGithub` / `bulkSyncExtGithub` / `enableGithubSync` は `bad_request` を返す |
| inbound GitHub sync | Non-GA | `apps/api/src/router.rs` が `NoOpGitHubClient` / `NoOpGitHubDataHandler` を配線。`docs/specs/integrations/readiness.md` でも Non-GA |

## 検証

秘密値、実 GitHub credentials、本番データは使用しない。

1. 静的確認: `rg` で GitHub Markdown import / sync の API、UI、docs、tests を棚卸し。
2. UI確認: GitHub import dialog は `enableGithubSync: false` を送信し、同期チェックボックスを表示しない。
3. API確認: `require_one_shot_github_markdown_import` の unit test で `None` / `false` は GA one-shot、`true` は拒否されることを確認する。
4. docs確認: `docs/specs/ga-scope.md` と `docs/specs/integrations/readiness.md` に GA / Non-GA 境界を反映する。

## 残リスク

実 GitHub repository に対する end-to-end import は利用者 OAuth token が必要なため、この検証では実施しない。実データ検証が必要な場合は、PdM が検証用 GitHub account / repository と明示的な OAuth 接続手順を用意する。
