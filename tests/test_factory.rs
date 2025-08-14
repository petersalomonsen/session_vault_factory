use near_workspaces::types::NearToken;
use serde_json::json;
use sha2::{Sha256, Digest};

const SESSION_VAULT_WASM_URL: &str = 
    "https://github.com/brainstems/intellex_vesting_contracts/raw/main/res/session_vault.wasm";

async fn download_session_vault_wasm() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("Downloading session_vault.wasm from GitHub...");
    let response = reqwest::get(SESSION_VAULT_WASM_URL).await?;
    let bytes = response.bytes().await?;
    println!("Downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

fn calculate_code_hash(wasm_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(wasm_bytes);
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[tokio::test]
async fn test_factory_deploys_global_contract() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    
    // Download the session_vault WASM
    let session_vault_wasm = download_session_vault_wasm().await?;
    
    // Calculate the code hash for the session_vault contract
    let code_hash = calculate_code_hash(&session_vault_wasm);
    println!("Session vault code hash: {:?}", code_hash);
    
    // Deploy session_vault as a global contract
    // First, we need to deploy it to a temporary account and then make it global
    let global_contract = sandbox.dev_deploy(&session_vault_wasm).await?;
    println!("Deployed session_vault to: {}", global_contract.id());
    
    // Deploy our factory contract
    let factory_wasm = near_workspaces::compile_project("./").await?;
    let factory = sandbox.dev_deploy(&factory_wasm).await?;
    
    // Initialize factory with the code hash
    let init_result = factory
        .call("new")
        .args_json(json!({
            "owner_id": factory.id(),
            "session_vault_code_hash": code_hash.to_string()
        }))
        .transact()
        .await?;
    
    assert!(init_result.is_success(), "Factory initialization failed: {:#?}", init_result);
    
    // Test deploying a vault through the factory
    let user_account = sandbox.dev_create_account().await?;
    
    let deploy_result = user_account
        .call(factory.id(), "create_instance")
        .args_json(json!({
            "name": "test-vault"
        }))
        .deposit(NearToken::from_near(5))
        .transact()
        .await?;
    
    assert!(deploy_result.is_success(), "Vault deployment failed: {:#?}", deploy_result);
    
    // Verify the vault was registered
    let instances = factory
        .view("get_instances")
        .args_json(json!({
            "from_index": 0,
            "limit": 10
        }))
        .await?;
    
    let instances_list: Vec<(String, String)> = instances.json()?;
    assert_eq!(instances_list.len(), 1, "Expected 1 instance to be registered");
    
    Ok(())
}

#[tokio::test]
async fn test_factory_pagination() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    
    // Download and deploy session_vault as global contract
    let session_vault_wasm = download_session_vault_wasm().await?;
    let code_hash = calculate_code_hash(&session_vault_wasm);
    let _global_contract = sandbox.dev_deploy(&session_vault_wasm).await?;
    
    // Deploy and initialize factory
    let factory_wasm = near_workspaces::compile_project("./").await?;
    let factory = sandbox.dev_deploy(&factory_wasm).await?;
    
    factory
        .call("new")
        .args_json(json!({
            "owner_id": factory.id(),
            "session_vault_code_hash": code_hash.to_string()
        }))
        .transact()
        .await?;
    
    // Deploy multiple vaults
    let user_account = sandbox.dev_create_account().await?;
    
    for i in 0..5 {
        let deploy_result = user_account
            .call(factory.id(), "create_instance")
            .args_json(json!({
                "name": format!("vault{}", i)
            }))
            .deposit(NearToken::from_near(5))
            .transact()
            .await?;
        
        assert!(deploy_result.is_success(), "Failed to deploy vault {}", i);
    }
    
    // Test pagination
    let first_page = factory
        .view("get_instances")
        .args_json(json!({
            "from_index": 0,
            "limit": 2
        }))
        .await?;
    
    let first_page_instances: Vec<(String, String)> = first_page.json()?;
    assert_eq!(first_page_instances.len(), 2, "Expected 2 instances in first page");
    
    let second_page = factory
        .view("get_instances")
        .args_json(json!({
            "from_index": 2,
            "limit": 2
        }))
        .await?;
    
    let second_page_instances: Vec<(String, String)> = second_page.json()?;
    assert_eq!(second_page_instances.len(), 2, "Expected 2 instances in second page");
    
    Ok(())
}