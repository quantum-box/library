# PLT-4366 Library MCP 接続

[Linear](https://linear.app/issue/PLT-4366) / [設計](design.md)

閉じたLibrary PR #312の共有永続化を取り下げ、Tachyon認可サーバーのaccess tokenを検証するリソースサーバー側の実装を進める。
対象はapps/api/src/handler/mcp.rsと専用OAuth検証モジュール、MCP運用文書。

- [x] issuer/JWKS/audience/subject検証
- [x] read/write scopeと既存データ認可の組合せ
- [x] metadata・旧エンドポイントの切替
- [x] ローカル回帰テスト（33件成功、[検証記録](verification-report.md)）

Library側実装をAPI v1.11.3としてReady PRにする。デスクトップ版はmainの0.1.16から0.1.17へ更新する。
本番接続は[後続タスク](../../../todo/plt-4366-library-mcp-rollout/task.md)として分離する。

本番設定変更・デプロイは未実施。Tachyon側の進行中作業には変更を加えない。
