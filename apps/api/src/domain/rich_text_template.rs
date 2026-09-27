use derive_getters::Getters;
use util::macros::*;

use super::RepoId;

def_id!(RichTextTemplateId, "rtt_");

#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct RichTextTemplate {
    id: RichTextTemplateId,
    repo_id: RepoId,
    name: String,
    rich_text: String,
}

impl RichTextTemplate {
    pub fn new(
        id: RichTextTemplateId,
        repo_id: RepoId,
        name: String,
        rich_text: String,
    ) -> Self {
        Self {
            id,
            repo_id,
            name,
            rich_text,
        }
    }
}

#[async_trait::async_trait]
pub trait RichTextTemplateRepository:
    Send + Sync + std::fmt::Debug
{
    async fn find_by_repo(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<RichTextTemplate>>;

    async fn create(
        &self,
        template: &RichTextTemplate,
    ) -> errors::Result<()>;

    async fn update(
        &self,
        template: &RichTextTemplate,
    ) -> errors::Result<bool>;

    async fn delete(
        &self,
        repo_id: &RepoId,
        template_id: &RichTextTemplateId,
    ) -> errors::Result<bool>;
}
