//! Repository access an organization API key can be issued with.
//!
//! Tachyon attaches policies to a service account, not to a key, so
//! Library gives every key a service account of its own: granting access
//! to a key is granting it to that account, and no key ever shares
//! another's grants. The account is an implementation detail Library does
//! not surface; its name carries the key's role so listing can report it
//! without asking Tachyon which policies are attached.

use async_graphql::Enum;
use tachyon_sdk::auth::PolicyId;

/// Service account every key was issued on before keys got accounts of
/// their own. It carries no Library policy; its keys are still listed and
/// revocable, as keys without a role.
pub const LEGACY_API_KEY_SERVICE_ACCOUNT_NAME: &str = "default";

const API_KEY_SERVICE_ACCOUNT_PREFIX: &str = "library-api-key-";
/// Role segment of a key issued without a role.
const NO_ROLE_SEGMENT: &str = "public";

/// Repository access granted to every repository of the organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ApiKeyRole {
    /// Read private repositories.
    Reader,
    /// Read private repositories and write their data.
    Writer,
    /// Everything a writer can do, plus deleting repositories and
    /// managing their members.
    Owner,
}

impl ApiKeyRole {
    pub const ALL: [ApiKeyRole; 3] =
        [ApiKeyRole::Reader, ApiKeyRole::Writer, ApiKeyRole::Owner];

    /// The same repository policy templates repository members get
    /// (see `change_repo_member_role`), attached without a resource
    /// scope so they cover every repository of the organization.
    pub fn policy_id(self) -> PolicyId {
        PolicyId::new(match self {
            ApiKeyRole::Reader => "pol_01libraryreporeader",
            ApiKeyRole::Writer => "pol_01libraryrepowriter",
            ApiKeyRole::Owner => "pol_01libraryrepoowner",
        })
    }

    fn segment(self) -> &'static str {
        match self {
            ApiKeyRole::Reader => "reader",
            ApiKeyRole::Writer => "writer",
            ApiKeyRole::Owner => "owner",
        }
    }
}

/// What a service account in an organization is to Library's API keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyServiceAccount {
    /// Holds exactly one key, issued with this role.
    Dedicated(Option<ApiKeyRole>),
    /// The shared account keys were issued on before they got their own.
    Legacy,
}

impl ApiKeyServiceAccount {
    /// Name for the account a new key gets. The suffix only has to keep
    /// names apart within the organization.
    pub fn new_name(role: Option<ApiKeyRole>) -> String {
        let segment = role.map_or(NO_ROLE_SEGMENT, ApiKeyRole::segment);
        format!(
            "{API_KEY_SERVICE_ACCOUNT_PREFIX}{segment}-{}",
            uuid::Uuid::new_v4().simple()
        )
    }

    /// `None` for accounts that are not Library's API key accounts.
    pub fn from_name(name: &str) -> Option<Self> {
        if name == LEGACY_API_KEY_SERVICE_ACCOUNT_NAME {
            return Some(Self::Legacy);
        }
        let rest = name.strip_prefix(API_KEY_SERVICE_ACCOUNT_PREFIX)?;
        let (segment, _suffix) = rest.split_once('-')?;
        if segment == NO_ROLE_SEGMENT {
            return Some(Self::Dedicated(None));
        }
        ApiKeyRole::ALL
            .into_iter()
            .find(|role| role.segment() == segment)
            .map(|role| Self::Dedicated(Some(role)))
    }

    pub fn role(self) -> Option<ApiKeyRole> {
        match self {
            Self::Dedicated(role) => role,
            Self::Legacy => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_account_name_reads_back_as_its_role() {
        for role in ApiKeyRole::ALL.map(Some).into_iter().chain([None]) {
            assert_eq!(
                ApiKeyServiceAccount::from_name(
                    &ApiKeyServiceAccount::new_name(role)
                ),
                Some(ApiKeyServiceAccount::Dedicated(role))
            );
        }
    }

    #[test]
    fn every_new_key_gets_a_distinct_account() {
        assert_ne!(
            ApiKeyServiceAccount::new_name(Some(ApiKeyRole::Reader)),
            ApiKeyServiceAccount::new_name(Some(ApiKeyRole::Reader))
        );
    }

    #[test]
    fn recognises_the_legacy_account_and_ignores_others() {
        assert_eq!(
            ApiKeyServiceAccount::from_name("default"),
            Some(ApiKeyServiceAccount::Legacy)
        );
        assert_eq!(ApiKeyServiceAccount::from_name("tachyon-cli"), None);
        assert_eq!(
            ApiKeyServiceAccount::from_name("library-api-key-admin-x"),
            None
        );
    }
}
