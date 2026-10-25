# PLT-6094: bernard-square MCP repository creation

Issue: https://linear.app/quantum-box/issue/PLT-6094

## Verified cause (2026-10-25)

The connected user `us_01j8zbyvkqpbz4zycm7nx4fc9t` is an OWNER and a
member of `bernard-square` (`tn_01kshr165k3wwr8xyk0e6vy36c`). The user's
tenant policies include LibraryUserPolicy and LibraryRepoOwnerPolicy.

A production MCP `create_repo` call was correlated with upstream request
`2c7a344b-0610-4d5e-aed1-940e20b32a6d` at 13:08:44 UTC. The upstream
`/v1/auth/policies/check-for-user` returned 403 before evaluating
`library:CreateRepo`. Library's runtime service account is
`sa_01kr0ggcbt50j4bjryhgs9ewqs`, acting in
`tn_01j91h09tpj5ehwbwfwfxpak2b`. Its only exact sibling grant covered
Quantum Box, not bernard-square. This was a missing service delegation
grant, not a missing end-user membership or a base-credential regression.

## Production configuration repair

The exact request body is retained in
`delegation-policy.json` in this directory. This is a REST
policy registration payload, **not** a `tachyon manifest apply` document:
the installed manifest schema does not support `resourceScope`.

Registered policy: `pol_01m5sjxk92qd7yrzxk798b1jnc`
(`library:DelegateBernardSquare`), attached only to the runtime service
account above. It contains one direct `auth:EvaluatePoliciesForUserInTenant`
Allow and the exact resource `trn:auth:operator:tn_01kshr165k3wwr8xyk0e6vy36c`.
It is tenant-owned, unshared, non-system, and has no wildcard actions.
Existing policies and user grants were not changed.

Live verification with that service credential:

| Case | Result |
| --- | --- |
| Affected user / target tenant / CreateRepo | 200, allowed=true |
| Nonexistent user / target tenant | 404, NOT_FOUND |
| Affected user / other ungranted tenant | 403, FORBIDDEN |
| Invalid service credential | 401, UNAUTHORIZED |
| Affected user / new repo TRN / ManageRepoPolicy | 200, allowed=true |

Rollback of this grant uses `tachyon org service-accounts detach-policy`
with the service account and policy IDs above in the Library platform
tenant. Do not remove or widen the separate Quantum Box grant.

## Second failure and code repair

After the delegation repair, the same MCP create call progressed past
CreateRepo authorization and failed with upstream 401. Upstream request
`8cb8fc08-0514-444a-b09e-04ef414a2dbc` at 13:10:18 UTC logged `token is
missing` on `/v1/auth/user-policies/attach-with-scope`. Creation always
attempted to attach a scoped owner policy, even when the user already had
tenant-wide repository management rights. A Library-resource OAuth token
cannot be forwarded to this upstream mutation endpoint.

Creation now first checks the authenticated creator's existing
`library:ManageRepoPolicy` permission against the exact new repo TRN. When
allowed, no redundant grant is needed. Otherwise the existing scoped
attachment still requires upstream authorization. The optimization only
applies to a user granting access to themselves. Authentication, dependency,
and malformed-response failures fail closed.

The authorization tenant must equal the resolved destination organization
before any database/property writes and before reusing owner access. This
rejects a GraphQL request combining a tenant-A header with a tenant-B org
argument. The HTTP regression asserts that no policy request or grant is
sent for this mismatch.

This does not add a delegated IAM write capability: a resource-bound OAuth
caller who lacks existing owner access and needs a new scoped grant still
cannot use the upstream write endpoint. That broader flow needs a separately
authorized provisioning design; no service-credential fallback was added.

## Acceptance

- Production delegation repair: verified above.
- Local owner-access and existing organization restoration tests: 10 passed.
- SDK credential selection / delegated policy response regression tests: 21 passed.
- Changed Rust file passes rustfmt; git diff --check passes.
- `cargo clippy -p library-api --lib -- -D warnings -A clippy::double_must_use -A clippy::redundant_field_names` passes.
- An additional `--tests` Clippy run found four existing lints outside the
  changed file (live.rs, strategy.rs, strategy_migration.rs, markdown_mutation.rs);
  those unrelated files were left unchanged.
- Deployment of the code repair: pending.
- Successful creation and private readback using the original MCP connection:
  pending deployment. Do not mark PLT-6093 unblocked before this succeeds.

Failed creation attempts can leave database/property rows before the final
repo save; no unverified production cleanup was attempted.
