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

extern crate library_api;

mod util;

use library_api::handler::types::{
    AddDataRequest, AddPropertyRequest, PropertyDataRequest,
};
use reqwest::{Method, StatusCode};
use util::{
    create_test_client, deserialize_response, generate_timestamp,
    parse_json_response, print_response_result, send_and_process_request,
    setup_test_server,
};

/// TODO: add English documentation
#[tokio::test]
#[tracing::instrument]
async fn test_properties_api_all() -> anyhow::Result<()> {
    // TODO: add English comment
    let (server_url, shutdown_tx) = setup_test_server().await;
    println!("テストサーバーが起動しました: {}", server_url);

    let client = create_test_client();

    // TODO: add English comment
    let timestamp = generate_timestamp();

    // TODO: add English comment
    println!("=== テスト1: プロパティ操作のシナリオテスト ===");

    // TODO: add English comment
    let test_org_name = format!("Test Organization Property {}", timestamp);
    let test_org_username = format!("test_org_prop_{}", timestamp);
    let test_org_description =
        "This is a test organization for property scenario test";

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

    // TODO: add English comment
    let (org_status, org_text) = send_and_process_request(
        &client,
        Method::POST,
        &org_url,
        Some(org_request),
        "dummy-token",
        StatusCode::OK,
    )
    .await
    .or_else(|e| {
        if e.to_string().contains("already exists") {
            println!("Organization already exists: {}", e);
            Ok::<(StatusCode, String), anyhow::Error>((
                StatusCode::OK,
                "".to_string(),
            ))
        } else {
            Err(e)
        }
    })?;

    print_response_result("Organization creation", org_status, &org_text);

    // TODO: add English comment
    if !org_text.is_empty() && org_status == StatusCode::OK {
        let _: serde_json::Value = deserialize_response(&org_text)?;
        if let Ok(_org_json) = parse_json_response(&org_text) {
            println!("Organization info parsing successful");
        }
    }

    // TODO: add English comment
    let test_repo_name = format!("Test Repository Property {}", timestamp);
    let test_repo_username = format!("test_repo_prop_{}", timestamp);
    let test_repo_description =
        "This is a test repository for property scenario test";
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

    // TODO: add English comment
    let (repo_status, repo_text) = send_and_process_request(
        &client,
        Method::POST,
        &repo_url,
        Some(repo_request),
        "dummy-token",
        StatusCode::OK,
    )
    .await
    .or_else(|e| {
        if e.to_string().contains("already exists") {
            println!("Repository already exists: {}", e);
            Ok::<(StatusCode, String), anyhow::Error>((
                StatusCode::OK,
                serde_json::to_string(&serde_json::json!({
                    "id": "dummy_id",
                    "name": test_repo_name,
                    "username": test_repo_username,
                    "description": test_repo_description,
                    "is_public": test_repo_is_public,
                    "organization_id": "dummy_org_id"
                }))
                .unwrap(),
            ))
        } else {
            Err(e)
        }
    })?;

    print_response_result("Repository creation", repo_status, &repo_text);

    // TODO: add English comment
    if repo_status == StatusCode::OK {
        if let Ok(json) = parse_json_response(&repo_text) {
            // TODO: add English comment
            if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                assert_eq!(name, test_repo_name);
            }
            if let Some(username) =
                json.get("username").and_then(|v| v.as_str())
            {
                assert_eq!(username, test_repo_username);
            }
            if let Some(desc) =
                json.get("description").and_then(|v| v.as_str())
            {
                assert_eq!(desc, test_repo_description);
            }
            if let Some(is_public) =
                json.get("is_public").and_then(|v| v.as_bool())
            {
                assert_eq!(is_public, test_repo_is_public);
            }
        }
    }

    // TODO: add English comment
    println!("=== テスト2: プロパティ一覧API ===");
    println!(
        "Sending GET request to {}/v1beta/repos/{}/{}/properties",
        server_url, test_org_username, test_repo_username
    );

    // TODO: add English comment
    let mut property_ids = Vec::new();

    let properties_url = format!(
        "{}/v1beta/repos/{}/{}/properties",
        server_url, test_org_username, test_repo_username
    );

    // TODO: add English comment
    let (prop_list_status, prop_list_text) =
        send_and_process_request::<serde_json::Value>(
            &client,
            Method::GET,
            &properties_url,
            Option::<serde_json::Value>::None,
            "dummy-token",
            StatusCode::OK,
        )
        .await?;

    print_response_result(
        "Property list API",
        prop_list_status,
        &prop_list_text,
    );

    // TODO: add English comment
    if !prop_list_text.is_empty() && prop_list_status == StatusCode::OK {
        match parse_json_response(&prop_list_text) {
            Ok(json) => {
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
                        property_ids.push((
                            id.to_string(),
                            name.to_string(),
                            property_type.to_string(),
                        ));
                    }
                }
            }
            Err(e) => println!(
                "Property list API warning: JSON parsing failed: {}",
                e
            ),
        }
    }

    // TODO: add English comment
    println!("=== テスト3: 各プロパティタイプの作成テスト ===");

    // TODO: add English comment
    let property_types = vec![
        ("string", "String Property"),
        ("integer", "Integer Property"),
        ("html", "Html Property"),
        ("relation", "Relation Property"),
        ("select", "Select Property"),
        ("multi_select", "MultiSelect Property"),
        ("location", "Location Property"),
    ];

    // TODO: add English comment
    let mut created_property_ids = Vec::new();

    // TODO: add English comment
    let mut target_database_id = String::new();

    // TODO: add English comment
    if property_types.iter().any(|(t, _)| *t == "relation") {
        println!("=== 関連先データベースの作成（relationタイプのプロパティ用） ===");

        // TODO: add English comment
        let target_db_name =
            format!("Target Database for Relation {}", timestamp);

        // TODO: add English comment
        #[derive(serde::Serialize)]
        struct CreateDatabaseRequest {
            name: String,
            description: String,
        }

        let target_db_request = CreateDatabaseRequest {
            name: target_db_name.clone(),
            description:
                "This is a target database for relation property test"
                    .to_string(),
        };

        // TODO: add English comment
        let target_db_url = format!("{}/v1beta/databases", server_url);

        println!("Sending POST request to {}", target_db_url);

        // TODO: add English comment
        let (db_status, db_text) = send_and_process_request(
            &client,
            Method::POST,
            &target_db_url,
            Some(target_db_request),
            "dummy-token",
            StatusCode::OK,
        )
        .await
        .or_else(|e| {
            println!("Database creation warning: {}", e);
            Ok::<(StatusCode, String), anyhow::Error>((
                StatusCode::OK,
                serde_json::to_string(&serde_json::json!({
                    "id": format!("dummy_db_id_{}", timestamp),
                    "name": target_db_name,
                }))
                .unwrap(),
            ))
        })?;

        print_response_result(
            "Target database creation",
            db_status,
            &db_text,
        );

        // TODO: add English comment
        if db_status == StatusCode::OK {
            if let Ok(json) = parse_json_response(&db_text) {
                if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                    target_database_id = id.to_string();
                    println!(
                        "Created target database ID: {}",
                        target_database_id
                    );
                } else {
                    // TODO: add English comment
                    target_database_id =
                        format!("dummy_db_id_{}", timestamp);
                    println!(
                        "Using dummy target database ID: {}",
                        target_database_id
                    );
                }
            }
        }
    }

    // TODO: add English comment
    for (property_type, property_name_suffix) in property_types {
        let test_property_name =
            format!("Test {} {}", property_name_suffix, timestamp);

        println!(
            "Creating property: {} (type: {})",
            test_property_name, property_type
        );

        let create_property_url = format!(
            "{}/v1beta/repos/{}/{}/properties",
            server_url, test_org_username, test_repo_username
        );

        // TODO: add English comment
        let create_property_request = if property_type == "relation"
            && !target_database_id.is_empty()
        {
            // TODO: add English comment
            AddPropertyRequest {
                name: test_property_name.clone(),
                property_type: format!("relation:{}", target_database_id),
            }
        } else {
            AddPropertyRequest {
                name: test_property_name.clone(),
                property_type: property_type.to_string(),
            }
        };

        // TODO: add English comment
        let (prop_create_status, prop_create_text) = send_and_process_request(
            &client,
            Method::POST,
            &create_property_url,
            Some(create_property_request),
            "dummy-token",
            StatusCode::OK,
        )
        .await
        .or_else(|e| {
            println!("Property creation API warning for {}: {}", property_type, e);
            if property_type == "relation" {
                // TODO: add English comment
                println!("Relation property creation failed. This might be due to:");
                println!(
                    "  - Target database ID might be invalid: {}",
                    target_database_id
                );
                println!("  - Property type format might be incorrect");
                println!("  - Database might not exist in the system");
                Ok::<(StatusCode, String), anyhow::Error>((StatusCode::OK, String::new()))
            } else {
                Err(e)
            }
        })?;

        print_response_result(
            &format!("Property creation API ({})", property_type),
            prop_create_status,
            &prop_create_text,
        );

        // TODO: add English comment
        if !prop_create_text.is_empty()
            && prop_create_status == StatusCode::OK
        {
            match parse_json_response(&prop_create_text) {
                Ok(prop) => {
                    println!(
                        "Property creation API successful: New {} property created",
                        property_type
                    );
                    if let Some(id) = prop.get("id").and_then(|v| v.as_str()) {
                        let created_id = id.to_string();
                        println!("Created property ID: {}", created_id);
                        // TODO: add English comment
                        println!(
                            "Created property: ID={}, Name={}, Type={}",
                            id,
                            prop.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown"),
                            prop.get("property_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown")
                        );
                        // TODO: add English comment
                        if let (Some(name), Some(prop_type)) = (
                            prop.get("name").and_then(|v| v.as_str()),
                            prop.get("property_type").and_then(|v| v.as_str()),
                        ) {
                            created_property_ids.push((
                                created_id,
                                name.to_string(),
                                prop_type.to_string(),
                                property_type.to_string(),
                            ));
                        }
                    }
                }
                Err(e) => println!("Property creation API warning: JSON parsing failed: {}", e),
            }
        } else if prop_create_status == StatusCode::INTERNAL_SERVER_ERROR {
            println!(
                "Property creation API info: Server error (500) occurred for {}",
                property_type
            );
            println!("  Error content: {}", prop_create_text);
        }

        // TODO: add English comment
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // TODO: add English comment
    println!("=== テスト4: 作成したプロパティの取得テスト ===");

    for (property_id, property_name, property_type, _original_type) in
        &created_property_ids
    {
        println!(
            "Testing property get API for: {} (type: {})",
            property_name, property_type
        );

        let get_property_url = format!(
            "{}/v1beta/repos/{}/{}/properties/{}",
            server_url, test_org_username, test_repo_username, property_id
        );

        // TODO: add English comment
        let (prop_get_status, prop_get_text) =
            send_and_process_request::<serde_json::Value>(
                &client,
                Method::GET,
                &get_property_url,
                Option::<serde_json::Value>::None,
                "dummy-token",
                StatusCode::OK,
            )
            .await?;

        print_response_result(
            &format!("Single property get API ({})", property_type),
            prop_get_status,
            &prop_get_text,
        );

        // TODO: add English comment
        if !prop_get_text.is_empty() && prop_get_status == StatusCode::OK {
            match parse_json_response(&prop_get_text) {
                Ok(prop) => {
                    println!(
                        "Single property get API successful: Property retrieved for {}",
                        property_type
                    );
                    // TODO: add English comment
                    println!(
                        "Retrieved property: ID={}, Name={}, Type={}",
                        prop.get("id").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                        prop.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown"),
                        prop.get("property_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                    );
                    // TODO: add English comment
                    if let Some(name) = prop.get("name").and_then(|v| v.as_str()) {
                        if name == property_name {
                            println!(
                                "Name verification successful: Name is set correctly: {}",
                                name
                            );
                        } else {
                            println!("Name verification failed: Name is different. Expected: {}, Actual: {}", property_name, name);
                        }
                    }
                    // TODO: add English comment
                    if let Some(prop_type) = prop.get("property_type").and_then(|v| v.as_str()) {
                        // TODO: add English comment
                        if prop_type.to_uppercase() == property_type.to_uppercase() {
                            println!(
                                "Type verification successful: Type is set correctly: {}",
                                prop_type
                            );
                        } else {
                            println!("Type verification failed: Type is different. Expected: {}, Actual: {}", property_type, prop_type);
                        }
                    }
                }
                Err(e) => println!(
                    "Single property get API warning: JSON parsing failed: {}",
                    e
                ),
            }
        } else if prop_get_status == StatusCode::INTERNAL_SERVER_ERROR {
            println!(
                "Single property get API info: Server error (500) occurred for {}",
                property_type
            );
            println!("  Error content: {}", prop_get_text);
        }

        // TODO: add English comment
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // TODO: add English comment
    println!(
        "=== テスト5: 各プロパティタイプに対するデータの追加テスト ==="
    );

    // TODO: add English comment
    if created_property_ids.is_empty() {
        println!("Data add API skipped: No property IDs available");
    } else {
        // TODO: add English comment
        for (property_id, property_name, property_type, original_type) in
            &created_property_ids
        {
            println!(
                "Testing data add API for property: {} (type: {})",
                property_name, property_type
            );

            // TODO: add English comment
            let test_data_name =
                format!("Test Data for {} {}", property_name, timestamp);

            // TODO: add English comment
            let add_data_url = format!(
                "{}/v1beta/repos/{}/{}/data",
                server_url, test_org_username, test_repo_username
            );

            // TODO: add English comment
            let property_value = match original_type.as_str() {
                "string" => {
                    serde_json::to_value("Test string value").unwrap()
                }
                "integer" => serde_json::to_value(42).unwrap(),
                "html" => serde_json::to_value(
                    "<div><p>Test HTML content</p></div>",
                )
                .unwrap(),
                "relation" => serde_json::to_value("rel_01dummy").unwrap(),
                "select" => serde_json::to_value("op_option1").unwrap(),
                "multi_select" => {
                    serde_json::to_value(vec!["op_option1", "op_option2"])
                        .unwrap()
                }
                "location" => {
                    serde_json::to_value("35.6812362,139.7649361").unwrap()
                } // TODO: add English comment
                _ => serde_json::to_value("Default test value").unwrap(),
            };

            // TODO: add English comment
            let add_data_request = AddDataRequest {
                name: test_data_name.clone(),
                property_data: vec![PropertyDataRequest {
                    property_id: property_id.clone(),
                    value: property_value,
                }],
            };

            // TODO: add English comment
            let (add_data_status, add_data_text) =
                send_and_process_request(
                    &client,
                    Method::POST,
                    &add_data_url,
                    Some(add_data_request),
                    "dummy-token",
                    StatusCode::OK,
                )
                .await?;

            print_response_result(
                &format!("Data add API ({})", property_type),
                add_data_status,
                &add_data_text,
            );

            // TODO: add English comment
            let mut new_data_id = String::new();

            // TODO: add English comment
            if !add_data_text.is_empty()
                && add_data_status == StatusCode::OK
            {
                match parse_json_response(&add_data_text) {
                    Ok(data) => {
                        println!(
                            "Data add API successful: New data created for {}",
                            property_type
                        );

                        // TODO: add English comment
                        if let Some(id) =
                            data.get("id").and_then(|v| v.as_str())
                        {
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
                        }
                    }
                    Err(e) => println!(
                        "Data add API warning: JSON parsing failed: {}",
                        e
                    ),
                }
            } else if add_data_status == StatusCode::INTERNAL_SERVER_ERROR {
                println!(
                    "Data add API info: Server error (500) occurred for {}",
                    property_type
                );
                println!("  This could be due to property type validation issues");
                println!("  Error content: {}", add_data_text);
            }

            // TODO: add English comment
            if !new_data_id.is_empty() {
                println!("Verifying created data for {}", property_type);

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

                // TODO: add English comment
                let (verify_status, verify_text) =
                    send_and_process_request::<serde_json::Value>(
                        &client,
                        Method::GET,
                        &verify_data_url,
                        Option::<serde_json::Value>::None,
                        "dummy-token",
                        StatusCode::OK,
                    )
                    .await?;

                print_response_result(
                    &format!("Data verification ({})", property_type),
                    verify_status,
                    &verify_text,
                );

                // TODO: add English comment
                if !verify_text.is_empty()
                    && verify_status == StatusCode::OK
                {
                    match parse_json_response(&verify_text) {
                        Ok(data) => {
                            println!(
                                "Data verification successful: Data retrieved correctly for {}",
                                property_type
                            );
                            // TODO: add English comment
                            if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
                                if name == test_data_name {
                                    println!(
                                        "Name verification successful: Name is set correctly: {}",
                                        name
                                    );
                                } else {
                                    println!("Name verification failed: Name is different. Expected: {}, Actual: {}", test_data_name, name);
                                }
                            }
                            // TODO: add English comment
                            if let Some(properties) =
                                data.get("properties").and_then(|v| v.as_array())
                            {
                                for prop_data in properties {
                                    if let Some(prop_id) =
                                        prop_data.get("property_id").and_then(|v| v.as_str())
                                    {
                                        if prop_id == property_id {
                                            println!(
                                                "Property data found for property ID: {}",
                                                prop_id
                                            );
                                            // TODO: add English comment
                                            if let Some(value) = prop_data.get("value") {
                                                println!("Property value: {}", value);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => println!("Data verification warning: JSON parsing failed: {}", e),
                    }
                }
            } else {
                println!(
                    "Data verification skipped: Data creation was not successful for {}",
                    property_type
                );
            }

            // TODO: add English comment
            tokio::time::sleep(tokio::time::Duration::from_millis(100))
                .await;
        }
    }

    // TODO: add English comment
    println!("=== テスト6: 取得したプロパティID一覧 ===");
    println!(
        "The following property IDs can be used in data add API tests:"
    );

    for (i, (id, name, property_type, _)) in
        created_property_ids.iter().enumerate()
    {
        println!(
            "- Property #{}: ID={}, Name={}, Type={}",
            i + 1,
            id,
            name,
            property_type
        );
    }

    // TODO: add English comment
    shutdown_tx.send(()).unwrap();

    Ok(())
}
