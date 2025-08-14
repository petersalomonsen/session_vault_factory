use serde_json::json;

#[tokio::test]
async fn test_contract_initialization() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    // Initialize the contract
    let init_result = contract
        .call("new")
        .args_json(json!({
            "owner_id": contract.id(),
            "session_vault_code_hash": "f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18"
        }))
        .transact()
        .await?;

    assert!(init_result.is_success(), "Contract initialization failed");

    // Check initial state
    let owner = contract.view("get_owner").await?;
    assert_eq!(owner.json::<String>()?, contract.id().to_string());

    let total_instances = contract.view("get_total_instances").await?;
    assert_eq!(total_instances.json::<u64>()?, 0);

    let code_hash = contract.view("get_code_hash").await?;
    assert_eq!(
        code_hash.json::<String>()?,
        "f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18"
    );

    Ok(())
}

#[tokio::test]
async fn test_create_instance() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    // Initialize the contract
    contract
        .call("new")
        .args_json(json!({
            "owner_id": contract.id(),
            "session_vault_code_hash": "test_hash"
        }))
        .transact()
        .await?;

    // Create an instance
    let user_account = sandbox.dev_create_account().await?;
    let instance_name = "vault1";
    
    let create_result = user_account
        .call(contract.id(), "create_instance")
        .args_json(json!({
            "name": instance_name
        }))
        .deposit(near_workspaces::types::NearToken::from_near(5))
        .transact()
        .await?;

    assert!(create_result.is_success(), "Failed to create instance: {:?}", create_result);

    // Verify the instance was created
    let instance = contract
        .view("get_instance")
        .args_json(json!({
            "name": instance_name
        }))
        .await?;

    let instance_account_id = instance.json::<Option<String>>()?;
    assert!(instance_account_id.is_some());
    
    let expected_account = format!("{}.{}", instance_name, contract.id());
    assert_eq!(instance_account_id.unwrap(), expected_account);

    // Check total instances
    let total = contract.view("get_total_instances").await?;
    assert_eq!(total.json::<u64>()?, 1);

    Ok(())
}

#[tokio::test]
async fn test_create_multiple_instances() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    // Initialize the contract
    contract
        .call("new")
        .args_json(json!({
            "owner_id": contract.id(),
            "session_vault_code_hash": "test_hash"
        }))
        .transact()
        .await?;

    let user_account = sandbox.dev_create_account().await?;

    // Create multiple instances
    for i in 0..3 {
        let instance_name = format!("vault{}", i);
        
        let create_result = user_account
            .call(contract.id(), "create_instance")
            .args_json(json!({
                "name": instance_name
            }))
            .deposit(near_workspaces::types::NearToken::from_near(5))
            .transact()
            .await?;

        assert!(create_result.is_success(), "Failed to create instance {}", i);
    }

    // Check total instances
    let total = contract.view("get_total_instances").await?;
    assert_eq!(total.json::<u64>()?, 3);

    // Get all instances with pagination
    let instances = contract
        .view("get_instances")
        .args_json(json!({
            "from_index": 0,
            "limit": 10
        }))
        .await?;

    let instances_list: Vec<(String, String)> = instances.json()?;
    assert_eq!(instances_list.len(), 3);

    // Verify instance names
    for i in 0..3 {
        let expected_name = format!("vault{}", i);
        assert!(instances_list.iter().any(|(name, _)| name == &expected_name));
    }

    Ok(())
}

#[tokio::test]
async fn test_invalid_instance_name() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    // Initialize the contract
    contract
        .call("new")
        .args_json(json!({
            "owner_id": contract.id(),
            "session_vault_code_hash": "test_hash"
        }))
        .transact()
        .await?;

    let user_account = sandbox.dev_create_account().await?;

    // Try to create instance with invalid name (contains dot)
    let create_result = user_account
        .call(contract.id(), "create_instance")
        .args_json(json!({
            "name": "invalid.name"
        }))
        .deposit(near_workspaces::types::NearToken::from_near(5))
        .transact()
        .await?;

    assert!(!create_result.is_success(), "Should have failed with invalid name");

    // Try to create instance with empty name
    let create_result = user_account
        .call(contract.id(), "create_instance")
        .args_json(json!({
            "name": ""
        }))
        .deposit(near_workspaces::types::NearToken::from_near(5))
        .transact()
        .await?;

    assert!(!create_result.is_success(), "Should have failed with empty name");

    Ok(())
}


