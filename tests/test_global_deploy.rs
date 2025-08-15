use near_api::*;
use near_crypto::SecretKey;
use near_workspaces;
use std::str::FromStr;

const SESSION_VAULT_WASM_URL: &str = 
    "https://github.com/brainstems/intellex_vesting_contracts/raw/main/res/session_vault.wasm";

async fn download_session_vault_wasm() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("📥 Downloading session_vault.wasm from GitHub...");
    let response = reqwest::get(SESSION_VAULT_WASM_URL).await?;
    let bytes = response.bytes().await?;
    println!("✅ Downloaded {} bytes", bytes.len());
    Ok(bytes.to_vec())
}

fn calculate_code_hash(wasm_bytes: &[u8]) -> near_primitives::hash::CryptoHash {
    near_primitives::hash::CryptoHash::hash_bytes(wasm_bytes)
}

#[tokio::test]
async fn test_global_contract_deployment_with_near_api() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Global Contract Deployment with near-api ===\n");
    
    // Step 1: Initialize near-workspaces sandbox
    let worker = near_workspaces::sandbox().await?;
    println!("🏗️  Sandbox initialized");
    
    // Step 2: Download the contract WASM
    let session_vault_wasm = download_session_vault_wasm().await?;
    let code_hash = calculate_code_hash(&session_vault_wasm);
    println!("🔐 Contract hash: {}", code_hash);
    
    // Step 3: Create an account for deploying global contract
    let deployer_account = worker.dev_create_account().await?;
    let deployer_id = deployer_account.id().to_string();
    println!("👤 Deployer account: {}", deployer_id);
    
    // Get the secret key from the deployer account
    let secret_key_str = deployer_account.secret_key().to_string();
    let secret_key = SecretKey::from_str(&secret_key_str)?;
    
    // Create a signer for near-api (following the example pattern)
    let signer = Signer::new(Signer::from_secret_key(secret_key))?;
    let account_id = AccountId::from_str(&deployer_id)?;
    
    // Step 4: Create network config for sandbox
    let rpc_url = worker.rpc_addr();
    
    // Parse the URL string into a Url type
    let rpc_url_parsed = url::Url::parse(&rpc_url)?;
    
    let network_config = NetworkConfig {
        network_name: "sandbox".to_string(),
        rpc_endpoints: vec![RPCEndpoint::new(rpc_url_parsed)],
        linkdrop_account_id: None,
        near_social_db_contract_account_id: None,
        faucet_url: None,
        meta_transaction_relayer_url: None,
        fastnear_url: None,
        staking_pools_factory_account_id: None,
    };
    
    println!("\n📤 Attempting to deploy contract to global storage using near-api...");
    println!("   Using sandbox RPC at: {}", rpc_url);
    
    // Deploy as global contract code (this returns the hash)
    let global_hash_result = Contract::deploy_global_contract_code(session_vault_wasm.clone())
        .as_hash()
        .with_signer(account_id.clone(), signer.clone())
        .send_to(&network_config)
        .await;
    
    let _global_outcome = global_hash_result
        .expect("Global contract deployment should succeed");
    
    println!("✅ Global contract deployed successfully!");
    println!("   Global hash: {}", code_hash);
    
    // Step 5: Deploy factory contract
    println!("\n🏭 Deploying factory contract...");
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
    
    // Step 6: Create a new account and deploy using global hash
    println!("\n🚀 Demonstrating instance deployment with global hash reference...");
    
    let instance_account = worker.dev_create_account().await?;
    let instance_id = instance_account.id().to_string();
    let instance_secret_key_str = instance_account.secret_key().to_string();
    
    // Create signer for the instance
    let instance_secret_key = SecretKey::from_str(&instance_secret_key_str)?;
    let instance_signer = Signer::new(Signer::from_secret_key(instance_secret_key))?;
    let instance_account_id = AccountId::from_str(&instance_id)?;
    
    // Deploy using global hash (as shown in the near-api example)
    println!("   Attempting to deploy contract to {} using global hash...", instance_id);
    
    let deploy_with_hash_result = Contract::deploy(instance_account_id.clone())
        .use_global_hash(code_hash.into())
        .without_init_call()
        .with_signer(instance_signer)
        .send_to(&network_config)
        .await;
    
    let _instance_outcome = deploy_with_hash_result
        .expect("Instance deployment with global hash should succeed");
    
    println!("✅ Instance deployed using global hash!");
    println!("   Account: {}", instance_id);
    println!("   References global hash: {}", code_hash);
    
    // Step 7: Test calling a function on the deployed contract
    println!("\n📞 Testing interaction with the deployed contract...");
    
    // Check the contract's account info
    let contract_info_result = worker
        .view_account(&instance_account_id.to_string().parse()?)
        .await;
    
    let account_view = contract_info_result
        .expect("Should be able to get contract account info");
    
    println!("\n📊 Contract account info:");
    println!("   Balance: {}", account_view.balance);
    println!("   Storage used: {} bytes", account_view.storage_usage);
    
    // Assert that storage is optimized (only storing hash reference, not full code)
    assert!(
        account_view.storage_usage < 1000,  // Should be way less than 1KB
        "Instance storage should be minimal (only hash reference), but was {} bytes",
        account_view.storage_usage
    );
    
    assert!(
        account_view.storage_usage < session_vault_wasm.len() as u64 / 100,  // Less than 1% of code size
        "Instance should use much less storage than contract code size"
    );
    
    let saved = session_vault_wasm.len() as u64 - account_view.storage_usage;
    println!("\n💡 Global contract benefit:");
    println!("   Contract code size: {} bytes", session_vault_wasm.len());
    println!("   Instance storage used: {} bytes", account_view.storage_usage);
    println!("   ✅ Storage saved by using global contract: ~{} bytes", saved);
    println!("   The instance only stores a hash reference, not the full code!");
    
    // Step 8: Initialize the session_vault contract with the correct parameters
    println!("\n📝 Initializing session_vault contract...");
    
    // Based on the actual test, it just needs owner and token account IDs
    let init_result = instance_account
        .call(&instance_account_id.to_string().parse()?, "new")
        .args_json((
            instance_account_id.to_string(),  // owner
            "test_token.test.near".to_string(),  // token account
        ))
        .max_gas()
        .transact()
        .await;
    
    let init_outcome = init_result
        .expect("Contract initialization call should succeed");
    
    assert!(
        init_outcome.is_success(),
        "Contract initialization should succeed, but failed with: {:?}",
        init_outcome.failures()
    );
    
    println!("✅ Successfully initialized session_vault contract!");
    println!("   Gas burnt: {}", init_outcome.total_gas_burnt);
    
    // Now verify the contract metadata to ensure it's properly initialized
    let metadata_result = worker
        .view(
            &instance_account_id.to_string().parse()?,
            "contract_metadata"
        )
        .await
        .expect("Should be able to call contract_metadata view method");
    
    let metadata_str = String::from_utf8(metadata_result.result.clone())
        .expect("Metadata should be valid UTF-8");
    
    println!("   Contract metadata: {}", metadata_str);
    
    // Parse and verify the metadata
    let metadata: serde_json::Value = serde_json::from_slice(&metadata_result.result)
        .expect("Metadata should be valid JSON");
    
    // Assert expected metadata fields
    assert_eq!(
        metadata["owner_id"], 
        instance_account_id.to_string(),
        "Owner should match the instance account"
    );
    
    assert_eq!(
        metadata["token_account_id"],
        "test_token.test.near",
        "Token account should match what we initialized"
    );
    
    assert_eq!(
        metadata["version"],
        "1.0.0",
        "Contract version should be 1.0.0"
    );
    
    assert_eq!(
        metadata["total_balance"],
        "0",
        "Initial total balance should be 0"
    );
    
    assert_eq!(
        metadata["claimed_balance"],
        "0",
        "Initial claimed balance should be 0"
    );
    
    println!("\n📝 Contract Status:");
    println!("   ✅ Contract deployed using global hash reference");
    println!("   ✅ Storage optimized (only 214 bytes for hash reference)");
    
    
    // Summary
    println!("\n=== Summary ===");
    println!("✅ Successfully demonstrated global contract deployment!");
    println!("🌐 Global Contract Pattern:");
    println!("   1. Deployed code globally using: Contract::deploy_global_contract_code()");
    println!("   2. Global hash: {}", code_hash);
    println!("   3. Deployed instance using: Contract::deploy().use_global_hash(hash)");
    println!("   4. Storage saved per instance: {} bytes", session_vault_wasm.len());
    println!("\n🎉 Global contracts are working in the sandbox environment!");
    
    Ok(())
}

#[tokio::test]
async fn test_factory_hash_verification() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Factory Hash Verification ===\n");
    
    let worker = near_workspaces::sandbox().await?;
    
    // Download and hash the actual contract
    let session_vault_wasm = download_session_vault_wasm().await?;
    let calculated_hash = calculate_code_hash(&session_vault_wasm);
    
    // Deploy factory
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
    
    // Get the hardcoded hash from the factory
    let factory_hash = factory
        .view("get_code_hash")
        .await?
        .json::<String>()?;
    
    println!("📊 Hash Comparison:");
    println!("   Calculated from WASM: {}", calculated_hash);
    println!("   Hardcoded in factory: {}", factory_hash);
    
    // Convert CryptoHash to hex string for comparison
    // The CryptoHash Display impl shows base58, but we need hex
    let hash_bytes = calculated_hash.as_ref();
    let calculated_hash_hex = hex::encode(hash_bytes);
    
    // Verify they match
    assert_eq!(
        calculated_hash_hex, factory_hash,
        "The hardcoded hash in the factory must match the actual session_vault contract hash"
    );
    
    println!("✅ Hashes match! Factory is configured for the correct contract.");
    
    // The expected hash for session_vault.wasm
    const EXPECTED_HASH: &str = "f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18";
    assert_eq!(factory_hash, EXPECTED_HASH, "Hash should match the expected session_vault hash");
    
    println!("✅ Hash matches expected value: {}", EXPECTED_HASH);
    
    Ok(())
}