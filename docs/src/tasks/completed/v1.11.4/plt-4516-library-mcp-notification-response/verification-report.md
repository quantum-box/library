# 検証記録

## 再現

- 2026-09-10、本番 `https://library-api.txcloud.app/mcp` の `notifications/initialized` は `200 OK`、`Content-Type: application/json`、本文 `{}` を返した。
- Codex `0.153.4` の一時セッションでは、初期化通知送信時に `Deserialize error: data did not match any variant of untagged enum JsonRpcMessage` となり、3回の再試行後も `get_me` を呼び出せなかった。

## ローカル品質ゲート

- `cargo +nightly-2026-06-04 fmt -p library-api -- --check`: 成功。
- `git diff --check`: 成功。
- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 check -p library-api --lib`: 成功。
- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 clippy -p library-api --lib --no-deps -- -D warnings`: 成功。
- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 build -p library-api --lib`: 成功。
- `OPENSSL_NO_VENDOR=1 CARGO_BUILD_JOBS=4 cargo +nightly-2026-06-04 test -p library-api --lib handler::mcp`: 34件成功。
- `apps/api/Cargo.toml`、`Cargo.lock`、`apps/api/library.openapi.yaml` のAPIバージョンが `1.11.4` で一致することを確認した。

既存の `apps/api/bin/migrate.rs` が2つのbin targetに登録されているmanifest warningのみ発生した。今回の差分によるwarningではない。

## スキップした確認と理由

- Library API scenario test: DBを含む業務CRUDは変更しておらず、HTTP通知変換を単体テストで直接検証したため未実施。
- 本番Codex接続: 修正版APIのマージ・デプロイ前であるため未実施。デプロイ後に `get_me` まで確認する。
