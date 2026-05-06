# Library Specification Index

この `docs/specs` 配下は `library` の仕様を分割して管理します。

## ファイル

- [概要 / 全体像](docs/specs/overview.md)
- [REST API 仕様](docs/specs/apis/rest-api.md)
- [GraphQL API 仕様](docs/specs/apis/graphql-api.md)
- [Webhook 受信仕様](docs/specs/integrations/webhooks.md)
- [外部連携 readiness](docs/specs/integrations/readiness.md)
- [コラボレーション WebSocket 仕様](docs/specs/integrations/collaboration-ws.md)
- [MCP 連携仕様](docs/specs/integrations/mcp.md)
- [運用手順 / トラブル対応](docs/specs/integrations/operations.md)
- [CMS利用ガイド](docs/specs/guides/cms-user-guide.md)
- [ドキュメントOS利用ガイド](docs/specs/guides/document-os-user-guide.md)
- [利用ユースケース](docs/specs/use-cases/use-cases.md)
- [ユースケース シーケンス図](docs/specs/use-cases/sequences.md)
- [実装差分ノート](docs/specs/implementation-notes.md)
- [利用ガイド（開発者向け / エンドユーザー向け）](docs/specs/guides/usage-guide.md)
- [ADR-0004: Library/bakuure stub の責務境界](docs/specs/decisions/ADR-0004-library-bakuure-stub-boundary.md)
- [ADR-0005: Shared Kernel を最小型セットに制限する](docs/specs/decisions/ADR-0005-shared-kernel-slim.md)

## 主な参照元

- API ルータ: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/router.rs)
- 認証・権限抽出: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/library_executor_extractor.rs)
- GraphQL 定義: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/graphql/mod.rs)
- REST ハンドラ: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler)
- 設定: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/config.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/config.rs)
- OpenAPI 生成: [/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/openapi.rs](/Users/takanorifukuyama/git/github.com/quantum-box/library/apps/api/src/handler/openapi.rs)
