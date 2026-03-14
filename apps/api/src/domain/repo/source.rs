use derive_getters::Getters;
use util::macros::*;
use value_object::{Text, Url};

use super::RepoId;

def_id!(SourceId, "src_");

#[async_trait::async_trait]
pub trait SourceRepository:
    std::marker::Send + Sync + std::fmt::Debug
{
    async fn save(&self, entity: &Source) -> errors::Result<()>;

    async fn get_by_id(
        &self,
        id: &SourceId,
    ) -> errors::Result<Option<Source>>;

    async fn find_by_repo_id(
        &self,
        repo_id: &RepoId,
    ) -> errors::Result<Vec<Source>>;

    async fn delete(&self, id: &SourceId) -> errors::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub struct Source {
    id: SourceId,
    repo_id: RepoId,
    name: Text,
    url: Option<Url>,
}

impl Source {
    pub fn new(
        id: &SourceId,
        repo_id: &RepoId,
        name: &Text,
        url: Option<Url>,
    ) -> Self {
        Self {
            id: id.clone(),
            repo_id: repo_id.clone(),
            name: name.clone(),
            url,
        }
    }

    pub fn create(repo_id: &RepoId, name: &Text, url: Option<Url>) -> Self {
        Self::new(&SourceId::default(), repo_id, name, url)
    }

    pub fn update(
        &self,
        name: Option<Text>,
        url: Option<Option<Url>>,
    ) -> Self {
        Self {
            name: name.unwrap_or_else(|| self.name.clone()),
            url: url.unwrap_or(self.url.clone()),
            ..self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    fn test_create_source() {
        let repo_id: RepoId =
            "rp_01hkz3700yt46snfewzpakeyj4".parse().unwrap();
        let name: Text = "GitHub".parse().unwrap();
        let url: Url = "https://github.com/example/repo".parse().unwrap();

        let source = Source::create(&repo_id, &name, Some(url));

        assert_eq!(source.repo_id(), &repo_id);
        assert_eq!(source.name(), &name);
    }

    #[rstest]
    fn test_update_source() {
        let source_id: SourceId =
            "src_01hkz3700yt46snfewzpakeyj4".parse().unwrap();
        let repo_id: RepoId =
            "rp_01hkz3700yt46snfewzpakeyj4".parse().unwrap();
        let name: Text = "GitHub".parse().unwrap();
        let url: Url = "https://github.com/example/repo".parse().unwrap();

        let source =
            Source::new(&source_id, &repo_id, &name, Some(url.clone()));

        // TODO: add English comment
        let new_name: Text = "GitLab".parse().unwrap();
        let updated = source.update(Some(new_name.clone()), None);
        assert_eq!(updated.name(), &new_name);
        assert_eq!(updated.url(), &Some(url.clone()));

        // TODO: add English comment
        let new_url: Url =
            "https://gitlab.com/example/repo".parse().unwrap();
        let updated = source.update(None, Some(Some(new_url.clone())));
        assert_eq!(updated.name(), &name);
        assert_eq!(updated.url(), &Some(new_url));

        // TODO: add English comment
        let updated = source.update(None, Some(None));
        assert_eq!(updated.name(), &name);
        assert_eq!(updated.url(), &None);
    }
}
