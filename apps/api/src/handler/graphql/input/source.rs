use async_graphql::InputObject;

#[derive(InputObject, Debug, Clone)]
pub struct CreateSourceInput {
    pub org_username: String,
    pub repo_username: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(InputObject, Debug, Clone)]
pub struct UpdateSourceInput {
    pub org_username: String,
    pub repo_username: String,
    pub source_id: String,
    pub name: Option<String>,
    pub url: Option<Option<String>>,
}
