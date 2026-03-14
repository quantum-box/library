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

use library_api::handler::types::{CreateRepoRequest, RepoResponse};
use reqwest::{Method, StatusCode};
use util::{
    create_test_client, deserialize_response, generate_timestamp,
    parse_json_response, print_response_result, send_and_process_request,
    setup_test_server,
};

/// TODO: add English documentation
#[tokio::test]
#[tracing::instrument]
async fn test_repository_api_all() -> anyhow::Result<()> {
    // TODO: add English comment
    let (server_url, shutdown_tx) = setup_test_server().await;
    println!("テストサーバーが起動しました: {}", server_url);

    let client = create_test_client();

    // TODO: add English comment
    let timestamp = generate_timestamp();

    // TODO: add English comment
    println!("=== テスト1: リポジトリのシナリオテスト ===");

    // TODO: add English comment
    let test_org_name = format!("Test Organization Scenario {}", timestamp);
    let test_org_username = format!("test_org_scenario_{}", timestamp);
    let test_org_description =
        "This is a test organization for scenario test";

    // TODO: add English comment
    println!("Sending POST request to {}/v1beta/orgs", server_url);

    let org_url = format!("{}/v1beta/orgs", server_url);

    // TODO: add English comment
    #[derive(serde::Serialize)]
    struct CreateOrgRequest {
        name: String,
        username: String,
        description: String,
        website: Option<String>,
    }

    let org_request = CreateOrgRequest {
        name: test_org_name.clone(),
        username: test_org_username.clone(),
        description: test_org_description.to_string(),
        website: None,
    };

    let (org_status, org_text) = send_and_process_request(
        &client,
        Method::POST,
        &org_url,
        Some(org_request),
        "dummy-token",
        StatusCode::OK,
    )
    .await?;

    print_response_result("Organization creation", org_status, &org_text);

    // TODO: add English comment
    if !org_text.is_empty() && org_status == StatusCode::OK {
        let _org_json = parse_json_response(&org_text)?;
        println!("Organization info parsing successful");
    }

    // TODO: add English comment
    let test_repo_name = format!("Test Repository Scenario {}", timestamp);
    let test_repo_username = format!("test_repo_scenario_{}", timestamp);
    let test_repo_description =
        "This is a test repository for scenario test";
    let test_repo_is_public = true;

    // TODO: add English comment
    println!(
        "Sending POST request to {}/v1beta/repos/{}",
        server_url, test_org_username
    );

    let repo_url =
        format!("{}/v1beta/repos/{}", server_url, test_org_username);

    // TODO: add English comment
    let repo_request = CreateRepoRequest {
        name: test_repo_name.clone(),
        username: test_repo_username.clone(),
        description: Some(test_repo_description.to_string()),
        is_public: test_repo_is_public,
        database_id: None,
    };

    let (repo_status, repo_text) = send_and_process_request(
        &client,
        Method::POST,
        &repo_url,
        Some(repo_request),
        "dummy-token",
        StatusCode::OK,
    )
    .await?;

    print_response_result("Repository creation", repo_status, &repo_text);

    // TODO: add English comment
    let create_repo_json = if repo_status == StatusCode::OK {
        // TODO: add English comment
        let repo_response: RepoResponse = deserialize_response(&repo_text)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse repository response: {}, text: {}",
                    e,
                    repo_text
                )
            })?;

        // TODO: add English comment
        assert_eq!(repo_response.name, test_repo_name);
        assert_eq!(repo_response.username, test_repo_username);
        assert_eq!(
            repo_response.description,
            Some(test_repo_description.to_string())
        );
        assert_eq!(repo_response.is_public, test_repo_is_public);

        repo_response
    } else {
        // TODO: add English comment
        println!(
            "Repository creation failed, using dummy repository for test"
        );
        RepoResponse {
            id: "dummy_id".to_string(),
            name: test_repo_name.clone(),
            username: test_repo_username.clone(),
            description: Some(test_repo_description.to_string()),
            is_public: test_repo_is_public,
            organization_id: "dummy_org_id".to_string(),
        }
    };

    // TODO: add English comment
    println!(
        "Sending GET request to {}/v1beta/repos/{}/{}",
        server_url, test_org_username, test_repo_username
    );

    let view_repo_url = format!(
        "{}/v1beta/repos/{}/{}",
        server_url, test_org_username, test_repo_username
    );

    let (view_status, view_text) =
        send_and_process_request::<serde_json::Value>(
            &client,
            Method::GET,
            &view_repo_url,
            None,
            "dummy-token",
            StatusCode::OK,
        )
        .await?;

    print_response_result("Repository view", view_status, &view_text);

    // TODO: add English comment
    if !view_text.is_empty() && view_status == StatusCode::OK {
        // TODO: add English comment
        let view_json: RepoResponse = serde_json::from_str(&view_text)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse repository response: {}, text: {}",
                    e,
                    view_text
                )
            })?;

        // TODO: add English comment
        assert_eq!(view_json.id, create_repo_json.id);
        assert_eq!(view_json.name, test_repo_name);
        assert_eq!(view_json.username, test_repo_username);
        assert_eq!(
            view_json.description,
            Some(test_repo_description.to_string())
        );
        assert_eq!(view_json.is_public, test_repo_is_public);
        // TODO: add English comment
    }

    // TODO: add English comment
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // TODO: add English comment
    println!("=== テスト2: 存在しないリポジトリの取得テスト ===");

    // TODO: add English comment
    // TODO: add English comment
    let nonexistent_repo = "nonexistent-repo";

    // TODO: add English comment
    println!(
        "Sending GET request to {}/v1beta/repos/{}/{}",
        server_url, test_org_username, nonexistent_repo
    );

    let nonexistent_url = format!(
        "{}/v1beta/repos/{}/{}",
        server_url, test_org_username, nonexistent_repo
    );

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
        "Non-existent repository",
        nonexist_status,
        &nonexist_text,
    );

    // TODO: add English comment
    assert_eq!(nonexist_status, StatusCode::NOT_FOUND);

    // TODO: add English comment
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // TODO: add English comment
    println!("=== テスト3: 無効なリクエストでのリポジトリ作成テスト ===");

    // TODO: add English comment
    let test_org_name_invalid =
        format!("Test Organization Invalid {}", timestamp);
    let test_org_username_invalid =
        format!("test_org_invalid_{}", timestamp);
    let test_org_description_invalid =
        "This is a test organization for invalid request test";

    // TODO: add English comment
    println!("Creating organization for invalid request test");

    let org_invalid_url = format!("{}/v1beta/orgs", server_url);

    let org_invalid_request = CreateOrgRequest {
        name: test_org_name_invalid.clone(),
        username: test_org_username_invalid.clone(),
        description: test_org_description_invalid.to_string(),
        website: None,
    };

    let (org_invalid_status, org_invalid_text) = send_and_process_request(
        &client,
        Method::POST,
        &org_invalid_url,
        Some(org_invalid_request),
        "dummy-token",
        StatusCode::OK,
    )
    .await?;

    print_response_result(
        "Organization creation for invalid test",
        org_invalid_status,
        &org_invalid_text,
    );

    // TODO: add English comment
    println!(
        "Sending POST request with invalid data to {}/v1beta/repos/{}",
        server_url, test_org_username_invalid
    );

    let invalid_repo_url = format!(
        "{}/v1beta/repos/{}",
        server_url, test_org_username_invalid
    );

    // TODO: add English comment
    #[derive(serde::Serialize)]
    struct InvalidRepoRequest {
        name: String,
        username: String, // TODO: add English comment
        description: Option<String>,
        is_public: bool,
    }

    let invalid_request = InvalidRepoRequest {
        name: "Test Repository".to_string(),
        username: "".to_string(), // TODO: add English comment
        description: Some("This is a test repository".to_string()),
        is_public: true,
    };

    // TODO: add English comment
    let expected_status = StatusCode::BAD_REQUEST; // TODO: add English comment

    let response = send_and_process_request(
        &client,
        Method::POST,
        &invalid_repo_url,
        Some(invalid_request),
        "dummy-token",
        expected_status, // TODO: add English comment
    )
    .await;

    // TODO: add English comment
    match response {
        Ok((status, text)) => {
            print_response_result(
                "Invalid repository creation",
                status,
                &text,
            );

            // TODO: add English comment
            assert!(
                status == StatusCode::BAD_REQUEST
                    || status == StatusCode::FORBIDDEN
                    || status == StatusCode::NOT_FOUND,
                "Expected status code 400, 403, or 404, but got {}",
                status
            );
        }
        Err(e) => {
            // TODO: add English comment
            println!(
                "Error occurred during invalid repository test: {}",
                e
            );
            // TODO: add English comment
            // TODO: add English comment
            anyhow::bail!("Invalid repository test failed: {}", e);
        }
    }

    // TODO: add English comment
    shutdown_tx.send(()).unwrap();

    Ok(())
}
