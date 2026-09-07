# PLT-4366 Library MCP 本番接続

[Linear](https://linear.app/issue/PLT-4366) / [Library側実装](../../completed/v1.11.3/plt-4366-library-mcp/task.md)

Tachyon側のresource対応待ちのためtodoで管理する。Library側のReady PRと本番接続完了を区別する。

- [ ] TachyonでLibrary resourceの認可、認可コード・refreshへの拘束、resource向けaudienceの発行
- [ ] 複数インスタンスの認可コード交換とcallback/consent replay防止
- [ ] Tachyon互換性修正のCI・merge・deploy
- [ ] Library PreviewでJWKS/issuer/resource設定と実MCPクライアント接続
- [ ] 実ユーザー対応、scope不足、別resource、権限外組織・データの拒否
- [ ] 本番設定切替・再登録・再認可と接続確認
- [ ] 切替確認後のLibrary旧OAuth実装削除
