/*!
 * # Organization Repository Implementation
 *
 * This module implements the repository pattern for Organization entities.
 *
 * ## Important Concepts:
 *
 * - `LIBRARY_TENANT`: A constant representing the platform ID for the library service.
 *   This is a fixed value used to identify the service itself.
 *
 * - Organization ID: Each organization entity has its own unique ID, which is of type `TenantId`.
 *   Despite the type name, this is used as an entity identifier for organizations.
 *
 * In the database, organizations are stored with both their own ID and the platform ID (LIBRARY_TENANT).
 */

use crate::domain::Organization;
use crate::domain::OrganizationRepository;
use crate::domain::LIBRARY_TENANT;
use derive_new::new;
use std::sync::Arc;
use value_object::TenantId;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrganizationRow {
    id: String,
    name: String,
    username: String,
    description: Option<String>,
    website: Option<String>,
}

impl TryFrom<OrganizationRow> for Organization {
    type Error = errors::Error;
    fn try_from(value: OrganizationRow) -> Result<Self, Self::Error> {
        Ok(Organization::new(
            &value.id.parse()?,
            &value.name.parse()?,
            &value.username.parse()?,
            value.description.map(|d| d.parse()).transpose()?.as_ref(),
            value.website.map(|w| w.parse()).transpose()?.as_ref(),
        ))
    }
}

#[derive(Debug, Clone, new)]
pub struct OrganizationRepositoryImpl {
    db: Arc<persistence::Db>,
}

#[async_trait::async_trait]
impl OrganizationRepository for OrganizationRepositoryImpl {
    async fn insert(&self, entity: &Organization) -> errors::Result<()> {
        self.save(entity).await
    }

    async fn update(&self, entity: &Organization) -> errors::Result<()> {
        self.save(entity).await
    }

    async fn get_by_id(
        &self,
        id: &TenantId,
    ) -> errors::Result<Option<Organization>> {
        let row = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT id, name, username, description, website
            FROM library.organizations
            WHERE platform_id = ? AND id = ?
            "#,
        )
        .bind(LIBRARY_TENANT.to_string())
        .bind(id.to_string())
        .fetch_optional(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        Ok(row.map(|r| r.try_into()).transpose()?)
    }

    async fn find_all(&self) -> errors::Result<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, OrganizationRow>(
            r#"
            SELECT id, name, username, description, website
            FROM library.organizations
            WHERE platform_id = ?
            "#,
        )
        .bind(LIBRARY_TENANT.to_string())
        .fetch_all(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;

        let mut result = Vec::new();
        for row in orgs {
            result.push(row.try_into()?);
        }
        Ok(result)
    }

    async fn delete(&self, org_id: &TenantId) -> errors::Result<()> {
        sqlx::query(
            r#"DELETE FROM library.organizations WHERE platform_id = ? AND id = ?"#,
        )
        .bind(LIBRARY_TENANT.to_string())
        .bind(org_id.to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;
        Ok(())
    }
}

impl OrganizationRepositoryImpl {
    async fn save(&self, entity: &Organization) -> errors::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO library.organizations (id, name, username, description, website, platform_id)
            VALUES (?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                name = VALUES(name),
                description = VALUES(description),
                website = VALUES(website)
            "#,
        )
        .bind(entity.id().to_string())
        .bind(entity.name().to_string())
        .bind(entity.username().to_string())
        .bind(entity.description().as_ref().map(|d| d.to_string()))
        .bind(entity.website().as_ref().map(|w| w.to_string()))
        .bind(LIBRARY_TENANT.to_string())
        .execute(self.db.pool().as_ref())
        .await
        .map_err(errors::Error::internal_server_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::LIBRARY_TENANT;
    use persistence::test_helper::setup_test_db;
    use pretty_assertions::assert_eq;
    use value_object::{LongText, Text};

    async fn create_test_organization(
        tenant_id: &TenantId,
    ) -> Organization {
        Organization::new(
            tenant_id,
            &Text::new("Test Organization").unwrap(),
            &"test-org".parse().unwrap(),
            Some(&LongText::new("Test Description").unwrap()),
            None,
        )
    }

    async fn setup_test_env(
    ) -> (Arc<persistence::Db>, OrganizationRepositoryImpl) {
        let db = setup_test_db("library").await;

        // Cleanup existing data - use non-macro query for test code to avoid SQLX_OFFLINE cache issues
        sqlx::query(
            "DELETE FROM library.organizations WHERE platform_id = ?",
        )
        .bind(LIBRARY_TENANT.to_string())
        .execute(db.pool().as_ref())
        .await
        .unwrap();

        let repo = OrganizationRepositoryImpl::new(db.clone());
        (db, repo)
    }

    #[tokio::test]
    async fn test_save_and_get_by_id() -> errors::Result<()> {
        let (_db, repo) = setup_test_env().await;
        let tenant_id = TenantId::default();

        let org = create_test_organization(&tenant_id).await;
        repo.insert(&org).await?;

        let saved_org = repo.get_by_id(org.id()).await?.unwrap();
        assert_eq!(saved_org.id(), org.id());
        assert_eq!(saved_org.name(), org.name());
        assert_eq!(saved_org.username(), org.username());
        assert_eq!(saved_org.description(), org.description());

        // Test update
        let updated_name = Text::new("Updated Organization").unwrap();
        let updated_description =
            LongText::new("Updated Description").unwrap();
        let updated_org = Organization::new(
            org.id(),
            &updated_name,
            org.username(),
            Some(&updated_description),
            None,
        );

        repo.update(&updated_org).await?;

        let saved_org = repo.get_by_id(org.id()).await?.unwrap();
        assert_eq!(saved_org.name().to_string(), "Updated Organization");
        assert_eq!(
            saved_org.description().as_ref().unwrap().to_string(),
            "Updated Description"
        );
        assert_eq!(saved_org.username(), org.username());

        // Test that attempting to update username does not affect the stored username
        let attempt_username_update = Organization::new(
            org.id(),
            &updated_name,
            &"new-username".parse().unwrap(),
            Some(&updated_description),
            None,
        );

        repo.update(&attempt_username_update).await?;

        let saved_org = repo.get_by_id(org.id()).await?.unwrap();
        assert_eq!(saved_org.username(), org.username());

        Ok(())
    }

    #[tokio::test]
    async fn test_find_all() -> errors::Result<()> {
        let (_db, repo) = setup_test_env().await;

        // Create first organization with its own ID
        let org1_id = TenantId::default();
        let org1 = Organization::new(
            &org1_id,
            &Text::new("Test Organization").unwrap(),
            &"test-org".parse().unwrap(),
            Some(&LongText::new("Test Description").unwrap()),
            None,
        );

        // Create second organization with a different ID
        let org2_id = TenantId::default();
        let org2 = Organization::new(
            &org2_id,
            &Text::new("Another Organization").unwrap(),
            &"another-org".parse().unwrap(),
            Some(&LongText::new("Another Description").unwrap()),
            None,
        );

        // Save both organizations
        repo.insert(&org1).await?;
        repo.insert(&org2).await?;

        // Debug: Print organization IDs
        println!("org1 id: {}", org1.id());
        println!("org2 id: {}", org2.id());

        let orgs = repo.find_all().await?;
        println!("Found {} organizations", orgs.len());
        for org in &orgs {
            println!("Found org: {} ({})", org.name(), org.id());
        }

        assert_eq!(
            orgs.len(),
            2,
            "Expected to find only organizations with the given tenant_id"
        );
        assert!(orgs.iter().any(|o| o.id() == org1.id()));
        assert!(orgs.iter().any(|o| o.id() == org2.id()));

        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> errors::Result<()> {
        let (_db, repo) = setup_test_env().await;
        let tenant_id = TenantId::default();

        let org = create_test_organization(&tenant_id).await;
        repo.insert(&org).await?;

        // Verify organization exists
        let saved_org = repo.get_by_id(org.id()).await?;
        assert!(saved_org.is_some());

        // Delete organization
        repo.delete(org.id()).await?;

        // Verify organization no longer exists
        let deleted_org = repo.get_by_id(org.id()).await?;
        assert!(deleted_org.is_none());

        Ok(())
    }
}
