use near_sdk::json_types::Base64VecU8;
use near_workspaces::types::NearToken;
use sha2::Digest;

const SESSION_VAULT_WASM_URL: &str =
    "https://github.com/brainstems/intellex_vesting_contracts/raw/main/res/session_vault.wasm";

async fn download_session_vault_wasm() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("📥 Downloading session_vault.wasm from GitHub...");
    let response = reqwest::get(SESSION_VAULT_WASM_URL).await?;
    let bytes = response.bytes().await?;
    println!("✅ Downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

/// Test that verifies instance tracking and pagination with 50 instances
#[tokio::test]
async fn test_instance_tracking_with_pagination() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Instance Tracking with 50 Instances ===\n");

    // Initialize sandbox
    let worker = near_workspaces::sandbox_with_version("2.7.0").await?;
    println!("🏗️  Sandbox initialized with version 2.7.0");

    // Compile and deploy the factory contract
    println!("🏭 Deploying factory contract...");
    let factory_wasm = near_workspaces::compile_project("./").await?;
    let factory = worker.dev_deploy(&factory_wasm).await?;

    // Initialize factory
    factory
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": factory.id()
        }))
        .transact()
        .await?
        .into_result()?;

    println!("✅ Factory deployed at: {}", factory.id());

    // Download and deploy global contract
    let session_vault_code = download_session_vault_wasm().await?;
    let code_hash_vec = sha2::Sha256::digest(&session_vault_code);
    let code_hash_hex = hex::encode(code_hash_vec);
    println!("🔐 Contract hash: {}", code_hash_hex);

    let global_account_id: near_workspaces::AccountId =
        format!("global.{}", factory.id()).parse()?;

    println!("📤 Deploying global contract...");
    factory
        .call("deploy_global_contract")
        .args_json((Base64VecU8::from(session_vault_code), &global_account_id))
        .max_gas()
        .deposit(NearToken::from_near(25))
        .transact()
        .await?
        .into_result()?;

    println!("✅ Global contract deployed");

    // Verify initial state
    let initial_count = factory
        .view("get_total_instances")
        .await?
        .json::<u64>()?;
    assert_eq!(initial_count, 0, "Should start with 0 instances");

    // Create 50 instances
    println!("\n🚀 Creating 50 instances...");
    for i in 0..50 {
        let instance_name = format!("instance{:02}", i);
        
        let result = factory
            .call("create_instance")
            .args_json((instance_name.clone(),))
            .deposit(NearToken::from_near(1))
            .max_gas()
            .transact()
            .await?;

        if !result.is_success() {
            panic!("Failed to create instance {}: {:?}", instance_name, result.failures());
        }

        if i % 10 == 9 {
            println!("   ✅ Created {} instances", i + 1);
        }
    }
    println!("✅ Successfully created all 50 instances");

    // Verify total count
    let total_count = factory
        .view("get_total_instances")
        .await?
        .json::<u64>()?;
    assert_eq!(total_count, 50, "Should have exactly 50 instances");
    println!("✅ Total instances count: {}", total_count);

    // Test pagination - Get first page (0-9)
    println!("\n📊 Testing pagination...");
    
    let first_page = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 0,
            "limit": 10
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(first_page.len(), 10, "First page should have 10 instances");
    println!("✅ First page (0-9): {} instances", first_page.len());
    
    // Verify first page contains correct instances
    for i in 0..10 {
        let expected_name = format!("instance{:02}", i);
        assert_eq!(first_page[i].0, expected_name, "Instance name mismatch at index {}", i);
        let expected_account = format!("{}.{}", expected_name, factory.id());
        assert_eq!(
            first_page[i].1.to_string(), 
            expected_account, 
            "Instance account mismatch at index {}", 
            i
        );
    }
    println!("   ✓ All instances in correct order");

    // Test second page (10-19)
    let second_page = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 10,
            "limit": 10
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(second_page.len(), 10, "Second page should have 10 instances");
    println!("✅ Second page (10-19): {} instances", second_page.len());

    // Verify second page contains correct instances
    for i in 0..10 {
        let expected_name = format!("instance{:02}", i + 10);
        assert_eq!(second_page[i].0, expected_name, "Instance name mismatch at index {}", i + 10);
    }

    // Test middle page (25-34)
    let middle_page = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 25,
            "limit": 10
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(middle_page.len(), 10, "Middle page should have 10 instances");
    println!("✅ Middle page (25-34): {} instances", middle_page.len());

    // Test last page (45-49) - should only have 5 instances
    let last_page = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 45,
            "limit": 10
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(last_page.len(), 5, "Last page should have 5 instances");
    println!("✅ Last page (45-49): {} instances", last_page.len());

    // Verify last page contains correct instances
    for i in 0..5 {
        let expected_name = format!("instance{:02}", i + 45);
        assert_eq!(last_page[i].0, expected_name, "Instance name mismatch at index {}", i + 45);
    }

    // Test getting all instances at once
    let all_instances = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 0,
            "limit": 100
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(all_instances.len(), 50, "Should return all 50 instances");
    println!("✅ Get all at once: {} instances", all_instances.len());

    // Test beyond range
    let beyond_range = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 100,
            "limit": 10
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(beyond_range.len(), 0, "Should return empty for out of range index");
    println!("✅ Beyond range returns empty list");

    // Test individual instance retrieval
    println!("\n🔍 Testing individual instance retrieval...");
    
    // Test first, middle, and last instances
    let test_indices = vec![0, 25, 49];
    for idx in test_indices {
        let instance_name = format!("instance{:02}", idx);
        let instance = factory
            .view("get_instance")
            .args_json((instance_name.clone(),))
            .await?
            .json::<Option<near_workspaces::AccountId>>()?;

        assert!(instance.is_some(), "Instance {} should exist", instance_name);
        let expected_account = format!("{}.{}", instance_name, factory.id());
        assert_eq!(
            instance.unwrap().to_string(),
            expected_account,
            "Instance account mismatch for {}",
            instance_name
        );
    }
    println!("✅ Individual instance retrieval works correctly");

    // Test non-existent instance
    let non_existent = factory
        .view("get_instance")
        .args_json(("instance99",))
        .await?
        .json::<Option<near_workspaces::AccountId>>()?;

    assert!(non_existent.is_none(), "Non-existent instance should return None");
    println!("✅ Non-existent instance returns None");

    println!("\n=== Summary ===");
    println!("✅ Successfully created and tracked 50 instances");
    println!("✅ Pagination works correctly with different page sizes");
    println!("✅ Individual instance retrieval works");
    println!("✅ Boundary conditions handled properly");
    println!("\n🎉 Instance tracking and pagination fully functional!");

    Ok(())
}

/// Test edge cases for pagination
#[tokio::test]
async fn test_pagination_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Pagination Edge Cases ===\n");

    // Initialize sandbox and factory
    let worker = near_workspaces::sandbox_with_version("2.7.0").await?;
    let factory_wasm = near_workspaces::compile_project("./").await?;
    let factory = worker.dev_deploy(&factory_wasm).await?;

    factory
        .call("new")
        .args_json(serde_json::json!({
            "owner_id": factory.id()
        }))
        .transact()
        .await?
        .into_result()?;

    // Deploy global contract
    let session_vault_code = download_session_vault_wasm().await?;
    let global_account_id: near_workspaces::AccountId =
        format!("global.{}", factory.id()).parse()?;

    factory
        .call("deploy_global_contract")
        .args_json((Base64VecU8::from(session_vault_code), &global_account_id))
        .max_gas()
        .deposit(NearToken::from_near(25))
        .transact()
        .await?
        .into_result()?;

    // Create just 3 instances for edge case testing
    for i in 0..3 {
        factory
            .call("create_instance")
            .args_json((format!("test{}", i),))
            .deposit(NearToken::from_near(1))
            .max_gas()
            .transact()
            .await?
            .into_result()?;
    }

    // Test with limit = 0 (should return empty)
    let zero_limit = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 0,
            "limit": 0
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(zero_limit.len(), 0, "Limit 0 should return empty list");
    println!("✅ Limit 0 returns empty list");

    // Test with very large limit
    let large_limit = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 0,
            "limit": 1000000
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(large_limit.len(), 3, "Large limit should return all instances");
    println!("✅ Large limit returns all available instances");

    // Test with from_index = total_instances
    let at_end = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": 3,
            "limit": 10
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(at_end.len(), 0, "from_index at end should return empty");
    println!("✅ from_index at end returns empty list");

    // Test default parameters (no args)
    let default_params = factory
        .view("get_instances")
        .args_json(serde_json::json!({
            "from_index": null,
            "limit": null
        }))
        .await?
        .json::<Vec<(String, near_workspaces::AccountId)>>()?;

    assert_eq!(default_params.len(), 3, "Default params should return all instances");
    println!("✅ Default parameters work correctly");

    println!("\n✅ All pagination edge cases handled correctly!");

    Ok(())
}