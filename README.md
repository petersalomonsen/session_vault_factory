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

### Creating and Initializing Session Vault Instances

#### Step 1: Create Instance
Once the factory is deployed and the global contract is registered, you can create session vault instances:

```bash
# Create a new session vault instance
INSTANCE_NAME="vault1"
near contract call-function as-transaction $FACTORY_ACCOUNT create_instance \
  json-args "{\"name\":\"$INSTANCE_NAME\"}" \
  prepaid-gas '100.0 Tgas' attached-deposit '3 NEAR' \
  sign-as $FACTORY_ACCOUNT network-config testnet sign-with-keychain send
```

This will create a new session vault at: `$INSTANCE_NAME.$FACTORY_ACCOUNT`

#### Step 2: Initialize the Instance
The session vault instance must be initialized after creation:

```bash
# Initialize the vault instance
VAULT_INSTANCE="$INSTANCE_NAME.$FACTORY_ACCOUNT"
TOKEN_ID="your-token.testnet"  # The FT token this vault will manage

near contract call-function as-transaction $VAULT_INSTANCE new \
  json-args "{\"owner_id\":\"$FACTORY_ACCOUNT\",\"token_id\":\"$TOKEN_ID\"}" \
  prepaid-gas '30.0 Tgas' attached-deposit '0 NEAR' \
  sign-as $FACTORY_ACCOUNT network-config testnet sign-with-keychain send
```

**Note**: The `new` method does not accept any deposit. The initialization parameters are:
- `owner_id`: The account that will manage the vault (typically the factory or your admin account)
- `token_id`: The fungible token contract ID that this vault will handle

#### Step 3: Verify the Instance
Check that the instance is properly initialized:

```bash
# Get vault metadata
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
