use near_sdk::{base64, NearToken};
use near_workspaces::{self as workspace};
use serde_json::json;

const SESSION_VAULT_WASM: &[u8] = include_bytes!("../res/session_vault.wasm");
const TEST_TOKEN_WASM: &[u8] = include_bytes!("../res/test_token.wasm");

// ============ TEST CONFIGURATION ============
// Adjust these values to test different scenarios
const TEST_USER_COUNT: usize = 50; // Number of users to test
const INSTANCE_DEPOSIT_MILLINEAR: u128 = 170; // 0.17 NEAR - minimum for 50 users
const PER_USER_DEPOSIT_MILLINEAR: u128 = 3; // 0.003 NEAR - minimum per user (excess refunded)
                                            // Note: Due to a bug in vault's storage management, instance deposit must be
                                            // sufficient for the total number of users. Per-user deposits are mostly refunded.
                                            // ==========================================

#[tokio::test]
async fn test_session_vault_50_users_storage() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "\n=== Testing Session Vault Storage Requirements for {} Users (Factory Pattern) ===\n",
        TEST_USER_COUNT
    );
    println!(
        "This test measures storage costs for factory-deployed vaults with global contracts.\n"
    );
    println!("Configuration:");
    println!(
        "  - Instance deposit: {} NEAR",
        NearToken::from_millinear(INSTANCE_DEPOSIT_MILLINEAR)
    );
    println!(
        "  - Per-user deposit: {} NEAR",
        NearToken::from_millinear(PER_USER_DEPOSIT_MILLINEAR)
    );
    println!("  - Target users: {}\n", TEST_USER_COUNT);

    let worker = workspace::sandbox_with_version("2.7.0").await?;

    // Step 1: Deploy test token
    println!("📦 Step 1: Deploy test token");
    let token_account = worker.dev_create_account().await?;
    let token_id = token_account.id();

    token_account.deploy(TEST_TOKEN_WASM).await?.into_result()?;

    token_account
        .call(token_id, "new")
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    println!("✅ Test token deployed at: {}", token_id);

    // Step 2: Deploy factory
    println!("\n🏭 Step 2: Deploy factory contract");
    let factory_wasm = near_workspaces::compile_project("./").await?;
    let factory_account = worker.dev_deploy(&factory_wasm).await?;
    let factory_id = factory_account.id();

    factory_account
        .call("new")
        .args_json(json!({
            "owner_id": factory_id.to_string()
        }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    println!("✅ Factory deployed at: {}", factory_id);

    // Step 3: Deploy global contract
    println!("\n🌍 Step 3: Deploy session_vault as global contract");

    let code_base64 = base64::encode(SESSION_VAULT_WASM);
    let global_deployer_id = format!("global.{}", factory_id);

    factory_account
        .call("deploy_global_contract")
        .args_json(json!({
            "code": code_base64,
            "deployer_account_id": global_deployer_id
        }))
        .deposit(NearToken::from_near(20))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    println!("✅ Global contract deployed");

    // Use configured deposit amount
    let deposit_amount = NearToken::from_millinear(INSTANCE_DEPOSIT_MILLINEAR);
    println!(
        "\n📊 Creating vault instance with {} deposit",
        deposit_amount
    );
    println!("{}", "─".repeat(50));

    // Create instance through factory
    let instance_name = "vault_minimal";

    let create_result = factory_account
        .call("create_instance")
        .args_json(json!({
            "name": instance_name,
            "owner_id": factory_id.to_string(),
            "token_id": token_id.to_string()
        }))
        .deposit(deposit_amount)
        .max_gas()
        .transact()
        .await?;

    if !create_result.is_success() {
        println!(
            "❌ Failed to create instance with {}: {:?}",
            deposit_amount,
            create_result.failures()
        );
        panic!(
            "Failed to create instance with {} deposit: {:?}",
            deposit_amount,
            create_result.failures()
        );
    }

    let vault_id: near_workspaces::AccountId =
        format!("{}.{}", instance_name, factory_id).parse()?;

    println!("✅ Vault instance created at: {}", vault_id);

    // Get initial metrics
    let initial_view = worker.view_account(&vault_id).await?;
    let initial_storage = initial_view.storage_usage;
    let initial_balance = initial_view.balance;
    let initial_locked = initial_view.locked;

    println!("\n📝 Initial state:");
    println!("   Storage: {} bytes", initial_storage);
    println!("   Balance: {}", initial_balance);
    println!("   Locked for storage: {}", initial_locked);
    println!("   Note: Using global contract (132 bytes ref, not 156KB WASM)");

    // Try to add users with configured deposits
    println!(
        "\n👥 Adding {} users with configured deposits...",
        TEST_USER_COUNT
    );
    let mut success_count = 0;
    let mut total_storage_deposit = 0u128;

    // Use configured per-user deposit
    let storage_deposit_per_user = NearToken::from_millinear(PER_USER_DEPOSIT_MILLINEAR);
    println!("   Using {} deposit per user", storage_deposit_per_user);

    for i in 0..TEST_USER_COUNT {
        let user_id = format!("user{}.test.near", i);
        total_storage_deposit += storage_deposit_per_user.as_yoctonear();

        // Check factory account balance before add_account
        let factory_balance_before = worker.view_account(factory_id).await?.balance;

        let add_result = factory_account
            .as_account()
            .call(&vault_id, "add_account")
            .args_json(json!({
                "account_id": user_id,
                "start_timestamp": format!("{}", 1700000000 + i * 86400),
                "session_interval": "2592000",
                "session_num": 12,
                "release_per_session": "1000000000000000000000000"
            }))
            .deposit(storage_deposit_per_user)
            .max_gas()
            .transact()
            .await;

        match add_result {
            Ok(outcome) => {
                if outcome.is_success() {
                    success_count += 1;

                    // Check vault balance to see how much was kept
                    let vault_balance_after = worker.view_account(&vault_id).await?.balance;

                    if i == 0 {
                        let storage_bytes = worker.view_account(&vault_id).await?.storage_usage;
                        let storage_cost = (storage_bytes as u128 - initial_storage as u128)
                            * 10_000_000_000_000_000_000u128;
                        println!("   First user added:");
                        println!("      Deposit sent: {}", storage_deposit_per_user);
                        println!("      Vault balance after: {}", vault_balance_after);
                        println!("      Vault storage: {} bytes", storage_bytes);
                        println!(
                            "      Storage cost for {} bytes: {}",
                            storage_bytes - initial_storage,
                            NearToken::from_yoctonear(storage_cost)
                        );
                        println!(
                            "      Amount kept by vault: {}",
                            vault_balance_after.saturating_sub(initial_balance)
                        );
                    } else if (i + 1) % 10 == 0 {
                        println!(
                            "   ✓ {} users added - Vault balance: {}",
                            i + 1,
                            vault_balance_after
                        );
                    }
                } else {
                    println!("\n❌ Failed at user {}: {:?}", i, outcome.failures());
                    panic!(
                        "Failed to add user {} - transaction failed: {:?}",
                        i,
                        outcome.failures()
                    );
                }
            }
            Err(e) => {
                println!("\n❌ Error at user {}: {}", i, e);
                panic!("Failed to add user {}: {}", i, e);
            }
        }
    }

    // Get final metrics
    let final_view = worker.view_account(&vault_id).await?;
    let final_storage = final_view.storage_usage;
    let final_balance = final_view.balance;
    let final_locked = final_view.locked;

    let storage_used_for_users = final_storage - initial_storage;
    let locked_for_users = final_locked.saturating_sub(initial_locked);
    let _balance_used = initial_balance.saturating_sub(final_balance);

    println!("\n📈 Test Results:");
    println!("   Deposit amount: {}", deposit_amount);
    println!(
        "   Users successfully added: {}/{}",
        success_count, TEST_USER_COUNT
    );
    println!("   User storage growth: {} bytes", storage_used_for_users);
    println!("   Additional amount locked: {}", locked_for_users);

    if success_count > 0 {
        let bytes_per_user = storage_used_for_users / success_count;
        println!("   Average storage per user: {} bytes", bytes_per_user);

        let storage_cost_per_byte = 10_000_000_000_000_000_000u128;
        let storage_cost = (storage_used_for_users as u128) * storage_cost_per_byte;
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
    println!("   Final balance: {}", final_balance);

    if success_count == TEST_USER_COUNT as u64 {
        println!(
            "\n✅ SUCCESS! {} is sufficient for {} users!",
            deposit_amount, TEST_USER_COUNT
        );

        // Calculate precise requirements
        let bytes_per_user = storage_used_for_users / TEST_USER_COUNT as u64;
        let storage_cost_per_byte = 10_000_000_000_000_000_000u128;
        let storage_cost_per_user = (bytes_per_user as u128) * storage_cost_per_byte;
        let total_storage_cost_50_users = (storage_used_for_users as u128) * storage_cost_per_byte;

        println!("\n💰 === DEPOSIT BREAKDOWN ===");
        println!("Instance creation deposit: {}", deposit_amount);
        println!("   → Initial balance: {}", initial_balance);
        println!("   → Initial locked: {}", initial_locked);
        println!("\nAfter adding 50 users:");
        println!("   → Final balance: {}", final_balance);
        println!("   → Final locked: {}", final_locked);
        println!(
            "   → Balance change: {}",
            if final_balance > initial_balance {
                format!("+{}", final_balance.saturating_sub(initial_balance))
            } else {
                format!("-{}", initial_balance.saturating_sub(final_balance))
            }
        );

        println!("\n📈 === STORAGE COST ANALYSIS ===");
        println!("Per-user metrics:");
        println!("   Storage per user: {} bytes", bytes_per_user);
        println!(
            "   Storage cost per user: {}",
            NearToken::from_yoctonear(storage_cost_per_user)
        );
        println!(
            "   Locked amount per user: {}",
            NearToken::from_yoctonear(locked_for_users.as_yoctonear() / TEST_USER_COUNT as u128)
        );
        println!(
            "   Each add_account deposit: {}",
            NearToken::from_yoctonear(total_storage_deposit / TEST_USER_COUNT as u128)
        );

        println!("\nTotal for {} users:", TEST_USER_COUNT);
        println!("   User storage: {} bytes", storage_used_for_users);
        println!(
            "   Calculated storage cost: {}",
            NearToken::from_yoctonear(total_storage_cost_50_users)
        );
        println!("   Actual locked for users: {}", locked_for_users);
        println!(
            "   Total add_account deposits: {}",
            NearToken::from_yoctonear(total_storage_deposit)
        );
        println!(
            "   Net balance increase: {}",
            final_balance.saturating_sub(initial_balance)
        );

        // Calculate total requirements for factory instances
        let total_instance_storage = initial_storage + storage_used_for_users;
        let total_instance_cost = (total_instance_storage as u128) * storage_cost_per_byte;

        println!("\n📝 === MINIMUM REQUIREMENTS FOUND ===");
        println!("For {} users:", TEST_USER_COUNT);
        println!("   • Instance creation: {} minimum", deposit_amount);
        println!(
            "   • Per user deposit: {} ({} × {} = {})",
            NearToken::from_yoctonear(total_storage_deposit / TEST_USER_COUNT as u128),
            TEST_USER_COUNT,
            NearToken::from_yoctonear(total_storage_deposit / TEST_USER_COUNT as u128),
            NearToken::from_yoctonear(total_storage_deposit)
        );
        println!("   • Total storage used: {} bytes", total_instance_storage);
        println!(
            "   • Storage cost: {} NEAR",
            NearToken::from_yoctonear(total_instance_cost)
        );
        println!(
            "   • Final balance: {} (started with {})",
            final_balance, initial_balance
        );
    } else {
        println!("\n❌ FAILED: {} was NOT sufficient!", deposit_amount);
        println!(
            "   Could only add {} users out of {}",
            success_count, TEST_USER_COUNT
        );
        println!("   Instance may need more deposit for operational costs");
        panic!(
            "Test failed: {} deposit was insufficient for {} users (only {} users added)",
            deposit_amount, TEST_USER_COUNT, success_count
        );
    }

    Ok(())
}
