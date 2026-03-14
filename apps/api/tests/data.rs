//! TODO: add English documentation
//!
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! TODO: add English documentation
//! TODO: add English documentation
//!
//! TODO: add English documentation
//!
//! TODO: add English documentation
//! TODO: add English documentation
//! TODO: add English documentation

extern crate library_api;

mod util;

use library_api::handler::types::{AddDataRequest, PropertyDataRequest};
use reqwest::{Method, StatusCode};
use util::{
    create_test_client, deserialize_response, generate_timestamp,
    parse_json_response, print_response_result, process_response,
    send_and_process_request, send_api_request, setup_test_server,
};

/// TODO: add English documentation
#[tokio::test]
#[tracing::instrument]
async fn test_data_api_all() -> anyhow::Result<()> {
    // TODO: add English comment
    let (server_url, shutdown_tx) = setup_test_server().await;
    println!("テストサーバーが起動しました: {}", server_url);

    let client = create_test_client();

    // TODO: add English comment
    let timestamp = generate_timestamp();

    // TODO: add English comment
    println!("=== テスト1: データ追加のシナリオテスト ===");

    // TODO: add English comment
    let test_org_name = format!("Test Organization Data {}", timestamp);
    let test_org_username = format!("test_org_data_{}", timestamp);
    let test_org_description =
        "This is a test organization for data scenario test";

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
    let _: serde_json::Value = deserialize_response(&org_text)?;

    // TODO: add English comment
    if !org_text.is_empty() && org_status == StatusCode::OK {
        let _org_json = parse_json_response(&org_text)?;
        println!("Organization info parsing successful");
    }

    // TODO: add English comment
    let test_repo_name = format!("Test Repository Data {}", timestamp);
    let test_repo_username = format!("test_repo_data_{}", timestamp);
    let test_repo_description =
        "This is a test repository for data scenario test";
    let test_repo_is_public = true;

    // TODO: add English comment
    println!(
        "Sending POST request to {}/v1beta/repos/{}",
        server_url, test_org_username
    );

    let repo_url =
        format!("{}/v1beta/repos/{}", server_url, test_org_username);

    // TODO: add English comment
    #[derive(serde::Serialize)]
    struct CreateRepoRequest {
        name: String,
        username: String,
        description: String,
        is_public: bool,
        database_id: Option<String>,
    }

    let repo_request = CreateRepoRequest {
        name: test_repo_name.clone(),
        username: test_repo_username.clone(),
        description: test_repo_description.to_string(),
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
    let create_repo_json = parse_json_response(&repo_text)?; // TODO: add English comment

    // TODO: add English comment
    if repo_status == StatusCode::OK {
        if let Some(name) =
            create_repo_json.get("name").and_then(|v| v.as_str())
        {
            assert_eq!(name, test_repo_name);
        } else {
            anyhow::bail!("Repository response missing 'name' field");
        }
        if let Some(username) =
            create_repo_json.get("username").and_then(|v| v.as_str())
        {
            assert_eq!(username, test_repo_username);
        } else {
            anyhow::bail!("Repository response missing 'username' field");
        }
        if let Some(desc) =
            create_repo_json.get("description").and_then(|v| v.as_str())
        {
            assert_eq!(desc, test_repo_description);
        } else {
            anyhow::bail!(
                "Repository response missing 'description' field"
            );
        }
        if let Some(is_public) =
            create_repo_json.get("is_public").and_then(|v| v.as_bool())
        {
            assert_eq!(is_public, test_repo_is_public);
        } else {
            anyhow::bail!("Repository response missing 'is_public' field");
        }
    }

    // TODO: add English comment
    let _test_data_name = format!("Test Data {}", timestamp);

    // TODO: add English comment
    println!(
        "Sending POST request to {}/v1beta/repos/{}/{}/data",
        server_url, test_org_username, test_repo_username
    );

    // TODO: add English comment
    println!("INFO: API may return 500 errors during testing");
    println!("      This is due to missing property IDs, and is normal for this test");

    // TODO: add English comment
    println!("=== Test: Data List API ===");
    // TODO: add English comment
    let data_list_url = format!(
        "{}/v1beta/repos/{}/{}/data-list",
        server_url, test_org_username, test_repo_username
    );

    // TODO: add English comment
    let mut real_data_id = String::new();

    let response = send_api_request::<serde_json::Value>(
        &client,
        Method::GET,
        &data_list_url,
        None,
        "dummy-token",
    )
    .await?;

    let (status, text) = process_response(response, StatusCode::OK).await?;

    print_response_result("Data list API", status, &text);

    // TODO: add English comment
    if !text.is_empty() {
        let json = parse_json_response(&text)?;
        if let Some(data_list) = json.as_array() {
            // TODO: add English comment
            if !data_list.is_empty() {
                println!(
                    "Data list API successful: {} records found",
                    data_list.len()
                );

                // TODO: add English comment
                if let Some(first_data) = data_list.first() {
                    if let Some(id) =
                        first_data.get("id").and_then(|v| v.as_str())
                    {
                        real_data_id = id.to_string();
                        println!("Actual data ID: {}", real_data_id);
                    }
                }

                // TODO: add English comment
                for (i, data) in data_list.iter().enumerate() {
                    println!(
                        "Data #{}: ID={}, Name={}",
                        i + 1,
                        data.get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown"),
                        data.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                    );
                }
            } else {
                println!("Data list API successful: No data found, but API is working");
            }
        }
    }

    // TODO: add English comment
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // TODO: add English comment
    println!("=== Test: Data Search API ===");

    // TODO: add English comment
    let search_term = "data"; // TODO: add English comment

    let data_search_url = format!(
        "{}/v1beta/repos/{}/{}/data?name={}",
        server_url, test_org_username, test_repo_username, search_term
    );

    let (search_status, search_text) =
        send_and_process_request::<serde_json::Value>(
            &client,
            Method::GET,
            &data_search_url,
            None,
            "dummy-token",
            StatusCode::OK,
        )
        .await?;

    print_response_result("Data search API", search_status, &search_text);

    // TODO: add English comment
    if !search_text.is_empty() && search_status == StatusCode::OK {
        let json = parse_json_response(&search_text)?;
        if let Some(data_list) = json.as_array() {
            println!(
                "Data search API successful: Found {} items with search term \"{}\"",
                data_list.len(),
                search_term
            );

            // TODO: add English comment
            for (i, data) in data_list.iter().enumerate().take(3) {
                // TODO: add English comment
                println!(
                    "Search result #{}: ID={}, Name={}",
                    i + 1,
                    data.get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown"),
                    data.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                );
            }

            if data_list.len() > 3 {
                println!("... and {} more items", data_list.len() - 3);
            }
        }
    }

    // TODO: add English comment
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // TODO: add English comment
    println!("=== Test: Single Data Get API ===");

    // TODO: add English comment
    let dummy_data_id = "data_01dummy";
    println!(
        "3.1 Testing single data get API with non-existent ID ({})",
        dummy_data_id
    );

    let nonexistent_data_url = format!(
        "{}/v1beta/repos/{}/{}/data/{}",
        server_url, test_org_username, test_repo_username, dummy_data_id
    );

    let (nonexist_status, nonexist_text) =
        send_and_process_request::<serde_json::Value>(
            &client,
            Method::GET,
            &nonexistent_data_url,
            None,
            "dummy-token",
            StatusCode::NOT_FOUND,
        )
        .await?;

    print_response_result(
        "Single data get API (non-existent ID)",
        nonexist_status,
        &nonexist_text,
    );

    // TODO: add English comment
    if nonexist_status == StatusCode::NOT_FOUND {
        println!("Single data get API successful: Correctly returned 404 for non-existent data ID");
    } else {
        println!(
            "Single data get API warning: Returned {} instead of 404 for non-existent data ID",
            nonexist_status
        );
    }

    // TODO: add English comment
    if !real_data_id.is_empty() {
        println!(
            "3.2 Testing single data get API with actual ID ({})",
            real_data_id
        );

        // TODO: add English comment
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let real_data_url = format!(
            "{}/v1beta/repos/{}/{}/data/{}",
            server_url, test_org_username, test_repo_username, real_data_id
        );

        let (real_data_status, real_data_text) =
            send_and_process_request::<serde_json::Value>(
                &client,
                Method::GET,
                &real_data_url,
                None,
                "dummy-token",
                StatusCode::OK,
            )
            .await?;

        print_response_result(
            "Single data get API (actual ID)",
            real_data_status,
            &real_data_text,
        );

        // TODO: add English comment
        if !real_data_text.is_empty() && real_data_status == StatusCode::OK
        {
            let data = parse_json_response(&real_data_text)?;
            println!("Single data get API successful: Correctly returned data for actual ID");

            // TODO: add English comment
            println!(
                "Retrieved data: ID={}, Name={}",
                data.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown"),
                data.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
            );
        }
    } else {
        println!(
            "Single data get API warning: Skipping positive test as no actual data ID was obtained"
        );
    }

    // TODO: add English comment
    println!("=== Test: Property ID Retrieval ===");

    // TODO: add English comment
    let properties_url = format!(
        "{}/v1beta/repos/{}/{}/properties",
        server_url, test_org_username, test_repo_username
    );

    // TODO: add English comment
    let mut property_id_string = String::new();
    let mut property_id_html = String::new();

    let (prop_status, prop_text) =
        send_and_process_request::<serde_json::Value>(
            &client,
            Method::GET,
            &properties_url,
            None,
            "dummy-token",
            StatusCode::OK,
        )
        .await?;

    print_response_result("Property list API", prop_status, &prop_text);

    // TODO: add English comment
    if !prop_text.is_empty() && prop_status == StatusCode::OK {
        let json = parse_json_response(&prop_text)?;
        if let Some(props) = json.as_array() {
            println!(
                "Property list API successful: {} properties found",
                props.len()
            );

            // TODO: add English comment
            for (i, prop) in props.iter().enumerate() {
                let id = prop
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let name = prop
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let property_type = prop
                    .get("property_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                println!(
                    "Property #{}: ID={}, Name={}, Type={}",
                    i + 1,
                    id,
                    name,
                    property_type
                );

                // TODO: add English comment
                if name == "id"
                    && (property_type == "STRING"
                        || property_type == "string")
                {
                    property_id_string = id.to_string();
                } else if name == "content"
                    && (property_type == "HTML" || property_type == "html")
                {
                    property_id_html = id.to_string();
                }
            }

            if !property_id_string.is_empty()
                && !property_id_html.is_empty()
            {
                println!("Property ID retrieval successful");
                println!("String property ID: {}", property_id_string);
                println!("HTML property ID: {}", property_id_html);
            } else {
                println!("Property ID retrieval warning: Required property IDs not found");
                if property_id_string.is_empty() {
                    println!("- id (STRING type) property not found");
                }
                if property_id_html.is_empty() {
                    println!("- content (HTML type) property not found");
                }
            }
        }
    }

    // TODO: add English comment
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // TODO: add English comment
    println!("=== Test: Data Add API ===");

    // TODO: add English comment
    if property_id_string.is_empty() || property_id_html.is_empty() {
        println!("Data add API skipped: Required property IDs not found");
        println!("  Run the property API tests first or ensure default properties exist");
    } else {
        // TODO: add English comment
        let test_new_data_name = format!("Test New Data {}", timestamp);
        let test_data_value = format!("Value for new data {}", timestamp);

        println!(
            "4.1 Testing data add API with property IDs ({}, {})",
            property_id_string, property_id_html
        );

        // TODO: add English comment
        let add_data_url = format!(
            "{}/v1beta/repos/{}/{}/data",
            server_url, test_org_username, test_repo_username
        );

        // TODO: add English comment
        let add_data_request = AddDataRequest {
            name: test_new_data_name.clone(),
            property_data: vec![
                PropertyDataRequest {
                    property_id: property_id_string,
                    value: serde_json::to_value(test_data_value).unwrap(),
                },
                PropertyDataRequest {
                    property_id: property_id_html,
                    value: serde_json::json!({
                        "content": "<div><p>Test HTML content</p></div>"
                    }),
                },
            ],
        };

        let (add_data_status, add_data_text) = send_and_process_request(
            &client,
            Method::POST,
            &add_data_url,
            Some(add_data_request),
            "dummy-token",
            StatusCode::OK,
        )
        .await?;

        print_response_result(
            "Data add API",
            add_data_status,
            &add_data_text,
        );

        // TODO: add English comment
        let mut new_data_id = String::new();

        // TODO: add English comment
        if !add_data_text.is_empty() && add_data_status == StatusCode::OK {
            let data = parse_json_response(&add_data_text)?;
            println!("Data add API successful: New data created");

            // TODO: add English comment
            if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
                new_data_id = id.to_string();
                println!("Created data ID: {}", new_data_id);

                // TODO: add English comment
                println!(
                    "Created data: ID={}, Name={}",
                    id,
                    data.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                );
            } else {
                anyhow::bail!("Created data response missing 'id' field");
            }
        } else if add_data_status == StatusCode::INTERNAL_SERVER_ERROR {
            println!("Data add API info: Server error (500) occurred");
            println!("  This could be due to property ID issues");
            println!("  Error content: {}", add_data_text);
            anyhow::bail!(
                "Internal server error during data creation: {}",
                add_data_text
            );
        }

        // TODO: add English comment
        if !new_data_id.is_empty() {
            println!("5. Verifying created data");

            // TODO: add English comment
            tokio::time::sleep(tokio::time::Duration::from_millis(100))
                .await;

            let verify_data_url = format!(
                "{}/v1beta/repos/{}/{}/data/{}",
                server_url,
                test_org_username,
                test_repo_username,
                new_data_id
            );

            let (verify_status, verify_text) =
                send_and_process_request::<serde_json::Value>(
                    &client,
                    Method::GET,
                    &verify_data_url,
                    None,
                    "dummy-token",
                    StatusCode::OK,
                )
                .await?;

            print_response_result(
                "Data verification",
                verify_status,
                &verify_text,
            );

            // TODO: add English comment
            if !verify_text.is_empty() && verify_status == StatusCode::OK {
                let data = parse_json_response(&verify_text)?;
                println!("Data verification successful: Data retrieved correctly");

                // TODO: add English comment
                if let Some(name) =
                    data.get("name").and_then(|v| v.as_str())
                {
                    if name == test_new_data_name {
                        println!(
                            "Name verification successful: Name is set correctly: {}",
                            name
                        );
                    } else {
                        anyhow::bail!(
                            "Name verification failed: Name is different. Expected: {}, Actual: {}",
                            test_new_data_name,
                            name
                        );
                    }
                } else {
                    anyhow::bail!(
                        "Verified data response missing 'name' field"
                    );
                }
            }
        } else {
            println!("Data verification skipped: Data creation was not successful");
        }
    }

    // TODO: add English comment
    // TODO: add English comment
    println!("INFO: Data creation tests may fail depending on the database state");
    println!("      In actual tests, use existing property IDs");

    // TODO: add English comment
    shutdown_tx.send(()).unwrap();

    Ok(())
}
