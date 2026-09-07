use super::*;
use crate::domain::{Organization, OrganizationRepository};
use std::sync::Mutex;
use value_object::{Identifier, TenantId};

fn user(tenants: Vec<TenantId>) -> LibraryExecutor {
    LibraryExecutor {
        inner: LibraryExecutorKind::User(Box::new(
            tachyon_sdk::auth::User {
                id: "us_01hs2yepy5hw4rz8pdq2wywnwt".parse().unwrap(),
                username: "mcp-test-user".into(),
                tenants,
                email: None,
                name: Some("MCP Test User".into()),
                email_verified: None,
                image: None,
                role: tachyon_sdk::auth::DefaultRole::General,
                metadata: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
        )),
        original_token: Some("test-user-token".into()),
    }
}

#[derive(Debug)]
struct OrgRepository {
    rows: Vec<Organization>,
    lookups: Mutex<Vec<TenantId>>,
}

#[async_trait::async_trait]
impl OrganizationRepository for OrgRepository {
    async fn get_by_id(
        &self,
        id: &TenantId,
    ) -> errors::Result<Option<Organization>> {
        self.lookups.lock().unwrap().push(id.clone());
        Ok(self.rows.iter().find(|org| org.id() == id).cloned())
    }
    async fn get_by_username(
        &self,
        _: &Identifier,
    ) -> errors::Result<Option<Organization>> {
        panic!("unexpected slug lookup")
    }
    async fn find_all(&self) -> errors::Result<Vec<Organization>> {
        panic!("must never enumerate all tenants")
    }
    async fn insert(&self, _: &Organization) -> errors::Result<()> {
        panic!("read must not write")
    }
    async fn update(&self, _: &Organization) -> errors::Result<()> {
        panic!("read must not write")
    }
    async fn delete(&self, _: &TenantId) -> errors::Result<()> {
        panic!("read must not delete")
    }
}

fn organization(slug: &str) -> Organization {
    Organization::new(
        &TenantId::default(),
        &slug.parse().unwrap(),
        &slug.parse().unwrap(),
        None,
        None,
    )
}

#[test]
fn anonymous_library_callers_have_no_system_or_tenant_privileges() {
    let executor = anonymous_executor();
    assert!(executor.is_none());
    assert!(!executor.is_system_user());
    assert!(!executor.has_tenant_id(&TenantId::default()));
    assert!(tachyon_sdk::auth::Executor::SystemUser.is_system_user());
}

#[tokio::test]
async fn org_discovery_intersects_verified_memberships_with_library_orgs() {
    let alpha = organization("alpha");
    let zeta = organization("zeta");
    let outsider = organization("outsider");
    let another_product = TenantId::default();
    let executor = user(vec![
        zeta.id().clone(),
        alpha.id().clone(),
        alpha.id().clone(),
        another_product,
    ]);
    let repository = OrgRepository {
        rows: vec![outsider.clone(), zeta, alpha],
        lookups: Mutex::new(vec![]),
    };
    let orgs = member_organizations(&repository, &executor).await.unwrap();
    assert_eq!(
        orgs.iter()
            .map(|org| org.username.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
    let lookups = repository.lookups.lock().unwrap();
    assert_eq!(
        lookups.len(),
        3,
        "duplicate memberships must be looked up once"
    );
    assert!(!lookups.contains(outsider.id()));
}

#[tokio::test]
async fn service_account_org_discovery_stays_in_the_key_tenant() {
    let own = organization("own");
    let other = organization("other");
    let executor = LibraryExecutor {
        inner: LibraryExecutorKind::ServiceAccount(Box::new(
            tachyon_sdk::auth::ServiceAccount {
                id: Default::default(),
                tenant_id: own.id().clone(),
                name: "test key".into(),
                created_at: chrono::Utc::now(),
            },
        )),
        original_token: Some("pk_test".into()),
    };
    let repository = OrgRepository {
        rows: vec![own, other],
        lookups: Mutex::new(vec![]),
    };
    let orgs = member_organizations(&repository, &executor).await.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].username, "own");
    assert_eq!(
        member_organizations(&repository, &anonymous_executor())
            .await
            .unwrap_err()["code"],
        -32001
    );
}

#[test]
fn every_write_tool_requires_auth_and_discovery_is_available_before_login()
{
    let anonymous = tools_list_result(false);
    for name in ["get_me", "list_orgs", "list_repos"] {
        assert!(anonymous["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == name));
    }
    for tool in tools_list_result(true)["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        if tool["annotations"]["readOnlyHint"] == false {
            assert!(
                requires_auth_tool(name),
                "write {name} must initiate OAuth"
            );
        }
    }
    for name in ["get_me", "list_orgs", "search_repos"] {
        assert!(requires_auth_tool(name));
    }
    assert!(
        !requires_auth_tool("get_data"),
        "known public records remain anonymous-readable"
    );
}

#[test]
fn patch_null_and_omission_are_distinct() {
    let missing: UpdateSourceArgs =
        parse_tool_args(json!({"org":"a", "repo":"b", "source_id":"s"}))
            .unwrap();
    let clear: UpdateSourceArgs = parse_tool_args(
        json!({"org":"a", "repo":"b", "source_id":"s", "url":null}),
    )
    .unwrap();
    assert_eq!(missing.url, None);
    assert_eq!(clear.url, Some(None));
    let org: UpdateOrgArgs =
        parse_tool_args(json!({"org":"a", "website":null})).unwrap();
    assert_eq!(org.name, None);
    assert_eq!(org.description, None);
    assert_eq!(org.website, Some(None));
}

#[test]
fn typed_values_round_trip_without_markdown_or_csv_loss() {
    use database_manager::domain::{
        DataId, DatabaseId, SelectItem, SelectItemId, TypeMultiSelect,
        TypeSelect,
    };
    let variants = [
        PropertyDataValue::String("a,b\nquote: \"value\"".into()),
        PropertyDataValue::Integer(42),
        PropertyDataValue::Html("<p>Hello</p>".into()),
        PropertyDataValue::Markdown("# Hello".into()),
        PropertyDataValue::RichText(
            json!([{"type":"paragraph","content":"Hello"}]),
        ),
        PropertyDataValue::Boolean(false),
        PropertyDataValue::Id("external-42".into()),
        PropertyDataValue::Location(
            value_object::Location::new(35.0, 139.0).unwrap(),
        ),
        PropertyDataValue::Relation(
            DatabaseId::default(),
            vec![DataId::default()],
        ),
        PropertyDataValue::Select(SelectItemId::default()),
        PropertyDataValue::MultiSelect(vec![SelectItemId::default()]),
        PropertyDataValue::Date("2026-09-07".into()),
        PropertyDataValue::Image("https://example.com/image.png".into()),
    ];
    for value in variants {
        let item = |id: &SelectItemId| {
            SelectItem::new(
                id.clone(),
                "option".parse().unwrap(),
                "Option".parse().unwrap(),
            )
        };
        let property_type = match &value {
            PropertyDataValue::Select(id) => {
                PropertyType::Select(TypeSelect::new(vec![item(id)]))
            }
            PropertyDataValue::MultiSelect(ids) => {
                PropertyType::MultiSelect(TypeMultiSelect::new(
                    ids.iter().map(item).collect(),
                ))
            }
            _ => value.property_type(),
        };
        let property = Property::new(
            &Default::default(),
            &TenantId::default(),
            &Default::default(),
            "test",
            &property_type,
            false,
            0,
        );
        let (typ, json) = property_value_to_mcp(&value);
        let input = property_data_value(json, Some(&typ)).unwrap();
        let command =
            crate::usecase::property_value_adapter::property_value_command(
                &property, &input,
            )
            .unwrap();
        let restored =
            database_manager::domain::PropertyData::from_command(
                &property, command,
            )
            .unwrap();
        assert_eq!(
            restored.value().as_ref(),
            Some(&value),
            "round-trip {typ}"
        );
    }
}

#[test]
fn invalid_locations_and_pages_are_rejected_and_null_is_not_literal_text() {
    assert_eq!(
        property_data_value(
            json!({"latitude":91,"longitude":0}),
            Some("location")
        )
        .unwrap_err()["code"],
        -32602
    );
    assert!(
        matches!(property_data_value(Value::Null, Some("string")).unwrap(), PropertyDataValueInputData::String(text) if text.is_empty())
    );
    let args: PaginationArgs = parse_tool_args(Value::Null).unwrap();
    assert_eq!(
        OffsetPage::from_options(args.page, args.page_size)
            .unwrap()
            .current_page(),
        1
    );
    assert!(OffsetPage::from_options(Some(0), None).is_err());
    assert!(OffsetPage::from_options(None, Some(101)).is_err());
    let (items, paginator) =
        paginate(vec![1, 2, 3], OffsetPage::new(3, 2).unwrap());
    assert!(items.is_empty());
    assert_eq!(paginator.current_page, 3);
    assert_eq!(paginator.total_pages, 2);
}

#[test]
fn bearer_authentication_accepts_case_insensitive_scheme() {
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, "bearer test-token".parse().unwrap());
    assert_eq!(bearer_token(&headers).as_deref(), Some("test-token"));
}
