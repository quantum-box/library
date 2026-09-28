use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, ExecutorAction,
    MultiTenancyAction,
};

use crate::domain::{
    RepoId, RichTextTemplate, RichTextTemplateId,
    RichTextTemplateRepository,
};

use super::{ViewRepoInputData, ViewRepoInputPort};

const MAX_TEMPLATE_NAME_LENGTH: usize = 255;
const MAX_TEMPLATE_BODY_BYTES: usize = 1_048_576;

pub struct ListRichTextTemplatesInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: String,
    pub repo_username: String,
}

pub struct SaveRichTextTemplateInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: String,
    pub repo_username: String,
    pub name: String,
    pub rich_text: String,
}

pub struct UpdateRichTextTemplateInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: String,
    pub repo_username: String,
    pub template_id: String,
    pub name: String,
    pub rich_text: String,
}

pub struct DeleteRichTextTemplateInputData<'a> {
    pub executor: &'a dyn ExecutorAction,
    pub multi_tenancy: &'a dyn MultiTenancyAction,
    pub org_username: String,
    pub repo_username: String,
    pub template_id: String,
}

#[async_trait]
pub trait ListRichTextTemplatesInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &ListRichTextTemplatesInputData<'_>,
    ) -> errors::Result<Vec<RichTextTemplate>>;
}

#[async_trait]
pub trait SaveRichTextTemplateInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &SaveRichTextTemplateInputData<'_>,
    ) -> errors::Result<RichTextTemplate>;
}

#[async_trait]
pub trait UpdateRichTextTemplateInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &UpdateRichTextTemplateInputData<'_>,
    ) -> errors::Result<RichTextTemplate>;
}

#[async_trait]
pub trait DeleteRichTextTemplateInputPort: Debug + Send + Sync {
    async fn execute(
        &self,
        input: &DeleteRichTextTemplateInputData<'_>,
    ) -> errors::Result<bool>;
}

fn normalize_template(
    name: &str,
    rich_text: &str,
) -> errors::Result<(String, String)> {
    let name = name.trim();
    if name.is_empty() {
        return Err(errors::Error::bad_request(
            "A RichText template name cannot be empty",
        ));
    }
    if name.chars().count() > MAX_TEMPLATE_NAME_LENGTH {
        return Err(errors::Error::bad_request(format!(
            "A RichText template name cannot exceed {MAX_TEMPLATE_NAME_LENGTH} characters"
        )));
    }
    if rich_text.len() > MAX_TEMPLATE_BODY_BYTES {
        return Err(errors::Error::bad_request(
            "A RichText template body cannot exceed 4 MiB",
        ));
    }
    let document: serde_json::Value = serde_json::from_str(rich_text)
        .map_err(|_| {
            errors::Error::bad_request(
                "A RichText template body must be a serialized JSON document",
            )
        })?;
    if !document.is_array() {
        return Err(errors::Error::bad_request(
            "A RichText template body must be a block document array",
        ));
    }
    Ok((name.to_string(), rich_text.to_string()))
}

async fn resolve_repo(
    view_repo: &dyn ViewRepoInputPort,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    org_username: String,
    repo_username: String,
) -> errors::Result<RepoId> {
    Ok(view_repo
        .execute(&ViewRepoInputData {
            executor,
            multi_tenancy,
            organization_username: org_username,
            repo_username,
        })
        .await?
        .repo
        .id()
        .clone())
}

async fn authorize_edit(
    auth_app: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    repo_id: &RepoId,
) -> errors::Result<()> {
    auth_app
        .check_policy_for_resource(&CheckPolicyForResourceInput {
            executor,
            multi_tenancy,
            action: "library:UpdateRepo",
            resource_trn: &format!("trn:library:repo:{repo_id}"),
        })
        .await
}

#[derive(Debug)]
pub struct ListRichTextTemplates {
    auth_app: Arc<dyn AuthApp>,
    view_repo: Arc<dyn ViewRepoInputPort>,
    repository: Arc<dyn RichTextTemplateRepository>,
}

impl ListRichTextTemplates {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        view_repo: Arc<dyn ViewRepoInputPort>,
        repository: Arc<dyn RichTextTemplateRepository>,
    ) -> Self {
        Self {
            auth_app,
            view_repo,
            repository,
        }
    }
}

#[async_trait]
impl ListRichTextTemplatesInputPort for ListRichTextTemplates {
    async fn execute(
        &self,
        input: &ListRichTextTemplatesInputData<'_>,
    ) -> errors::Result<Vec<RichTextTemplate>> {
        let repo_id = resolve_repo(
            self.view_repo.as_ref(),
            input.executor,
            input.multi_tenancy,
            input.org_username.clone(),
            input.repo_username.clone(),
        )
        .await?;
        authorize_edit(
            self.auth_app.as_ref(),
            input.executor,
            input.multi_tenancy,
            &repo_id,
        )
        .await?;
        self.repository.find_by_repo(&repo_id).await
    }
}

#[derive(Debug)]
pub struct SaveRichTextTemplate {
    auth_app: Arc<dyn AuthApp>,
    view_repo: Arc<dyn ViewRepoInputPort>,
    repository: Arc<dyn RichTextTemplateRepository>,
}

impl SaveRichTextTemplate {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        view_repo: Arc<dyn ViewRepoInputPort>,
        repository: Arc<dyn RichTextTemplateRepository>,
    ) -> Self {
        Self {
            auth_app,
            view_repo,
            repository,
        }
    }
}

#[async_trait]
impl SaveRichTextTemplateInputPort for SaveRichTextTemplate {
    async fn execute(
        &self,
        input: &SaveRichTextTemplateInputData<'_>,
    ) -> errors::Result<RichTextTemplate> {
        let (name, rich_text) =
            normalize_template(&input.name, &input.rich_text)?;
        let repo_id = resolve_repo(
            self.view_repo.as_ref(),
            input.executor,
            input.multi_tenancy,
            input.org_username.clone(),
            input.repo_username.clone(),
        )
        .await?;
        authorize_edit(
            self.auth_app.as_ref(),
            input.executor,
            input.multi_tenancy,
            &repo_id,
        )
        .await?;
        ensure_unique_name(self.repository.as_ref(), &repo_id, &name, None)
            .await?;

        let template = RichTextTemplate::new(
            RichTextTemplateId::default(),
            repo_id,
            name,
            rich_text,
        );
        self.repository.create(&template).await?;
        Ok(template)
    }
}

#[derive(Debug)]
pub struct UpdateRichTextTemplate {
    auth_app: Arc<dyn AuthApp>,
    view_repo: Arc<dyn ViewRepoInputPort>,
    repository: Arc<dyn RichTextTemplateRepository>,
}

impl UpdateRichTextTemplate {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        view_repo: Arc<dyn ViewRepoInputPort>,
        repository: Arc<dyn RichTextTemplateRepository>,
    ) -> Self {
        Self {
            auth_app,
            view_repo,
            repository,
        }
    }
}

#[async_trait]
impl UpdateRichTextTemplateInputPort for UpdateRichTextTemplate {
    async fn execute(
        &self,
        input: &UpdateRichTextTemplateInputData<'_>,
    ) -> errors::Result<RichTextTemplate> {
        let (name, rich_text) =
            normalize_template(&input.name, &input.rich_text)?;
        let repo_id = resolve_repo(
            self.view_repo.as_ref(),
            input.executor,
            input.multi_tenancy,
            input.org_username.clone(),
            input.repo_username.clone(),
        )
        .await?;
        authorize_edit(
            self.auth_app.as_ref(),
            input.executor,
            input.multi_tenancy,
            &repo_id,
        )
        .await?;
        let template_id: RichTextTemplateId =
            input.template_id.parse().map_err(errors::Error::from)?;
        ensure_unique_name(
            self.repository.as_ref(),
            &repo_id,
            &name,
            Some(&template_id),
        )
        .await?;
        let template =
            RichTextTemplate::new(template_id, repo_id, name, rich_text);
        if !self.repository.update(&template).await? {
            return Err(errors::Error::not_found(
                "RichText template not found in this repository",
            ));
        }
        Ok(template)
    }
}

#[derive(Debug)]
pub struct DeleteRichTextTemplate {
    auth_app: Arc<dyn AuthApp>,
    view_repo: Arc<dyn ViewRepoInputPort>,
    repository: Arc<dyn RichTextTemplateRepository>,
}

impl DeleteRichTextTemplate {
    pub fn new(
        auth_app: Arc<dyn AuthApp>,
        view_repo: Arc<dyn ViewRepoInputPort>,
        repository: Arc<dyn RichTextTemplateRepository>,
    ) -> Self {
        Self {
            auth_app,
            view_repo,
            repository,
        }
    }
}

#[async_trait]
impl DeleteRichTextTemplateInputPort for DeleteRichTextTemplate {
    async fn execute(
        &self,
        input: &DeleteRichTextTemplateInputData<'_>,
    ) -> errors::Result<bool> {
        let repo_id = resolve_repo(
            self.view_repo.as_ref(),
            input.executor,
            input.multi_tenancy,
            input.org_username.clone(),
            input.repo_username.clone(),
        )
        .await?;
        authorize_edit(
            self.auth_app.as_ref(),
            input.executor,
            input.multi_tenancy,
            &repo_id,
        )
        .await?;
        let template_id: RichTextTemplateId =
            input.template_id.parse().map_err(errors::Error::from)?;
        self.repository.delete(&repo_id, &template_id).await
    }
}

async fn ensure_unique_name(
    repository: &dyn RichTextTemplateRepository,
    repo_id: &RepoId,
    name: &str,
    except_id: Option<&RichTextTemplateId>,
) -> errors::Result<()> {
    if repository
        .find_by_repo(repo_id)
        .await?
        .iter()
        .any(|existing| {
            existing.name().eq_ignore_ascii_case(name)
                && Some(existing.id()) != except_id
        })
    {
        return Err(errors::Error::bad_request(
            "A RichText template with this name already exists",
        ));
    }
    Ok(())
}
