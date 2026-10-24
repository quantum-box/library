# Strategy projection persistence

Owns private PM projection rows, immutable Library document releases and a
transactional consumer checkpoint. HTTP and authorization live in
`apps/api/src/handler/strategy.rs`; deployment/bootstrap is documented in
`docs/specs/operations/strategy-projection.md`.

`wire` mirrors the published Tachyon PM sync JSON, and verifies its payload
hash before converting to local changes. It deliberately has no absolute-path
or cross-repository source dependency. Source fields are isolated from local
notes/document references. Unknown schema versions fail before any transaction.

The MySQL integration test requires an explicitly isolated `strategy_test`
database. Never set its test URL to an application database.
