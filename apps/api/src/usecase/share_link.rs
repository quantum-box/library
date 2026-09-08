//! Minting, listing, revoking and redeeming read-only share links.
//!
//! Managing links is gated on `library:UpdateRepo` for the repo, not on
//! read access: being allowed to read a private document is not the same
//! as being allowed to hand it to the world, and reusing the read gate
//! would have let every reader publish.
//!
//! Redeeming one is the mirror image -- no policy at all. The token is
//! the credential, and everything the request is allowed to touch is
//! derived from the row it resolves to.

use std::sync::Arc;

use chrono::Utc;
use database_manager::{
    domain::{Data, Property},
    usecase::FindAllPropertiesInputData,
};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, ExecutorAction,
    MultiTenancyAction,
};
use value_object::Text;

use crate::domain::{
    hash_share_token, Repo, RepoRepository, ShareLink, ShareLinkId,
    ShareLinkRepository, ShareToken, LIBRARY_TENANT,
};

use super::{GetOrganizationByUsernameQuery, GetRepoByUsernameQuery};

/// What every management call names, before the operation's own fields.
pub struct ShareLinkRepoTarget<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: &'a str,
    pub repo_username: &'a str,
}

pub struct CreateShareLinkInputData<'a> {
    pub target: ShareLinkRepoTarget<'a>,
    pub data_id: &'a str,
    pub name: Option<&'a str>,
}

pub struct ListShareLinksInputData<'a> {
    pub target: ShareLinkRepoTarget<'a>,
    pub data_id: &'a str,
}

pub struct RevokeShareLinkInputData<'a> {
    pub target: ShareLinkRepoTarget<'a>,
    pub share_link_id: &'a str,
}

/// A minted link and the one chance to see its secret.
pub struct CreatedShareLink {
    pub link: ShareLink,
    pub token: ShareToken,
}

/// Everything a viewer page needs, resolved from the token alone.
///
/// Neither the link nor the repository is here. The link decided this
/// read is allowed and has nothing further to say to the page; the
/// repository was only how the document was found, and its username is
/// part of what a private repository keeps private.
pub struct SharedData {
    pub data: Data,
    pub properties: Vec<Property>,
}

#[async_trait::async_trait]
pub trait ManageShareLinksInputPort: std::fmt::Debug + Send + Sync {
    async fn create<'a>(
        &self,
        input: &CreateShareLinkInputData<'a>,
    ) -> errors::Result<CreatedShareLink>;

    async fn list<'a>(
        &self,
        input: &ListShareLinksInputData<'a>,
    ) -> errors::Result<Vec<ShareLink>>;

    async fn revoke<'a>(
        &self,
        input: &RevokeShareLinkInputData<'a>,
    ) -> errors::Result<ShareLink>;
}

#[async_trait::async_trait]
pub trait ViewSharedDataInputPort: std::fmt::Debug + Send + Sync {
    /// Redeem a token. `Err(not_found)` for anything that does not
    /// resolve to a live link -- an unknown token and a revoked one are
    /// deliberately indistinguishable to whoever holds the URL.
    async fn execute(&self, token: &str) -> errors::Result<SharedData>;
}

/// The single answer every failed redemption gives.
fn share_link_not_found() -> errors::Error {
    errors::Error::not_found("share link not found")
}

/// Collapse a downstream "not found" into that answer, and let anything
/// else through -- a database outage is not a missing link, and saying
/// so is what makes the 404 mean something.
fn hide_missing_target(error: errors::Error) -> errors::Error {
    if error.is_not_found() {
        return share_link_not_found();
    }
    error
}

#[derive(Debug)]
pub struct ShareLinks {
    auth: Arc<dyn AuthApp>,
    get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
    get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
    repo_repository: Arc<dyn RepoRepository>,
    share_links: Arc<dyn ShareLinkRepository>,
    database: Arc<database_manager::App>,
}

impl ShareLinks {
    pub fn new(
        auth: Arc<dyn AuthApp>,
        get_org_by_username: Arc<dyn GetOrganizationByUsernameQuery>,
        get_repo_by_username: Arc<dyn GetRepoByUsernameQuery>,
        repo_repository: Arc<dyn RepoRepository>,
        share_links: Arc<dyn ShareLinkRepository>,
        database: Arc<database_manager::App>,
    ) -> Arc<Self> {
        Arc::new(Self {
            auth,
            get_org_by_username,
            get_repo_by_username,
            repo_repository,
            share_links,
            database,
        })
    }

    /// Resolve the repo named in the path and check the caller may hand
    /// its documents out.
    async fn authorized_repo(
        &self,
        target: &ShareLinkRepoTarget<'_>,
    ) -> errors::Result<Repo> {
        let org = self
            .get_org_by_username
            .execute(&target.org_username.parse()?)
            .await?
            .ok_or(errors::not_found!("organization not found"))?;
        let repo = self
            .get_repo_by_username
            .execute(org.username(), &target.repo_username.parse()?)
            .await?
            .ok_or(errors::not_found!("repo not found"))?;

        let resource_trn = format!("trn:library:repo:{}", repo.id());
        self.auth
            .check_policy_for_resource(&CheckPolicyForResourceInput {
                executor: target.executor,
                multi_tenancy: target.multi_tenancy,
                action: "library:UpdateRepo",
                resource_trn: &resource_trn,
            })
            .await?;

        Ok(repo)
    }

    fn database_id(
        repo: &Repo,
    ) -> errors::Result<database_manager::domain::DatabaseId> {
        repo.databases()
            .first()
            .cloned()
            .ok_or(errors::not_found!("repo has no database"))
    }
}

#[async_trait::async_trait]
impl ManageShareLinksInputPort for ShareLinks {
    #[tracing::instrument(name = "ShareLinks::create", skip_all)]
    async fn create<'a>(
        &self,
        input: &CreateShareLinkInputData<'a>,
    ) -> errors::Result<CreatedShareLink> {
        let repo = self.authorized_repo(&input.target).await?;

        // A public repository already serves this document anonymously
        // at `/public/<org>/<repo>/<data_id>`, so a token would add no
        // access -- but it would outlive the repository being made
        // private again, quietly keeping open the one thing that change
        // was meant to close. Refuse here rather than leaving it to
        // callers to remember.
        if *repo.is_public() {
            return Err(errors::Error::invalid(
                "Share links are for private repositories; a public \
                 repository already serves this document anonymously",
            ));
        }

        // Minting a link to a document that is not in this repo would
        // produce a URL that 404s later, with nothing at creation time
        // to say why. Resolve it now instead.
        self.database
            .get_data_usecase()
            .execute(&database_manager::GetDataInputData {
                executor: input.target.executor,
                multi_tenancy: input.target.multi_tenancy,
                tenant_id: repo.organization_id(),
                database_id: &Self::database_id(&repo)?,
                data_id: &input.data_id.parse()?,
            })
            .await?;

        let name = input
            .name
            .map(|name| name.parse::<Text>())
            .transpose()
            .map_err(|e: anyhow::Error| {
                errors::Error::invalid(e.to_string())
            })?;
        let created_by = Some(input.target.executor.get_id().to_string())
            .filter(|id| !id.is_empty());

        let (link, token) = ShareLink::issue(
            repo.id().clone(),
            input.data_id.to_string(),
            name,
            created_by,
        );
        self.share_links.insert(&link).await?;

        Ok(CreatedShareLink { link, token })
    }

    #[tracing::instrument(name = "ShareLinks::list", skip_all)]
    async fn list<'a>(
        &self,
        input: &ListShareLinksInputData<'a>,
    ) -> errors::Result<Vec<ShareLink>> {
        let repo = self.authorized_repo(&input.target).await?;
        self.share_links
            .find_for_data(repo.id(), input.data_id)
            .await
    }

    #[tracing::instrument(name = "ShareLinks::revoke", skip_all)]
    async fn revoke<'a>(
        &self,
        input: &RevokeShareLinkInputData<'a>,
    ) -> errors::Result<ShareLink> {
        let repo = self.authorized_repo(&input.target).await?;
        let id: ShareLinkId = input.share_link_id.parse()?;

        let link = self
            .share_links
            .get_by_id(&id)
            .await?
            .ok_or(errors::not_found!("share link not found"))?;

        // The path names the repo the caller was authorized against, so
        // a link belonging to another repo has to miss here rather than
        // be revoked on that repo's behalf.
        if link.repo_id() != repo.id() {
            return Err(errors::not_found!("share link not found"));
        }

        if link.is_revoked() {
            return Ok(link);
        }

        let revoked_at = Utc::now();
        self.share_links.revoke(&id, revoked_at).await?;
        Ok(ShareLink::new(
            id,
            link.token_hash().clone(),
            link.repo_id().clone(),
            link.data_id().clone(),
            link.name().clone(),
            link.created_by().clone(),
            *link.created_at(),
            Some(revoked_at),
        ))
    }
}

#[async_trait::async_trait]
impl ViewSharedDataInputPort for ShareLinks {
    #[tracing::instrument(name = "ShareLinks::view_shared", skip_all)]
    async fn execute(&self, token: &str) -> errors::Result<SharedData> {
        let link = self
            .share_links
            .find_by_token_hash(&hash_share_token(token))
            .await?
            .filter(|link| !link.is_revoked())
            .ok_or_else(share_link_not_found)?;

        // From here on, every "it is not there" becomes the same answer
        // the unknown and revoked tokens got. A deleted document, a
        // deleted repository and a revoked link must be one outcome to
        // whoever holds the URL, or the 404 text tells them which.
        let repo = self
            .repo_repository
            .get_by_id(&LIBRARY_TENANT, link.repo_id())
            .await
            .map_err(hide_missing_target)?
            .ok_or_else(share_link_not_found)?;
        let database_id =
            Self::database_id(&repo).map_err(hide_missing_target)?;

        // `SystemExecutor` rather than the anonymous caller: the token
        // has already decided this read is allowed, and the layer below
        // takes an executor only to scope the query. Nothing here is
        // reachable except through the row the token resolved to.
        let executor = inbound_sync::sdk::SystemExecutor;
        let multi_tenancy =
            crate::usecase::LibraryOrg::with_org_and_operator(
                repo.org_username().to_string(),
                repo.organization_id().clone(),
            );

        let properties = self
            .database
            .find_all_properties()
            .execute(FindAllPropertiesInputData {
                tenant_id: repo.organization_id().clone(),
                database_id: database_id.clone(),
            })
            .await
            .map_err(hide_missing_target)?;
        let data = self
            .database
            .get_data_usecase()
            .execute(&database_manager::GetDataInputData {
                executor: &executor,
                multi_tenancy: &multi_tenancy,
                tenant_id: repo.organization_id(),
                database_id: &database_id,
                data_id: &link
                    .data_id()
                    .parse()
                    .map_err(|_| share_link_not_found())?,
            })
            .await
            .map_err(hide_missing_target)?;

        Ok(SharedData { data, properties })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deleted document, a deleted repository and a revoked link have
    /// to be one outcome to whoever holds the URL. Letting the
    /// downstream text through ("resource not found") would tell them
    /// which of the three they are holding.
    #[test]
    fn every_missing_target_answers_as_a_missing_link() {
        let downstream = errors::Error::not_found("resource not found");
        let hidden = hide_missing_target(downstream);

        assert!(hidden.is_not_found());
        assert_eq!(hidden.to_string(), share_link_not_found().to_string());
    }

    /// A database outage is not a missing link. Collapsing it into 404
    /// would tell the holder their link is gone and hide the incident
    /// from everyone else.
    #[test]
    fn a_failure_that_is_not_a_miss_survives() {
        let outage =
            errors::Error::internal_server_error("connection reset");
        let passed = hide_missing_target(outage);

        assert!(!passed.is_not_found());
        assert!(passed.to_string().contains("connection reset"));
    }
}
