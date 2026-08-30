use crate::domain::SourceRepository;
use crate::domain::VisibilityService;
use crate::domain::{OrganizationRepository, RepoRepository};
use crate::interface_adapter;
use crate::sdk_auth::{
    SdkAuthApp, SdkUserPolicyMappingRepository, SdkUserQuery,
};
use crate::usecase;
use std::sync::Arc;
use tachyon_sdk::auth::AuthApp;

#[derive(Clone)]
pub struct LibraryApp {
    pub view_org: Arc<dyn usecase::ViewOrganizationInputPort>,
    pub update_organization: Arc<dyn usecase::UpdateOrganizationInputPort>,
    pub create_organization: Arc<dyn usecase::CreateOrganizationInputPort>,
    pub create_repo: Arc<dyn usecase::CreateRepoInputPort>,
    pub update_repo: Arc<dyn usecase::UpdateRepoInputPort>,
    pub change_repo_username: Arc<dyn usecase::ChangeRepoUsernameInputPort>,
    pub view_repo: Arc<dyn usecase::ViewRepoInputPort>,
    pub search_data: Arc<dyn usecase::SearchDataInputPort>,
    pub search_repo: Arc<dyn usecase::SearchRepoInputPort>,
    pub add_data: Arc<dyn usecase::AddDataInputPort>,
    pub save_data: Arc<dyn usecase::AddDataInputPort>,
    pub view_data: Arc<dyn usecase::ViewDataInputPort>,
    pub update_data: Arc<dyn usecase::UpdateDataInputPort>,
    pub upsert_data: Arc<dyn usecase::UpsertDataInputPort>,
    pub delete_data: Arc<dyn usecase::DeleteDataInputPort>,
    pub add_property: Arc<dyn usecase::AddPropertyInputPort>,
    pub update_property: Arc<dyn usecase::UpdatePropertyInputPort>,
    pub get_properties: Arc<dyn usecase::GetPropertiesInputPort>,
    pub delete_property: Arc<dyn usecase::DeletePropertyInputPort>,
    pub delete_repo: Arc<dyn usecase::DeleteRepoInputPort>,
    pub view_data_list: Arc<dyn usecase::ViewDataListInputPort>,
    pub change_repo_policy: Arc<dyn usecase::ChangeRepoPolicyInputPort>,
    pub invite_repo_member: Arc<dyn usecase::InviteRepoMemberInputPort>,
    pub remove_repo_member: Arc<dyn usecase::RemoveRepoMemberInputPort>,
    pub change_repo_member_role:
        Arc<dyn usecase::ChangeRepoMemberRoleInputPort>,
    pub get_repo_policies: Arc<dyn usecase::GetRepoPoliciesInputPort>,
    pub get_repo_members: Arc<dyn usecase::GetRepoMembersInputPort>,
    pub create_api_key: Arc<dyn usecase::CreateApiKeyInputPort>,
    pub list_api_keys: Arc<dyn usecase::ListApiKeysInputPort>,
    pub revoke_api_key: Arc<dyn usecase::RevokeApiKeyInputPort>,
    pub create_source: Arc<dyn usecase::CreateSourceInputPort>,
    pub update_source: Arc<dyn usecase::UpdateSourceInputPort>,
    pub delete_source: Arc<dyn usecase::DeleteSourceInputPort>,
    pub get_source: Arc<dyn usecase::GetSourceInputPort>,
    pub find_sources: Arc<dyn usecase::FindSourcesInputPort>,
    // PLT-942 Library MDM Hub Phase 1 (Registry)
    pub create_global_id_mapping:
        Arc<dyn usecase::CreateGlobalIdMappingInputPort>,
    pub update_global_id_mapping:
        Arc<dyn usecase::UpdateGlobalIdMappingInputPort>,
    pub get_global_id_mapping:
        Arc<dyn usecase::GetGlobalIdMappingInputPort>,
    pub find_global_id_mappings:
        Arc<dyn usecase::FindGlobalIdMappingsInputPort>,
    pub sign_in: Arc<dyn usecase::SignInInputPort>,
    pub organization_repo: Arc<dyn OrganizationRepository>,
    pub auth_app: Arc<dyn AuthApp>,
    pub invite_org_member: Arc<dyn usecase::InviteOrgMemberInputPort>,
    pub change_org_member_role:
        Arc<dyn usecase::ChangeOrgMemberRoleInputPort>,
    pub bulk_sync_ext_github: Arc<dyn usecase::BulkSyncExtGithubInputPort>,
    pub sync_data_to_github: Arc<dyn usecase::SyncDataToGithubInputPort>,
    // GitHub Import usecases
    pub list_github_directory:
        Arc<dyn usecase::ListGitHubDirectoryInputPort>,
    pub get_markdown_previews:
        Arc<dyn usecase::GetMarkdownPreviewsInputPort>,
    pub analyze_frontmatter: Arc<dyn usecase::AnalyzeFrontmatterInputPort>,
    pub import_markdown_from_github:
        Arc<dyn usecase::ImportMarkdownFromGitHubInputPort>,
    /// Exposed for resolvers that need to construct
    /// request-scoped usecase instances with a caller-token
    /// AuthApp (e.g. `GetRepoPolicies`).
    pub user_policy_mapping_repo:
        Arc<dyn tachyon_sdk::auth::UserPolicyMappingRepository>,
}

impl std::fmt::Debug for LibraryApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryApp").finish_non_exhaustive()
    }
}

impl LibraryApp {
    /// Wire the app onto the caller's `library` pool.
    ///
    /// Takes the pool rather than a DSN so it shares the one the router
    /// already holds: opening a second one here meant two pools to the
    /// same database, and a second connection to establish before the
    /// process could serve anything.
    pub async fn new(
        library_db: Arc<persistence::Db>,
        database_app: Arc<database_manager::App>,
        sdk: Arc<SdkAuthApp>,
        sync_data: Arc<dyn outbound_sync::SyncDataInputPort>,
        webhook_endpoint_repo: Arc<
            dyn inbound_sync_domain::WebhookEndpointRepository,
        >,
        sync_state_repo: Arc<dyn inbound_sync_domain::SyncStateRepository>,
    ) -> Self {
        // auth trait object (SdkAuthApp implements AuthApp)
        let auth_app: Arc<dyn AuthApp> = sdk.clone();

        // SDK-backed repositories
        let user_policy_mapping_repo =
            Arc::new(SdkUserPolicyMappingRepository::new(sdk.clone()));
        let user_query = Arc::new(SdkUserQuery::new(sdk.clone()));

        // library:repository
        let organization_repo =
            Arc::new(interface_adapter::OrganizationRepositoryImpl::new(
                library_db.clone(),
            ));
        let repo_repo: Arc<dyn RepoRepository> = Arc::new(
            interface_adapter::RepoRepositoryImpl::new(library_db.clone()),
        );
        let source_repo: Arc<dyn SourceRepository> =
            Arc::new(interface_adapter::SourceRepositoryImpl::new(
                library_db.clone(),
            ));
        let global_id_mapping_repo: Arc<
            dyn crate::domain::GlobalIdMappingRepository,
        > = Arc::new(
            interface_adapter::GlobalIdMappingRepositoryImpl::new(
                library_db.clone(),
            ),
        );
        let find_all_repo_query =
            Arc::new(interface_adapter::AllRepoQueryServiceImpl::new(
                library_db.clone(),
            ));

        let get_organization_by_username = Arc::new(
            interface_adapter::GetOrganizationByUsernameQueryImpl::new(
                sdk.clone(),
                organization_repo.clone(),
            ),
        );
        let get_repo_by_username =
            interface_adapter::GetRepoByUsernameQueryImpl::new(
                library_db.clone(),
            );

        // domain
        let visibility_service = Arc::new(VisibilityService::new());

        // usecase
        let view_org = usecase::ViewOrganization::new(
            get_organization_by_username.clone(),
            repo_repo.clone(),
        );
        let create_repo = Arc::new(usecase::CreateRepo::new(
            repo_repo.clone(),
            get_organization_by_username.clone(),
            database_app.clone(),
        ));
        let update_repo = usecase::UpdateRepo::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            repo_repo.clone(),
        );
        let change_repo_username = usecase::ChangeRepoUsername::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            repo_repo.clone(),
            auth_app.clone(),
        );
        let view_repo = Arc::new(usecase::ViewRepo::new(
            auth_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            visibility_service.clone(),
        ));
        let search_data = usecase::SearchData::new(
            database_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
        );
        let search_repo = usecase::SearchRepo::new(
            find_all_repo_query,
            get_organization_by_username.clone(),
        );
        // GitHub auto-writeback: wraps the Data mutation ports so that
        // saving a Data item with ext_github.enabled=true pushes its
        // markdown back to GitHub (best-effort, echo-suppressed).
        let github_writeback = usecase::GithubWritebackDispatch::new(
            sync_data.clone(),
            webhook_endpoint_repo,
            sync_state_repo,
        );
        let add_data: Arc<dyn usecase::AddDataInputPort> =
            usecase::AddDataWithGithubWriteback::new(
                usecase::AddData::new(
                    auth_app.clone(),
                    get_repo_by_username.clone(),
                    get_organization_by_username.clone(),
                    database_app.clone(),
                ),
                github_writeback.clone(),
            );
        let save_data = add_data.clone();
        let change_repo_policy =
            Arc::new(usecase::ChangeRepoPolicy::new(auth_app.clone()));
        let get_repo_members = Arc::new(usecase::GetRepoMembers::new(
            user_policy_mapping_repo.clone(),
            auth_app.clone(),
        ));
        let get_repo_policies = Arc::new(usecase::GetRepoPolicies::new(
            user_policy_mapping_repo.clone(),
            auth_app.clone(),
        ));
        let invite_repo_member = Arc::new(usecase::InviteRepoMember::new(
            auth_app.clone(),
            user_query.clone(),
            get_repo_members.clone(),
        ));
        let remove_repo_member = Arc::new(usecase::RemoveRepoMember::new(
            auth_app.clone(),
            get_repo_members.clone(),
        ));
        let change_repo_member_role =
            Arc::new(usecase::ChangeRepoMemberRole::new(
                auth_app.clone(),
                get_repo_members.clone(),
            ));
        let view_data = usecase::ViewData::new(
            auth_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            database_app.clone(),
        );
        let update_data: Arc<dyn usecase::UpdateDataInputPort> =
            usecase::UpdateDataWithGithubWriteback::new(
                usecase::UpdateData::new(
                    get_organization_by_username.clone(),
                    get_repo_by_username.clone(),
                    auth_app.clone(),
                    database_app.clone(),
                ),
                github_writeback.clone(),
            );
        let upsert_data: Arc<dyn usecase::UpsertDataInputPort> =
            usecase::UpsertDataWithGithubWriteback::new(
                usecase::UpsertData::new(
                    get_organization_by_username.clone(),
                    get_repo_by_username.clone(),
                    auth_app.clone(),
                    database_app.clone(),
                ),
                github_writeback.clone(),
            );
        let delete_data = usecase::DeleteData::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            database_app.clone(),
        );
        let add_property = usecase::AddProperty::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            database_app.clone(),
        );
        let update_property = usecase::UpdateProperty::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            database_app.clone(),
        );

        let get_properties = usecase::GetProperties::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            database_app.clone(),
            auth_app.clone(),
        );
        let delete_property = usecase::DeleteProperty::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            database_app.clone(),
        );
        let delete_repo = usecase::DeleteRepo::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            repo_repo.clone(),
            auth_app.clone(),
        );

        let update_organization = usecase::UpdateOrganization::new(
            get_organization_by_username.clone(),
            organization_repo.clone(),
            auth_app.clone(),
        );
        let create_organization = usecase::CreateOrganization::new(
            organization_repo.clone(),
            auth_app.clone(),
        );
        let view_data_list = usecase::ViewDataList::new(
            database_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
        );

        let create_api_key = Arc::new(usecase::CreateApiKey::new(
            auth_app.clone(),
            get_organization_by_username.clone(),
        ));

        let list_api_keys = Arc::new(usecase::ListApiKeys::new(
            auth_app.clone(),
            get_organization_by_username.clone(),
        ));

        let revoke_api_key = Arc::new(usecase::RevokeApiKey::new(
            auth_app.clone(),
            get_organization_by_username.clone(),
        ));

        let sign_in = Arc::new(usecase::SignIn::new(sdk.clone()));

        let invite_org_member =
            Arc::new(usecase::InviteOrgMember::new(sdk.clone()));
        let change_org_member_role =
            Arc::new(usecase::ChangeOrgMemberRole::new(sdk.clone()));

        let bulk_sync_ext_github = usecase::BulkSyncExtGithub::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            database_app.clone(),
            sync_data.clone(),
        );

        let sync_data_to_github = usecase::SyncDataToGithub::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            database_app.clone(),
            sync_data.clone(),
        );

        let create_source = Arc::new(usecase::CreateSource::new(
            source_repo.clone(),
            auth_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
        ));
        let update_source = Arc::new(usecase::UpdateSource::new(
            source_repo.clone(),
            auth_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
        ));
        let delete_source = Arc::new(usecase::DeleteSource::new(
            source_repo.clone(),
            auth_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
        ));
        let get_source = Arc::new(usecase::GetSource::new(
            source_repo.clone(),
            auth_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            visibility_service.clone(),
        ));
        let find_sources = Arc::new(usecase::FindSources::new(
            source_repo.clone(),
            auth_app.clone(),
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            visibility_service.clone(),
        ));

        // PLT-942 Library MDM Hub Phase 1
        let create_global_id_mapping =
            Arc::new(usecase::CreateGlobalIdMapping::new(
                global_id_mapping_repo.clone(),
                auth_app.clone(),
            ));
        let update_global_id_mapping =
            Arc::new(usecase::UpdateGlobalIdMapping::new(
                global_id_mapping_repo.clone(),
                auth_app.clone(),
            ));
        let get_global_id_mapping =
            Arc::new(usecase::GetGlobalIdMapping::new(
                global_id_mapping_repo.clone(),
                auth_app.clone(),
            ));
        let find_global_id_mappings =
            Arc::new(usecase::FindGlobalIdMappings::new(
                global_id_mapping_repo.clone(),
                auth_app.clone(),
            ));

        // GitHub Import usecases
        let list_github_directory =
            usecase::ListGitHubDirectory::new(auth_app.clone());
        let get_markdown_previews =
            usecase::GetMarkdownPreviews::new(auth_app.clone());
        let analyze_frontmatter =
            usecase::AnalyzeFrontmatter::new(get_markdown_previews.clone());
        let import_markdown_from_github =
            usecase::ImportMarkdownFromGitHub::new(
                view_org.clone(),
                create_repo.clone(),
                get_properties.clone(),
                add_property.clone(),
                view_data_list.clone(),
                add_data.clone(),
                update_data.clone(),
            );

        Self {
            view_org,
            update_organization,
            create_organization,
            create_repo,
            update_repo,
            change_repo_username,
            view_repo,
            search_data,
            search_repo,
            save_data,
            view_data,
            update_data,
            upsert_data,
            delete_data,
            add_property,
            update_property,
            get_properties,
            delete_property,
            delete_repo,
            view_data_list,
            change_repo_policy,
            invite_repo_member,
            remove_repo_member,
            change_repo_member_role,
            get_repo_policies,
            get_repo_members,
            create_api_key,
            list_api_keys,
            revoke_api_key,
            add_data,
            create_source,
            update_source,
            delete_source,
            get_source,
            find_sources,
            create_global_id_mapping,
            update_global_id_mapping,
            get_global_id_mapping,
            find_global_id_mappings,
            sign_in,
            organization_repo,
            auth_app,
            invite_org_member,
            change_org_member_role,
            bulk_sync_ext_github,
            sync_data_to_github,
            list_github_directory,
            get_markdown_previews,
            analyze_frontmatter,
            import_markdown_from_github,
            user_policy_mapping_repo,
        }
    }
}
