use super::*;
use inbound_sync_domain::{
    PropertyMapping, Provider, WebhookEndpoint, WebhookEndpointId,
};
use integration_domain::{
    ExternalScope, ExternalSyncBinding, ExternalSyncBindingId,
    InboundChangeSetStatus, LibraryRepoId,
};
use serde_json::json;

mockall::mock! {
    #[derive(Debug)]
    Bindings {}
    #[async_trait::async_trait]
    impl ExternalSyncBindingRepository for Bindings {
        async fn save(
            &self,
            binding: &ExternalSyncBinding,
        ) -> errors::Result<()>;
        async fn find_by_id(
            &self,
            tenant_id: &TenantId,
            id: &ExternalSyncBindingId,
        ) -> errors::Result<Option<ExternalSyncBinding>>;
        async fn find_by_scope(
            &self,
            tenant_id: &TenantId,
            repo_id: &LibraryRepoId,
            provider: OAuthProvider,
            external_scope: &ExternalScope,
        ) -> errors::Result<Option<ExternalSyncBinding>>;
        async fn find_by_repo(
            &self,
            tenant_id: &TenantId,
            repo_id: &LibraryRepoId,
        ) -> errors::Result<Vec<ExternalSyncBinding>>;
    }
}

mockall::mock! {
    #[derive(Debug)]
    Links {}
    #[async_trait::async_trait]
    impl ExternalObjectLinkRepository for Links {
        async fn save(
            &self,
            tenant_id: &TenantId,
            link: &ExternalObjectLink,
        ) -> errors::Result<()>;
        async fn find_by_data(
            &self,
            tenant_id: &TenantId,
            binding_id: &ExternalSyncBindingId,
            data_id: &LibraryDataId,
        ) -> errors::Result<Option<ExternalObjectLink>>;
        async fn find_by_external_object(
            &self,
            tenant_id: &TenantId,
            binding_id: &ExternalSyncBindingId,
            external_object_id: &str,
        ) -> errors::Result<Option<ExternalObjectLink>>;
        async fn find_by_binding(
            &self,
            tenant_id: &TenantId,
            binding_id: &ExternalSyncBindingId,
        ) -> errors::Result<Vec<ExternalObjectLink>>;
        async fn find_by_tenant_and_data(
            &self,
            tenant_id: &TenantId,
            data_id: &LibraryDataId,
        ) -> errors::Result<Vec<ExternalObjectLink>>;
    }
}

mockall::mock! {
    #[derive(Debug)]
    Changes {}
    #[async_trait::async_trait]
    impl InboundChangeSetRepository for Changes {
        async fn save(
            &self,
            change_set: &InboundChangeSet,
        ) -> errors::Result<()>;
        async fn find_by_id(
            &self,
            tenant_id: &TenantId,
            id: &InboundChangeSetId,
        ) -> errors::Result<Option<InboundChangeSet>>;
        async fn find_by_binding(
            &self,
            tenant_id: &TenantId,
            binding_id: &ExternalSyncBindingId,
            status: Option<InboundChangeSetStatus>,
            limit: u32,
        ) -> errors::Result<Vec<InboundChangeSet>>;
    }
}

mockall::mock! {
    #[derive(Debug)]
    Endpoints {}
    #[async_trait::async_trait]
    impl WebhookEndpointRepository for Endpoints {
        async fn save(&self, endpoint: &WebhookEndpoint) -> errors::Result<()>;

        async fn find_by_id(
            &self,
            id: &WebhookEndpointId,
        ) -> errors::Result<Option<WebhookEndpoint>>;

        async fn find_by_tenant(
            &self,
            tenant_id: &TenantId,
        ) -> errors::Result<Vec<WebhookEndpoint>>;

        async fn find_by_tenant_and_provider(
            &self,
            tenant_id: &TenantId,
            provider: Provider,
        ) -> errors::Result<Vec<WebhookEndpoint>>;

        async fn delete(&self, id: &WebhookEndpointId) -> errors::Result<()>;
    }
}

#[derive(Debug)]
struct NoDataWrites;

#[async_trait::async_trait]
impl GitHubDataHandler for NoDataWrites {
    async fn upsert_data(
        &self,
        _endpoint: &WebhookEndpoint,
        _path: &str,
        _content: &str,
        _mapping: Option<&PropertyMapping>,
    ) -> errors::Result<String> {
        panic!("a rejected decision must not update Library data")
    }
    async fn delete_data(
        &self,
        _endpoint: &WebhookEndpoint,
        _data_id: &str,
    ) -> errors::Result<()> {
        panic!("a reviewed decision must not hard-delete Library data")
    }
}

fn change() -> InboundChangeSet {
    InboundChangeSet::create(
        TenantId::default(), ExternalSyncBindingId::generate(), None,
        "docs/verification.md", "commit-a", None, ExternalChangeType::Upsert,
        json!({"repository":"owner/repo", "ref":"main", "content":"reviewed content"}),
    ).unwrap()
}

#[tokio::test]
async fn accepting_a_terminal_change_never_reaches_the_apply_adapter() {
    for prior_decision in [
        InboundChangeSetDecision::Accept,
        InboundChangeSetDecision::Reject,
    ] {
        let mut change = change();
        match prior_decision {
            InboundChangeSetDecision::Accept => {
                change.accept(None).unwrap()
            }
            InboundChangeSetDecision::Reject => {
                change.reject(None).unwrap()
            }
        }
        let stored = change.clone();
        let mut changes = MockChanges::new();
        changes
            .expect_find_by_id()
            .times(1)
            .returning(move |_, _| Ok(Some(stored.clone())));
        // Every other mock has zero expected calls: no endpoint lookup, data write,
        // link update, or decision save is allowed for an already-decided change.
        let usecase = DecideInboundChangeSet::new(
            Arc::new(MockBindings::new()),
            Arc::new(MockLinks::new()),
            Arc::new(changes),
            Arc::new(MockEndpoints::new()),
            Arc::new(NoDataWrites),
        );
        let error = usecase
            .execute(
                change.tenant_id(),
                change.id(),
                InboundChangeSetDecision::Accept,
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(error, errors::Error::Conflict { .. }));
    }
}

#[tokio::test]
async fn failed_acceptance_does_not_persist_an_accepted_decision() {
    let change = change();
    let stored = change.clone();
    let mut changes = MockChanges::new();
    changes
        .expect_find_by_id()
        .times(1)
        .returning(move |_, _| Ok(Some(stored.clone())));
    let mut bindings = MockBindings::new();
    bindings.expect_find_by_id().times(1).returning(|_, _| {
        Err(errors::Error::service_unavailable(
            "temporary lookup failure",
        ))
    });
    let usecase = DecideInboundChangeSet::new(
        Arc::new(bindings),
        Arc::new(MockLinks::new()),
        Arc::new(changes),
        Arc::new(MockEndpoints::new()),
        Arc::new(NoDataWrites),
    );
    assert!(usecase
        .execute(
            change.tenant_id(),
            change.id(),
            InboundChangeSetDecision::Accept,
            None
        )
        .await
        .is_err());
}

#[tokio::test]
async fn rejecting_a_pending_change_only_saves_the_decision() {
    let change = change();
    let stored = change.clone();
    let mut changes = MockChanges::new();
    changes
        .expect_find_by_id()
        .times(1)
        .returning(move |_, _| Ok(Some(stored.clone())));
    changes
        .expect_save()
        .times(1)
        .withf(|change| change.status() == InboundChangeSetStatus::Rejected)
        .returning(|_| Ok(()));
    let usecase = DecideInboundChangeSet::new(
        Arc::new(MockBindings::new()),
        Arc::new(MockLinks::new()),
        Arc::new(changes),
        Arc::new(MockEndpoints::new()),
        Arc::new(NoDataWrites),
    );
    let result = usecase
        .execute(
            change.tenant_id(),
            change.id(),
            InboundChangeSetDecision::Reject,
            Some("keep Library content".into()),
        )
        .await
        .unwrap();
    assert_eq!(result.status(), InboundChangeSetStatus::Rejected);
    assert_eq!(result.decision_note(), Some("keep Library content"));
}
