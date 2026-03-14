use crate::domain::RepoRepository;
use crate::domain::SourceRepository;
use crate::domain::VisibilityService;
use crate::interface_adapter;
use crate::sdk_auth::{
    SdkAuthApp, SdkUserPolicyMappingRepository, SdkUserQuery,
};
use crate::usecase;
use std::sync::Arc;
use tachyon_sdk::auth::AuthApp;
use value_object::DatabaseUrl;

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
    pub create_source: Arc<dyn usecase::CreateSourceInputPort>,
    pub update_source: Arc<dyn usecase::UpdateSourceInputPort>,
    pub delete_source: Arc<dyn usecase::DeleteSourceInputPort>,
    pub get_source: Arc<dyn usecase::GetSourceInputPort>,
    pub find_sources: Arc<dyn usecase::FindSourcesInputPort>,
    pub sign_in: Arc<dyn usecase::SignInInputPort>,
    pub invite_org_member: Arc<dyn usecase::InviteOrgMemberInputPort>,
    pub change_org_member_role:
        Arc<dyn usecase::ChangeOrgMemberRoleInputPort>,
    pub bulk_sync_ext_github: Arc<dyn usecase::BulkSyncExtGithubInputPort>,
    pub sync_data: Arc<dyn outbound_sync::SyncDataInputPort>,
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
        f.debug_struct("LibraryApp")
            .field("sync_data", &"<SyncDataInputPort>")
            .finish_non_exhaustive()
    }
}

impl LibraryApp {
    pub async fn new(
        dsn: &DatabaseUrl,
        database_app: Arc<database_manager::App>,
        sdk: Arc<SdkAuthApp>,
        sync_data: Arc<dyn outbound_sync::SyncDataInputPort>,
    ) -> Self {
        let library_db = persistence::Db::new(&dsn).await;

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
            auth_app.clone(),
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
        );
        let search_repo = usecase::SearchRepo::new(find_all_repo_query);
        let save_data = usecase::AddData::new(
            auth_app.clone(),
            get_repo_by_username.clone(),
            get_organization_by_username.clone(),
            database_app.clone(),
        );
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
        let update_data = usecase::UpdateData::new(
            get_organization_by_username.clone(),
            get_repo_by_username.clone(),
            auth_app.clone(),
            database_app.clone(),
            sync_data.clone(),
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

        let add_data = usecase::AddData::new(
            auth_app.clone(),
            get_repo_by_username.clone(),
            get_organization_by_username.clone(),
            database_app.clone(),
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

        // GitHub Import usecases
        let list_github_directory =
            usecase::ListGitHubDirectory::new(auth_app.clone());
        let get_markdown_previews =
            usecase::GetMarkdownPreviews::new(auth_app.clone());
        let analyze_frontmatter =
            usecase::AnalyzeFrontmatter::new(get_markdown_previews.clone());
        let import_markdown_from_github =
            usecase::ImportMarkdownFromGitHub::new(
                auth_app.clone(),
                view_org.clone(),
                create_repo.clone(),
                get_properties.clone(),
                add_property.clone(),
                update_property.clone(),
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
            add_data,
            create_source,
            update_source,
            delete_source,
            get_source,
            find_sources,
            sign_in,
            invite_org_member,
            change_org_member_role,
            bulk_sync_ext_github,
            sync_data,
            list_github_directory,
            get_markdown_previews,
            analyze_frontmatter,
            import_markdown_from_github,
            user_policy_mapping_repo,
        }
    }
}
