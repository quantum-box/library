use super::{CreatePullRequestInputData, CreatePullRequestInputPort};

#[derive(Debug)]
#[allow(dead_code)]
pub struct CreatePullRequest {
    // TODO: add English comment
}

#[async_trait::async_trait]
impl CreatePullRequestInputPort for CreatePullRequest {
    async fn execute(
        &self,
        _input: CreatePullRequestInputData,
    ) -> errors::Result<()> {
        // TODO: add English comment
        unimplemented!()
    }
}
