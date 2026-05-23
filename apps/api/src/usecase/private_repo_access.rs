use tachyon_sdk::auth::{
    AuthApp, CheckPolicyForResourceInput, CheckPolicyInput, ExecutorAction,
    MultiTenancyAction,
};

pub async fn authorize_private_repo_read(
    auth_app: &dyn AuthApp,
    executor: &dyn ExecutorAction,
    multi_tenancy: &dyn MultiTenancyAction,
    repo_id: &str,
) -> errors::Result<()> {
    let resource_trn = format!("trn:library:repo:{repo_id}");

    let resource_access = auth_app
        .check_policy_for_resource(&CheckPolicyForResourceInput {
            executor,
            multi_tenancy,
            action: "library:ViewRepo",
            resource_trn: &resource_trn,
        })
        .await;

    if resource_access.is_err() {
        auth_app
            .check_policy(&CheckPolicyInput {
                executor,
                multi_tenancy,
                action: "library:ViewPrivateRepo",
            })
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tachyon_sdk::auth::{
        test_helper::{create_test_executor, create_test_multi_tenancy},
        MockAuthApp,
    };

    fn mock_auth(
        calls: Arc<Mutex<Vec<String>>>,
        deny_resource: bool,
    ) -> MockAuthApp {
        let mut auth = MockAuthApp::new();
        auth.expect_check_policy_for_resource().returning({
            let calls = calls.clone();
            move |input| {
                calls.lock().unwrap().push(format!(
                    "resource:{}:{}",
                    input.action, input.resource_trn
                ));
                Box::pin(async move {
                    if deny_resource {
                        Err(errors::Error::forbidden("denied"))
                    } else {
                        Ok(())
                    }
                })
            }
        });
        auth.expect_check_policy().returning({
            let calls = calls.clone();
            move |input| {
                calls
                    .lock()
                    .unwrap()
                    .push(format!("policy:{}", input.action));
                Box::pin(async { Ok(()) })
            }
        });
        auth
    }

    #[tokio::test]
    async fn resource_access_allows_without_org_wide_fallback() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = mock_auth(calls.clone(), false);
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        authorize_private_repo_read(
            &auth,
            &executor,
            &multi_tenancy,
            "rp_01test",
        )
        .await
        .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &["resource:library:ViewRepo:trn:library:repo:rp_01test"]
        );
    }

    #[tokio::test]
    async fn falls_back_to_org_wide_private_repo_policy() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let auth = mock_auth(calls.clone(), true);
        let executor = create_test_executor();
        let multi_tenancy = create_test_multi_tenancy();

        authorize_private_repo_read(
            &auth,
            &executor,
            &multi_tenancy,
            "rp_01test",
        )
        .await
        .unwrap();

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            &[
                "resource:library:ViewRepo:trn:library:repo:rp_01test",
                "policy:library:ViewPrivateRepo"
            ]
        );
    }
}
