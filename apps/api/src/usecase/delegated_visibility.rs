//! What a caller whose token does not carry a membership may still list.
//!
//! A token issued for a resource server carries only its client's tenant
//! (tachyon ADR-0104), so an MCP caller belongs, by the token alone, to
//! Library's platform and nothing else. Reading one repository already asks
//! tachyon, through delegated evaluation, whether the caller may read it;
//! these listings ask the same question, so a list shows exactly what a read
//! would allow.
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, CheckPolicyInput, ExecutorAction,
    MultiTenancyAction,
};
use value_object::TenantId;

use super::{LibraryOrg, OrgScopes};
use crate::domain::{Organization, OrganizationRepository, Repo};

/// A refusal leaves the item out, and so does a caller tachyon does not
/// know in that tenant (delegated evaluation answers 404 rather than reveal
/// a membership). Anything else is a failure of the check.
fn admitted(result: errors::Result<()>) -> errors::Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(errors::Error::Forbidden { .. })
        | Err(errors::Error::NotFound { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

async fn reads_every_private_repo(
    auth: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    scope: &dyn MultiTenancyAction,
) -> errors::Result<bool> {
    admitted(
        auth.check_policy(&CheckPolicyInput {
            executor,
            multi_tenancy: scope,
            action: "library:ViewPrivateRepo",
        })
        .await,
    )
}

/// The private repositories among `repos` the caller may read, by the rule
/// `authorize_private_repo_read` applies to one: the organization-wide
/// `library:ViewPrivateRepo`, else `library:ViewRepo` on the repository.
pub async fn readable_private_repos(
    auth: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    scope: &dyn MultiTenancyAction,
    repos: Vec<Repo>,
) -> errors::Result<Vec<Repo>> {
    let private: Vec<Repo> = repos
        .into_iter()
        .filter(|repo| !*repo.is_public())
        .collect();
    if private.is_empty() || executor.is_none() {
        return Ok(Vec::new());
    }
    if reads_every_private_repo(auth, executor, scope).await? {
        return Ok(private);
    }
    let mut readable = Vec::new();
    for repo in private {
        let resource_trn = format!("trn:library:repo:{}", repo.id());
        if admitted(
            auth.check_policy_for_resource(&CheckPolicyForResourceInput {
                executor,
                multi_tenancy: scope,
                action: "library:ViewRepo",
                resource_trn: &resource_trn,
            })
            .await,
        )? {
            readable.push(repo);
        }
    }
    Ok(readable)
}

/// The Library organizations among `tenants` whose private repositories the
/// caller may read through delegated evaluation.
pub async fn organizations_reached(
    auth: &dyn AuthApp,
    org_scopes: &OrgScopes,
    organizations: &dyn OrganizationRepository,
    executor: &dyn ExecutorAction,
    tenants: impl IntoIterator<Item = TenantId>,
) -> errors::Result<Vec<Organization>> {
    let mut reached = Vec::new();
    for tenant in tenants {
        let Some(org) = organizations.get_by_id(&tenant).await? else {
            continue;
        };
        let scope: LibraryOrg = org_scopes.scope_for(&org).await;
        if reads_every_private_repo(auth, executor, &scope).await? {
            reached.push(org);
        }
    }
    Ok(reached)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tachyon_sdk::auth::{
        test_helper::create_test_multi_tenancy, Executor, MockAuthApp,
    };
    use value_object::Text;

    use super::*;
    use crate::domain::{RepoId, LIBRARY_TENANT};
    use crate::sdk_auth::SdkAuthApp;

    const ORG: &str = "tn_01kshr165k3wwr8xyk0e6vy36c";

    fn repo(id: &str, username: &str, public: bool) -> Repo {
        Repo::create(
            &id.parse::<RepoId>().unwrap(),
            &ORG.parse().unwrap(),
            &"bernard-square".parse().unwrap(),
            &Text::new(username).unwrap(),
            &username.parse().unwrap(),
            &"us_01testcaller".parse().unwrap(),
            public,
            None,
            vec![],
            vec![],
        )
    }

    /// A caller whose token names Library's platform only.
    fn user() -> Executor {
        Executor::User(Box::new(tachyon_sdk::auth::User {
            id: "us_01hs2yepy5hw4rz8pdq2wywnwt".parse().unwrap(),
            username: "mcp-test-user".into(),
            tenants: vec![LIBRARY_TENANT.clone()],
            email: None,
            name: None,
            email_verified: None,
            image: None,
            role: tachyon_sdk::auth::DefaultRole::General,
            metadata: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }))
    }

    /// Answers the organization-wide check with `org_wide` and allows the
    /// per-repository check only for `allowed_repo`; records every check.
    fn auth(
        org_wide: errors::Result<()>,
        allowed_repo: &'static str,
        calls: Arc<Mutex<Vec<String>>>,
    ) -> MockAuthApp {
        let org_wide = Arc::new(Mutex::new(Some(org_wide)));
        let mut auth = MockAuthApp::new();
        let policy_calls = calls.clone();
        auth.expect_check_policy().returning(move |input| {
            policy_calls.lock().unwrap().push(input.action.to_string());
            let answer = match org_wide.lock().unwrap().as_ref().unwrap() {
                Ok(()) => Ok(()),
                Err(errors::Error::NotFound { .. }) => {
                    Err(errors::Error::not_found("user"))
                }
                Err(errors::Error::Forbidden { .. }) => {
                    Err(errors::Error::forbidden("denied"))
                }
                Err(_) => Err(errors::Error::service_unavailable("down")),
            };
            Box::pin(async move { answer })
        });
        auth.expect_check_policy_for_resource()
            .returning(move |input| {
                calls.lock().unwrap().push(input.resource_trn.to_string());
                let allowed = input.resource_trn
                    == format!("trn:library:repo:{allowed_repo}");
                Box::pin(async move {
                    if allowed {
                        Ok(())
                    } else {
                        Err(errors::Error::forbidden("denied"))
                    }
                })
            });
        auth
    }

    fn repos() -> Vec<Repo> {
        vec![
            repo("rp_01m5sp2x5bxpe0brn0ja2c2xwd", "recipes", false),
            repo("rp_01m7rvyh5wrdyp20mcjth2380v", "knowledge", false),
            repo("rp_01m7s5d3q8n2k4f6h8j0m2p4r6", "handbook", true),
        ]
    }

    fn names(repos: &[Repo]) -> Vec<String> {
        repos
            .iter()
            .map(|r| r.username().value().to_string())
            .collect()
    }

    #[tokio::test]
    async fn private_repos_follow_the_read_rule() {
        let scope = create_test_multi_tenancy();
        let executor = user();

        // Organization-wide access: every private repository, one check.
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth_app = auth(Ok(()), "", calls.clone());
        let listed =
            readable_private_repos(&auth_app, &executor, &scope, repos())
                .await
                .unwrap();
        assert_eq!(names(&listed), ["recipes", "knowledge"]);
        assert_eq!(calls.lock().unwrap().len(), 1);

        // Without it, only a repository granted on its own.
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth_app = auth(
            Err(errors::Error::forbidden("denied")),
            "rp_01m7rvyh5wrdyp20mcjth2380v",
            calls.clone(),
        );
        let listed =
            readable_private_repos(&auth_app, &executor, &scope, repos())
                .await
                .unwrap();
        assert_eq!(names(&listed), ["knowledge"]);

        // Not a member there: nothing private.
        let auth_app = auth(
            Err(errors::Error::not_found("user")),
            "",
            Arc::new(Mutex::new(Vec::new())),
        );
        let listed =
            readable_private_repos(&auth_app, &executor, &scope, repos())
                .await
                .unwrap();
        assert!(listed.is_empty());

        // A failing check is a failure, not an empty list.
        let auth_app = auth(
            Err(errors::Error::service_unavailable("down")),
            "",
            Arc::new(Mutex::new(Vec::new())),
        );
        assert!(readable_private_repos(
            &auth_app,
            &executor,
            &scope,
            repos()
        )
        .await
        .is_err());

        // An anonymous caller is never asked about.
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth_app = auth(Ok(()), "", calls.clone());
        let listed = readable_private_repos(
            &auth_app,
            &Executor::None,
            &scope,
            repos(),
        )
        .await
        .unwrap();
        assert!(listed.is_empty());
        assert!(calls.lock().unwrap().is_empty());
    }

    #[derive(Debug)]
    struct Orgs(Vec<Organization>);

    #[async_trait::async_trait]
    impl OrganizationRepository for Orgs {
        async fn insert(&self, _: &Organization) -> errors::Result<()> {
            Ok(())
        }
        async fn update(&self, _: &Organization) -> errors::Result<()> {
            Ok(())
        }
        async fn get_by_id(
            &self,
            id: &TenantId,
        ) -> errors::Result<Option<Organization>> {
            Ok(self.0.iter().find(|org| org.id() == id).cloned())
        }
        async fn get_by_username(
            &self,
            _: &value_object::Identifier,
        ) -> errors::Result<Option<Organization>> {
            Ok(None)
        }
        async fn find_all(&self) -> errors::Result<Vec<Organization>> {
            Ok(self.0.clone())
        }
        async fn delete(&self, _: &TenantId) -> errors::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn only_reached_library_organizations_are_listed() {
        let org = Organization::new(
            &ORG.parse().unwrap(),
            &Text::new("Bernard Square").unwrap(),
            &"bernard-square".parse().unwrap(),
            None,
            None,
        );
        let organizations = Arc::new(Orgs(vec![org]));
        let scopes = OrgScopes::new(
            Arc::new(SdkAuthApp::new(
                "http://127.0.0.1:9",
                &LIBRARY_TENANT,
                "unused",
            )),
            organizations.clone(),
        );
        let executor = user();
        // The caller's tenants: the organization, and one with no Library
        // organization.
        let tenants = || {
            vec![
                ORG.parse::<TenantId>().unwrap(),
                "tn_01j702ts2yy0cemmb16s7sgyfp".parse().unwrap(),
            ]
        };

        let allowed = auth(Ok(()), "", Arc::new(Mutex::new(Vec::new())));
        let reached = organizations_reached(
            &allowed,
            &scopes,
            organizations.as_ref(),
            &executor,
            tenants(),
        )
        .await
        .unwrap();
        assert_eq!(reached.len(), 1);
        assert_eq!(reached[0].id().as_str(), ORG);

        let refused = auth(
            Err(errors::Error::forbidden("denied")),
            "",
            Arc::new(Mutex::new(Vec::new())),
        );
        let reached = organizations_reached(
            &refused,
            &scopes,
            organizations.as_ref(),
            &executor,
            tenants(),
        )
        .await
        .unwrap();
        assert!(reached.is_empty());
    }
}
