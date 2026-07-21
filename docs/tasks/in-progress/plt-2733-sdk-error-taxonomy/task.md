# PLT-2733 Library API SDK error taxonomy

## Status

In Progress — `feature/plt-2733-sdk-error-taxonomy`

## Goal

Library API の public operator alias lookup で、upstream transport / HTTP status / response decode の失敗を分離する。401/403 だけを認証・認可として扱い、依存サービス障害を auth failure と誤分類しない。

## Scope

- `apps/api/src/sdk_auth.rs` の raw REST GET helper と operator alias lookup
- idempotent GET の retryable connect / timeout だけを対象にした bounded retry
- operation、error kind、retryability、timeout/connect 判定だけを記録する safe observability
- transport、401、403、404、5xx、decode の contract regression tests
- message / event title の秘匿性と、1 logical failure = 1 error event の保証

## Non-goals

- production への deploy、設定変更、データ変更
- host、query、tenant/operator identifier、credential の記録
- feature branch の merge

## Implementation plan

1. 現行の SDK error mapping と GraphQL error capture 経路を確認する。
2. transport / HTTP status / decode の typed taxonomy と public error mapping を導入する。
3. retryable connect / timeout に、試行上限、jitter、総時間 budget を持つ retry を追加する。
4. root failure の capture は一度に限定し、wrapper context は breadcrumb/span として扱う。
5. fake connector の contract tests を workspace の required CI (`cargo nextest run --workspace`) で実行される通常 test target に追加する。

## Acceptance criteria

- [ ] transport failure は authN/authZ error ではなく `ServiceUnavailable` に分類される。
- [ ] 401 / 403 / 404 / 5xx / decode / timeout / connect の mapping test がある。
- [ ] 401 / 403 / 404 / decode は retry されず、connect / timeout だけが bounded retry される。
- [ ] user-facing message と error event title に接続先、query、identifier、credential が含まれない。
- [ ] 一つの upstream failure が一つの error event だけを生成する。
- [ ] 対象 test が required CI の `rust-test` job で実走し、PR CI が green になる。

## Verification plan

- `cargo fmt --all -- --check`
- `cargo test -p library-api <targeted tests>` または同等の nextest filter
- `cargo nextest run --workspace`（required CI path）
- `cargo clippy --workspace -- -D warnings`
- PR required checks の完了確認

## References

- Linear: PLT-2733（connector 再認証が必要なため、更新は PdM hand-off）
- Incident investigation: local redacted report; sensitive connection details are intentionally not copied here.
