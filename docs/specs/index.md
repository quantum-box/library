# Library Specification Index

この `docs/specs` 配下は `library` の仕様を分割して管理します。

## ファイル

- [概要 / 全体像](overview.md)
- [Library GA scope](ga-scope.md)
- [認証・認可・API Key セキュリティ監査](auth-security-audit.md)
- [REST API 仕様](apis/rest-api.md)
- [GraphQL API 仕様](apis/graphql-api.md)
- [Webhook 受信仕様](integrations/webhooks.md)
- [外部連携 readiness](integrations/readiness.md)
- [コラボレーション WebSocket 仕様](integrations/collaboration-ws.md)
- [MCP 連携仕様](integrations/mcp.md)
- [運用手順 / トラブル対応](integrations/operations.md)
- [Library GA env / secrets Runbook](operations/library-ga-env-secrets-runbook.md)
- [CMS利用ガイド](guides/cms-user-guide.md)
- [ドキュメントOS利用ガイド](guides/document-os-user-guide.md)
- [利用ユースケース](use-cases/use-cases.md)
- [ユースケース シーケンス図](use-cases/sequences.md)
- [実装差分ノート](implementation-notes.md)
- [利用ガイド（開発者向け / エンドユーザー向け）](guides/usage-guide.md)
- [ADR-0004: Library/bakuure stub の責務境界](decisions/ADR-0004-library-bakuure-stub-boundary.md)
- [ADR-0005: Shared Kernel を最小型セットに制限する](decisions/ADR-0005-shared-kernel-slim.md)

## 主な参照元

- API ルータ: [apps/api/src/router.rs](../../apps/api/src/router.rs)
- 認証・権限抽出: [apps/api/src/handler/library_executor_extractor.rs](../../apps/api/src/handler/library_executor_extractor.rs)
- GraphQL 定義: [apps/api/src/handler/graphql/mod.rs](../../apps/api/src/handler/graphql/mod.rs)
- REST ハンドラ: [apps/api/src/handler](../../apps/api/src/handler)
- 設定: [apps/api/src/config.rs](../../apps/api/src/config.rs)
- OpenAPI 生成: [apps/api/src/handler/openapi.rs](../../apps/api/src/handler/openapi.rs)
