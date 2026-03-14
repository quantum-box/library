# Library API Changelog

All notable changes to the Library API will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

