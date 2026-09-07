# PLT-4361: Library MCP OAuth state across Lambda instances

Issue: https://linear.app/issue/PLT-4361

## Problem and change

Dynamic registrations and authorization codes lived in a static in-process
map. Register, authorize and token requests reaching different Lambda instances
could not complete sign-in; recycling an instance also lost the state.

Both kinds of state now use the existing Library MySQL pool and database
layout. Production uses Library's database; previews use their isolated
per-PR database. There is no in-memory fallback on database failure.

- Persist public client metadata without expiry, so a registered client remains
  valid after cold starts and deployment.
- Codes expire after 600 seconds using the database UTC clock. Store only their
  SHA-256 lookup hash. Encrypt the token-bearing payload with AES-256-GCM and
  a domain-separated SHA-256 key derived from the 256-bit random authorization code;
  neither the code nor the encryption key is persisted. Nonces are random.
- Require client ID, redirect URI and PKCE verifier for exchange. Validate
  bindings before the conditional DELETE; one concurrent caller can consume
  a code. Invalid bindings leave the legitimate exchange available.
- Reject expired access tokens and return their remaining lifetime, rather
  than restarting the Cognito lifetime on exchange.
- Remove at most 100 expired codes per issuance. Expired rows are unusable
  regardless of cleanup, and contain only encrypted payloads.

## Validation

`SQLX_OFFLINE=true cargo +nightly-2026-06-04 test -p library-api --lib oauth_store -- --include-ignored` with
`DATABASE_URL` pointing to an existing database on a disposable local MySQL
server. SQLx creates
an isolated test database. The DB test is also wired into the MySQL CI job.
It covers separate pools and destroyed issuing instances, case-sensitive client
lookup, incorrect bindings, replay, concurrent exchange, expiration, cleanup,
and register/authorize/token handlers. Unit coverage rejects altered ciphertext
and decrypting with the wrong code.

## Release

The additive migration is included by both existing production and preview
migration gates. Promote only after that gate passes. Previously issued
in-memory registrations/codes cannot be migrated: reconnect/re-register the
MCP client once after rollout. The old binary ignores these new tables, so a
rollback reintroduces the original instance-local limitation.

Implementation and local test results do not establish production Cognito
sign-in or Lambda rollout success; those must be verified after deployment.

## Local result (2026-09-07)

- API unit test binary: 2 passed, including the MySQL regression and handler
  coverage above. The run used the pinned nightly toolchain,
  `SQLX_OFFLINE=true`, `OPENSSL_NO_VENDOR=1` (installed OpenSSL 3.6.3), and
  `DATABASE_URL=mysql://root@127.0.0.1:14361/plt4361_migration_check`.
- Migration up/down both succeeded on the disposable local MySQL instance.
- Pinned-toolchain formatting and `git diff --check` passed.
- Production migration, deployment and a real Cognito/MCP sign-in have not
  been performed.
