use derive_new::new;
use errors::Result;
use tachyon_sdk::auth::ExecutorAction;

use super::Repo;

/// TODO: add English documentation
#[derive(Debug, Clone, new)]
pub struct VisibilityService;

impl VisibilityService {
    /// Access check for query
    /// TODO: add English documentation
    ///
    /// # Arguments
    ///
    /// TODO: add English documentation
    /// TODO: add English documentation
    ///
    /// # Returns
    ///
    /// TODO: add English documentation
    /// TODO: add English documentation
    /// TODO: add English documentation
    pub fn check_access(
        &self,
        repo: &Repo,
        executor: &dyn ExecutorAction,
    ) -> Result<bool> {
        // TODO: add English comment
        if *repo.is_public() {
            return Ok(false);
        }

        // TODO: add English comment
        if executor.is_none() {
            return Err(errors::Error::permission_denied("Access denied"));
        }

        // TODO: add English comment
        if !executor.has_tenant_id(repo.organization_id()) {
            return Err(errors::Error::permission_denied(
                "Access denied: tenant mismatch",
            ));
        }

        // TODO: add English comment
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Repo;
    use database_manager::domain::DatabaseId;
    use std::str::FromStr;

    #[test]
    fn test_check_access_public_repo() {
        // TODO: add English comment
        let repo = create_test_repo(true);
        let visibility_service = VisibilityService::new();

        // TODO: add English comment
        let mut mock_executor =
            tachyon_sdk::auth::MockExecutorAction::new();
        mock_executor.expect_is_none().return_const(true);

        // TODO: add English comment
        let result = visibility_service.check_access(&repo, &mock_executor);
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn test_check_access_private_repo_unauthorized() {
        // TODO: add English comment
        let repo = create_test_repo(false);
        let visibility_service = VisibilityService::new();

        // TODO: add English comment
        let mut mock_executor =
            tachyon_sdk::auth::MockExecutorAction::new();
        mock_executor.expect_is_none().return_const(true);

        // TODO: add English comment
        let result = visibility_service.check_access(&repo, &mock_executor);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_access_private_repo_authorized_owner() {
        // TODO: add English comment
        let repo = create_test_repo(false);
        let visibility_service = VisibilityService::new();

        // TODO: add English comment
        let mut mock_executor =
            tachyon_sdk::auth::MockExecutorAction::new();
        mock_executor.expect_is_none().return_const(false);
        mock_executor.expect_has_tenant_id().return_once(|_| true);

        // TODO: add English comment
        let result = visibility_service.check_access(&repo, &mock_executor);
        assert!(matches!(result, Ok(true)));
    }

    #[test]
    fn test_check_access_private_repo_authorized_non_owner() {
        // TODO: add English comment
        let repo = create_test_repo(false);
        let visibility_service = VisibilityService::new();

        // TODO: add English comment
        let mut mock_executor =
            tachyon_sdk::auth::MockExecutorAction::new();
        mock_executor.expect_is_none().return_const(false);
        mock_executor.expect_has_tenant_id().return_once(|_| true);

        // TODO: add English comment
        let result = visibility_service.check_access(&repo, &mock_executor);
        assert!(matches!(result, Ok(true)));
    }

    // TODO: add English comment
    fn create_test_repo(is_public: bool) -> Repo {
        Repo::new(
            &"rp_01hkz3700yt46snfewzpakeyj4".parse().unwrap(),
            &"tn_01hkz3700yt46snfewzpakeyj4".parse().unwrap(),
            &"orgUsername".parse().unwrap(),
            &"repo name".parse().unwrap(),
            &"test".parse().unwrap(),
            is_public,
            None,
            vec![DatabaseId::from_str("db_01hkz3700yt46snfewzpakeyj4")
                .unwrap()],
            vec![],
        )
    }
}
