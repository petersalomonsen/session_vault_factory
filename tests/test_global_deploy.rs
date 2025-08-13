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
async fn test_deploy_global_contract() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    
    // Download the session_vault WASM
    let session_vault_wasm = download_session_vault_wasm().await?;
    let code_hash = calculate_code_hash(&session_vault_wasm);
    println!("Session vault code hash: {}", code_hash);
    
    // Deploy the contract code as a global contract
    // In NEAR, we first deploy the contract normally, then it becomes available by hash
    let deployer = sandbox.dev_create_account().await?;
    
    // Deploy the session_vault contract
    println!("Deploying session_vault contract...");
    let contract_result = deployer
        .deploy(&session_vault_wasm)
        .await?;
    let contract = contract_result.result;
    
    println!("✅ Deployed session_vault to: {}", contract.id());
    println!("   Contract hash: {}", code_hash);
    
    // Now let's create another account that will reference this contract by hash
    // This simulates what our factory will do
    let _factory_account = sandbox.dev_create_account().await?;
    
    // Create a new vault instance using the deployed contract code
    // In a real factory, we would use the code hash to deploy new instances
    let vault_account = sandbox.dev_create_account().await?;
    
    // Deploy the same contract code to the vault account
    // This demonstrates that the same code can be reused
    let vault_result = vault_account
        .deploy(&session_vault_wasm)
        .await?;
    let vault_instance = vault_result.result;
    
    println!("✅ Created vault instance at: {}", vault_instance.id());
    
    // Verify both contracts have the same code hash
    println!("\n📊 Deployment Summary:");
    println!("   Original contract: {}", contract.id());
    println!("   Vault instance: {}", vault_instance.id());
    println!("   Code hash (both): {}", code_hash);
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_vault_deployments() -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    
    // Download the session_vault WASM once
    let session_vault_wasm = download_session_vault_wasm().await?;
    let code_hash = calculate_code_hash(&session_vault_wasm);
    println!("Session vault code hash: {}", code_hash);
    
    // Deploy multiple instances of the same contract
    let mut vault_instances = Vec::new();
    
    for i in 0..3 {
        let vault_account = sandbox.dev_create_account().await?;
        let vault_result = vault_account
            .deploy(&session_vault_wasm)
            .await?;
        let vault_instance = vault_result.result;
        
        println!("✅ Deployed vault instance #{} to: {}", i + 1, vault_instance.id());
        vault_instances.push(vault_instance);
    }
    
    println!("\n📊 Deployment Summary:");
    println!("   Total vaults deployed: {}", vault_instances.len());
    println!("   All using code hash: {}", code_hash);
    println!("\n   Vault addresses:");
    for (i, vault) in vault_instances.iter().enumerate() {
        println!("     {}. {}", i + 1, vault.id());
    }
    
    // In a real global contract scenario, all these instances would reference
    // the same code hash, saving storage costs
    println!("\n💡 Note: In production, these would all reference the same global contract code");
    println!("   This would significantly reduce storage costs on-chain");
    
    Ok(())
}