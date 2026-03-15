use std::sync::Arc;

use super::input;
use super::model::{
    ApiKeyResponse, Data, GitHubAuthUrl, GitHubConnection, Operator,
    Organization, Property, PropertyType, Repo, Source, SyncResult, User,
};
use crate::app::LibraryApp;
use crate::sdk_auth::SdkAuthApp;
use crate::usecase::{self};
use async_graphql::{
    ErrorExtensions, InputObject, Object, OneofObject, Result,
};
use database_manager::domain::SelectItemId;
use github_provider::OAuthProvider;
use hmac::{Hmac, Mac};
use outbound_sync::{SyncDataInputData, SyncPayload, SyncTarget};
use sha2::Sha256;
use tachyon_sdk::auth::ExecutorAction;
use value_object::{
    IdOrEmail as ValueIdOrEmail, OperatorId, PlatformId, Text, Url, UserId,
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

    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if signature != expected_signature {
        return Err(errors::Error::bad_request(
            "Invalid OAuth state: signature mismatch",
        ));
    }

    Ok(state.to_string())
}

/// Get OAuth state secret from GitHub client or environment variable.
fn get_oauth_state_secret(
    github: &github_provider::GitHub,
) -> errors::Result<String> {
    // First try to get from GitHub client (IAC config)
    if let Some(secret) = github.client_secret() {
        return Ok(secret.to_string());
    }

    // Fall back to environment variables
    std::env::var("OAUTH_STATE_SECRET")
        .or_else(|_| std::env::var("GITHUB_CLIENT_SECRET"))
        .map_err(|_| {
            errors::Error::internal_server_error(
                "GitHub OAuth not configured. Please configure GitHub provider in IAC manifest.",
            )
        })
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
    /// TODO: add English documentation
    #[tracing::instrument(name = "health_check", skip_all)]
    async fn check(&self) -> String {
        "ok".to_string()
    }

    /// [AUTH] Verify the token and return the user
    #[tracing::instrument(skip(self, ctx))]
    async fn verify(
        &self,
        ctx: &async_graphql::Context<'_>,
        token: String,
    ) -> Result<User> {
        let sdk = ctx.data::<Arc<SdkAuthApp>>()?;

        let user = sdk.verify_token(&token).await.map_err(|e| {
            tracing::error!("error: {:?}", e);
            e.extend()
        })?;

        Ok(user.into())
    }

    /// [AUTH] Sign in or sign up via platform access token (library)
    #[tracing::instrument(skip(self, ctx))]
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
            .execute(platform_id.parse()?, access_token, allow_sign_up)
            .await
            .map_err(|e| {
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        Ok(user.into())
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
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        let operator = crate::sdk_auth::operator_from_resp(&resp.operator)
            .map_err(|e| e.extend())?;
        Ok(operator.into())
    }

    /// [LIBRARY-API] Invite a user to an organization with library-specific policy setup
    ///
    /// This wraps auth's InviteUser and additionally:
    /// - Attaches LibraryUserPolicy to the invited user
    /// - If the invited user becomes org owner, attaches repo owner policy
    #[tracing::instrument(skip(self, ctx))]
    async fn invite_user(
        &self,
        ctx: &async_graphql::Context<'_>,
        platform_id: Option<String>,
        tenant_id: String,
        invitee: input::IdOrEmail,
        notify_user: Option<bool>,
        role: Option<crate::domain::OrgRole>,
    ) -> Result<User> {
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let platform_id = platform_id
            .map(|value| value.parse::<PlatformId>())
            .transpose()?;
        let tenant_id = tenant_id.parse::<OperatorId>()?;
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
                tracing::error!("error: {:?}", e);
                e.extend()
            })?;

        Ok(output.user.into())
    }

    /// [LIBRARY-API] Change a user's role in an organization
    ///
    /// Updates the user's DefaultRole and manages library-specific policies:
    /// - If upgrading to Owner, attaches repo owner policy for full repo access
    /// - If downgrading from Owner, detaches repo owner policy
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

        let tenant_id = input.tenant_id.parse::<OperatorId>()?;
        let target_user_id = input.user_id.parse::<UserId>()?;

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

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .create_repo
            .execute(usecase::CreateRepoInputData {
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

        let name = name.map(|value| value.parse()).transpose()?;
        let description =
            description.map(|value| value.parse()).transpose()?;
        let tags = tags
            .map(|values| {
                values
                    .into_iter()
                    .map(|value| value.parse())
                    .collect::<Result<Vec<Text>, _>>()
            })
            .transpose()?;

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
        let _executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let _multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;

        Ok(ctx
            .data::<Arc<LibraryApp>>()?
            .change_repo_username
            .execute(usecase::ChangeRepoUsernameInput {
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

        let (data, properties) = ctx
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
            .await?;

        // Check for ext_github and trigger auto-sync if enabled
        if let Some(ext_github) =
            crate::handler::data::extract_ext_github(&data, &properties)
        {
            if ext_github.enabled {
                tracing::info!(
                    "Auto-syncing new data {} to GitHub: {}/{}",
                    data.id(),
                    ext_github.repo,
                    ext_github.path
                );

                // Build markdown
                let markdown = crate::handler::data::compose_markdown(
                    &data,
                    &properties,
                );

                // Build sync target
                let target = SyncTarget::git_with_branch(
                    &ext_github.repo,
                    &ext_github.path,
                    "main".to_string(),
                );

                // Build payload
                let message =
                    format!("chore(library): auto-sync {}", data.name());
                let payload =
                    SyncPayload::markdown_with_message(&markdown, &message);

                // Execute sync (non-blocking, log errors)
                let sync_data = ctx
                    .data::<Arc<dyn outbound_sync::SyncDataInputPort>>()?;
                match sync_data
                    .execute(&SyncDataInputData {
                        executor,
                        multi_tenancy,
                        data_id: data.id().to_string(),
                        provider: "github".to_string(),
                        target,
                        payload,
                        dry_run: false,
                    })
                    .await
                {
                    Ok(result) => {
                        tracing::info!(
                            "Auto-sync successful: {:?}",
                            result.result_id
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Auto-sync failed: {:?}", e);
                        // Don't fail the add operation, just log the error
                    }
                }
            }
        }

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

        // GitHub sync is handled in the usecase layer
        let (data, _properties) = ctx
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
            .await?;

        Ok(data.into())
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

        // Validate that property name doesn't start with "ext_" (reserved for system extensions)
        // Exception: Allow updating existing ext_* properties if the name is not being changed
        if input.property_name.starts_with("ext_") {
            // Get the existing property to check if this is a rename attempt
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
                properties.iter().find(|p| *p.id() == id);

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

        Ok(app
            .update_property
            .execute(usecase::UpdatePropertyInputData {
                executor,
                multi_tenancy,
                property_id: id,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
                property_name: Some(input.property_name.clone()),
                property_type: Some(&input.try_into()?),
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
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to create organization: {:?}", e);
                e.extend()
            })?
            .into())
    }

    /// TODO: add English documentation
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
                service_account_name: input.service_account_name.as_deref(),
            })
            .await?;

        Ok(ApiKeyResponse {
            api_key: result.api_key.into(),
            service_account: result.service_account.into(),
        })
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
        let source_id = input.source_id.parse()?;

        // TODO: add English comment
        let name = input
            .name
            .map(|name_str| name_str.parse::<Text>())
            .transpose()?;

        let url = match input.url {
            Some(url_opt) => Some(match url_opt {
                Some(url_str) => Some(url_str.parse::<Url>()?),
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
        let source_id_parsed = source_id.parse()?;

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

    // ==================== GitHub OAuth ====================

    /// [LIBRARY-API] Get GitHub OAuth authorization URL
    ///
    /// Signs the state parameter with HMAC-SHA256 for CSRF protection.
    /// The signed state will be validated in github_exchange_token.
    #[tracing::instrument(name = "github_auth_url", skip(self, ctx))]
    async fn github_auth_url(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "State parameter containing encoded return URL")]
        state: String,
    ) -> Result<GitHubAuthUrl> {
        let github = ctx.data::<Arc<github_provider::GitHub>>()?;

        // Get secret from GitHub client (IAC) or environment
        let secret = get_oauth_state_secret(github).map_err(|e| {
            tracing::error!("Failed to get OAuth state secret: {:?}", e);
            e.extend()
        })?;

        // Sign the state for CSRF protection
        let signed_state =
            sign_oauth_state(&state, &secret).map_err(|e| {
                tracing::error!("Failed to sign OAuth state: {:?}", e);
                e.extend()
            })?;

        let url = github
            .authorization_url(
                &github_provider::DEFAULT_SCOPES,
                &signed_state,
            )
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
        skip(self, ctx, code)
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
        let auth_app = ctx.data::<Arc<dyn tachyon_sdk::auth::AuthApp>>()?;
        let github = ctx.data::<Arc<github_provider::GitHub>>()?;

        // Get secret from GitHub client (IAC) or environment
        let secret = get_oauth_state_secret(github).map_err(|e| {
            tracing::error!("Failed to get OAuth state secret: {:?}", e);
            e.extend()
        })?;

        // Verify OAuth state signature for CSRF protection
        let _original_state =
            verify_oauth_state(&state, &secret).map_err(|e| {
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
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // Get data and properties
        let (data, properties) = app
            .view_data
            .execute(&usecase::ViewDataInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
                data_id: input.data_id.clone(),
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to get data: {:?}", e);
                e.extend()
            })?;

        // Generate markdown
        let markdown =
            crate::handler::data::compose_markdown(&data, &properties);

        // Build sync target
        let target = SyncTarget::git_with_branch(
            &input.target_repo,
            &input.target_path,
            input.target_branch.unwrap_or_else(|| "main".to_string()),
        );

        // Build payload
        let message = input.commit_message.unwrap_or_else(|| {
            format!("chore(library): sync {}", data.name())
        });
        let payload =
            SyncPayload::markdown_with_message(&markdown, &message);

        // Execute sync
        let sync_data =
            ctx.data::<Arc<dyn outbound_sync::SyncDataInputPort>>()?;
        let result = sync_data
            .execute(&SyncDataInputData {
                executor,
                multi_tenancy,
                data_id: input.data_id,
                provider: "github".to_string(),
                target,
                payload,
                dry_run: input.dry_run.unwrap_or(false),
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to sync data: {:?}", e);
                e.extend()
            })?;

        Ok(SyncResult {
            success: matches!(
                result.status,
                outbound_sync::SyncStatus::Synced
            ),
            status: result.status.into(),
            result_id: result.result_id,
            url: result.url,
            diff: result.diff,
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
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        let repo_configs = input
            .repo_configs
            .into_iter()
            .map(|c| usecase::ExtGithubRepoConfig {
                repo: c.repo,
                label: c.label,
                default_path: c.default_path,
            })
            .collect();

        let result = app
            .bulk_sync_ext_github
            .execute(usecase::BulkSyncExtGithubInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username,
                repo_username: input.repo_username,
                ext_github_property_id: input.ext_github_property_id,
                repo_configs,
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to bulk sync ext_github: {:?}", e);
                e.extend()
            })?;

        Ok(BulkSyncExtGithubResult {
            updated_count: result.updated_count as i32,
            skipped_count: result.skipped_count as i32,
            total_count: result.total_count as i32,
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
        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

        // Create ext_github property (bypassing the ext_ validation since this is system use)
        let property = app
            .add_property
            .execute(usecase::AddPropertyInputData {
                executor,
                multi_tenancy,
                org_username: input.org_username.clone(),
                repo_username: input.repo_username.clone(),
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

        let executor = ctx.data::<tachyon_sdk::auth::Executor>()?;
        let multi_tenancy =
            ctx.data::<tachyon_sdk::auth::MultiTenancy>()?;
        let app = ctx.data::<Arc<LibraryApp>>()?;

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
                    _ => "STRING".to_string(),
                },
                select_options: m.select_options,
            })
            .collect();

        let result = app
            .import_markdown_from_github
            .execute(usecase::ImportMarkdownFromGitHubInputData {
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
                    .unwrap_or(true),
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

pub use crate::usecase::change_repo_username::ChangeRepoUsernameInput;

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
                ..
            } => Ok(db::PropertyType::Date),
            PropertyInput {
                property_type: PropertyType::Image,
                ..
            } => Ok(db::PropertyType::Image),
            _ => Err(errors::Error::invalid("invalid property type")),
        }
    }
}

#[derive(InputObject, Debug, Clone)]
pub struct PropertyInput {
    pub org_username: String,
    pub repo_username: String,
    pub property_name: String,
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
    /// TODO: add English documentation
    Json(String),
}

#[derive(InputObject, Debug, Clone)]
pub struct OptionInput {
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
