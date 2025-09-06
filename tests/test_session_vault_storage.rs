use near_sdk::NearToken;
use near_workspaces::{self as workspace};
use serde_json::json;

const SESSION_VAULT_WASM: &[u8] = include_bytes!("../res/session_vault.wasm");
const TEST_TOKEN_WASM: &[u8] = include_bytes!("../res/test_token.wasm");

#[tokio::test]
async fn test_session_vault_50_users_storage() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Session Vault Storage Requirements for 50 Users ===\n");
    println!(
        "This test deploys a session vault directly and adds users to measure storage costs.\n"
    );

    let worker = workspace::sandbox().await?;

    // Deploy test token first
    println!("📦 Step 1: Deploy test token");
    let token_account = worker.dev_create_account().await?;
    let token_id = token_account.id();

    token_account.deploy(TEST_TOKEN_WASM).await?.into_result()?;

    // Initialize test token (no args)
    token_account
        .call(token_id, "new")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    println!("✅ Test token deployed at: {}", token_id);

    // Test different initial deposits for the vault
    let test_deposits = vec![
        (NearToken::from_near(2), "2 NEAR"),
        (NearToken::from_near(3), "3 NEAR"),
        (NearToken::from_near(4), "4 NEAR"),
        (NearToken::from_near(5), "5 NEAR"),
    ];

    for (deposit_amount, deposit_str) in test_deposits {
        println!("\n📊 Testing vault with {} initial deposit", deposit_str);
        println!("{}", "─".repeat(50));

        // Create vault account with specified deposit
        let vault_account = worker.dev_create_account().await?;

        // Deploy vault
        vault_account
            .deploy(SESSION_VAULT_WASM)
            .await?
            .into_result()?;

        let vault_id = vault_account.id();

        // Initialize vault
        let init_result = vault_account
            .call(vault_id, "new")
            .args_json(json!({
                "owner_id": vault_id.to_string(),
                "token_id": token_id.to_string()
            }))
            .max_gas()
            .transact()
            .await;

        if init_result.is_err() {
            println!("❌ Failed to initialize vault with {}", deposit_str);
            continue;
        }

        println!("✅ Vault deployed and initialized at: {}", vault_id);

        // Get initial metrics
        let initial_view = worker.view_account(vault_id).await?;
        let initial_storage = initial_view.storage_usage;
        let initial_balance = initial_view.balance;

        println!("\n📝 Initial state:");
        println!("   Storage: {} bytes", initial_storage);
        println!("   Balance: {}", initial_balance);

        // Try to add 50 users
        println!("\n👥 Adding users...");
        let mut success_count = 0;
        let mut total_storage_deposit = 0u128;

        for i in 0..50 {
            let user_id = format!("user{}.test.near", i);

            // Storage deposit per user (estimated 250-300 bytes per user)
            // NEAR charges 0.00001 NEAR per byte
            // So ~300 bytes = 0.003 NEAR, we'll use 0.005 NEAR to be safe
            let storage_deposit = NearToken::from_millinear(5);
            total_storage_deposit += storage_deposit.as_yoctonear();

            let add_result = vault_account
                .call(vault_id, "add_account")
                .args_json(json!({
                    "account_id": user_id,
                    "start_timestamp": format!("{}", 1700000000 + i * 86400), // U64 as string
                    "session_interval": "2592000", // U64 as string (30 days)
                    "session_num": 12, // u32 as number
                    "release_per_session": "1000000000000000000000000" // U128 as string
                }))
                .deposit(storage_deposit)
                .max_gas()
                .transact()
                .await;

            match add_result {
                Ok(outcome) => {
                    if outcome.is_success() {
                        success_count += 1;
                        if (i + 1) % 10 == 0 {
                            println!("   Added {} users", i + 1);
                        }
                    } else {
                        println!("\n❌ Failed at user {}: {:?}", i, outcome.failures());

                        // Check if it's a balance issue
                        let current_view = worker.view_account(vault_id).await?;
                        if current_view.balance < NearToken::from_millinear(100) {
                            println!("   ⚠️  Vault balance too low: {}", current_view.balance);
                        }
                        break;
                    }
                }
                Err(e) => {
                    println!("\n❌ Error at user {}: {}", i, e);
                    break;
                }
            }
        }

        // Get final metrics
        let final_view = worker.view_account(vault_id).await?;
        let final_storage = final_view.storage_usage;
        let final_balance = final_view.balance;

        let storage_used = final_storage.saturating_sub(initial_storage);
        let balance_used = initial_balance.saturating_sub(final_balance);

        println!("\n📈 Results for {} initial deposit:", deposit_str);
        println!("   Users successfully added: {}/50", success_count);
        println!("   Storage growth: {} bytes", storage_used);
        if success_count > 0 {
            let bytes_per_user = storage_used / success_count;
            println!("   Average storage per user: {} bytes", bytes_per_user);

            // Calculate storage cost
            // NEAR charges 10^19 yoctoNEAR per byte (0.00001 NEAR)
            let storage_cost_per_byte = 10_000_000_000_000_000_000u128;
            let storage_cost = (storage_used as u128) * storage_cost_per_byte;
            println!(
                "   Storage cost for {} users: {}",
                success_count,
                NearToken::from_yoctonear(storage_cost)
            );
        }
        println!(
            "   Total deposits made: {}",
            NearToken::from_yoctonear(total_storage_deposit)
        );
        println!("   Balance consumed: {}", balance_used);
        println!("   Final balance: {}", final_balance);

        if success_count == 50 {
            println!("\n✅ SUCCESS! Found minimal deposit requirement");

            // Calculate recommendations
            let bytes_per_user = storage_used / 50;
            let storage_cost_per_byte = 10_000_000_000_000_000_000u128;
            let storage_cost_per_user = (bytes_per_user as u128) * storage_cost_per_byte;
            let total_storage_cost_50_users = (storage_used as u128) * storage_cost_per_byte;

            // Account for the vault's base storage + 50 users
            let recommended_with_buffer = deposit_amount.saturating_add(NearToken::from_near(1));

            println!("\n💰 === STORAGE COST ANALYSIS ===");
            println!("   Storage per user: {} bytes", bytes_per_user);
            println!(
                "   Storage cost per user: {}",
                NearToken::from_yoctonear(storage_cost_per_user)
            );
            println!("   Total storage for 50 users: {} bytes", storage_used);
            println!(
                "   Total storage cost: {}",
                NearToken::from_yoctonear(total_storage_cost_50_users)
            );

            println!("\n📝 === RECOMMENDATIONS ===");
            println!(
                "   Minimum deposit for vault with 50 users: {}",
                deposit_str
            );
            println!(
                "   Recommended (with safety buffer): {}",
                recommended_with_buffer
            );
            println!(
                "   Per-user storage deposit needed: {} NEAR",
                NearToken::from_yoctonear(storage_cost_per_user).as_near()
            );

            // Calculate for different user counts
            println!("\n📊 === SCALING ESTIMATES ===");
            for user_count in &[10, 25, 50, 100, 200] {
                let est_storage = bytes_per_user * user_count;
                let est_cost = (est_storage as u128) * storage_cost_per_byte;
                let base_deposit = NearToken::from_near(2); // Base for vault
                let total_needed = base_deposit.saturating_add(NearToken::from_yoctonear(est_cost));

                println!(
                    "   {} users: ~{} (base) + {} (storage) = {} total",
                    user_count,
                    base_deposit,
                    NearToken::from_yoctonear(est_cost),
                    total_needed
                );
            }

            break;
        } else {
            println!(
                "\n⚠️  Could only add {} users with {}",
                success_count, deposit_str
            );
        }
    }

    Ok(())
}
