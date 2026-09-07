# Library MCP の Tachyon OAuth 接続

[PLT-4366](https://linear.app/issue/PLT-4366) の認可サーバー集約方針をLibrary側に適用する。
Tachyonの既存ADR-0022（public client/PKCE）に基づく接続実装であり、新規認証基盤は導入しない。

## 接続契約

`MCP_AUTHORIZATION_SERVER(S)` を設定した環境を外部認可サーバーモードとする。
信頼するissuerは一つに限定し、管理者がdiscoveryから確認した `MCP_OAUTH_JWKS_URL` を指定する。
issuer/JWKS/resource URLはHTTPSを必須にし、token由来のURLを参照しない。
RS256、kid、署名、issuer、期限、nbf、Library自身のresource URLを含むaudience、subを検証する。
SDKで同じtokenを検証して得たユーザーIDとsubを一致確認し、既存の組織所属・データ権限検証へ渡す。
JWTの署名検証にはjsonwebtokenを使用し、鍵は短時間キャッシュする。取得失敗時は期限切れ鍵や旧認証にフォールバックしない。

MCPの読取ツールにはmcp:read、変更ツールにはmcp:writeを要求する。
ツール一覧と直接呼出しの双方に適用し、scopeは既存データ認可を置き換えない。
APIキーは既存の組織を指定した検証・ポリシー評価を維持する。
外部モードではLibrary独自の登録・認可・tokenエンドポイントを410で停止する。
未設定環境は従来の認証経路を維持し、本番接続確認後に旧実装を物理削除する。
閉じたPR #312のDB永続化差分・マイグレーションは採用しない。

## 切替条件と上流依存

2026-09-07に確認したTachyonブランチ feature/plt-4366-cognito-mcp は、認可コード交換・refreshでaudにclient_idを設定する。
Library resource向けaccess tokenの発行は追加対応が必要。audience検証を緩めて回避しない。
標準form/public client修正のデプロイ、resourceの認可・コード保存・refreshまでの拘束、複数インスタンスのcallback/consent replay防止をTachyon側で検証する。
その後、Library Previewで新経路を有効化し、実クライアントによる再登録・再認可、許可ツール利用と拒否ケースを検証して本番設定を切り替える。
旧tokenは外部モードの検証条件を満たさなければ拒否する。切戻しは設定を戻すが、旧プロセス内状態は復元されず再登録が必要。

## 検証

署名付きJWTで正常系、別issuer/audience、期限切れ、nbf、kid、改ざん、scope不足を検証する。
ツール一覧の絞込みと直接変更呼出し拒否、APIキー経路、SDKのsubject一致確認を検証する。
ローカル/CI/デプロイ/実クライアント接続はそれぞれ記録する。
