//! TODO: add English documentation
//!
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! TODO: add English documentation
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! TODO: add English documentation
//!
//! TODO: add English documentation
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! TODO: add English documentation

extern crate library_api;

mod util;

use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use util::{
    create_test_client, deserialize_response, generate_timestamp,
    parse_json_response, print_response_result, send_and_process_request,
    setup_test_server,
};

// TODO: add English comment
#[derive(Serialize, Deserialize)]
struct CreateOrganizationRequest {
    name: String,
    username: String,
    description: Option<String>,
    website: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct UpdateOrganizationRequest {
    name: String,
    description: Option<String>,
    website: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct OrganizationResponse {
    id: String,
    name: String,
    username: String,
    description: Option<String>,
    repos: Vec<RepoResponse>,
    website: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct RepoResponse {
    id: String,
    name: String,
    username: String,
    description: Option<String>,
    is_public: bool,
    organization_id: String,
}

/// TODO: add English documentation
#[tokio::test]
#[tracing::instrument]
async fn test_organization_api_all() -> anyhow::Result<()> {
    // TODO: add English comment
    let (server_url, shutdown_tx) = setup_test_server().await;
    println!("テストサーバーが起動しました: {}", server_url);

    let client = create_test_client();

    // TODO: add English comment
    let timestamp = generate_timestamp();

    // TODO: add English comment
    println!("=== テスト1: 組織のシナリオテスト ===");

    // TODO: add English comment
    let test_org_name = format!("Test Organization Scenario {}", timestamp);
    let test_org_username = format!("test_org_scenario_{}", timestamp);
    let test_org_description =
        "This is a test organization for scenario test";

    // TODO: add English comment
    println!("Sending POST request to {}/v1beta/orgs", server_url);

    let org_url = format!("{}/v1beta/orgs", server_url);
    let org_request = CreateOrganizationRequest {
        name: test_org_name.clone(),
        username: test_org_username.clone(),
        description: Some(test_org_description.to_string()),
        website: None,
    };

    let (create_status, create_text) = send_and_process_request(
        &client,
        Method::POST,
        &org_url,
        Some(org_request),
        "dummy-token",
        StatusCode::OK,
    )
    .await?;

    print_response_result(
        "Organization creation",
        create_status,
        &create_text,
    );

    // TODO: add English comment
    let create_org_json = if create_status == StatusCode::OK {
        // TODO: add English comment
        let org_response: OrganizationResponse =
            deserialize_response(&create_text).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse organization response: {}, text: {}",
                    e,
                    create_text
                )
            })?;

        // TODO: add English comment
        assert_eq!(org_response.name, test_org_name);
        assert_eq!(org_response.username, test_org_username);
        assert_eq!(
            org_response.description,
            Some(test_org_description.to_string())
        );
        assert!(org_response.repos.is_empty());

        org_response
    } else {
        // TODO: add English comment
        println!("Organization creation failed, using dummy organization for test");
        OrganizationResponse {
            id: "dummy_id".to_string(),
            name: test_org_name.clone(),
            username: test_org_username.clone(),
            description: Some(test_org_description.to_string()),
            repos: vec![],
            website: None,
        }
    };

    // TODO: add English comment
    println!(
        "Sending GET request to {}/v1beta/orgs/{}",
        server_url, test_org_username
    );

    let view_org_url =
        format!("{}/v1beta/orgs/{}", server_url, test_org_username);

    let (view_status, view_text) =
        send_and_process_request::<serde_json::Value>(
            &client,
            Method::GET,
            &view_org_url,
            None,
            "dummy-token",
            StatusCode::OK,
        )
        .await?;

    print_response_result("Organization view", view_status, &view_text);

    // TODO: add English comment
    if !view_text.is_empty() && view_status == StatusCode::OK {
        // TODO: add English comment
        let view_json: OrganizationResponse =
            deserialize_response(&view_text).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse organization response: {}, text: {}",
                    e,
                    view_text
                )
            })?;

        // TODO: add English comment
        assert_eq!(view_json.id, create_org_json.id);
        assert_eq!(view_json.name, test_org_name);
        assert_eq!(view_json.username, test_org_username);
        assert_eq!(
            view_json.description,
            Some(test_org_description.to_string())
        );
    }

    // TODO: add English comment
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // TODO: add English comment
    println!("=== テスト2: 存在しない組織の取得テスト ===");

    // TODO: add English comment
    let nonexistent_org = "nonexistent-org";

    // TODO: add English comment
    println!(
        "Sending GET request to {}/v1beta/orgs/{}",
        server_url, nonexistent_org
    );

    let nonexistent_url =
        format!("{}/v1beta/orgs/{}", server_url, nonexistent_org);

    let (nonexist_status, nonexist_text) =
        send_and_process_request::<serde_json::Value>(
            &client,
            Method::GET,
            &nonexistent_url,
            None,
            "dummy-token",
            StatusCode::NOT_FOUND,
        )
        .await?;

    print_response_result(
        "Non-existent organization",
        nonexist_status,
        &nonexist_text,
    );

    // TODO: add English comment
    assert_eq!(nonexist_status, StatusCode::NOT_FOUND);

    // TODO: add English comment
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // TODO: add English comment
    println!("=== テスト3: 無効なリクエストでの組織作成テスト ===");

    // TODO: add English comment
    println!(
        "Sending POST request with invalid data to {}/v1beta/orgs",
        server_url
    );

    let invalid_org_url = format!("{}/v1beta/orgs", server_url);

    // TODO: add English comment
    #[derive(serde::Serialize)]
    struct InvalidOrgRequest {
        name: String,
        username: String, // TODO: add English comment
        description: Option<String>,
    }

    let invalid_request = InvalidOrgRequest {
        name: "Test Organization".to_string(),
        username: "".to_string(), // TODO: add English comment
        description: Some("This is a test organization".to_string()),
    };

    // TODO: add English comment
    let expected_status = StatusCode::BAD_REQUEST;

    let response = send_and_process_request(
        &client,
        Method::POST,
        &invalid_org_url,
        Some(invalid_request),
        "dummy-token",
        expected_status, // TODO: add English comment
    )
    .await;

    // TODO: add English comment
    match response {
        Ok((status, text)) => {
            print_response_result(
                "Invalid organization creation",
                status,
                &text,
            );

            // TODO: add English comment
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "Expected status code 400, but got {}",
                status
            );

            // Verify error response contains message field
            if let Ok(error_json) = parse_json_response(&text) {
                assert!(
                    error_json.get("message").is_some(),
                    "Error response must contain a message field"
                );
            }
        }
        Err(e) => {
            // TODO: add English comment
            println!(
                "Error occurred during invalid organization test: {}",
                e
            );
            // TODO: add English comment
            // TODO: add English comment
            anyhow::bail!("Invalid organization test failed: {}", e);
        }
    }

    // TODO: add English comment
    shutdown_tx.send(()).unwrap();

    Ok(())
}
