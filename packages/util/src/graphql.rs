use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct Empty {
    pub id: String,
}
