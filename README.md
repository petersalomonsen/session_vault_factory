# Session Vault Factory

A factory contract for deploying and managing session_vault vesting contracts on NEAR Protocol. This project addresses [neardevhub-treasury-dashboard#655](https://github.com/NEAR-DevHub/neardevhub-treasury-dashboard/issues/655).

## Overview

The Session Vault Factory enables automated detection and tracking of locked tokens in NEAR treasuries by providing a centralized factory for deploying session_vault contracts. This solves the visibility problem where treasuries cannot automatically detect locked tokens without a central repository of vesting contract instances.

## Features

- **Factory Pattern**: Deploy new session_vault contract instances through a single factory
- **Instance Tracking**: Maintain a registry of all deployed session_vault contracts
- **Pagination Support**: Efficiently list and query deployed instances with pagination
- **Code Hash Deployment**: Uses global contract deployment by code hash for consistency
- **Treasury Integration**: Enables NEAR treasuries to automatically detect and display locked fungible tokens

## How to Build Locally?

Install [`cargo-near`](https://github.com/near/cargo-near) and run:

```bash
cargo near build
```

## How to Test Locally?

```bash
cargo test
```

## How to Deploy?

### Prerequisites

1. Install [`near-cli-rs`](https://github.com/near/near-cli-rs):
   ```bash
   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/near/near-cli-rs/releases/latest/download/near-cli-rs-installer.sh | sh
   ```

2. Install [`cargo-near`](https://github.com/near/cargo-near):
   ```bash
   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/near/cargo-near/releases/latest/download/cargo-near-installer.sh | sh
   ```

### Deployment Steps for Testnet/Mainnet

#### 1. Build the Contracts

```bash
# Build the factory contract (non-reproducible for local development)
cargo near build non-reproducible-wasm

# Download the pre-built session_vault contract
curl -L https://github.com/brainstems/intellex_vesting_contracts/raw/main/res/session_vault.wasm -o session_vault.wasm
```

#### 2. Create and Fund Factory Account

For **Testnet** (using faucet):
```bash
# Generate a unique account name
FACTORY_ACCOUNT="session-vault-factory-$(date +%s).testnet"

# Create account using testnet faucet (provides ~10 NEAR)
near account create-account sponsor-by-faucet-service $FACTORY_ACCOUNT \
  autogenerate-new-keypair save-to-legacy-keychain \
  network-config testnet create
```

For **Mainnet** (requires funding):
```bash
# Create account name
FACTORY_ACCOUNT="your-factory-name.near"

# Create account and fund with existing account
near account create-account fund-myself $FACTORY_ACCOUNT '30 NEAR' \
  autogenerate-new-keypair save-to-legacy-keychain \
  sign-as <your-funding-account> network-config mainnet sign-with-keychain send
```

**Important**: The factory account needs at least 20-25 NEAR to deploy the global contract. If using testnet faucet, you'll need to transfer additional funds:

```bash
# Transfer additional funds (if needed)
near tokens <FUNDING_ACCOUNT> send-near $FACTORY_ACCOUNT 20NEAR \
  network-config testnet sign-with-plaintext-private-key <YOUR_PRIVATE_KEY> send
```

#### 3. Deploy and Initialize Factory Contract

```bash
# Deploy the factory contract
near contract deploy $FACTORY_ACCOUNT \
  use-file ./target/near/session_vault_factory.wasm \
  without-init-call network-config testnet sign-with-keychain send

# Initialize the factory
near contract call-function as-transaction $FACTORY_ACCOUNT new \
  json-args "{\"owner_id\":\"$FACTORY_ACCOUNT\"}" \
  prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' \
  sign-as $FACTORY_ACCOUNT network-config testnet sign-with-keychain send
```

#### 4. Deploy Session Vault as Global Contract

```bash
# Convert session_vault.wasm to base64
base64 -i session_vault.wasm | tr -d '\n' > session_vault.base64

# Deploy as global contract (requires ~20 NEAR deposit)
# Storage cost calculation:
# - NEAR charges 1 NEAR per 10 KB of storage
# - session_vault.wasm is ~156 KB
# - 156 KB ÷ 10 KB = 15.6 NEAR for contract storage
# - Plus ~2 NEAR for account creation and metadata
# - Total: ~17.56 NEAR minimum (we use 20 NEAR for safety margin)
CODE_BASE64=$(cat session_vault.base64)
near contract call-function as-transaction $FACTORY_ACCOUNT deploy_global_contract \
  json-args "{\"code\":\"$CODE_BASE64\",\"deployer_account_id\":\"global.$FACTORY_ACCOUNT\"}" \
  prepaid-gas '300.0 Tgas' attached-deposit '20 NEAR' \
  sign-as $FACTORY_ACCOUNT network-config testnet sign-with-keychain send
```

**Note**: The global deployment creates the account `global.$FACTORY_ACCOUNT` automatically and deploys the contract code globally by hash.

#### 5. Verify Deployment

```bash
# Check if global contract is deployed
near contract call-function as-read-only $FACTORY_ACCOUNT \
  is_global_contract_deployed json-args '{}' \
  network-config testnet now
```

### Creating Session Vault Instances

Once the factory is deployed and the global contract is registered, you can create and initialize session vault instances in a single step:

```bash
# Create and initialize a new session vault instance
INSTANCE_NAME="vault1"
OWNER_ID="$FACTORY_ACCOUNT"  # The account that will manage the vault
TOKEN_ID="your-token.testnet"  # The FT token this vault will manage

# Note: Due to storage bug, deposit must match expected user count:
# - 50 users: 0.17 NEAR
# - 100 users: 0.30 NEAR
near contract call-function as-transaction $FACTORY_ACCOUNT create_instance \
  json-args "{\"name\":\"$INSTANCE_NAME\",\"owner_id\":\"$OWNER_ID\",\"token_id\":\"$TOKEN_ID\"}" \
  prepaid-gas '100.0 Tgas' attached-deposit '0.17 NEAR' \
  sign-as $FACTORY_ACCOUNT network-config testnet sign-with-keychain send
```

This will:
1. Create a new session vault at: `$INSTANCE_NAME.$FACTORY_ACCOUNT`
2. Initialize it with the specified owner and token configuration

**Parameters**:
- `name`: The instance name (will become a sub-account of the factory)
- `owner_id`: The account that will manage the vault (typically the factory or your admin account)
- `token_id`: The fungible token contract ID that this vault will handle

### Storage Requirements for Factory-Deployed Vaults

When deploying vaults through the factory with global contracts, storage costs are dramatically reduced:

#### Storage Breakdown
- **Base instance storage**: 320 bytes (132-byte contract reference + vault state)
- **Per user storage**: 234 bytes (~0.00234 NEAR per user)
- **Savings per instance**: ~171KB (99.8% reduction vs traditional deployment)

#### Minimum Deposit Requirements (Tested and Verified)

| Component | Minimum Amount | Notes |
|-----------|---------------|-------|
| **Per User (add_account)** | **0.003 NEAR** | Minimum to cover 234 bytes storage. Excess is automatically refunded |

#### Instance Creation Deposit Requirements

⚠️ **Important**: Due to a bug in the session_vault's storage management, the instance creation deposit must be sufficient for the total number of users you plan to add. The vault's `internal_check_storage` function incorrectly refunds most of the per-user deposits instead of keeping enough to maintain the account's minimum balance.

| Number of Users | Required Instance Deposit | Notes |
|-----------------|--------------------------|-------|
| 50 users        | **0.17 NEAR**           | Tested minimum |
| 100 users       | **0.30 NEAR**           | Tested minimum |
| 200 users       | **~0.50 NEAR**          | Estimated |

**The Storage Bug**: The vault only keeps the exact storage cost (~0.00234 NEAR per user) from each `add_account` deposit and refunds the rest. It doesn't account for NEAR's minimum balance requirement which grows with storage usage. This means:
- Per-user deposits don't build up the vault's balance as intended
- The initial instance deposit must cover the entire minimum balance requirement
- Alternative: You can send additional NEAR directly to the vault instance after creation

#### Cost Examples (with current bug)

| Number of Users | Instance Deposit | User Deposits | Total Cost | Actual Cost After Refunds |
|-----------------|------------------|---------------|------------|---------------------------|
| 50 users        | 0.17 NEAR       | 50 × 0.003 = 0.15 NEAR  | 0.32 NEAR | ~0.19 NEAR (most refunded) |
| 100 users       | 0.30 NEAR       | 100 × 0.003 = 0.30 NEAR | 0.60 NEAR | ~0.33 NEAR (most refunded) |

**Refund Mechanism**: The `add_account` function automatically refunds excess deposits, but due to the bug, it refunds too much - keeping only ~0.00234 NEAR per user instead of keeping enough to maintain the account's minimum balance.

**Why so efficient?** With global contracts, each instance only stores a 132-byte reference to the contract code instead of the full 156KB WASM binary. This results in 99.8% storage savings per instance.

#### Adding Users to a Vault

When adding users to a session vault, each `add_account` call requires a storage deposit:

```bash
# Add a user with vesting schedule
VAULT_INSTANCE="vault1.session-factory-1757014869.testnet"
USER_ID="alice.testnet"

near contract call-function as-transaction $VAULT_INSTANCE add_account \
  json-args "{
    \"account_id\":\"$USER_ID\",
    \"start_timestamp\":\"1700000000\",
    \"session_interval\":\"2592000\",
    \"session_num\":12,
    \"release_per_session\":\"1000000000000000000000000\"
  }" \
  prepaid-gas '30.0 Tgas' attached-deposit '0.003 NEAR' \
  sign-as $OWNER_ID network-config testnet sign-with-keychain send
```

**Parameters for add_account**:
- `account_id`: The user's NEAR account
- `start_timestamp`: Unix timestamp when vesting starts (as string)
- `session_interval`: Duration of each vesting period in seconds (as string, e.g., "2592000" for 30 days)
- `session_num`: Number of vesting periods (as number)
- `release_per_session`: Amount to release per period in yoctoNEAR (as string)

### Verifying the Instance

Check that the instance is properly created and initialized:

```bash
# Get vault metadata
VAULT_INSTANCE="$INSTANCE_NAME.$FACTORY_ACCOUNT"
near contract call-function as-read-only $VAULT_INSTANCE contract_metadata \
  json-args '{}' network-config testnet now
```

Expected output:
```json
{
  "claimed_balance": "0",
  "owner_id": "your-factory.testnet",
  "token_account_id": "your-token.testnet",
  "total_balance": "0",
  "version": "1.0.0"
}
```

### Listing Deployed Instances

```bash
# List all instances (with pagination)
near contract call-function as-read-only $FACTORY_ACCOUNT list_instances \
  json-args '{"from_index":0,"limit":10}' \
  network-config testnet now
```

### Important Notes

1. **Testnet Deployment**: The example above uses testnet. Replace `testnet` with `mainnet` for production deployment.

2. **Global Contract Support**: The global contract deployment feature requires NEAR runtime support for the `deploy_global_contract` action. This feature is part of [near-sdk PR #1369](https://github.com/near/near-sdk-rs/pull/1369) and may not be fully available on all networks yet.

3. **Account Credentials**: The NEAR CLI saves credentials in `~/.near-credentials/[network]/[account].json`

4. **Session Vault Hash**: The factory verifies the session_vault contract hash for security. The expected hash is hardcoded in the factory contract.

### Example Successfully Deployed Contracts

- **Factory**: `session-factory-1757014869.testnet`
- **Global Contract**: `global.session-factory-1757014869.testnet` 
- **Instance Example**: `vault1.session-factory-1757014869.testnet`
- **Global Contract Hash**: `f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18`

## Useful Links

- [cargo-near](https://github.com/near/cargo-near) - NEAR smart contract development toolkit for Rust
- [near CLI](https://near.cli.rs) - Interact with NEAR blockchain from command line
- [NEAR Rust SDK Documentation](https://docs.near.org/sdk/rust/introduction)
- [NEAR Documentation](https://docs.near.org)
- [NEAR StackOverflow](https://stackoverflow.com/questions/tagged/nearprotocol)
- [NEAR Discord](https://near.chat)
- [NEAR Telegram Developers Community Group](https://t.me/neardev)
- NEAR DevHub: [Telegram](https://t.me/neardevhub), [Twitter](https://twitter.com/neardevhub)
