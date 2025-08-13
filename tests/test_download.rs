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
async fn test_download_and_hash() -> Result<(), Box<dyn std::error::Error>> {
    // Download the session_vault WASM
    let session_vault_wasm = download_session_vault_wasm().await?;
    
    // Calculate and verify the code hash
    let code_hash = calculate_code_hash(&session_vault_wasm);
    println!("Session vault code hash: {}", code_hash);
    
    // Expected hash from the file we downloaded earlier
    let expected_hash = "f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18";
    assert_eq!(code_hash, expected_hash, "Code hash mismatch!");
    
    println!("✅ Successfully downloaded and verified session_vault.wasm");
    println!("   Size: {} bytes", session_vault_wasm.len());
    println!("   Hash: {}", code_hash);
    
    Ok(())
}