# Library API Changelog

All notable changes to the Library API will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.11.13] - 2026-10-21

### Security
- **eMudhra の中間 CA 秘密鍵の漏洩（2026-10-20 公表）への対応**: 証明書から秘密鍵を復元できる eMudhra の中間 CA 8 本と、その上の root（emSign ECC Root CA - G3、Sectigo のクロス署名経由の AAA Certificate Services）を信頼しないようにする。
  - `rustls-webpki` 0.103.9 → 0.103.16。漏洩した中間 CA の公開鍵を含む証明書パスを、信頼ストアに関係なく拒否する版。rustls 0.23 を使う接続（reqwest 0.12、hyper-rustls 0.27、sqlx、AWS SDK の既定 HTTP クライアント）が対象。`rustls-webpki` 0.103.16 が `aws-lc-rs` 1.18 以上を要るため、`aws-lc-rs` 1.16.1 → 1.18.2、`aws-lc-sys` 0.38.0 → 0.46.0 も上がる。
  - `webpki-roots` 1.0.6 → 1.0.10（emSign ECC Root CA - G3 を削除した版。0.26.11 はこれを再 export する）。
  - sentry の `rustls` feature を外し、emSign ECC Root CA - G3 と AAA Certificate Services を含む `webpki-roots` 0.25.4 を依存から除く。0.25.4 を使うのは sentry の ureq transport だけで、library-api は ureq を有効にしていない。sentry の送信は reqwest transport で、reqwest は `default-tls`（OpenSSL と OS の証明書ストア）が有効なため、外す前から OpenSSL で送っており、送信の挙動は変わらない。

## [1.11.12] - 2026-10-17

### Fixed
- **コールドスタート後の最初のリクエストを速くする（失敗は増やさない）**:
  - Lambda の起動時（CPU を全部使える段階）に、DB の接続（プールごとに3本）と tachyon-api への TLS 接続を先に張る。最大 1.5 秒で打ち切り、失敗や時間切れでも起動は止めず、残りは最初のリクエストが張る。
  - tachyon-api への HTTP クライアントを1つにして全員で共有する。これまではヘッダーの組（呼び出し元のトークンなど）ごとにクライアントを作っていたため、新しい利用者ごとに接続と TLS ハンドシェイクをやり直し、起動時に張った接続も使えなかった。ヘッダーは `tachyon_sdk::Configuration::headers` に持たせ、送るときに足す（リクエスト側で付けたヘッダーは上書きしない。これまでの既定ヘッダーと同じ規則）。
  - `Configuration` を `{:?}` で表示しても、`Authorization` などのトークンは出ないようにする。

## [1.11.11] - 2026-10-16

### Fixed
- **応答の高速化（失敗は増やさない）**:
  - 組織を指定したリクエストで組織を解決するとき、組織の取得だけにする。これまでは使わない組織内の全リポジトリまで読み込んでいた（トレースで中央値 約90ms）。
  - `RepoRepositoryImpl::find_all` で、リポジトリごとに `databases` を引いていた N+1 を、JOIN 1回のクエリにまとめる。
  - 本番の Lambda のログを `INFO` までにする。共通の既定値が `DEBUG` で終わっていたため、データ一覧1回で数百行の debug ログ（PropertyValue dual-read parity など）が CloudWatch と OpenTelemetry に出ていた。

## [1.11.10] - 2026-10-10

### Fixed
- **PLT-5336: SDK認可応答の検証**: 単一・一括action応答を要求したaction集合と照合し、欠落、余分なaction、不一致、矛盾する重複、Allowとerrorの併存を固定error `Upstream protocol error` で拒否する。正しいAllow/Deny混在応答の判定は維持する。
  - 認可応答が不完全または矛盾する場合、許可として扱わずfail-closedにする。tokenや上流応答本文をerrorへ含めない。
  - resource応答の必須booleanが欠落・不正なら拒否する既存挙動の回帰テストを追加する。

## [1.5.0] - 2026-01-04

### Added
- **Org Owner Repo Full Access**: 組織オーナー（DefaultRole::Owner）に対して組織内全リポジトリへのフルアクセス権限を自動付与
  - `InviteOrgMember` usecase: 招待時に `role` パラメータでロール指定可能（OWNER/MANAGER/GENERAL）
  - `ChangeOrgMemberRole` usecase: ユーザーのロール変更（オーナー昇格時にポリシー付与、降格時に剥奪）
  - `SignIn` usecase: 既存オーナーのサインイン時に `pol_01libraryrepoowner` ポリシーを自動付与
  - GraphQL mutation `changeOrgMemberRole` 追加
  - GraphQL mutation `inviteUser` に `role` パラメータ追加
- `User.with_role()` メソッド追加（`packages/auth/domain`）
- `GetPolicyById` usecase追加（`packages/auth`）- SDK経由でポリシー取得をサポート

### Changed
- `GetRepoMembers` usecase に `AuthApp` 依存を追加（ポリシー情報取得のため）

## [1.4.0] - 2025-12-04

### Added
- **GitHub Sync 機能**: ライブラリデータをFrontmatter付きMarkdownとしてGitHubへ同期
  - `packages/database_sync` サブコンテキスト（SyncProvider トレイト、SyncConfig）
  - `packages/providers/github` GitHub OAuth & SyncProvider 実装
  - GraphQL mutations: `enableGithubSync`, `disableGithubSync`, `syncDataToGithub`, `bulkSyncExtGithub`
  - GraphQL queries: `githubConnection`, `githubListRepositories`
  - `BulkSyncExtGithub` usecase（一括同期）
  - `markdown_composer` usecase（Markdown生成のusecase層移動）
  - データ更新時の自動同期（UpdateData mutation 後）
- **ext_ プレフィックス予約語化**: `addProperty` mutation で `ext_` 開始名を拒否

### Changed
- `compose_markdown` を `handler/data.rs` から `usecase/markdown_composer.rs` へ移動

## [1.3.0] - 2025-11-29
### Added
- 公開リポジトリ向けの Markdown 出力エンドポイント `/v1beta/repos/{org}/{repo}/data/{data_id}/md` を追加。frontmatter に `id`/`title` と本文以外のプロパティを YAML で含め、本文は `content` プロパティ（markdown/html/string）を優先出力。`Content-Type: text/markdown; charset=utf-8`。

