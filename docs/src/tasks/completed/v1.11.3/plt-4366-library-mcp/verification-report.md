# 検証記録

## ローカル

- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 check -p library-api --lib`: 成功。
- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 test -p library-api --lib handler::mcp`: 33件成功。MCP/SSEの既存回帰、署名付きJWTの検証、scopeごとの一覧・直接呼出し制御を含む。
- HTTP契約は環境変数を隔離した子プロセスで検証。外部metadataのissuer/resource/scope、旧認証エンドポイントの410、設定不備の503を確認。
- JWT検証はローカル生成RSA鍵を使用。別issuer、client_idや別URLのaudience、期限切れ、将来nbf、必須claim欠落、署名改ざん、HS256、未知kid、暗号化用途鍵を拒否。
- 変更Rustファイルの指定nightly rustfmt、`git diff --check`: 成功。

- `cargo +nightly-2026-06-04 clippy -p library-api --lib --no-deps`: 成功。追加コードの警告は解消済み。既存のmigrate.rs複数bin登録によるmanifest警告のみ。

## 未検証・上流依存

Tachyon本番のJWKSを使った実トークンでのLibrary接続、SDKが返す実ユーザーとの対応、業務データの権限拒否、CI/merge/deployは未実施。
Tachyonの確認対象ブランチは認可コード交換・refresh時にaudへclient_idを設定しており、このままでは外部モードのLibraryが拒否する。
Tachyon側でresourceの認可、認可コードとrefreshへの拘束、resource向けaudienceの発行を先に実装する必要がある。
callback/consentの複数インスタンスreplay防止も上流で確認する。

本番設定は変更していない。最新mainを基準にした feature/plt-4366-library-mcp でReady PRを作成する。旧PR #312の永続化コミットは含めない。
