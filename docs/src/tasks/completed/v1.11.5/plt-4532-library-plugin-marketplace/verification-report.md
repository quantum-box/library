# 検証記録

## ローカル品質ゲート

- `cargo +nightly-2026-06-04 fmt --all -- --check`: 成功。
- `git diff --check`: 成功。
- plugin validator: 成功。
- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 check -p library-api --lib`: 成功。
- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 test -p library-api handler::mcp`: lib/bin合計72件成功。
- 専用の一時`CARGO_TARGET_DIR`で`cargo +nightly-2026-06-04 clippy -p library-api --lib --no-deps -- -D warnings`: 成功。
- 同じ一時`CARGO_TARGET_DIR`で`cargo +nightly-2026-06-04 build -p library-api --lib`: 成功。
- `apps/api/Cargo.toml`、`Cargo.lock`、`apps/api/library.openapi.yaml`を`1.11.5`へ同期し、plugin packageを`0.5.1`へpatch更新した。

共有`target`のbuild-script出力欠損でClippyが失敗したため、ソース不良と切り分ける目的で5.2GBの一時targetを作成した。専用targetではClippyとbuildが成功し、確認後に一時targetだけを削除した。既存の`apps/api/bin/migrate.rs`重複target warningと、テスト時の既存unused import warningは今回の差分外である。

## 公開面とポータル

- 本番MCP `initialize`: HTTP 200、server version `1.11.4`（修正デプロイ前）。
- protected-resource metadata: `openid`、`profile`、`email`、`mcp:read`、`mcp:write`を広告。
- Tachyon OIDC discovery: UserInfo endpointを広告。
- OpenAI Platform: `Quantum Box株式会社` / `Default project` でIndividualとBusinessがApproved。Ownerとして`With MCP` draftを作成した。
- Draft Info: Library、`1.0.0`、Productivity、Business identity、公開説明、website、supportを保存した。
- Draft MCP: `https://library-api.txcloud.app/mcp` とOAuthを保存し、domain challenge token発行まで確認した。token値はrepository、taskdoc、ログへ保存していない。
- Scan Tools: Library OAuth authorizationに成功し、本番の32ツールを取得した。本番は修正版デプロイ前のため、旧annotationのままで、全toolに`outputSchema`追加の推奨が表示されることを確認した。PLT-4553で修正し、デプロイ後再scanを残した。
- `https://library.n1.tachy.one/privacy` と `/terms` はHTTP 200でもSPA上は`Not Found`のため、提出URLとして不採用。

## スキップした確認と理由

- Domain verification: challenge endpointが未デプロイで、本番は404のため。
- 修正版annotation / `outputSchema`の再scan、positive 5件 / negative 3件: 審査用fixtureとdemo accountを作成し、修正版をデプロイした後に実行するため。
- Privacy policy / Terms of Service: 法務・事業承認済みの公開文書が未整備のため。
- Directory/composer icon uploadとdemo recording: 外部アップロードおよび実クライアント録画を運用ゲートで行うため。
