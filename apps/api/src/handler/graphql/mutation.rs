use std::sync::Arc;

use super::input;
use super::model::{
    ApiKeyResponse, Data, GitHubAuthUrl, GitHubConnection, GlobalIdMapping,
    Operator, Organization, PersonalSpace, Property, PropertyType,
    PublicApiKey, Repo, RichTextTemplate, SeedLibraryTenantPayload, Source,
    SyncResult, User,
};
use crate::app::LibraryApp;
use crate::domain::{library_repo_owner_policy_id, library_user_policy_id};
use crate::handler::library_executor_extractor::CallerAuthApp;
use crate::sdk_auth::SdkAuthApp;
use crate::usecase::{self};
use async_graphql::{
    ErrorExtensions, InputObject, Object, OneofObject, Result,
};
use base64::Engine as _;
use database_manager::domain::SelectItemId;
use github_provider::OAuthProvider;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tachyon_sdk::auth::{
    AuthApp as AuthAppTrait, DefaultRole, ExecutorAction,
};
use value_object::{
    IdOrEmail as ValueIdOrEmail, OperatorId, PlatformId, TenantId, Text,
    Url, UserId,
};

type HmacSha256 = Hmac<Sha256>;

/// Sign an OAuth state parameter with HMAC-SHA256 for CSRF protection.
///
/// Returns the state with signature appended: `{state}.{signature_hex}`
fn sign_oauth_state(state: &str, secret: &str) -> errors::Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;
    mac.update(state.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    Ok(format!("{state}.{signature}"))
}

/// Verify an OAuth state parameter signature.
///
/// Expects format: `{state}.{signature_hex}`
/// Returns the original state if valid, or an error if invalid.
fn verify_oauth_state(
    signed_state: &str,
    secret: &str,
) -> errors::Result<String> {
    // Split from the last '.' to separate signature from state
    let (state, signature) =
        signed_state.rsplit_once('.').ok_or_else(|| {
            errors::Error::bad_request("Invalid OAuth state format")
        })?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| errors::Error::internal_server_error(e.to_string()))?;
    mac.update(state.as_bytes());

    let signature = hex::decode(signature).map_err(|_| {
        errors::Error::bad_request("Invalid OAuth state signature")
    })?;
    mac.verify_slice(&signature).map_err(|_| {
        errors::Error::bad_request(
            "Invalid OAuth state: signature mismatch",
        )
    })?;

    Ok(state.to_string())
}

/// How long a GitHub authorization started by `githubAuthUrl` may take to
/// come back to `githubExchangeToken`.
const GITHUB_OAUTH_STATE_TTL_SECS: i64 = 600;

/// Version of the states `mint_github_oauth_state` signs. States signed
/// before they were bound to a caller carry none and are refused.
const GITHUB_OAUTH_STATE_VERSION: i64 = 2;

/// Hosts of the Library web a GitHub authorization may return to.
///
/// The same list the shared Tachyon callback proxy enforces
/// (`GITHUB_OAUTH_PROXY_RETURN_HOSTS` in tachyon-apps
/// `packages/integration/src/adapter/axum/callback_handler.rs`). The proxy
/// forwards the GitHub `code` to this URL, so Library checks it too
/// rather than relying on the proxy alone.
const GITHUB_OAUTH_RETURN_HOSTS: &[&str] = &[
    "planet-library.txcloud.app",
    "planet-library.pages.dev",
    "library.n1.tachy.one",
];

/// Whether a GitHub authorization may return to `return_url`: HTTPS, no
/// credentials, no port, an allowed host. A development build may also
/// return to a local web server.
fn is_allowed_github_oauth_return_url(return_url: &str) -> bool {
    let Ok(url) = url::Url::parse(return_url) else {
        return false;
    };
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let host = url.host_str();
    if url.scheme() == "https"
        && url.port().is_none()
        && host
            .is_some_and(|host| GITHUB_OAUTH_RETURN_HOSTS.contains(&host))
    {
        return true;
    }
    cfg!(debug_assertions)
        && matches!(url.scheme(), "http" | "https")
        && host.is_some_and(|host| {
            matches!(host, "localhost" | "127.0.0.1" | "[::1]")
        })
}

/// Who a GitHub authorization was started by, and for which tenant's
/// connection.
#[derive(Debug, Clone, Copy)]
struct GitHubOAuthBinding<'a> {
    user_id: &'a str,
    tenant_id: &'a str,
}

/// Mint the `state` of a GitHub authorization for `binding`.
///
/// The caller's state is read only for what the web page needs back: the
/// return URL and the nonce it keeps in sessionStorage. What decides
/// whether the callback is accepted — who started it, for which tenant,
/// until when — is set here and signed, so this is no longer an oracle that
/// signs whatever it is handed.
///
/// The result is `{base64 payload}.{hmac}`, which the v1 page decodes to
/// find its nonce. With `proxy_compatible` it is wrapped in the base64 JSON
/// envelope the shared callback proxy reads its `returnUrl` from.
fn mint_github_oauth_state(
    client_state: &str,
    binding: GitHubOAuthBinding<'_>,
    secret: &str,
    now: i64,
    proxy_compatible: bool,
) -> errors::Result<String> {
    let client = decode_github_oauth_state(client_state)?;
    let return_url = client["returnUrl"].as_str();
    if proxy_compatible && return_url.is_none() {
        return Err(errors::Error::bad_request("Missing OAuth return URL"));
    }
    if return_url
        .is_some_and(|url| !is_allowed_github_oauth_return_url(url))
    {
        return Err(errors::Error::bad_request(
            "OAuth return URL is not an allowed destination",
        ));
    }
    let payload = serde_json::json!({
        "v": GITHUB_OAUTH_STATE_VERSION,
        "returnUrl": return_url,
        "nonce": client["nonce"].as_str(),
        "userId": binding.user_id,
        "tenantId": binding.tenant_id,
        "expiresAt": now + GITHUB_OAUTH_STATE_TTL_SECS,
        "sid": hex::encode(rand::random::<[u8; 16]>()),
    });
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        serde_json::to_vec(&payload).map_err(|_| {
            errors::Error::bad_request("Invalid OAuth state")
        })?,
    );
    let signed = sign_oauth_state(&encoded, secret)?;
    if !proxy_compatible {
        return Ok(signed);
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(
        serde_json::to_vec(&serde_json::json!({"returnUrl": return_url, "signedState": signed}))
            .map_err(|_| errors::Error::bad_request("Invalid OAuth state"))?,
    ))
}

fn decode_github_oauth_state(
    encoded: &str,
) -> errors::Result<serde_json::Value> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
        errors::Error::bad_request("Invalid OAuth state")
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|_| errors::Error::bad_request("Invalid OAuth state"))
}

/// Accept a GitHub callback only for the authorization `binding` started,
/// within its lifetime.
///
/// Nothing records that a state was used: Library keeps no store shared
/// across its instances for it. A replay gains nothing, though: the state
/// is bound to the caller and tenant that started it, and GitHub accepts
/// each code once.
fn verify_github_oauth_callback(
    state: &str,
    secret: &str,
    binding: GitHubOAuthBinding<'_>,
    now: i64,
) -> errors::Result<()> {
    let (signed, envelope_return_url) = if state.contains('.') {
        (state.to_string(), None)
    } else {
        let envelope = decode_github_oauth_state(state)?;
        let signed = envelope["signedState"].as_str().ok_or_else(|| {
            errors::Error::bad_request("Missing signed OAuth state")
        })?;
        (signed.to_string(), Some(envelope["returnUrl"].clone()))
    };
    let encoded = verify_oauth_state(&signed, secret)?;
    let payload = decode_github_oauth_state(&encoded)?;
    let return_url_allowed = match payload["returnUrl"].as_str() {
        Some(url) => is_allowed_github_oauth_return_url(url),
        None => envelope_return_url.is_none(),
    };
    if payload["v"].as_i64() != Some(GITHUB_OAUTH_STATE_VERSION)
        || payload["userId"].as_str() != Some(binding.user_id)
        || payload["tenantId"].as_str() != Some(binding.tenant_id)
        || !payload["sid"].as_str().is_some_and(|sid| !sid.is_empty())
        || !payload["expiresAt"].as_i64().is_some_and(|expiry| {
            expiry > now && expiry <= now + GITHUB_OAUTH_STATE_TTL_SECS
        })
        || !return_url_allowed
        || envelope_return_url
            .is_some_and(|return_url| return_url != payload["returnUrl"])
    {
        return Err(errors::Error::bad_request(
            "Expired or mismatched OAuth state",
        ));
    }
    Ok(())
}

/// Get OAuth state secret from the IaC configuration or an environment
/// variable.
///
/// Resolving the IaC value here rather than at startup is what keeps
/// the tachyon-api round trip off the cold-start path; this is one of
/// the few places that needs it.
async fn get_oauth_state_secret(
    oauth_bootstrap: &crate::oauth_bootstrap::OAuthBootstrap,
) -> errors::Result<String> {
    let configuration_error =
        match oauth_bootstrap.github_client_secret().await {
            Ok(secret) => return Ok(secret),
            Err(error) => error,
        };

    // Fall back to environment variables
    std::env::var("OAUTH_STATE_SECRET")
        .or_else(|_| std::env::var("GITHUB_CLIENT_SECRET"))
        .map_err(|_| configuration_error)
}

#[derive(Default)]
pub struct LibraryMutation;

fn to_value_id_or_email(
    input: input::IdOrEmail,
) -> errors::Result<ValueIdOrEmail<UserId>> {
    match input {
        input::IdOrEmail::Id(id) => Ok(ValueIdOrEmail::Id(id.parse()?)),
        input::IdOrEmail::Email(email) => {
            Ok(ValueIdOrEmail::Email(email.parse()?))
        }
    }
}

#[Object]
impl LibraryMutation {
    /// Delete a data item from a Library repository.
    #[tracing::instrument(name = "health_check", skip_all)]
    async fn check(&self) -> String {
        "ok".to_string()
    }

    #[tracing::instrument(
        name = "save_rich_text_template",
        skip(self, ctx)
    )]
    async fn save_rich_text_template(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::RichTextTemplateInput,
    ) -> Result<RichTextTemplate> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        ctx.data::<Arc<LibraryApp>>()?
            .save_rich_text_template
            .execute(&usecase::SaveRichTextTemplateInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                name: input.name,
                rich_text: input.rich_text,
            })
            .await
            .map(Into::into)
            .map_err(|error| error.extend())
    }

    #[tracing::instrument(
        name = "update_rich_text_template",
        skip(self, ctx)
    )]
    async fn update_rich_text_template(
        &self,
        ctx: &async_graphql::Context<'_>,
        template_id: String,
        input: input::RichTextTemplateInput,
    ) -> Result<RichTextTemplate> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        ctx.data::<Arc<LibraryApp>>()?
            .update_rich_text_template
            .execute(&usecase::UpdateRichTextTemplateInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                template_id,
                name: input.name,
                rich_text: input.rich_text,
            })
            .await
            .map(Into::into)
            .map_err(|error| error.extend())
    }

    #[tracing::instrument(
        name = "delete_rich_text_template",
        skip(self, ctx)
    )]
    async fn delete_rich_text_template(
        &self,
        ctx: &async_graphql::Context<'_>,
        org_username: String,
        repo_username: String,
        template_id: String,
    ) -> Result<bool> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        ctx.data::<Arc<LibraryApp>>()?
            .delete_rich_text_template
            .execute(&usecase::DeleteRichTextTemplateInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
                template_id,
            })
            .await
            .map_err(|error| error.extend())
    }

    /// [AUTH] Verify the token and return the user
    #[tracing::instrument(skip(self, ctx, token))]
    async fn verify(
        &self,
        ctx: &async_graphql::Context<'_>,
        token: String,
    ) -> Result<User> {
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;

        let user = sdk.verify_token(&token).await.map_err(|e| {
            super::log_graphql_operation_error("library_mutation", &e);
            e.extend()
        })?;

        Ok(user.into())
    }

    /// [AUTH] Sign in or sign up via platform access token (library)
    #[tracing::instrument(skip(self, ctx, access_token))]
    #[graphql(name = "signInWithPlatform")]
    async fn sign_in(
        &self,
        ctx: &async_graphql::Context<'_>,
        platform_id: String,
        access_token: String,
        allow_sign_up: Option<bool>,
    ) -> Result<User> {
        let library_app = ctx.data::<Arc<LibraryApp>>()?;

        let user = library_app
            .sign_in
            .execute(
                platform_id
                    .parse()
                    .map_err(|e| errors::Error::from(e).extend())?,
                access_token,
                allow_sign_up,
            )
            .await
            .map_err(|e| {
                super::log_graphql_operation_error("library_mutation", &e);
                e.extend()
            })?;

        Ok(user.into())
    }

    /// [LIBRARY-API] Seed a tachyon tenant into Library organizations.
    #[tracing::instrument(skip(self, ctx))]
    #[graphql(name = "seedLibraryTenant")]
    async fn seed_library_tenant(
        &self,
        ctx: &async_graphql::Context<'_>,
        tenant_id: String,
    ) -> Result<SeedLibraryTenantPayload> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let library_app = ctx.data::<Arc<LibraryApp>>()?;
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;

        let tenant_id =
            TenantId::new(&tenant_id).map_err(|e| e.extend())?;
        let user = sdk.get_caller_user().await.map_err(|e| e.extend())?;

        if !user.tenants().contains(&tenant_id) {
            return Err(errors::permission_denied!(
                "User is not a member of this tenant"
            )
            .extend());
        }

        ensure_tenant_seed_admin(
            library_app.auth_app.as_ref(),
            executor,
            &tenant_id,
        )
        .await
        .map_err(|e| e.extend())?;

        let operator = sdk
            .get_operator(tenant_id.as_ref())
            .await
            .map_err(|e| e.extend())?
            .ok_or_else(|| async_graphql::Error::new("Tenant not found"))?;

        let home_platform = operator
            .platform_id
            .parse::<value_object::PlatformId>()
            .map_err(|e| errors::Error::from(e).extend())?;
        let existing = library_app
            .organization_repo
            .get_by_id(&tenant_id)
            .await
            .map_err(|e| e.extend())?;
        let seeded = existing.is_none();
        let organization = match existing {
            Some(org) => org
                .place_on_home_platform(&home_platform)
                .map_err(|e| e.extend())?,
            None => crate::domain::Organization::adopt(
                &tenant_id,
                &Text::new(&operator.name)
                    .map_err(|e| errors::Error::bad_request(e).extend())?,
                &operator
                    .operator_name
                    .parse()
                    .map_err(|e| errors::Error::bad_request(e).extend())?,
                &home_platform,
            ),
        };

        // Every upstream call about the tenant being imported is scoped to
        // that tenant on its own platform: tachyon rejects an operator scope
        // other than the tenant it asks about, and an x-platform-id other
        // than that tenant's platform.
        let tenant_scope =
            usecase::LibraryOrg::for_organization(&organization)
                .ok_or_else(|| {
                    errors::Error::internal_server_error(
                        "an imported organization has no recorded platform",
                    )
                    .extend()
                })?;

        let users = AuthAppTrait::find_users_by_tenant(
            library_app.auth_app.as_ref(),
            &tachyon_sdk::auth::FindUsersByTenantInput {
                executor,
                multi_tenancy: &tenant_scope,
                tenant_id: &tenant_id,
            },
        )
        .await
        .map_err(|e| e.extend())?;

        let policy_id = library_user_policy_id();
        let repo_owner_policy_id = library_repo_owner_policy_id();
        // Grant as the caller, who `ensure_tenant_seed_admin` has just
        // confirmed owns or manages this tenant and therefore holds
        // `AdministratorAccess` in it. Every grant must land before the
        // organization is stored: an organization whose members hold no
        // Library policy is unusable, and a failed grant is retried by
        // importing again (grants are idempotent).
        for tenant_user in &users {
            AuthAppTrait::attach_user_policy(
                library_app.auth_app.as_ref(),
                &tachyon_sdk::auth::AttachUserPolicyInput {
                    executor,
                    multi_tenancy: &tenant_scope,
                    user_id: tenant_user.id(),
                    policy_id: &policy_id,
                    tenant_id: &tenant_id,
                },
            )
            .await
            .map_err(|e| e.extend())?;

            let owns_repos = member_owns_repos(
                library_app.user_policy_mapping_repo.as_ref(),
                tenant_user,
                &tenant_id,
            )
            .await;

            if owns_repos {
                AuthAppTrait::attach_user_policy(
                    library_app.auth_app.as_ref(),
                    &tachyon_sdk::auth::AttachUserPolicyInput {
                        executor,
                        multi_tenancy: &tenant_scope,
                        user_id: tenant_user.id(),
                        policy_id: &repo_owner_policy_id,
                        tenant_id: &tenant_id,
                    },
                )
                .await
                .map_err(|e| e.extend())?;
            }
        }

        // The tenant consents to Library's service account evaluating its
        // users' policies, so Library can authorize them without a grant
        // registered by hand in Library's own tenant (tachyon ADR-0177).
        // Like the grants above, it must land before the organization is
        // stored, and importing again retries it.
        sdk.grant_delegation_consent(executor, &tenant_scope)
            .await
            .map_err(|e| e.extend())?;

        // An existing organization only gains its platform: saving the
        // snapshot read before the grants would overwrite a profile edit made
        // meanwhile.
        if seeded {
            library_app.organization_repo.insert(&organization).await
        } else {
            library_app
                .organization_repo
                .record_home_platform(&tenant_id, &home_platform)
                .await
        }
        .map_err(|e| e.extend())?;

        Ok(SeedLibraryTenantPayload {
            organization: organization.into(),
            seeded,
            staff_count: users.len() as i32,
        })
    }

    /// [AUTH] Create operator via SDK REST call
    #[tracing::instrument(skip(self, ctx))]
    async fn create_operator(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::CreateOperatorInput,
    ) -> Result<Operator> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;

        let resp = sdk
            .create_operator_rest(
                executor,
                multi_tenancy,
                &crate::sdk_auth::CreateOperatorReq {
                    platform_id: input.platform_id,
                    operator_alias: input.operator_alias,
                    operator_name: input.operator_name,
                    new_operator_owner_method: match input
                        .new_operator_owner_method
                    {
                        tachyon_sdk::auth::NewOperatorOwnerMethod::Inherit => {
                            "Inherit".to_string()
                        }
                        tachyon_sdk::auth::NewOperatorOwnerMethod::Create => {
                            "Create".to_string()
                        }
                    },
                    new_operator_owner_id: input.new_operator_owner_id,
                    new_operator_owner_password: input
                        .new_operator_owner_password,
                },
            )
            .await
            .map_err(|e| {
                super::log_graphql_operation_error("library_mutation", &e);
                e.extend()
            })?;

        let operator = crate::sdk_auth::operator_from_resp(&resp.operator)
            .map_err(|e| e.extend())?;
        Ok(operator.into())
    }

    /// [LIBRARY-API] Invite a user to an organization with library-specific policy setup
    ///
    /// This wraps auth's InviteUser and grants with the invitation:
    /// - LibraryUserPolicy in the organization's tenant
    /// - the repo owner policy, if the invited user becomes an org owner
    ///
    /// Returns null when the email has no account yet: an invitation was
    /// sent, and the policies are granted when it is accepted.
    #[tracing::instrument(skip(self, ctx))]
    async fn invite_user(
        &self,
        ctx: &async_graphql::Context<'_>,
        platform_id: Option<String>,
        tenant_id: String,
        invitee: input::IdOrEmail,
        notify_user: Option<bool>,
        role: Option<crate::domain::OrgRole>,
    ) -> Result<Option<User>> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let platform_id = platform_id
            .map(|value| value.parse::<PlatformId>())
            .transpose()
            .map_err(|e| errors::Error::from(e).extend())?;
        let tenant_id = tenant_id
            .parse::<OperatorId>()
            .map_err(|e| errors::Error::from(e).extend())?;
        let invitee = to_value_id_or_email(invitee)?;

        let output = app
            .invite_org_member
            .execute(usecase::InviteOrgMemberInputData {
                executor,
                multi_tenancy,
                platform_id: platform_id.as_ref(),
                tenant_id: &tenant_id,
                invitee,
                notify_user,
                role,
            })
            .await
            .map_err(|e| {
                super::log_graphql_operation_error("library_mutation", &e);
                e.extend()
            })?;

        Ok(output.user.map(Into::into))
    }

    /// [LIBRARY-API] Change a user's role in an organization
    ///
    /// The role is carried by Library's policies in the organization's
    /// tenant (tachyon no longer lets the role label be set):
    /// - Owner: attaches the repo owner policy for full repo access
    /// - Manager / General: detaches the repo owner and API key policies,
    ///   whatever role the member held before
    #[tracing::instrument(skip(self, ctx))]
    async fn change_org_member_role(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: ChangeOrgMemberRoleInput,
    ) -> Result<User> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let tenant_id = input
            .tenant_id
            .parse::<OperatorId>()
            .map_err(|e| errors::Error::from(e).extend())?;
        let target_user_id = input
            .user_id
            .parse::<UserId>()
            .map_err(|e| errors::Error::from(e).extend())?;

        let output = app
            .change_org_member_role
            .execute(usecase::ChangeOrgMemberRoleInputData {
                executor,
                multi_tenancy,
                tenant_id: &tenant_id,
                target_user_id: &target_user_id,
                new_role: input.new_role,
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to change org member role: {:?}",
                    e
                );
                e.extend()
            })?;

        Ok(output.user.into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "create_repo", skip(self, ctx))]
    async fn create_repo(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: CreateRepoInput,
    ) -> Result<Repo> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let caller_auth = ctx.data::<CallerAuthApp>()?.auth_app();

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .create_repo
            .execute(usecase::CreateRepoInputData {
                auth: caller_auth,
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_name: input.repo_name.clone(),
                repo_username: input.repo_username.clone(),
                user_id: input.user_id.clone(),
                is_public: input.is_public,
                database_id: input.database_id,
                description: input.description,
                skip_sample_data: false,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to create repo: {:?}", e);
                e.extend()
            })?
            .into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "update_repo", skip(self, ctx))]
    async fn update_repo(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::UpdateRepoInput,
    ) -> Result<Repo> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let input::UpdateRepoInput {
            org_username,
            repo_username,
            name,
            description,
            is_public,
            tags,
        } = input;

        let name = name
            .map(|value| value.parse())
            .transpose()
            .map_err(|e| errors::Error::from(e).extend())?;
        let description = description
            .map(|value| value.parse())
            .transpose()
            .map_err(|e| errors::Error::from(e).extend())?;
        let tags = tags
            .map(|values| {
                values
                    .into_iter()
                    .map(|value| value.parse())
                    .collect::<Result<Vec<Text>, _>>()
            })
            .transpose()
            .map_err(|e| errors::Error::from(e).extend())?;

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .update_repo
            .execute(usecase::UpdateRepoInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
                name,
                description,
                is_public,
                tags,
            })
            .await?
            .into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "delete_repo", skip(self, ctx))]
    async fn delete_repo(
        &self,
        ctx: &async_graphql::Context<'_>,
        org_username: String,
        repo_username: String,
    ) -> Result<String> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let ctx = ctx.data::<Arc<LibraryApp>>()?;

        ctx.delete_repo
            .execute(usecase::DeleteRepoInputData {
                executor,
                multi_tenancy,
                org_username,
                repo_username,
            })
            .await?;
        Ok("ok".to_string())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "change_repo_username", skip(self, ctx))]
    async fn change_repo_username(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: ChangeRepoUsernameInput,
    ) -> Result<Repo> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .change_repo_username
            .execute(usecase::ChangeRepoUsernameInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                old_repo_username: input.old_repo_username,
                new_repo_username: input.new_repo_username,
            })
            .await?
            .into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "create_data", skip(self, ctx))]
    async fn create_data(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: AddDataInputData,
    ) -> Result<Data> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .add_data
            .execute(usecase::AddDataInputData {
                executor,
                multi_tenancy,
                actor: &input.actor,
                org_username: &input.org_username,
                repo_username: &input.repo_username,
                data_name: &input.data_name,
                property_data: input.property_data,
            })
            .await?
            .0
            .into())
    }

    async fn add_data(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: AddDataInputData,
    ) -> Result<Data> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let data = ctx
            .data::<Arc<LibraryApp>>()?
            .add_data
            .execute(usecase::AddDataInputData {
                executor,
                multi_tenancy,
                actor: &input.actor,
                org_username: &input.org_username,
                repo_username: &input.repo_username,
                data_name: &input.data_name,
                property_data: input.property_data.clone(),
            })
            .await?
            .0;

        Ok(data.into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "update_data", skip(self, ctx))]
    async fn update_data(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: UpdateDataInputData,
    ) -> Result<Data> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        let data = ctx
            .data::<Arc<LibraryApp>>()?
            .update_data
            .execute(usecase::UpdateDataInputData {
                executor,
                multi_tenancy,
                actor: &input.actor,
                org_username: &input.org_username,
                repo_username: &input.repo_username,
                data_id: &input.data_id,
                data_name: &input.data_name,
                property_data: input.property_data.clone(),
            })
            .await?
            .0;

        Ok(data.into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "delete_data", skip(self, ctx))]
    async fn delete_data(
        &self,
        ctx: &async_graphql::Context<'_>,
        org_username: String,
        repo_username: String,
        data_id: String,
    ) -> Result<String> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        ctx.data::<Arc<LibraryApp>>()?
            .delete_data
            .execute(usecase::DeleteDataInputData {
                executor,
                multi_tenancy,
                actor: executor.get_id().to_string(),
                org_username,
                repo_username,
                data_id: data_id.clone(),
            })
            .await?;

        Ok(data_id)
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "add_property", skip(self, ctx))]
    async fn add_property(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: PropertyInput,
    ) -> Result<Property> {
        // Validate that property name doesn't start with "ext_" (reserved for system extensions)
        if input.property_name.starts_with("ext_") {
            return Err(errors::Error::bad_request(
                "Property names starting with 'ext_' are reserved for system extensions",
            )
            .into());
        }

        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .add_property
            .execute(usecase::AddPropertyInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
                property_display_name: input.display_name.clone(),
                property_name: input.property_name.clone(),
                property_type: input.clone().try_into()?,
            })
            .await?
            .into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "update_property", skip(self, ctx))]
    async fn update_property(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: String,
        input: PropertyInput,
    ) -> Result<Property> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // Resolve the current definition before converting select metadata so
        // stable option IDs can be retained. Older clients omit `OptionInput.id`;
        // for those requests the converter preserves IDs by identifier.
        let properties = app
            .get_properties
            .execute(usecase::GetPropertiesInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await
            .map_err(|e| e.extend())?;
        let existing_property =
            properties.iter().find(|property| *property.id() == id);

        // Validate that property name doesn't start with "ext_" (reserved for system extensions)
        // Exception: Allow updating existing ext_* properties if the name is not being changed
        if input.property_name.starts_with("ext_") {
            match existing_property {
                Some(prop) if *prop.name() == input.property_name => {
                    // Same name as existing ext_* property - allow update (e.g., meta changes)
                }
                Some(_) => {
                    // Attempting to rename to an ext_* name - not allowed
                    return Err(errors::Error::bad_request(
                        "Property names starting with 'ext_' are reserved for system extensions",
                    )
                    .into());
                }
                None => {
                    // Property not found by ID - will be handled by usecase
                    return Err(errors::Error::bad_request(
                        "Property names starting with 'ext_' are reserved for system extensions",
                    )
                    .into());
                }
            }
        }

        // Extract meta_json if Json meta is provided
        let meta_json = match &input.meta {
            Some(PropertyMetaInput::Json(json)) => Some(Some(json.clone())),
            _ => None,
        };
        let property_type = property_type_from_input(
            input.clone(),
            existing_property.map(|property| property.property_type()),
        )?;

        Ok(app
            .update_property
            .execute(usecase::UpdatePropertyInputData {
                executor,
                multi_tenancy,
                property_id: id,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
                property_name: Some(input.property_name.clone()),
                property_display_name: input.display_name.clone(),
                property_type: Some(&property_type),
                meta_json,
            })
            .await?
            .into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "delete_property", skip(self, ctx))]
    async fn delete_property(
        &self,
        ctx: &async_graphql::Context<'_>,
        org_username: String,
        repo_username: String,
        property_id: String,
    ) -> Result<String> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        ctx.data::<Arc<LibraryApp>>()?
            .delete_property
            .execute(usecase::DeletePropertyInputData {
                executor,
                multi_tenancy,

                org_username,
                repo_username,
                property_id: property_id.clone(),
            })
            .await?;
        Ok(property_id)
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "update_organization", skip(self, ctx))]
    async fn update_organization(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::UpdateOrganizationInput,
    ) -> Result<Organization> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .update_organization
            .execute(&usecase::UpdateOrganizationInputData {
                executor,
                multi_tenancy,
                username: input.username,
                name: input.name,
                description: input.description,
                website: input.website,
            })
            .await?
            .organization
            .into())
    }

    /// Deletes a Library organization and its repositories. The tachyon
    /// tenant behind it is not touched; tachyon removes it on its side.
    #[tracing::instrument(name = "delete_organization", skip(self, ctx))]
    async fn delete_organization(
        &self,
        ctx: &async_graphql::Context<'_>,
        username: String,
    ) -> Result<bool> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        ctx.data::<Arc<LibraryApp>>()?
            .delete_organization
            .execute(&usecase::DeleteOrganizationInputData {
                executor,
                multi_tenancy,
                username,
            })
            .await?;
        Ok(true)
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "create_organization", skip(self, ctx))]
    async fn create_organization(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::CreateOrganizationInput,
    ) -> Result<Organization> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .create_organization
            .execute(&usecase::CreateOrganizationInputData {
                executor,
                multi_tenancy,
                name: input.name,
                username: input.username,
                description: input.description,
                website: input.website,
                personal_owner_user_id: None,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to create organization: {:?}", e);
                e.extend()
            })?
            .into())
    }

    /// Create the signed-in user's personal Library space, or return it if it exists.
    #[tracing::instrument(name = "create_personal_space", skip(self, ctx))]
    async fn create_personal_space(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<PersonalSpace> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;
        let user_id =
            executor.get_user_id().map_err(|error| error.extend())?;

        if let Some(organization) = app
            .organization_repo
            .get_by_personal_owner_user_id(&user_id)
            .await
            .map_err(|error| error.extend())?
        {
            return Ok(organization.into());
        }

        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;
        let caller = sdk.get_caller_user().await.map_err(|error| {
            super::log_graphql_operation_error("library_mutation", &error);
            error.extend()
        })?;
        if caller.id() != &user_id {
            return Err(async_graphql::Error::new(
                "Authenticated user identity did not match the request executor",
            ));
        }

        let username = caller.username().to_string();
        let created = app
            .create_organization
            .execute(&usecase::CreateOrganizationInputData {
                executor,
                multi_tenancy,
                name: caller
                    .name()
                    .map(|name| name.to_string())
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| username.clone()),
                username,
                description: None,
                website: None,
                personal_owner_user_id: Some(user_id.to_string()),
            })
            .await;

        match created {
            Ok(organization) => Ok(organization.into()),
            Err(create_error) => {
                // Two first-use requests can race after both observed no
                // personal space. If the other request committed, return its
                // row; otherwise preserve the original creation failure.
                if let Some(organization) = app
                    .organization_repo
                    .get_by_personal_owner_user_id(&user_id)
                    .await
                    .map_err(|lookup_error| lookup_error.extend())?
                {
                    return Ok(organization.into());
                }
                tracing::error!(
                    error = ?create_error,
                    user_id = %user_id,
                    "Failed to create personal Library space"
                );
                Err(create_error.extend())
            }
        }
    }

    /// [LIBRARY-API] Issue an API key for an organization.
    ///
    /// The returned `apiKey.value` is the only time the key itself is
    /// readable; afterwards only its name and id can be listed.
    #[tracing::instrument(name = "create_api_key", skip(self, ctx))]
    async fn create_api_key(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::CreateApiKeyInput,
    ) -> Result<ApiKeyResponse> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        // TODO: add English comment
        let result = ctx
            .data::<Arc<LibraryApp>>()?
            .create_api_key
            .execute(&usecase::CreateApiKeyInputData {
                executor,
                multi_tenancy,
                org_name: &input.organization_username.parse()?,
                name: &input.name,
                role: input.role,
            })
            .await?;

        Ok(ApiKeyResponse {
            api_key: PublicApiKey::with_role(result.api_key, result.role),
        })
    }

    /// [LIBRARY-API] Revoke an API key so it stops authenticating.
    #[tracing::instrument(name = "revoke_api_key", skip(self, ctx))]
    async fn revoke_api_key(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::RevokeApiKeyInput,
    ) -> Result<bool> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        ctx.data::<Arc<LibraryApp>>()?
            .revoke_api_key
            .execute(&usecase::RevokeApiKeyInputData {
                executor,
                multi_tenancy,
                org_name: &input.organization_username.parse()?,
                api_key_id: &input.api_key_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to revoke API key: {:?}", e);
                e.extend()
            })?;

        Ok(true)
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "create_source", skip(self, ctx))]
    async fn create_source(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::CreateSourceInput,
    ) -> Result<Source> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // TODO: add English comment
        let name = input.name.parse::<Text>().map_err(|e| {
            tracing::trace!("error: {:?}", e);
            async_graphql::Error::new(format!("Invalid name: {e}"))
        })?;

        let url = match input.url {
            Some(url_str) => Some(url_str.parse::<Url>().map_err(|e| {
                tracing::trace!("error: {:?}", e);
                async_graphql::Error::new(format!("Invalid URL: {e}"))
            })?),
            None => None,
        };

        // TODO: add English comment
        if executor.get_id().is_empty() {
            return Err(async_graphql::Error::new("User ID not found"));
        }

        // TODO: add English comment
        let source = app
            .create_source
            .execute(usecase::CreateSourceInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                name: &name,
                url,
            })
            .await?;

        Ok(source.into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "update_source", skip(self, ctx))]
    async fn update_source(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::UpdateSourceInput,
    ) -> Result<Source> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // TODO: add English comment
        let source_id = input
            .source_id
            .parse()
            .map_err(|e| errors::Error::from(e).extend())?;

        // TODO: add English comment
        let name = input
            .name
            .map(|name_str| name_str.parse::<Text>())
            .transpose()
            .map_err(|e| errors::Error::from(e).extend())?;

        let url = match input.url {
            Some(url_opt) => Some(match url_opt {
                Some(url_str) => Some(
                    url_str
                        .parse::<Url>()
                        .map_err(|e| errors::Error::from(e).extend())?,
                ),
                None => None,
            }),
            None => None,
        };

        // TODO: add English comment
        let source = app
            .update_source
            .execute(usecase::UpdateSourceInputData {
                executor,
                multi_tenancy,
                source_id: &source_id,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
                name,
                url,
            })
            .await?;

        Ok(source.into())
    }

    /// TODO: add English documentation
    #[tracing::instrument(name = "delete_source", skip(self, ctx))]
    async fn delete_source(
        &self,
        ctx: &async_graphql::Context<'_>,
        org_username: String,
        repo_username: String,
        source_id: String,
    ) -> Result<String> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // TODO: add English comment
        let source_id_parsed = source_id
            .parse()
            .map_err(|e| errors::Error::from(e).extend())?;

        // TODO: add English comment
        app.delete_source
            .execute(usecase::DeleteSourceInputData {
                executor,
                multi_tenancy,
                source_id: &source_id_parsed,
                org_username,
                repo_username,
            })
            .await?;

        Ok(source_id)
    }

    // ==================== PLT-942 GlobalIdMapping ====================

    /// Create a `global_id_mapping` row for the caller's tenant.
    #[tracing::instrument(
        name = "create_global_id_mapping",
        skip(self, ctx)
    )]
    async fn create_global_id_mapping(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::CreateGlobalIdMappingInput,
    ) -> Result<GlobalIdMapping> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let global_id = input
            .global_id
            .map(|s| s.parse::<crate::domain::GlobalId>())
            .transpose()
            .map_err(|e| {
                async_graphql::Error::new(format!("Invalid global_id: {e}"))
            })?;
        let system = input.system.parse::<Text>().map_err(|e| {
            async_graphql::Error::new(format!("Invalid system: {e}"))
        })?;
        let system_code =
            input.system_code.parse::<Text>().map_err(|e| {
                async_graphql::Error::new(format!(
                    "Invalid system_code: {e}"
                ))
            })?;
        let name = input.name.parse::<Text>().map_err(|e| {
            async_graphql::Error::new(format!("Invalid name: {e}"))
        })?;

        let mapping = app
            .create_global_id_mapping
            .execute(usecase::CreateGlobalIdMappingInputData {
                executor,
                multi_tenancy,
                global_id,
                system,
                system_code,
                name,
            })
            .await?;

        Ok(mapping.into())
    }

    /// Update a `global_id_mapping` row's `name`. Other fields are immutable
    /// in Phase 1; mutation of `system` / `system_code` / `global_id` awaits
    /// Phase 1.5+ (deletion is out of scope for Phase 1).
    #[tracing::instrument(
        name = "update_global_id_mapping",
        skip(self, ctx)
    )]
    async fn update_global_id_mapping(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::UpdateGlobalIdMappingInput,
    ) -> Result<GlobalIdMapping> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let id = input
            .id
            .parse::<crate::domain::GlobalIdMappingId>()
            .map_err(|e| {
                async_graphql::Error::new(format!("Invalid id: {e}"))
            })?;
        let name = input.name.parse::<Text>().map_err(|e| {
            async_graphql::Error::new(format!("Invalid name: {e}"))
        })?;

        let mapping = app
            .update_global_id_mapping
            .execute(usecase::UpdateGlobalIdMappingInputData {
                executor,
                multi_tenancy,
                id,
                name,
            })
            .await?;

        Ok(mapping.into())
    }

    // ==================== GitHub OAuth ====================

    /// Start user authorization using Tachyon's shared GitHub App.
    #[tracing::instrument(name = "github_sync_auth_url", skip_all)]
    async fn github_sync_auth_url(
        &self,
        ctx: &async_graphql::Context<'_>,
        return_url: String,
        code_challenge: String,
    ) -> Result<GitHubAuthUrl> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;
        let result = sdk
            .start_github_oauth(
                executor,
                tenancy,
                &return_url,
                &code_challenge,
            )
            .await
            .map_err(|e| e.extend())?;
        Ok(GitHubAuthUrl {
            url: result.authorization_url,
            state: result.state,
        })
    }

    /// Confirm the browser-bound broker session without exposing tokens.
    #[tracing::instrument(name = "github_sync_complete_oauth", skip_all)]
    async fn github_sync_complete_oauth(
        &self,
        ctx: &async_graphql::Context<'_>,
        session: String,
        code_verifier: String,
    ) -> Result<GitHubConnection> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let tenancy = ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;
        let result = sdk
            .complete_github_oauth(
                executor,
                tenancy,
                &session,
                &code_verifier,
            )
            .await
            .map_err(|e| e.extend())?;
        Ok(GitHubConnection {
            connected: result.connected,
            username: Some(result.username),
            connected_at: None,
            expires_at: Some(result.expires_at),
        })
    }

    /// [LIBRARY-API] Get GitHub OAuth authorization URL
    ///
    /// Mints the state on the server, bound to the signed-in caller, the
    /// organization tenant named by `x-operator-id` and a 10-minute expiry,
    /// and signs it with HMAC-SHA256. github_exchange_token accepts it only
    /// for that caller and tenant.
    #[tracing::instrument(name = "github_auth_url", skip(self, ctx, state))]
    async fn github_auth_url(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(
            desc = "Base64 JSON with the page's returnUrl and nonce; the server adds the binding"
        )]
        state: String,
        #[graphql(default = false)] proxy_compatible: bool,
    ) -> Result<GitHubAuthUrl> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let tenant =
            crate::sdk_auth::github_token_tenant(executor, multi_tenancy)
                .map_err(|e| e.extend())?;
        let github = ctx.data::<Arc<github_provider::GitHub>>()?;
        let oauth_bootstrap =
            ctx.data::<Arc<crate::oauth_bootstrap::OAuthBootstrap>>()?;

        let secret =
            get_oauth_state_secret(oauth_bootstrap).await.map_err(|e| {
                tracing::error!(
                    "Failed to get OAuth state secret: {:?}",
                    e
                );
                e.extend()
            })?;

        let signed_state = mint_github_oauth_state(
            &state,
            GitHubOAuthBinding {
                user_id: executor.get_id(),
                tenant_id: tenant.as_str(),
            },
            &secret,
            chrono::Utc::now().timestamp(),
            proxy_compatible,
        )
        .map_err(|e| e.extend())?;
        let url = github
            .authorization_url(
                &github_provider::DEFAULT_SCOPES,
                &signed_state,
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to generate GitHub auth URL: {:?}",
                    e
                );
                e.extend()
            })?;

        Ok(GitHubAuthUrl {
            url,
            state: signed_state,
        })
    }

    /// [LIBRARY-API] Exchange GitHub OAuth code for token
    ///
    /// Validates the signed state parameter for CSRF protection before
    /// exchanging the code.
    #[tracing::instrument(
        name = "github_exchange_token",
        skip(self, ctx, code, state)
    )]
    async fn github_exchange_token(
        &self,
        ctx: &async_graphql::Context<'_>,
        code: String,
        #[graphql(desc = "State parameter for CSRF verification")]
        state: String,
    ) -> Result<GitHubConnection> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let tenant =
            crate::sdk_auth::github_token_tenant(executor, multi_tenancy)
                .map_err(|e| e.extend())?;
        let auth_app = ctx.data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()?;
        let github = ctx.data::<Arc<github_provider::GitHub>>()?;
        let oauth_bootstrap =
            ctx.data::<Arc<crate::oauth_bootstrap::OAuthBootstrap>>()?;

        let secret =
            get_oauth_state_secret(oauth_bootstrap).await.map_err(|e| {
                tracing::error!(
                    "Failed to get OAuth state secret: {:?}",
                    e
                );
                e.extend()
            })?;

        // The state must be one githubAuthUrl minted for this caller and
        // tenant, so a callback started by someone else cannot connect
        // their GitHub account here.
        verify_github_oauth_callback(
            &state,
            &secret,
            GitHubOAuthBinding {
                user_id: executor.get_id(),
                tenant_id: tenant.as_str(),
            },
            chrono::Utc::now().timestamp(),
        )
        .map_err(|e| {
            tracing::warn!("OAuth state verification failed: {:?}", e);
            e.extend()
        })?;

        // Exchange code for token
        let token = github.exchange_token(&code).await.map_err(|e| {
            tracing::error!("Failed to exchange GitHub token: {:?}", e);
            e.extend()
        })?;

        // Save token to database via AuthApp
        auth_app
            .save_oauth_token(&tachyon_sdk::auth::SaveOAuthTokenInput {
                executor,
                multi_tenancy,
                provider: "github",
                provider_user_id: &token.provider_user_id,
                access_token: &token.access_token,
                refresh_token: token.refresh_token.as_deref(),
                expires_in: token.expires_in,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to save GitHub token: {:?}", e);
                e.extend()
            })?;

        Ok(GitHubConnection {
            connected: true,
            username: Some(token.provider_user_id),
            connected_at: Some(chrono::Utc::now()),
            expires_at: if token.expires_in > 0 {
                Some(
                    chrono::Utc::now()
                        + chrono::Duration::seconds(token.expires_in),
                )
            } else {
                None
            },
        })
    }

    /// [LIBRARY-API] Disconnect GitHub OAuth
    #[tracing::instrument(name = "github_disconnect", skip(self, ctx))]
    async fn github_disconnect(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> Result<bool> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let auth_app = ctx.data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()?;

        auth_app
            .delete_oauth_token(&tachyon_sdk::auth::DeleteOAuthTokenInput {
                executor,
                multi_tenancy,
                provider: "github",
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to delete GitHub token: {:?}", e);
                e.extend()
            })?;

        Ok(true)
    }

    // ==================== Data Sync ====================

    /// [LIBRARY-API] Sync data to GitHub
    #[tracing::instrument(name = "sync_data_to_github", skip(self, ctx))]
    async fn sync_data_to_github(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: SyncToGitHubInput,
    ) -> Result<SyncResult> {
        ensure_experimental_github_sync_available()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let output = app
            .sync_data_to_github
            .execute(usecase::SyncDataToGithubInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                data_id: input.data_id,
                target_repo: input.target_repo,
                target_path: input.target_path,
                target_branch: input.target_branch,
                commit_message: input.commit_message,
                dry_run: input.dry_run.unwrap_or(false),
            })
            .await
            .map_err(|e| e.extend())?;

        let success =
            !matches!(output.status, outbound_sync::SyncStatus::Failed(_));

        Ok(SyncResult {
            success,
            status: output.status.into(),
            result_id: output.result_id,
            url: output.url,
            diff: output.diff,
        })
    }

    // ==================== Bulk Sync ====================

    /// [LIBRARY-API] Bulk sync ext_github property for all data items
    #[tracing::instrument(name = "bulk_sync_ext_github", skip(self, ctx))]
    async fn bulk_sync_ext_github(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: BulkSyncExtGithubInput,
    ) -> Result<BulkSyncExtGithubResult> {
        ensure_experimental_github_sync_available()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let output = app
            .bulk_sync_ext_github
            .execute(usecase::BulkSyncExtGithubInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                ext_github_property_id: input.ext_github_property_id,
                repo_configs: input
                    .repo_configs
                    .into_iter()
                    .map(|config| usecase::ExtGithubRepoConfig {
                        repo: config.repo,
                        label: config.label,
                        default_path: config.default_path,
                        branch: config.branch,
                    })
                    .collect(),
            })
            .await
            .map_err(|e| e.extend())?;

        Ok(BulkSyncExtGithubResult {
            updated_count: output.updated_count as i32,
            skipped_count: output.skipped_count as i32,
            total_count: output.total_count as i32,
        })
    }

    // ==================== Enable GitHub Sync ====================

    /// [LIBRARY-API] Enable GitHub sync by creating the ext_github property (system use only)
    #[tracing::instrument(name = "enable_github_sync", skip(self, ctx))]
    async fn enable_github_sync(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: EnableGitHubSyncInput,
    ) -> Result<EnableGitHubSyncResult> {
        ensure_experimental_github_sync_available()?;
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let properties = app
            .get_properties
            .execute(usecase::GetPropertiesInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await
            .map_err(|e| e.extend())?;

        if let Some(prop) =
            properties.iter().find(|p| p.name() == "ext_github")
        {
            return Ok(EnableGitHubSyncResult {
                success: true,
                property_id: prop.id().to_string(),
            });
        }

        let property = app
            .add_property
            .execute(usecase::AddPropertyInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
                property_display_name: None,
                property_name: "ext_github".to_string(),
                property_type:
                    database_manager::domain::PropertyType::String,
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create ext_github property: {:?}",
                    e
                );
                e.extend()
            })?;

        Ok(EnableGitHubSyncResult {
            success: true,
            property_id: property.id().to_string(),
        })
    }

    /// [LIBRARY-API] Enable Linear sync by creating the ext_linear property (system use only)
    #[tracing::instrument(name = "enable_linear_sync", skip(self, ctx))]
    async fn enable_linear_sync(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: EnableLinearSyncInput,
    ) -> Result<EnableLinearSyncResult> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let properties = app
            .get_properties
            .execute(usecase::GetPropertiesInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await
            .map_err(|e| e.extend())?;

        if let Some(prop) =
            properties.iter().find(|p| p.name() == "ext_linear")
        {
            return Ok(EnableLinearSyncResult {
                success: true,
                property_id: prop.id().to_string(),
            });
        }

        let property = app
            .add_property
            .execute(usecase::AddPropertyInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
                property_display_name: None,
                property_name: "ext_linear".to_string(),
                property_type:
                    database_manager::domain::PropertyType::String,
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to create ext_linear property: {:?}",
                    e
                );
                e.extend()
            })?;

        Ok(EnableLinearSyncResult {
            success: true,
            property_id: property.id().to_string(),
        })
    }

    /// [LIBRARY-API] Disable GitHub sync by deleting the ext_github property
    #[tracing::instrument(name = "disable_github_sync", skip(self, ctx))]
    async fn disable_github_sync(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: DisableGitHubSyncInput,
    ) -> Result<DisableGitHubSyncResult> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // Find and delete ext_github property
        let properties = app
            .get_properties
            .execute(usecase::GetPropertiesInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
            })
            .await
            .map_err(|e| e.extend())?;

        let ext_github_prop =
            properties.iter().find(|p| p.name() == "ext_github");

        if let Some(prop) = ext_github_prop {
            app.delete_property
                .execute(usecase::DeletePropertyInputData {
                    executor,
                    multi_tenancy,
                    org_username: input.org_username,
                    repo_username: input.repo_username,
                    property_id: prop.id().to_string(),
                })
                .await
                .map_err(|e| {
                    tracing::error!(
                        "Failed to delete ext_github property: {:?}",
                        e
                    );
                    e.extend()
                })?;

            Ok(DisableGitHubSyncResult {
                success: true,
                deleted: true,
            })
        } else {
            Ok(DisableGitHubSyncResult {
                success: true,
                deleted: false,
            })
        }
    }

    /// [LIBRARY-API] Invite a user to a repository with a specific role (owner/writer/reader)
    #[tracing::instrument(name = "invite_repo_member", skip(self, ctx))]
    async fn invite_repo_member(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: InviteRepoMemberInput,
    ) -> Result<bool> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        app.invite_repo_member
            .execute(usecase::InviteRepoMemberInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                repo_id: input.repo_id,
                username_or_email: input.username_or_email,
                role: input.role,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to invite repo member: {:?}", e);
                e.extend()
            })?;

        Ok(true)
    }

    /// [LIBRARY-API] Remove a user from a repository
    #[tracing::instrument(name = "remove_repo_member", skip(self, ctx))]
    async fn remove_repo_member(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: RemoveRepoMemberInput,
    ) -> Result<bool> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        app.remove_repo_member
            .execute(usecase::RemoveRepoMemberInputData {
                executor,
                multi_tenancy,
                repo_id: input.repo_id,
                user_id: input.user_id,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to remove repo member: {:?}", e);
                e.extend()
            })?;

        Ok(true)
    }

    /// [LIBRARY-API] Change a user's role in a repository
    #[tracing::instrument(
        name = "change_repo_member_role",
        skip(self, ctx)
    )]
    async fn change_repo_member_role(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: ChangeRepoMemberRoleInput,
    ) -> Result<bool> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        app.change_repo_member_role
            .execute(usecase::ChangeRepoMemberRoleInputData {
                executor,
                multi_tenancy,
                repo_id: input.repo_id,
                user_id: input.user_id,
                new_role: input.new_role,
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to change repo member role: {:?}",
                    e
                );
                e.extend()
            })?;

        Ok(true)
    }

    /// [LIBRARY-API] Import Markdown files from GitHub
    #[tracing::instrument(
        name = "import_markdown_from_github",
        skip(self, ctx)
    )]
    async fn import_markdown_from_github(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: input::ImportMarkdownFromGitHubInput,
    ) -> Result<super::model::ImportMarkdownResult> {
        use super::model::{ImportError, ImportMarkdownResult};

        if input.enable_github_sync.unwrap_or(false) {
            ensure_experimental_github_sync_available()?;
        }

        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;
        let caller_auth = ctx.data::<CallerAuthApp>()?.auth_app();

        // Build property mappings
        let property_mappings: Vec<usecase::PropertyMapping> = input
            .property_mappings
            .into_iter()
            .map(|m| usecase::PropertyMapping {
                frontmatter_key: m.frontmatter_key,
                property_name: m.property_name,
                property_type: match m.property_type {
                    PropertyType::String => "STRING".to_string(),
                    PropertyType::Integer => "INTEGER".to_string(),
                    PropertyType::Html => "HTML".to_string(),
                    PropertyType::Markdown => "MARKDOWN".to_string(),
                    PropertyType::Select => "SELECT".to_string(),
                    PropertyType::MultiSelect => "MULTI_SELECT".to_string(),
                    PropertyType::RichText => "RICH_TEXT".to_string(),
                    _ => "STRING".to_string(),
                },
                select_options: m.select_options,
            })
            .collect();

        let result = app
            .import_markdown_from_github
            .execute(usecase::ImportMarkdownFromGitHubInputData {
                auth: caller_auth,
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                repo_name: input.repo_name,
                github_repo: input.github_repo,
                paths: input.paths,
                ref_name: input.ref_name,
                property_mappings,
                content_property_name: input.content_property_name,
                skip_existing: input.skip_existing.unwrap_or(false),
                enable_github_sync: input
                    .enable_github_sync
                    .unwrap_or(false),
            })
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to import markdown from GitHub: {:?}",
                    e
                );
                e.extend()
            })?;

        Ok(ImportMarkdownResult {
            imported_count: result.imported_count,
            updated_count: result.updated_count,
            skipped_count: result.skipped_count,
            errors: result
                .errors
                .into_iter()
                .map(|e| ImportError {
                    path: e.path,
                    message: e.message,
                })
                .collect(),
            data_ids: result.data_ids,
            repo_id: result.repo_id,
        })
    }
}

fn ensure_experimental_github_sync_available() -> Result<()> {
    inbound_sync_domain::Provider::Github
        .ensure_runtime_available()
        .map_err(|error| error.extend())
}

/// Reject a caller who cannot grant policies inside the tenant.
///
/// The check is a policy evaluation rather than a look at the user's
/// `role`: tachyon authorizes by policy, and the role field the API
/// returns is a label that says nothing about what the caller may do
/// here.
async fn ensure_tenant_seed_admin(
    auth_app: &dyn AuthAppTrait,
    executor: &dyn ExecutorAction,
    tenant_id: &TenantId,
) -> errors::Result<()> {
    if super::caller_can_seed_tenant(auth_app, executor, tenant_id).await? {
        return Ok(());
    }

    Err(errors::permission_denied!(
        "Importing a tenant into Library needs permission to grant policies in it"
    ))
}

/// Whether a member of a tenant being imported should get full access
/// to the new organization's repositories.
///
/// The primary signal is the tachyon administrator policy attached to
/// them inside that tenant, because that is what tachyon itself
/// authorizes against -- `role` is a label on the user record that
/// says nothing about what its holder may do.
///
/// The `role` check stays as a second, OR-ed signal rather than being
/// replaced. Tenants that predate policy management still carry
/// members recorded as `Owner` with nothing attached at all -- half
/// the `OWNER`s in the platform tenant hold no `AdministratorAccess`
/// today -- and dropping them would import an organization whose
/// repositories nobody can manage. That is the worse failure of the
/// two: an extra grant is something an owner can undo from the
/// members screen, an ownerless organization is not.
///
/// A lookup that fails is not fatal: it degrades to the role check and
/// the seed continues, since the alternative is an import that silently
/// leaves the organization ownerless.
async fn member_owns_repos(
    mappings: &dyn tachyon_sdk::auth::UserPolicyMappingRepository,
    member: &tachyon_sdk::auth::User,
    tenant_id: &TenantId,
) -> bool {
    if *member.role() == DefaultRole::Owner {
        return true;
    }

    match mappings.find_policies_by_user(member.id(), tenant_id).await {
        Ok(policies) => policies
            .iter()
            .any(crate::domain::is_tenant_administrator_policy),
        Err(err) => {
            tracing::warn!(
                user = %member.id(),
                tenant = %tenant_id,
                error = ?err,
                "failed to read attached policies during tenant seed; \
                 falling back to the role field"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_tenant_seed_admin, member_owns_repos, LibraryMutation,
    };
    use crate::domain::TENANT_ADMINISTRATOR_POLICY_ID;
    use async_graphql::{EmptySubscription, Object, Schema};
    use chrono::Utc;
    use tachyon_sdk::auth::{
        DefaultRole, EvaluatePoliciesBatchOutcome, MockAuthApp, PolicyId,
        User, UserPolicy, UserPolicyMappingRepository,
    };
    use value_object::{TenantId, UserId};

    struct TestQuery;

    #[Object]
    impl TestQuery {
        async fn health(&self) -> &str {
            "ok"
        }
    }

    #[test]
    fn schema_exposes_sign_in_with_platform_field() {
        let schema =
            Schema::build(TestQuery, LibraryMutation, EmptySubscription)
                .finish();
        let sdl = schema.sdl();

        assert!(
            sdl.contains("signInWithPlatform("),
            "schema must expose the field called by the web sign-in flow"
        );
        assert!(
            sdl.contains("seedLibraryTenant("),
            "schema must expose the Library tenant seed mutation"
        );
        assert!(
            !sdl.contains("\n\tsignIn("),
            "do not expose the implementation method name as the public field"
        );
    }

    /// Regression guard for PLT-1692: seeding must work for non-Library platform tenants.
    #[test]
    fn seed_library_tenant_does_not_reject_non_library_platform_id() {
        let source = include_str!("mutation.rs");
        let marker = "#[cfg(test)]";
        let impl_source = source.split(marker).next().unwrap_or(source);
        assert!(
            !impl_source
                .contains("Tenant does not belong to the Library platform"),
            "seedLibraryTenant must accept non-Library platform tenants"
        );
        assert!(
            impl_source.contains("get_caller_user"),
            "seedLibraryTenant must authorize against all caller tenant memberships"
        );
    }

    /// Tachyon rejects a user listing whose operator scope differs from
    /// the tenant it asks about, and the request's scope is whichever
    /// operator the client app runs under -- never the tenant being
    /// imported. Listing with the request scope failed every import
    /// with "Upstream authorization rejected".
    #[test]
    fn seed_library_tenant_lists_users_in_the_imported_tenant_scope() {
        let source = include_str!("mutation.rs");
        let impl_source =
            source.split("#[cfg(test)]").next().unwrap_or(source);
        let seed_source = impl_source
            .split("async fn seed_library_tenant")
            .nth(1)
            .expect("the seed mutation must exist");
        let listing = seed_source
            .split("find_users_by_tenant(")
            .nth(1)
            .expect("the seed must list the tenant's members");
        let call = listing.split(".await").next().unwrap_or(listing);
        assert!(
            call.contains("multi_tenancy: &tenant_scope"),
            "the member listing must be scoped to the tenant being \
             imported, not the request's operator scope"
        );
    }

    #[tokio::test]
    async fn tenant_seed_admin_allows_a_caller_the_tenant_authorizes() {
        let mut auth_app = MockAuthApp::new();
        auth_app
            .expect_evaluate_policies_batch()
            .returning(|input| {
                let outcomes: Vec<_> = input
                    .actions
                    .iter()
                    .map(|action| EvaluatePoliciesBatchOutcome {
                        action: (*action).to_string(),
                        allowed: true,
                        error: None,
                    })
                    .collect();
                Box::pin(async move { Ok(outcomes) })
            });

        assert!(ensure_tenant_seed_admin(
            &auth_app,
            &executor(),
            &tenant(),
        )
        .await
        .is_ok());
    }

    /// The role field says nothing about what the caller may do: an
    /// `Owner` whose tenant denies the action must still be rejected,
    /// and that is what makes this check policy-driven.
    #[tokio::test]
    async fn tenant_seed_admin_rejects_a_caller_the_tenant_denies() {
        let mut auth_app = MockAuthApp::new();
        auth_app
            .expect_evaluate_policies_batch()
            .returning(|input| {
                let outcomes: Vec<_> = input
                    .actions
                    .iter()
                    .map(|action| EvaluatePoliciesBatchOutcome {
                        action: (*action).to_string(),
                        allowed: false,
                        error: None,
                    })
                    .collect();
                Box::pin(async move { Ok(outcomes) })
            });

        assert!(ensure_tenant_seed_admin(
            &auth_app,
            &executor(),
            &tenant(),
        )
        .await
        .is_err());
    }

    fn tenant() -> TenantId {
        TenantId::new("tn_01j702qf86pc2j35s0kv0gv3gy").unwrap()
    }

    fn executor() -> tachyon_sdk::auth::Executor {
        tachyon_sdk::auth::Executor::User(Box::new(test_user(
            DefaultRole::Owner,
        )))
    }

    fn test_user(role: DefaultRole) -> User {
        let now = Utc::now();
        User {
            id: UserId::new("us_01hs2yepy5hw4rz8pdq2wywnwt").unwrap(),
            username: "test-user".to_string(),
            tenants: vec![
                TenantId::new("tn_01j702qf86pc2j35s0kv0gv3gy").unwrap()
            ],
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            email_verified: None,
            image: None,
            role,
            metadata: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// A `UserPolicyMappingRepository` that answers only the one method
    /// the seed decision reads, with a canned result.
    ///
    /// `None` stands for a lookup the upstream refuses to answer.
    #[derive(Debug)]
    struct StubPolicies(Option<Vec<PolicyId>>);

    impl StubPolicies {
        fn attached(ids: &[&str]) -> Self {
            Self(Some(ids.iter().copied().map(PolicyId::from).collect()))
        }

        fn unavailable() -> Self {
            Self(None)
        }
    }

    #[async_trait::async_trait]
    impl UserPolicyMappingRepository for StubPolicies {
        async fn find_policies_by_user(
            &self,
            _user_id: &UserId,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<PolicyId>> {
            self.0.clone().ok_or_else(|| {
                errors::Error::internal_server_error(
                    "upstream is down".to_string(),
                )
            })
        }

        async fn create_mapping(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn delete_mapping(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn find_users_by_policy(
            &self,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<Vec<UserId>> {
            unimplemented!("not used by these tests")
        }

        async fn exists_mapping(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
        ) -> errors::Result<bool> {
            unimplemented!("not used by these tests")
        }

        async fn create_mapping_with_scope(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
            _resource_scope: &str,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn delete_mapping_with_scope(
            &self,
            _user_id: &UserId,
            _policy_id: &PolicyId,
            _tenant_id: &TenantId,
            _resource_scope: &str,
        ) -> errors::Result<()> {
            unimplemented!("not used by these tests")
        }

        async fn find_by_resource_scope(
            &self,
            _tenant_id: &TenantId,
            _resource_scope: &str,
        ) -> errors::Result<Vec<UserPolicy>> {
            unimplemented!("not used by these tests")
        }
    }

    /// The point of PLT-3885: an administrator the tenant never
    /// labelled `Owner` still owns the imported organization's repos.
    #[tokio::test]
    async fn seed_gives_repo_ownership_to_an_administrator_by_policy() {
        let mappings =
            StubPolicies::attached(&[TENANT_ADMINISTRATOR_POLICY_ID]);

        assert!(
            member_owns_repos(
                &mappings,
                &test_user(DefaultRole::General),
                &tenant(),
            )
            .await
        );
    }

    #[tokio::test]
    async fn seed_withholds_repo_ownership_from_a_plain_member() {
        let mappings =
            StubPolicies::attached(&["pol_01compute_readonly001"]);

        assert!(
            !member_owns_repos(
                &mappings,
                &test_user(DefaultRole::General),
                &tenant(),
            )
            .await
        );
    }

    /// The OR-ed role check: tenants that never adopted policy
    /// management record owners with nothing attached, and importing
    /// them must not leave the organization without a repo owner.
    #[tokio::test]
    async fn seed_still_honours_an_owner_with_no_policies_attached() {
        let mappings = StubPolicies::attached(&[]);

        assert!(
            member_owns_repos(
                &mappings,
                &test_user(DefaultRole::Owner),
                &tenant(),
            )
            .await
        );
    }

    /// A failed lookup degrades to the role check instead of failing
    /// the seed or granting ownership on a guess.
    #[tokio::test]
    async fn seed_falls_back_to_the_role_when_policies_cannot_be_read() {
        assert!(
            member_owns_repos(
                &StubPolicies::unavailable(),
                &test_user(DefaultRole::Owner),
                &tenant(),
            )
            .await
        );
        assert!(
            !member_owns_repos(
                &StubPolicies::unavailable(),
                &test_user(DefaultRole::General),
                &tenant(),
            )
            .await
        );
    }
}

/// Input for inviting a user to a repository
#[derive(Debug, Clone, InputObject)]
pub struct InviteRepoMemberInput {
    /// Organization username
    pub org_username: String,
    /// Repository username
    pub repo_username: String,
    /// Repository ID
    pub repo_id: String,
    /// Username or email of the user to invite
    pub username_or_email: String,
    /// Role to assign: "owner", "writer", or "reader"
    pub role: String,
}

/// Input for removing a user from a repository
#[derive(Debug, Clone, InputObject)]
pub struct RemoveRepoMemberInput {
    /// Repository ID
    pub repo_id: String,
    /// User ID to remove
    pub user_id: String,
}

/// Input for changing a user's role in a repository
#[derive(Debug, Clone, InputObject)]
pub struct ChangeRepoMemberRoleInput {
    /// Repository ID
    pub repo_id: String,
    /// User ID whose role to change
    pub user_id: String,
    /// New role: "owner", "writer", or "reader"
    pub new_role: String,
}

/// Input for changing a user's role in an organization
#[derive(Debug, Clone, InputObject)]
pub struct ChangeOrgMemberRoleInput {
    /// Tenant/Organization ID
    pub tenant_id: String,
    /// User ID whose role to change
    pub user_id: String,
    /// New role to assign
    pub new_role: crate::domain::OrgRole,
}

/// Input for enabling GitHub sync
#[derive(Debug, Clone, InputObject)]
pub struct EnableGitHubSyncInput {
    /// Organization username
    pub org_username: String,
    /// Repository username
    pub repo_username: String,
}

/// Result of enabling GitHub sync
#[derive(Debug, Clone, async_graphql::SimpleObject)]
pub struct EnableGitHubSyncResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The created ext_github property ID
    pub property_id: String,
}

/// Input for enabling Linear sync
#[derive(Debug, Clone, InputObject)]
pub struct EnableLinearSyncInput {
    /// Organization username
    pub org_username: String,
    /// Repository username
    pub repo_username: String,
}

/// Result of enabling Linear sync
#[derive(Debug, Clone, async_graphql::SimpleObject)]
pub struct EnableLinearSyncResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The created ext_linear property ID
    pub property_id: String,
}

/// Input for disabling GitHub sync
#[derive(Debug, Clone, InputObject)]
pub struct DisableGitHubSyncInput {
    /// Organization username
    pub org_username: String,
    /// Repository username
    pub repo_username: String,
}

/// Result of disabling GitHub sync
#[derive(Debug, Clone, async_graphql::SimpleObject)]
pub struct DisableGitHubSyncResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Whether the property was actually deleted
    pub deleted: bool,
}

/// Input for bulk syncing ext_github property
#[derive(Debug, Clone, InputObject)]
pub struct BulkSyncExtGithubInput {
    /// Organization username
    pub org_username: String,
    /// Repository username
    pub repo_username: String,
    /// ext_github property ID
    pub ext_github_property_id: String,
    /// Repository configurations
    pub repo_configs: Vec<ExtGithubRepoConfigInput>,
}

/// Repository configuration for ext_github
#[derive(Debug, Clone, InputObject)]
pub struct ExtGithubRepoConfigInput {
    /// GitHub repository (owner/repo)
    pub repo: String,
    /// Display label (optional)
    pub label: Option<String>,
    /// Default path (optional, supports {{name}} placeholder)
    pub default_path: Option<String>,
    /// Target branch (defaults to "main")
    pub branch: Option<String>,
}

/// Result of bulk sync operation
#[derive(Debug, Clone, async_graphql::SimpleObject)]
pub struct BulkSyncExtGithubResult {
    /// Number of data items updated
    pub updated_count: i32,
    /// Number of data items skipped (already configured)
    pub skipped_count: i32,
    /// Total number of data items
    pub total_count: i32,
}

#[derive(Debug, Clone, InputObject)]
pub struct CreateRepoInput {
    pub org_username: String,
    pub repo_name: String,
    pub repo_username: String,
    pub user_id: String,
    pub is_public: bool,
    /// TODO: add English documentation
    /// TODO: add English documentation
    pub database_id: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
pub struct AddDataInputData {
    pub actor: String,
    pub org_username: String,
    pub repo_username: String,
    pub data_name: String,
    pub property_data: Vec<usecase::PropertyDataInputData>,
}

#[derive(Debug, Clone, InputObject)]
pub struct UpdateDataInputData {
    /// user_id
    pub actor: String,
    pub org_username: String,
    pub repo_username: String,
    pub data_id: String,
    pub data_name: String,
    pub property_data: Vec<usecase::PropertyDataInputData>,
}

#[derive(Debug, Clone, InputObject)]
pub struct ChangeRepoUsernameInput {
    pub org_username: String,
    pub old_repo_username: String,
    pub new_repo_username: String,
}

fn property_type_from_input(
    value: PropertyInput,
    existing: Option<&database_manager::domain::PropertyType>,
) -> errors::Result<database_manager::domain::PropertyType> {
    use database_manager::domain as db;

    if matches!(&value.meta, Some(PropertyMetaInput::Date(_)))
        && !matches!(&value.property_type, PropertyType::Date)
    {
        return Err(errors::Error::invalid(
            "date metadata is only valid for a Date Property",
        ));
    }

    let changes_option_property_type = match existing {
        Some(db::PropertyType::Select(current)) => {
            !current.items().is_empty()
                && value.property_type != PropertyType::Select
        }
        Some(db::PropertyType::MultiSelect(current)) => {
            !current.items().is_empty()
                && value.property_type != PropertyType::MultiSelect
        }
        _ => false,
    };
    if changes_option_property_type {
        return Err(errors::Error::invalid(
            "Select Property type cannot be changed while options exist",
        ));
    }

    fn select_items(
        options: Vec<OptionInput>,
        existing: Option<&[db::SelectItem]>,
    ) -> errors::Result<Vec<db::SelectItem>> {
        let updated = options
            .into_iter()
            .map(|option| {
                let id = match option.id {
                    Some(id) => {
                        let id: SelectItemId = id.parse()?;
                        if !existing.is_some_and(|items| {
                            items.iter().any(|item| item.id() == &id)
                        }) {
                            return Err(errors::Error::invalid(format!(
                                "select option id {id} does not belong to the Property being updated"
                            )));
                        }
                        id
                    }
                    None => existing
                        .and_then(|items| {
                            items
                                .iter()
                                .find(|item| {
                                    item.key().to_string()
                                        == option.identifier
                                })
                                .map(|item| item.id().clone())
                        })
                        .unwrap_or_default(),
                };
                Ok(db::SelectItem::new(
                    id,
                    option.identifier.parse()?,
                    option.label.parse()?,
                ))
            })
            .collect::<errors::Result<Vec<_>>>()?;
        let mut ids = std::collections::HashSet::new();
        let mut identifiers = std::collections::HashSet::new();
        for item in &updated {
            if !ids.insert(item.id()) {
                return Err(errors::Error::invalid(format!(
                    "duplicate select option id {}",
                    item.id()
                )));
            }
            if !identifiers.insert(item.key().to_string()) {
                return Err(errors::Error::invalid(format!(
                    "duplicate select option identifier {}",
                    item.key()
                )));
            }
        }
        if let Some(existing) = existing {
            for item in existing {
                if !updated.iter().any(|updated| updated.id() == item.id())
                {
                    return Err(errors::Error::invalid(format!(
                        "select option id {} cannot be removed while Property values may reference it",
                        item.id()
                    )));
                }
            }
        }
        Ok(updated)
    }

    match &value {
        PropertyInput {
            property_type: PropertyType::Date,
            meta: Some(PropertyMetaInput::Date(date)),
            ..
        } => {
            let updated = db::TypeDate {
                include_time: date.include_time,
                date_format: date
                    .date_format
                    .parse()
                    .map_err(errors::Error::invalid)?,
                time_format: date
                    .time_format
                    .parse()
                    .map_err(errors::Error::invalid)?,
            };
            return Ok(db::PropertyType::Date(updated));
        }
        PropertyInput {
            property_type: PropertyType::Date,
            meta: None,
            ..
        } => {
            return Ok(match existing {
                Some(db::PropertyType::Date(current)) => {
                    db::PropertyType::Date(current.clone())
                }
                _ => db::PropertyType::Date(Default::default()),
            });
        }
        PropertyInput {
            property_type: PropertyType::Date,
            meta: Some(PropertyMetaInput::Json(_)),
            ..
        } => {
            return Ok(match existing {
                Some(db::PropertyType::Date(current)) => {
                    db::PropertyType::Date(current.clone())
                }
                _ => db::PropertyType::Date(Default::default()),
            });
        }
        _ => {}
    }

    match value {
        PropertyInput {
            property_type: PropertyType::Select,
            meta: Some(PropertyMetaInput::Select(options)),
            ..
        } => Ok(db::PropertyType::Select(db::TypeSelect {
            items: select_items(
                options,
                match existing {
                    Some(db::PropertyType::Select(current)) => {
                        Some(current.items())
                    }
                    _ => None,
                },
            )?,
        })),
        PropertyInput {
            property_type: PropertyType::MultiSelect,
            meta: Some(PropertyMetaInput::MultiSelect(options)),
            ..
        } => Ok(db::PropertyType::MultiSelect(db::TypeMultiSelect {
            items: select_items(
                options,
                match existing {
                    Some(db::PropertyType::MultiSelect(current)) => {
                        Some(current.items())
                    }
                    _ => None,
                },
            )?,
        })),
        value => value.try_into(),
    }
}

impl TryFrom<PropertyInput> for database_manager::domain::PropertyType {
    type Error = errors::Error;
    fn try_from(value: PropertyInput) -> Result<Self, Self::Error> {
        use database_manager::domain as db;
        match value {
            PropertyInput {
                property_type: PropertyType::String,
                ..
            } => Ok(db::PropertyType::String),
            PropertyInput {
                property_type: PropertyType::Integer,
                ..
            } => Ok(db::PropertyType::Integer),
            PropertyInput {
                property_type: PropertyType::Html,
                ..
            } => Ok(db::PropertyType::Html),
            PropertyInput {
                property_type: PropertyType::Markdown,
                ..
            } => Ok(db::PropertyType::Markdown),
            PropertyInput {
                property_type: PropertyType::Relation,
                meta: Some(PropertyMetaInput::Relation(database_id)),
                ..
            } => Ok(db::PropertyType::Relation(db::TypeRelation {
                database_id: database_id.parse()?,
            })),
            PropertyInput {
                property_type: PropertyType::Select,
                meta: Some(PropertyMetaInput::Select(options)),
                ..
            } => Ok(db::PropertyType::Select(db::TypeSelect {
                items: options
                    .iter()
                    .map(|o| -> errors::Result<_> {
                        Ok(db::SelectItem::new(
                            SelectItemId::default(),
                            o.identifier.parse()?,
                            o.label.parse()?,
                        ))
                    })
                    .collect::<errors::Result<Vec<_>>>()?,
            })),
            PropertyInput {
                property_type: PropertyType::MultiSelect,
                meta: Some(PropertyMetaInput::MultiSelect(options)),
                ..
            } => Ok(db::PropertyType::MultiSelect(db::TypeMultiSelect {
                items: options
                    .iter()
                    .map(|o| -> errors::Result<_> {
                        Ok(db::SelectItem::new(
                            SelectItemId::default(),
                            o.identifier.parse()?,
                            o.label.parse()?,
                        ))
                    })
                    .collect::<errors::Result<Vec<_>>>()?,
            })),
            PropertyInput {
                property_type: PropertyType::Id,
                meta: Some(PropertyMetaInput::Id(auto_generate)),
                ..
            } => Ok(db::PropertyType::Id(db::TypeId { auto_generate })),
            PropertyInput {
                property_type: PropertyType::Location,
                ..
            } => Ok(db::PropertyType::Location(Default::default())),
            PropertyInput {
                property_type: PropertyType::Date,
                meta: Some(PropertyMetaInput::Date(date)),
                ..
            } => Ok(db::PropertyType::Date(db::TypeDate {
                include_time: date.include_time,
                date_format: date
                    .date_format
                    .parse()
                    .map_err(errors::Error::invalid)?,
                time_format: date
                    .time_format
                    .parse()
                    .map_err(errors::Error::invalid)?,
            })),
            PropertyInput {
                property_type: PropertyType::Date,
                meta: None,
                ..
            } => Ok(db::PropertyType::Date(Default::default())),
            PropertyInput {
                property_type: PropertyType::Date,
                meta: Some(PropertyMetaInput::Json(_)),
                ..
            } => Ok(db::PropertyType::Date(Default::default())),
            PropertyInput {
                property_type: PropertyType::Image,
                ..
            } => Ok(db::PropertyType::Image),
            // The catch-all below means a missing arm is not a compile
            // error: addProperty would simply reject the type with a 400,
            // leaving the settings picker entry dead on arrival.
            PropertyInput {
                property_type: PropertyType::RichText,
                ..
            } => Ok(db::PropertyType::RichText),
            PropertyInput {
                property_type: PropertyType::Boolean,
                ..
            } => Ok(db::PropertyType::Boolean),
            PropertyInput {
                property_type: PropertyType::Decimal,
                meta: Some(PropertyMetaInput::Json(config)),
                ..
            } => {
                let typ = db::PropertyType::from_meta(
                    "DECIMAL",
                    serde_json::from_str(&config)
                        .map_err(errors::Error::invalid)?,
                )?;
                typ.canonical_config()
                    .handler()
                    .validate_config(&typ.canonical_config())?;
                Ok(typ)
            }
            PropertyInput {
                property_type: PropertyType::Quantity,
                meta: Some(PropertyMetaInput::Json(config)),
                ..
            } => {
                let typ = db::PropertyType::from_meta(
                    "QUANTITY",
                    serde_json::from_str(&config)
                        .map_err(errors::Error::invalid)?,
                )?;
                typ.canonical_config()
                    .handler()
                    .validate_config(&typ.canonical_config())?;
                Ok(typ)
            }
            PropertyInput {
                property_type: PropertyType::Formula,
                meta: Some(PropertyMetaInput::Json(config)),
                ..
            } => {
                let typ = db::PropertyType::from_meta(
                    "FORMULA",
                    serde_json::from_str(&config)
                        .map_err(errors::Error::invalid)?,
                )?;
                typ.canonical_config()
                    .handler()
                    .validate_config(&typ.canonical_config())?;
                Ok(typ)
            }
            PropertyInput {
                property_type: PropertyType::Rollup,
                meta: Some(PropertyMetaInput::Json(config)),
                ..
            } => {
                let typ = db::PropertyType::from_meta(
                    "ROLLUP",
                    serde_json::from_str(&config)
                        .map_err(errors::Error::invalid)?,
                )?;
                typ.canonical_config()
                    .handler()
                    .validate_config(&typ.canonical_config())?;
                Ok(typ)
            }
            _ => Err(errors::Error::invalid("invalid property type")),
        }
    }
}

#[derive(InputObject, Debug, Clone)]
pub struct PropertyInput {
    pub org_username: String,
    pub repo_username: String,
    /// Stable key used by API clients to identify this Property. It must start
    /// with an ASCII letter and contain only ASCII letters, digits,
    /// underscores, or hyphens, up to 64 characters.
    pub property_name: String,
    /// Human-readable label shown in the user interface; defaults to the key.
    pub display_name: Option<String>,
    pub property_type: PropertyType,
    pub meta: Option<PropertyMetaInput>,
}

#[derive(OneofObject, Debug, Clone)]
pub enum PropertyMetaInput {
    /// TODO: add English documentation
    Relation(String),
    /// TODO: add English documentation
    Select(Vec<OptionInput>),
    /// TODO: add English documentation
    MultiSelect(Vec<OptionInput>),
    /// TODO: add English documentation
    Id(bool),
    /// Date/time behavior and display formats.
    Date(DateTypeInput),
    /// TODO: add English documentation
    Json(String),
}

#[derive(InputObject, Debug, Clone)]
pub struct DateTypeInput {
    pub include_time: bool,
    pub date_format: String,
    pub time_format: String,
}

#[derive(InputObject, Debug, Clone)]
pub struct OptionInput {
    /// Stable server-issued option ID. Omit when creating a new option.
    pub id: Option<String>,
    pub identifier: String,
    pub label: String,
}

// ==================== Sync Input Types ====================

#[derive(InputObject, Debug, Clone)]
pub struct SyncToGitHubInput {
    /// Organization username
    pub org_username: String,
    /// Repository username
    pub repo_username: String,
    /// Data ID to sync
    pub data_id: String,
    /// Target GitHub repository (owner/repo format)
    pub target_repo: String,
    /// Target path in the repository
    pub target_path: String,
    /// Target branch (defaults to "main")
    pub target_branch: Option<String>,
    /// Custom commit message
    pub commit_message: Option<String>,
    /// If true, only calculate diff without syncing
    pub dry_run: Option<bool>,
}

#[cfg(test)]
mod select_option_input_tests {
    use super::{
        property_type_from_input, OptionInput, PropertyInput,
        PropertyMetaInput, PropertyType,
    };
    use database_manager::domain::{
        PropertyType as DomainPropertyType, SelectItem, SelectItemId,
        TypeSelect,
    };

    fn option(
        id: Option<&SelectItemId>,
        identifier: &str,
        label: &str,
    ) -> OptionInput {
        OptionInput {
            id: id.map(ToString::to_string),
            identifier: identifier.to_string(),
            label: label.to_string(),
        }
    }

    fn input(options: Vec<OptionInput>) -> PropertyInput {
        PropertyInput {
            org_username: "quantum-box".to_string(),
            repo_username: "library".to_string(),
            property_name: "Status".to_string(),
            display_name: None,
            property_type: PropertyType::Select,
            meta: Some(PropertyMetaInput::Select(options)),
        }
    }

    fn existing(id: &SelectItemId) -> DomainPropertyType {
        DomainPropertyType::Select(TypeSelect::new(vec![SelectItem::new(
            id.clone(),
            "todo".parse().expect("identifier"),
            "Todo".parse().expect("label"),
        )]))
    }

    #[test]
    fn explicit_option_id_survives_identifier_and_label_edits() {
        let id = SelectItemId::default();
        let updated = property_type_from_input(
            input(vec![option(Some(&id), "inProgress", "In progress")]),
            Some(&existing(&id)),
        )
        .expect("stable option update");

        let DomainPropertyType::Select(updated) = updated else {
            panic!("expected Select")
        };
        assert_eq!(updated.items()[0].id(), &id);
        assert_eq!(updated.items()[0].key().to_string(), "inProgress");
        assert_eq!(updated.items()[0].name().to_string(), "In progress");
    }

    #[test]
    fn legacy_update_without_id_preserves_id_by_identifier() {
        let id = SelectItemId::default();
        let updated = property_type_from_input(
            input(vec![option(None, "todo", "To do")]),
            Some(&existing(&id)),
        )
        .expect("legacy option update");

        let DomainPropertyType::Select(updated) = updated else {
            panic!("expected Select")
        };
        assert_eq!(updated.items()[0].id(), &id);
        assert_eq!(updated.items()[0].name().to_string(), "To do");
    }

    #[test]
    fn update_rejects_an_option_id_from_another_property() {
        let current_id = SelectItemId::default();
        let foreign_id = SelectItemId::default();
        let error = property_type_from_input(
            input(vec![option(Some(&foreign_id), "todo", "Todo")]),
            Some(&existing(&current_id)),
        )
        .expect_err("foreign option IDs must fail closed");

        assert!(error.to_string().contains("does not belong"));
    }

    #[test]
    fn update_rejects_duplicate_option_ids() {
        let id = SelectItemId::default();
        let error = property_type_from_input(
            input(vec![
                option(Some(&id), "todo", "Todo"),
                option(Some(&id), "done", "Done"),
            ]),
            Some(&existing(&id)),
        )
        .expect_err("duplicate option IDs must fail closed");

        assert!(error.to_string().contains("duplicate select option id"));
    }

    #[test]
    fn update_fails_closed_when_an_existing_option_is_removed() {
        let id = SelectItemId::default();
        let error = property_type_from_input(
            input(Vec::new()),
            Some(&existing(&id)),
        )
        .expect_err("option removal must not leave dangling values");

        assert!(error.to_string().contains("cannot be removed"));
    }

    #[test]
    fn update_fails_closed_when_a_select_with_options_changes_type() {
        let id = SelectItemId::default();
        let mut changed = input(Vec::new());
        changed.property_type = PropertyType::String;
        changed.meta = None;

        let error = property_type_from_input(changed, Some(&existing(&id)))
            .expect_err("Select type changes must retain option identity");

        assert!(error.to_string().contains("type cannot be changed"));
    }
}

#[cfg(test)]
mod github_oauth_state_tests {
    use super::*;

    const SECRET: &str = "test-secret";
    const NOW: i64 = 1_000;
    const RETURN_URL: &str =
        "https://planet-library.txcloud.app/v1beta/acme?tab=settings";

    fn alice() -> GitHubOAuthBinding<'static> {
        GitHubOAuthBinding {
            user_id: "us_alice",
            tenant_id: "tn_acme",
        }
    }

    fn client_state(return_url: &str) -> String {
        base64::engine::general_purpose::STANDARD.encode(
            serde_json::json!({"returnUrl": return_url, "nonce": "page-nonce"})
                .to_string(),
        )
    }

    fn minted(proxy_compatible: bool) -> String {
        mint_github_oauth_state(
            &client_state(RETURN_URL),
            alice(),
            SECRET,
            NOW,
            proxy_compatible,
        )
        .unwrap()
    }

    /// What the v1 settings page reads back: everything before the last
    /// `.` is base64 JSON carrying the nonce it stored.
    fn page_nonce(state: &str) -> String {
        let (encoded, _) = state.rsplit_once('.').unwrap();
        decode_github_oauth_state(encoded).unwrap()["nonce"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn a_minted_state_is_accepted_for_the_caller_and_tenant_that_started_it(
    ) {
        for proxy_compatible in [false, true] {
            let state = minted(proxy_compatible);
            assert!(verify_github_oauth_callback(
                &state,
                SECRET,
                alice(),
                NOW
            )
            .is_ok());
            assert!(verify_github_oauth_callback(
                &state,
                SECRET,
                alice(),
                NOW + GITHUB_OAUTH_STATE_TTL_SECS - 1
            )
            .is_ok());
        }
    }

    #[test]
    fn the_page_still_finds_its_nonce() {
        assert_eq!(page_nonce(&minted(false)), "page-nonce");
    }

    #[test]
    fn the_proxy_envelope_is_base64_json_with_the_return_url() {
        let envelope = decode_github_oauth_state(&minted(true)).unwrap();
        assert_eq!(envelope["returnUrl"], RETURN_URL);
        assert_eq!(
            page_nonce(envelope["signedState"].as_str().unwrap()),
            "page-nonce"
        );
    }

    #[test]
    fn a_state_is_refused_for_another_caller_or_tenant() {
        for proxy_compatible in [false, true] {
            let state = minted(proxy_compatible);
            for binding in [
                GitHubOAuthBinding {
                    user_id: "us_mallory",
                    tenant_id: "tn_acme",
                },
                GitHubOAuthBinding {
                    user_id: "us_alice",
                    tenant_id: "tn_other",
                },
            ] {
                assert!(verify_github_oauth_callback(
                    &state, SECRET, binding, NOW
                )
                .is_err());
            }
        }
    }

    #[test]
    fn a_state_is_refused_once_expired_or_with_another_secret() {
        let state = minted(false);
        assert!(verify_github_oauth_callback(
            &state,
            SECRET,
            alice(),
            NOW + GITHUB_OAUTH_STATE_TTL_SECS
        )
        .is_err());
        assert!(verify_github_oauth_callback(
            &state,
            "wrong-secret",
            alice(),
            NOW
        )
        .is_err());
    }

    #[test]
    fn a_modified_proxy_redirect_is_refused() {
        let mut envelope =
            decode_github_oauth_state(&minted(true)).unwrap();
        envelope["returnUrl"] =
            serde_json::json!("https://library.n1.tachy.one/elsewhere");
        let changed = base64::engine::general_purpose::STANDARD
            .encode(envelope.to_string());
        assert!(verify_github_oauth_callback(
            &changed,
            SECRET,
            alice(),
            NOW
        )
        .is_err());
    }

    /// The endpoint used to sign whatever it was given, so a signature
    /// over a caller's own payload proved nothing. Only a payload minted
    /// with a binding passes now.
    #[test]
    fn a_caller_signed_payload_is_refused() {
        let forged = base64::engine::general_purpose::STANDARD.encode(
            serde_json::json!({
                "returnUrl": RETURN_URL, "userId": "us_alice",
                "tenantId": "tn_acme", "expiresAt": NOW + 60,
            })
            .to_string(),
        );
        let state = sign_oauth_state(&forged, SECRET).unwrap();
        assert!(verify_github_oauth_callback(&state, SECRET, alice(), NOW)
            .is_err());
        let legacy = sign_oauth_state("legacy-payload", SECRET).unwrap();
        assert!(verify_github_oauth_callback(
            &legacy,
            SECRET,
            alice(),
            NOW
        )
        .is_err());
        assert!(verify_github_oauth_callback(
            "untrusted",
            SECRET,
            alice(),
            NOW
        )
        .is_err());
    }

    #[test]
    fn only_library_web_hosts_are_allowed_return_urls() {
        for allowed in [
            "https://planet-library.txcloud.app/v1beta/acme",
            "https://planet-library.pages.dev/x",
            "https://library.n1.tachy.one/x?y=1",
        ] {
            assert!(
                is_allowed_github_oauth_return_url(allowed),
                "{allowed}"
            );
        }
        for refused in [
            "https://attacker.example/x",
            "http://planet-library.txcloud.app/x",
            "https://planet-library.txcloud.app:8443/x",
            "https://user:pw@planet-library.txcloud.app/x",
            "https://planet-library.txcloud.app.attacker.example/x",
            "javascript:alert(1)",
            "not a url",
        ] {
            assert!(
                !is_allowed_github_oauth_return_url(refused),
                "{refused}"
            );
        }
        // Test builds are debug builds, where a local web server is fine.
        assert!(is_allowed_github_oauth_return_url(
            "http://localhost:5010/v1beta/acme"
        ));
    }

    #[test]
    fn minting_refuses_a_return_url_outside_the_allowlist() {
        for proxy_compatible in [false, true] {
            assert!(mint_github_oauth_state(
                &client_state("https://attacker.example/steal"),
                alice(),
                SECRET,
                NOW,
                proxy_compatible,
            )
            .is_err());
        }
        // The proxy cannot route a state without a return URL.
        let no_return = base64::engine::general_purpose::STANDARD
            .encode(serde_json::json!({"nonce": "n"}).to_string());
        assert!(mint_github_oauth_state(
            &no_return,
            alice(),
            SECRET,
            NOW,
            true
        )
        .is_err());
    }
}
