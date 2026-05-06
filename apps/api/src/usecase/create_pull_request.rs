use super::{CreatePullRequestInputData, CreatePullRequestInputPort};

#[derive(Debug)]
#[allow(dead_code)]
pub struct CreatePullRequest {
    // Not wired into LibraryApp or any GA handler. Keep this as an explicit
    // non-GA boundary instead of allowing an accidental panic.
}

#[async_trait::async_trait]
impl CreatePullRequestInputPort for CreatePullRequest {
    async fn execute(
        &self,
        input: CreatePullRequestInputData,
    ) -> errors::Result<()> {
        Err(errors::Error::not_supported(format!(
            "create_pull_request is not available in Library GA (organization_id={}, repo_id={})",
            input.organization_id, input.repo_id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn execute_returns_not_supported_instead_of_panicking() {
        let usecase = CreatePullRequest {};

        let err = usecase
            .execute(CreatePullRequestInputData {
                organization_id: "tn_01j702qf86pc2j35s0kv0gv3gy"
                    .to_string(),
                repo_id: "repo_01j702qf86pc2j35s0kv0gv3gy".to_string(),
            })
            .await
            .unwrap_err();

        match err {
            errors::Error::BadRequest { message, .. } => {
                assert!(message.contains("NotSupported"));
                assert!(message.contains("Library GA"));
            }
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }
}
