//! Which scope Library sends to tachyon for an organization, verified with
//! Verus (`scripts/verus/verify.py`). library-api calls these functions.
//!
//! On 2026-11-18 repository creation in bernard-square, an organization
//! adopted from another platform, failed with 403: every scope Library built
//! claimed Library's platform, and tachyon-api rejects an `x-platform-id`
//! other than the operator's own. The rules below were reviewed with the
//! product owner on 2026-11-19 (library-org-platform-authority design).
//!
//! | recorded home platform | scope                                  |
//! |------------------------|----------------------------------------|
//! | none                   | none can be built from the organization |
//! | Library's platform     | names Library's platform               |
//! | any other platform     | names no platform                      |
//!
//! Creating a repository is authorized only when the request's tenant is the
//! destination organization, and an existing owner grant is reused only for
//! a user creating the repository for themselves.
use vstd::prelude::*;

verus! {

pub fn same_bytes(left: &[u8], right: &[u8]) -> (same: bool)
    ensures
        same == (left@ == right@),
{
    if left.len() != right.len() {
        return false;
    }
    let mut i = 0;
    while i < left.len()
        invariant
            left.len() == right.len(),
            0 <= i <= left.len(),
            forall|j: int| 0 <= j < i ==> #[trigger] left[j] == right[j],
        decreases left.len() - i,
    {
        if left[i] != right[i] {
            return false;
        }
        i += 1;
    }
    assert(left@ =~= right@);
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopePlatform {
    /// The scope names Library's platform.
    Library,
    /// The scope names no platform; tachyon-api resolves it from the
    /// operator alone.
    Other,
    /// No scope can be built from the organization.
    Unrecorded,
}

pub open spec fn scope_platform_spec(
    recorded: bool,
    home: Seq<u8>,
    library: Seq<u8>,
) -> ScopePlatform {
    if !recorded {
        ScopePlatform::Unrecorded
    } else if home == library {
        ScopePlatform::Library
    } else {
        ScopePlatform::Other
    }
}

/// Where an organization's scope sits. `home` is read only when `recorded`.
pub fn scope_platform(recorded: bool, home: &[u8], library: &[u8]) -> (placed: ScopePlatform)
    ensures
        placed == scope_platform_spec(recorded, home@, library@),
        // A scope claims Library's platform exactly when the organization is
        // recorded on it.
        (placed == ScopePlatform::Library) == (recorded && home@ == library@),
        // An unrecorded organization never yields a scope.
        !recorded ==> placed == ScopePlatform::Unrecorded,
{
    if !recorded {
        return ScopePlatform::Unrecorded;
    }
    if same_bytes(home, library) {
        ScopePlatform::Library
    } else {
        ScopePlatform::Other
    }
}

/// A repository is created only in the organization the request's tenant
/// names.
pub fn creation_tenant_matches(request_tenant: &[u8], destination: &[u8]) -> (allowed: bool)
    ensures
        allowed == (request_tenant@ == destination@),
{
    same_bytes(request_tenant, destination)
}

/// An existing owner grant may stand in for a new one only when a user
/// creates the repository for themselves.
pub fn may_reuse_owner_access(
    executor_is_user: bool,
    executor_id: &[u8],
    grantee_id: &[u8],
) -> (reuse: bool)
    ensures
        reuse == (executor_is_user && executor_id@ == grantee_id@),
{
    executor_is_user && same_bytes(executor_id, grantee_id)
}

} // verus!
