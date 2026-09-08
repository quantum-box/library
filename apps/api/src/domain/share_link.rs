//! Read-only share links for one document in a private repo.
//!
//! Everything else that reads a private repo goes through
//! [`crate::domain::VisibilityService`] and then a Tachyon policy check,
//! which an anonymous visitor can never satisfy. A share link is the
//! single exception: the unguessable token in the URL *is* the
//! credential, in the same way the image route's unguessable id is (see
//! the note on the image router in `router.rs`).
//!
//! Two properties keep that exception narrow:
//!
//! - a link names exactly one `data_id`, so it cannot be walked outward
//!   into the rest of the repo;
//! - only the token's SHA-256 is persisted, so the secret exists in
//!   plaintext once, in the create response.

use chrono::{DateTime, Utc};
use derive_getters::Getters;
use rand::RngCore;
use sha2::{Digest, Sha256};
use util::macros::*;
use value_object::Text;

use super::RepoId;

def_id!(ShareLinkId, "sl_");

/// How many random bytes back a token.
///
/// 32 bytes is 256 bits of entropy, which is what makes "the URL is the
/// credential" defensible: the link cannot be found by guessing, and it
/// is not shortened for looks.
const TOKEN_BYTES: usize = 32;

/// Prefix every token carries, so one found in a log or a paste is
/// recognizable as a Library share token rather than an opaque blob.
pub const SHARE_TOKEN_PREFIX: &str = "shr_";

/// A freshly minted secret, held only long enough to answer the request
/// that created it.
///
/// Deliberately not `Debug`/`Clone`: the whole point is that this value
/// reaches the response body and nowhere else.
pub struct ShareToken(String);

impl ShareToken {
    /// Mint a token from the OS random source.
    pub fn generate() -> Self {
        let mut bytes = [0u8; TOKEN_BYTES];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(format!("{SHARE_TOKEN_PREFIX}{}", hex::encode(bytes)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn hash(&self) -> String {
        hash_share_token(&self.0)
    }
}

/// Lowercase hex SHA-256 of a token, the only form that reaches storage.
///
/// A plain hash rather than a password KDF on purpose: the input is 256
/// bits of uniform randomness, so there is no dictionary for a slow hash
/// to defend against, and the read path runs on every page view.
pub fn hash_share_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    hex::encode(digest)
}

#[async_trait::async_trait]
pub trait ShareLinkRepository:
    std::marker::Send + Sync + std::fmt::Debug
{
    async fn insert(&self, entity: &ShareLink) -> errors::Result<()>;

    /// Resolve a presented token. Returns revoked links too, so the read
    /// path can answer "this link was revoked" rather than "not found".
    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> errors::Result<Option<ShareLink>>;

    async fn find_for_data(
        &self,
        repo_id: &RepoId,
        data_id: &str,
    ) -> errors::Result<Vec<ShareLink>>;

    async fn get_by_id(
        &self,
        id: &ShareLinkId,
    ) -> errors::Result<Option<ShareLink>>;

    /// Mark a link revoked. Revoking one that is already revoked leaves
    /// the original timestamp alone, so the audit answer stays stable.
    async fn revoke(
        &self,
        id: &ShareLinkId,
        revoked_at: DateTime<Utc>,
    ) -> errors::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct ShareLink {
    id: ShareLinkId,
    token_hash: String,
    repo_id: RepoId,
    data_id: String,
    name: Option<Text>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl ShareLink {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ShareLinkId,
        token_hash: String,
        repo_id: RepoId,
        data_id: String,
        name: Option<Text>,
        created_by: Option<String>,
        created_at: DateTime<Utc>,
        revoked_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            token_hash,
            repo_id,
            data_id,
            name,
            created_by,
            created_at,
            revoked_at,
        }
    }

    /// Mint a link and the secret that opens it. The secret is returned
    /// beside the entity because it is never derivable from it again.
    pub fn issue(
        repo_id: RepoId,
        data_id: String,
        name: Option<Text>,
        created_by: Option<String>,
    ) -> (Self, ShareToken) {
        let token = ShareToken::generate();
        let link = Self::new(
            ShareLinkId::default(),
            token.hash(),
            repo_id,
            data_id,
            name,
            created_by,
            Utc::now(),
            None,
        );
        (link, token)
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_is_prefixed_and_full_entropy() {
        let token = ShareToken::generate();
        assert!(token.as_str().starts_with(SHARE_TOKEN_PREFIX));
        assert_eq!(
            token.as_str().len(),
            SHARE_TOKEN_PREFIX.len() + TOKEN_BYTES * 2
        );
    }

    #[test]
    fn two_tokens_never_collide() {
        let a = ShareToken::generate();
        let b = ShareToken::generate();
        assert_ne!(a.as_str(), b.as_str());
        assert_ne!(a.hash(), b.hash());
    }

    /// The stored hash has to be reproducible from the presented token
    /// alone -- that equality is the whole lookup on the read path.
    #[test]
    fn the_stored_hash_is_reproducible_from_the_token() {
        let token = ShareToken::generate();
        assert_eq!(token.hash(), hash_share_token(token.as_str()));
        assert_eq!(token.hash().len(), 64);
    }

    /// A token must not be recoverable from what a database dump holds.
    #[test]
    fn the_secret_is_not_the_stored_value() {
        let (link, token) = ShareLink::issue(
            RepoId::default(),
            "data_01k000000000000000000000".to_string(),
            None,
            None,
        );
        assert_ne!(link.token_hash(), token.as_str());
        assert!(!link.is_revoked());
    }
}
