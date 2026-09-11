# PLT-4532 Library plugin の OpenAI Plugins Directory 提出

[Linear](https://linear.app/issue/PLT-4532) / [提出資料](../../../../../plugins/library/submission/README.md) / [検証記録](verification-report.md)

## 概要

Library plugin を OpenAI Platform の審査ポータルへ `With MCP` として提出するための実装と提出資料を整える。Ready PRではplugin package、正確なtool annotations、ドメイン検証endpoint、再現可能なテストケースまでを完了し、デプロイ後のdomain verification、OAuth実行、公開ポリシー、最終提出は[運用todo](../../../todo/plt-4532-library-plugin-marketplace-submit/task.md)へ分離する。

## スコープ

- `plugins/library` の listing metadata、ブランド素材、starter prompts の提出適合
- Library MCP の tool annotations とドメイン検証 endpoint
- ポータル入力用の listing、positive 5件 / negative 3件、release notes の準備
- 公開 MCP、OAuth discovery、審査用アカウントを分離した検証記録

## 非スコープ

- OpenAI Platform の organization role や business verification の変更
- 法務レビューなしの利用規約・プライバシーポリシー公開
- 審査用アカウントの認証情報をリポジトリへ保存すること
- OpenAI による審査完了時期の保証

## 対象

- `apps/api/src/handler/mcp.rs`
- `apps/api/src/router.rs`
- `plugins/library/.codex-plugin/plugin.json`
- `plugins/library/assets/`
- `plugins/library/submission/README.md`

## 設計判断

- 新規 DD / ADR は作成しない。MCP の機能・認可境界は変更せず、OpenAI が定義する提出契約へ既存 metadata と検証経路を合わせるため。
- ドメイン検証トークンはソースへ埋め込まず、`OPENAI_APPS_CHALLENGE_TOKEN` から exact plain text として返す。
- portal が発行するトークン、demo credentials、Platform権限は運用値としてリポジトリ外で扱う。

## 実装と検証

1. create と destructive write を分けた tool annotation を実装し、代表ケースをテストする。
2. `/.well-known/openai-apps-challenge` を追加し、未設定時404・設定時exact tokenをテストする。
3. plugin manifest の starter prompt を3件に絞り、ブランド素材を同梱する。
4. portal 用提出資料と未解決ゲートを1か所にまとめる。
5. focused Rust test、plugin validator、公開 endpoint preflight を実行する。
6. 修正のデプロイ後に Scan Tools、OAuthログイン、positive/negative testを実クライアントで再実行する。

## 検証結果

- plugin validator: `Plugin validation passed`
- Rust format: `cargo +nightly-2026-06-04 fmt --all -- --check` 成功
- MCP focused tests: lib/binそれぞれ36件、合計72件成功
- 公開MCP: `initialize` がHTTP 200、server version `1.11.4`
- protected-resource metadata: `openid`、`profile`、`email`、`mcp:read`、`mcp:write` を広告
- Tachyon OIDC discovery: `userinfo_endpoint` を広告
- 公開website / user guide / support URL: HTTP 200
- 本番challenge endpoint: 未デプロイのためHTTP 404（期待どおり残ゲート）
- OpenAI Platform: `Quantum Box株式会社` / `Default project` でbusiness verificationのApprovedを確認し、Owner権限で`With MCP` draftを作成済み
- Portal draft: Infoの公開文言・Productivity・Business identity・website・support、および本番MCP URL・OAuthを保存済み。Scan ToolsはOAuth authorization直前で停止中
- Privacy / terms候補URLはHTTP 200でもSPAの`Not Found`表示であり、公開ポリシーとして使用不可
- `docs/SUMMARY.md` はこのrepositoryに存在しないため、taskdoc navigation更新は対象なし

## 完了条件

- 全29ツールに実態どおりの `readOnlyHint`、`openWorldHint`、`destructiveHint` が返る。
- challenge endpoint が設定時にportal発行トークンだけを返し、未設定時404になる。
- plugin package が validator を通る。
- OpenAI PlatformにBusiness identity付き`With MCP` draftがあり、本番MCP URLとOAuthが保存される。
- 提出に残るデプロイ・法務・審査用認証情報・実クライアント検証を運用todoへ明示する。

## Ready PR後の残作業

- 公開 privacy policy と terms URL は現時点で未確定であり、法務・事業判断後に公開する必要がある。
- business verification とdraft作成権限は確認済み。提出操作は全ゲート完了後に別途確認する。
- Tachyon OAuthのUserInfoが審査用ユーザーで `email` と `email_verified: true` を返すことは、demo account発行後に実トークンで確認する。
- challenge endpoint はデプロイと環境変数設定を経るまで本番では404のままである。
- 詳細な順序と完了条件は[運用todo](../../../todo/plt-4532-library-plugin-marketplace-submit/task.md)で追跡する。
