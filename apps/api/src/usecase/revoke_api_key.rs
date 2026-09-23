use std::sync::Arc;

use derive_new::new;
use futures_util::{StreamExt, TryStreamExt};
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyInput, DeleteServiceAccountInput,
    FindAllPublicApiKeyInput, FindAllServiceAccountsInput, PolicyId,
    PublicApiKeyId, RevokePublicApiKeyInput,
};
use value_object::{Identifier, TenantId};

use tachyon_sdk::auth::MultiTenancy;

use super::api_key_issuer::grant_api_key_policy;
use super::GetOrganizationByUsernameQuery;
use crate::domain::{ApiKeyServiceAccount, LIBRARY_TENANT};

#[derive(Debug, Clone)]
pub struct RevokeApiKeyInputData<'a> {
    pub executor: &'a dyn tachyon_sdk::auth::ExecutorAction,
    pub multi_tenancy: &'a dyn tachyon_sdk::auth::MultiTenancyAction,

    pub org_name: &'a Identifier,
    pub api_key_id: &'a str,
}

#[derive(Debug, Clone, new)]
pub struct RevokeApiKey {
    auth_app: Arc<dyn AuthApp>,
    get_org_by_name: Arc<dyn GetOrganizationByUsernameQuery>,
    /// See `library_api_key_issuer_policy_id`.
    api_key_issuer_policy_id: Option<PolicyId>,
}

#[async_trait::async_trait]
pub trait RevokeApiKeyInputPort: std::fmt::Debug + Send + Sync {
    async fn execute<'a>(
        &self,
        input: &RevokeApiKeyInputData<'a>,
    ) -> errors::Result<()>;
}

#[async_trait::async_trait]
impl RevokeApiKeyInputPort for RevokeApiKey {
    #[tracing::instrument(name = "RevokeApiKey::execute", skip(self))]
    async fn execute<'a>(
        &self,
        input: &RevokeApiKeyInputData<'a>,
    ) -> errors::Result<()> {
        self.auth_app
            .check_policy(&CheckPolicyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                action: "library:RevokeApiKey",
            })
            .await?;

        let organization = self
            .get_org_by_name
            .execute(&input.org_name.to_string().parse()?)
            .await?
            .ok_or(errors::not_found!("Organization not found"))?;
        let tenant_id: TenantId = organization.id().to_string().parse()?;
        let api_key_id = PublicApiKeyId::new(input.api_key_id);

        // Removing the key's account is authorized in the organization's
        // own tenant, which is where an owner's repository policy and the
        // account itself live; the v1 web client acts as the Library
        // platform, so the caller's own scope would refuse every owner.
        let org_scope = MultiTenancy::new(
            Some(LIBRARY_TENANT.clone()),
            Some(tenant_id.clone()),
        );

        // Upstream revoke succeeds silently for a key the named account
        // does not hold, so the account holding it is found first. Every
        // account is searched, not only the ones Library names today: keys
        // issued before accounts were Library's own could be put on an
        // account of the caller's choosing, and those still revoke.
        let service_accounts = self
            .auth_app
            .find_all_service_accounts(&FindAllServiceAccountsInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                tenant_id: &tenant_id,
            })
            .await?;
        // A few accounts are asked about at a time, and the search stops
        // at the account that holds the key: an organization has as many
        // accounts as keys, and asking about all of them at once would be
        // as many upstream requests as it has keys.
        let candidates: Vec<_> = service_accounts
            .iter()
            .map(|service_account| {
                (
                    service_account.id().clone(),
                    ApiKeyServiceAccount::from_name(service_account.name()),
                )
            })
            .collect();
        let organization = &organization;
        let api_key_id = &api_key_id;
        let mut lookups = futures_util::stream::iter(candidates)
            .map(|(service_account_id, account)| async move {
                let api_keys = self
                    .auth_app
                    .find_all_public_api_key(&FindAllPublicApiKeyInput {
                        executor: input.executor,
                        multi_tenancy: input.multi_tenancy,
                        operator_id: organization.id(),
                        service_account_id: &service_account_id,
                    })
                    .await?;
                let holds_key =
                    api_keys.iter().any(|key| key.id() == api_key_id);
                Ok::<_, errors::Error>(holds_key.then_some((
                    service_account_id,
                    account,
                    api_keys.len(),
                )))
            })
            .buffered(super::api_key_issuer::CONCURRENT_ACCOUNT_LOOKUPS);

        let mut holder = None;
        while let Some(found) = lookups.try_next().await? {
            if found.is_some() {
                holder = found;
                break;
            }
        }
        let (service_account_id, account, keys_on_account) =
            holder.ok_or(errors::not_found!("API key not found"))?;

        // Taking a key's repository access away is managing repository
        // access, the same as handing it out. It also has to be an
        // owner's to do for a plainer reason: revoking the key leaves its
        // account holding the role, and creating a key on an account is
        // authorized upstream without Library in the way, so anyone left
        // able to do that could mint the access back.
        let key_carries_a_role = matches!(
            account,
            Some(ApiKeyServiceAccount::Dedicated(Some(_)))
        );
        if key_carries_a_role {
            self.auth_app
                .check_policy(&CheckPolicyInput {
                    executor: input.executor,
                    multi_tenancy: &org_scope,
                    action: "library:ManageRepoPolicy",
                })
                .await?;
        }

        self.auth_app
            .revoke_public_api_key(&RevokePublicApiKeyInput {
                executor: input.executor,
                multi_tenancy: input.multi_tenancy,
                operator_id: organization.id(),
                service_account_id: &service_account_id,
                api_key_id,
            })
            .await?;

        // A key's own account has nothing left to authenticate. Removing
        // it is tidying, not revocation: the key is already refused, so a
        // failure here is logged rather than reported. An account Library
        // did not make for this key, or one that still holds another key,
        // stays.
        let account_is_spent =
            matches!(account, Some(ApiKeyServiceAccount::Dedicated(_)))
                && keys_on_account == 1;
        if account_is_spent {
            // Removing an account is authorized upstream as the caller;
            // owners get the grant that allows it (see
            // `grant_api_key_policy`). For anyone else the key is revoked
            // and the account stays: it holds nothing, so it authenticates
            // nobody, and asking upstream to remove it would only be
            // refused.
            let may_manage = key_carries_a_role
                || self
                    .auth_app
                    .check_policy(&CheckPolicyInput {
                        executor: input.executor,
                        multi_tenancy: &org_scope,
                        action: "library:ManageRepoPolicy",
                    })
                    .await
                    .is_ok();

            if may_manage {
                grant_api_key_policy(
                    self.auth_app.as_ref(),
                    self.api_key_issuer_policy_id.as_ref(),
                    input.executor,
                    &org_scope,
                    &tenant_id,
                )
                .await?;

                // Tidying, not revocation: the key is already refused, so
                // a failure here is logged rather than reported.
                if let Err(error) = self
                    .auth_app
                    .delete_service_account(&DeleteServiceAccountInput {
                        executor: input.executor,
                        multi_tenancy: &org_scope,
                        service_account_id: &service_account_id,
                    })
                    .await
                {
                    tracing::warn!(
                        service_account = %service_account_id,
                        error = %error,
                        "revoked key's service account was not removed"
                    );
                }
            } else {
                tracing::info!(
                    service_account = %service_account_id,
                    "left the revoked key's service account to an owner"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Organization;
    use crate::usecase::GetOrganizationByUsernameQuery;
    use async_trait::async_trait;
    use mockall::mock;
    use std::str::FromStr;
    use std::sync::Mutex;
    use tachyon_sdk::auth::{
        test_helper::create_test_multi_tenancy, MockAuthApp, PublicApiKey,
        PublicApiKeyValue, ServiceAccount,
    };
    use value_object::Text;

    mock! {
        #[derive(Debug)]
        GetOrgByUsername {}
        #[async_trait]
        impl GetOrganizationByUsernameQuery for GetOrgByUsername {
            async fn execute(&self, username: &Identifier) -> errors::Result<Option<Organization>>;
        }
    }

    fn org(id: &TenantId) -> Organization {
        Organization::new(
            id,
            &Text::new("Test Organization").unwrap(),
            &Identifier::from_str("test-org").unwrap(),
            None,
            None,
        )
    }

    /// A signed-in person: the issuer grant is given to users only.
    fn user_executor() -> tachyon_sdk::auth::Executor {
        let now = chrono::Utc::now();
        tachyon_sdk::auth::Executor::User(Box::new(
            tachyon_sdk::auth::User {
                id: tachyon_sdk::auth::UserId::new(
                    "us_01hs2yepy5hw4rz8pdq2wywnwt",
                )
                .unwrap(),
                username: "owner".to_string(),
                tenants: vec![TenantId::default()],
                email: None,
                name: None,
                email_verified: None,
                image: None,
                role: tachyon_sdk::auth::DefaultRole::Owner,
                metadata: None,
                created_at: now,
                updated_at: now,
            },
        ))
    }

    type Calls = Arc<Mutex<Vec<String>>>;

    /// An organization with a legacy shared account holding `pak_legacy`
    /// and a reader key `pak_reader` on its own account.
    fn auth(calls: &Calls, owner: bool) -> MockAuthApp {
        let tenant_id = TenantId::default();
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("policy:{}", input.action));
                let denied =
                    !owner && input.action == "library:ManageRepoPolicy";
                Box::pin(async move {
                    if denied {
                        Err(errors::Error::forbidden("denied"))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth.expect_attach_user_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("grant:{}", input.policy_id));
                Box::pin(async { Ok(()) })
            }
        });
        auth.expect_find_all_service_accounts().returning({
            let tenant_id = tenant_id.clone();
            move |_| {
                let accounts = [
                    ("sa_01legacy", "default"),
                    ("sa_01reader", "library-api-key-reader-0123456789abcdef0123456789abcdef"),
                    // A key's own account that somehow holds a second
                    // key: it is not spent when one of them goes.
                    ("sa_01shared", "library-api-key-writer-fedcba9876543210fedcba9876543210"),
                    // A key issued with no repository access, on an
                    // account of its own.
                    ("sa_01public", "library-api-key-public-00112233445566778899aabbccddeeff"),
                    // Named by whoever issued the key, back when the
                    // API took a service account name.
                    ("sa_01custom", "ci-bot"),
                ]
                .map(|(id, name)| ServiceAccount {
                    id: id.to_string().into(),
                    tenant_id: tenant_id.clone(),
                    name: name.to_string(),
                    created_at: chrono::Utc::now(),
                })
                .to_vec();
                Box::pin(async move { Ok(accounts) })
            }
        });
        auth.expect_find_all_public_api_key().returning({
            let tenant_id = tenant_id.clone();
            move |input| {
                let key_ids: &[&str] =
                    match input.service_account_id.as_str() {
                        "sa_01legacy" => &["pak_legacy"],
                        "sa_01custom" => &["pak_custom"],
                        "sa_01public" => &["pak_public"],
                        "sa_01shared" => &["pak_shared", "pak_other"],
                        _ => &["pak_reader"],
                    };
                let keys = key_ids
                    .iter()
                    .map(|key_id| PublicApiKey {
                        id: PublicApiKeyId::new(*key_id),
                        tenant_id: tenant_id.clone(),
                        service_account_id: input
                            .service_account_id
                            .clone(),
                        name: "key".to_string(),
                        value: PublicApiKeyValue::new("pk_****"),
                        created_at: chrono::Utc::now(),
                    })
                    .collect();
                Box::pin(async move { Ok(keys) })
            }
        });
        auth.expect_revoke_public_api_key().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "revoke:{}:{}",
                    input.service_account_id.as_str(),
                    input.api_key_id.as_str(),
                ));
                Box::pin(async { Ok(()) })
            }
        });
        auth.expect_delete_service_account().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "delete-sa:{}",
                    input.service_account_id.as_str()
                ));
                Box::pin(async { Ok(()) })
            }
        });
        auth
    }

    async fn revoke(
        auth: MockAuthApp,
        api_key_id: &str,
    ) -> errors::Result<()> {
        let mut org_query = MockGetOrgByUsername::new();
        org_query
            .expect_execute()
            .returning(|_| Ok(Some(org(&TenantId::default()))));
        let executor = user_executor();
        let multi_tenancy = create_test_multi_tenancy();
        RevokeApiKey::new(
            Arc::new(auth),
            Arc::new(org_query),
            Some(PolicyId::new("pol_01issuer")),
        )
        .execute(&RevokeApiKeyInputData {
            executor: &executor,
            multi_tenancy: &multi_tenancy,
            org_name: &Identifier::from_str("test-org").unwrap(),
            api_key_id,
        })
        .await
    }

    #[tokio::test]
    async fn revoking_a_key_removes_its_own_account_too() {
        let calls = Calls::default();
        revoke(auth(&calls, true), "pak_reader").await.unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "policy:library:RevokeApiKey",
                // Taking repository access away is an owner's to do.
                "policy:library:ManageRepoPolicy",
                "revoke:sa_01reader:pak_reader",
                "grant:pol_01issuer",
                "delete-sa:sa_01reader",
            ]
        );
    }

    /// Someone who may revoke but not manage repository policy cannot
    /// remove a service account either, so the key goes and the account
    /// it emptied is left for an owner rather than asking upstream for
    /// something it will refuse.
    #[tokio::test]
    async fn a_non_owner_revokes_a_key_without_a_role() {
        let calls = Calls::default();
        revoke(auth(&calls, false), "pak_public").await.unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "policy:library:RevokeApiKey",
                "revoke:sa_01public:pak_public",
                "policy:library:ManageRepoPolicy",
            ]
        );
    }

    /// Revoking leaves the account holding the role, and creating a key
    /// on an account is authorized upstream with Library out of the way,
    /// so anyone who could do both could mint the access back.
    #[tokio::test]
    async fn a_non_owner_cannot_revoke_a_key_that_carries_a_role() {
        let calls = Calls::default();
        let result = revoke(auth(&calls, false), "pak_reader").await;

        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "policy:library:RevokeApiKey",
                "policy:library:ManageRepoPolicy",
            ]
        );
    }

    /// Removing the account would take the other key with it.
    #[tokio::test]
    async fn an_account_that_still_holds_another_key_is_kept() {
        let calls = Calls::default();
        revoke(auth(&calls, true), "pak_shared").await.unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "policy:library:RevokeApiKey",
                "policy:library:ManageRepoPolicy",
                "revoke:sa_01shared:pak_shared",
            ]
        );
    }

    /// Keys issued before accounts were Library's own could name any
    /// account. Nothing points at those names now, so the key is found by
    /// searching, and the account -- which may hold other keys -- stays.
    #[tokio::test]
    async fn a_key_on_an_account_library_did_not_name_still_revokes() {
        let calls = Calls::default();
        revoke(auth(&calls, true), "pak_custom").await.unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "policy:library:RevokeApiKey",
                "revoke:sa_01custom:pak_custom",
            ]
        );
    }

    /// The legacy account is shared by every older key, so only the key
    /// goes.
    #[tokio::test]
    async fn revoking_a_legacy_key_keeps_the_shared_account() {
        let calls = Calls::default();
        revoke(auth(&calls, true), "pak_legacy").await.unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "policy:library:RevokeApiKey",
                "revoke:sa_01legacy:pak_legacy",
            ]
        );
    }

    #[tokio::test]
    async fn an_unknown_key_is_not_found_and_nothing_is_revoked() {
        let calls = Calls::default();
        let result = revoke(auth(&calls, true), "pak_missing").await;

        assert!(matches!(result, Err(errors::Error::NotFound { .. })));
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["policy:library:RevokeApiKey"]
        );
    }

    #[tokio::test]
    async fn a_denied_policy_stops_before_anything_is_revoked() {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy().returning(|_| {
            Box::pin(async { Err(errors::Error::forbidden("denied")) })
        });
        auth.expect_find_all_service_accounts().never();
        auth.expect_revoke_public_api_key().never();

        let result = revoke(auth, "pak_reader").await;

        assert!(matches!(result, Err(errors::Error::Forbidden { .. })));
    }
}
