use std::sync::Arc;

use derive_new::new;
use errors::Result;
use value_object::Identifier;

use crate::{
    domain::{Organization, OrganizationRepository, LIBRARY_TENANT},
    sdk_auth::SdkAuthApp,
    ttl_cache::TtlCache,
    usecase::GetOrganizationByUsernameQuery,
};

/// How long tachyon-api's "no such operator" answer is reused.
///
/// The name is taken from the request, often an anonymous one, so
/// without this every distinct unknown name in every request costs a
/// tachyon-api round trip. A name registered meanwhile resolves once
/// this window passes; one created in Library is found in the database,
/// which is always asked first.
const UNKNOWN_USERNAME_TTL: std::time::Duration =
    std::time::Duration::from_secs(30);

/// Distinct unknown names remembered at once. When full, the oldest is
/// dropped, so a flood of names costs memory up to here and no further.
const UNKNOWN_USERNAME_CAPACITY: usize = 4096;

#[derive(Debug, Clone, new)]
pub struct GetOrganizationByUsernameQueryImpl {
    sdk: Arc<SdkAuthApp>,
    org_repo: Arc<dyn OrganizationRepository>,
    #[new(value = "Arc::new(TtlCache::new(UNKNOWN_USERNAME_TTL, \
                   UNKNOWN_USERNAME_CAPACITY))")]
    unknown_usernames: Arc<TtlCache<String, ()>>,
}

#[async_trait::async_trait]
impl GetOrganizationByUsernameQuery for GetOrganizationByUsernameQueryImpl {
    #[tracing::instrument(
        name = "GetOrganizationByUsernameQueryImpl::execute",
        skip(self)
    )]
    async fn execute(
        &self,
        org_username: &Identifier,
    ) -> Result<Option<Organization>> {
        if let Some(org) =
            self.org_repo.get_by_username(org_username).await?
        {
            return Ok(Some(org));
        }

        let username_str = org_username.to_string();
        if self.unknown_usernames.get(&username_str).is_some() {
            return Err(unknown_operator());
        }
        let operator = match self
            .sdk
            .get_operator_by_alias(&LIBRARY_TENANT, &username_str)
            .await
        {
            Ok(operator) => operator,
            Err(error) => {
                if matches!(error, errors::Error::NotFound { .. }) {
                    self.unknown_usernames.insert(username_str, ());
                }
                return Err(error);
            }
        };

        let tenant_id = value_object::TenantId::new(&operator.id)?;
        let org = self.org_repo.get_by_id(&tenant_id).await?;

        Ok(org)
    }
}

/// The error `SdkAuthApp::get_operator_by_alias` returns for an alias
/// tachyon-api does not know, so a cached answer reads the same as a
/// fresh one.
fn unknown_operator() -> errors::Error {
    errors::Error::not_found("Operator".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::OrganizationRepository;
    use value_object::{LongText, TenantId, Text};

    #[derive(Debug)]
    struct LocalOrgRepo {
        org: Organization,
    }

    #[async_trait::async_trait]
    impl OrganizationRepository for LocalOrgRepo {
        async fn insert(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn update(
            &self,
            _organization: &Organization,
        ) -> errors::Result<()> {
            Ok(())
        }

        async fn get_by_id(
            &self,
            org_id: &TenantId,
        ) -> errors::Result<Option<Organization>> {
            Ok((self.org.id() == org_id).then(|| self.org.clone()))
        }

        async fn get_by_username(
            &self,
            username: &Identifier,
        ) -> errors::Result<Option<Organization>> {
            Ok((self.org.username() == username).then(|| self.org.clone()))
        }

        async fn find_all(&self) -> errors::Result<Vec<Organization>> {
            Ok(vec![self.org.clone()])
        }

        async fn delete(&self, _org_id: &TenantId) -> errors::Result<()> {
            Ok(())
        }
    }

    /// A tachyon-api stand-in that knows no operator and counts how
    /// often it is asked.
    async fn spawn_empty_tachyon(
    ) -> (String, Arc<std::sync::atomic::AtomicUsize>) {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let app = axum::Router::new().route(
            "/v1/auth/operators/by-alias",
            axum::routing::get(move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    axum::http::StatusCode::NOT_FOUND
                }
            }),
        );
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{addr}"), hits)
    }

    #[tokio::test]
    async fn an_unknown_username_is_asked_of_tachyon_once_per_window() {
        use std::sync::atomic::Ordering;

        let (base_url, hits) = spawn_empty_tachyon().await;
        let org = Organization::new(
            &TenantId::default(),
            &Text::new("Aser").unwrap(),
            &"aser".parse().unwrap(),
            None,
            None,
        );
        let query = GetOrganizationByUsernameQueryImpl::new(
            Arc::new(SdkAuthApp::new(&base_url, &LIBRARY_TENANT, "token")),
            Arc::new(LocalOrgRepo { org: org.clone() }),
        );
        let ghost: Identifier = "ghost".parse().unwrap();

        for _ in 0..3 {
            let error = query.execute(&ghost).await.unwrap_err();
            assert!(
                matches!(error, errors::Error::NotFound { .. }),
                "{error:?}"
            );
        }
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        // Another unknown name is its own lookup, and a local org is
        // still served from the database.
        let _ = query.execute(&"phantom".parse().unwrap()).await;
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        assert!(query
            .execute(&"aser".parse().unwrap())
            .await
            .unwrap()
            .is_some());
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn execute_prefers_local_library_org_username() {
        let org = Organization::new(
            &TenantId::default(),
            &Text::new("Aser").unwrap(),
            &"aser".parse().unwrap(),
            Some(&LongText::new("Local Library org").unwrap()),
            None,
        );
        let query = GetOrganizationByUsernameQueryImpl::new(
            Arc::new(SdkAuthApp::new(
                "http://127.0.0.1:9",
                &LIBRARY_TENANT,
                "unused-token",
            )),
            Arc::new(LocalOrgRepo { org: org.clone() }),
        );

        let resolved =
            query.execute(&"aser".parse().unwrap()).await.unwrap();

        assert_eq!(
            resolved.map(|org| org.id().clone()),
            Some(org.id().clone())
        );
    }
}
