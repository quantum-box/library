use crate::domain;
use async_graphql::SimpleObject;

#[derive(SimpleObject, Debug, Clone)]
pub struct Source {
    pub id: String,
    pub repo_id: String,
    pub name: String,
    pub url: Option<String>,
}

impl From<domain::Source> for Source {
    fn from(source: domain::Source) -> Self {
        Self {
            id: source.id().to_string(),
            repo_id: source.repo_id().to_string(),
            name: source.name().to_string(),
            url: source.url().as_ref().map(|u| u.to_string()),
        }
    }
}
