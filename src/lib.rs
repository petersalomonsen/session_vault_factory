use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::Base64VecU8;
use near_sdk::{
    env, log, near, store::IterableMap, AccountId, BorshStorageKey, Gas, NearToken, Promise,
};

// Hardcoded hash of the session_vault contract for security
const SESSION_VAULT_CODE_HASH: &str =
    "f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18";

#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    Instances,
}

#[near(contract_state)]
pub struct Contract {
    owner_id: AccountId,
    instances: IterableMap<String, AccountId>,
    global_contract_deployed: bool,
    global_deployer_account: Option<AccountId>,
}

impl Default for Contract {
    fn default() -> Self {
        env::panic_str("Contract must be initialized with new()")
    }
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            instances: IterableMap::new(StorageKey::Instances),
            global_contract_deployed: false,
            global_deployer_account: None,
        }
    }

    /// Deploy the session_vault contract as a global contract
    /// This should be called once to deploy the contract code globally
    #[payable]
    pub fn deploy_global_contract(
        &mut self,
        code: Base64VecU8,
        deployer_account_id: AccountId,
    ) -> Promise {
        if self.global_contract_deployed {
            env::panic_str("Global contract already deployed");
        }

        // Verify the code hash matches our expected hash
        let code_bytes: Vec<u8> = code.into();
        let code_hash_vec = env::sha256(&code_bytes);
        let code_hash_hex = hex::encode(&code_hash_vec);

        if code_hash_hex != SESSION_VAULT_CODE_HASH {
            env::panic_str(&format!(
                "Invalid contract code. Expected hash: {}, got: {}",
                SESSION_VAULT_CODE_HASH, code_hash_hex
            ));
        }

        self.global_contract_deployed = true;
        self.global_deployer_account = Some(deployer_account_id.clone());

        log!(
            "Deploying session_vault as global contract to: {}",
            deployer_account_id
        );

        // Deploy as global contract using the new SDK method
        Promise::new(deployer_account_id)
            .create_account()
            .transfer(env::attached_deposit())
            .add_full_access_key(env::signer_account_pk())
            .deploy_global_contract(code_bytes)
    }

    #[payable]
    pub fn create_instance(
        &mut self,
        name: String,
        owner_id: AccountId,
        token_id: AccountId,
    ) -> Promise {
        // Check if global contract has been deployed
        if !self.global_contract_deployed {
            env::panic_str("Global contract must be deployed first. Call deploy_global_contract()");
        }

        let attached_deposit = env::attached_deposit();
        // Validate the name
        if name.is_empty() || name.contains('.') {
            env::panic_str("Invalid instance name. Name must not be empty or contain dots.");
        }

        // Create the sub-account ID
        let factory_account = env::current_account_id();
        let instance_account_id: AccountId = format!("{}.{}", name, factory_account)
            .parse()
            .unwrap_or_else(|_| env::panic_str("Invalid account ID"));

        // Check if instance already exists
        if self.instances.get(&name).is_some() {
            env::panic_str(&format!("Instance '{}' already exists", name));
        }

        log!(
            "Creating and initializing session vault instance: {}",
            instance_account_id
        );

        // Store the instance
        self.instances.insert(name, instance_account_id.clone());

        // Create the sub-account and deploy contract using global hash
        // Using the new use_global_contract method from near-sdk PR #1369
        let code_hash_bytes = hex::decode(SESSION_VAULT_CODE_HASH)
            .unwrap_or_else(|_| env::panic_str("Invalid code hash hex"));

        // Split the deposit: most for account creation, some for initialization gas
        let account_creation_deposit =
            attached_deposit.saturating_sub(NearToken::from_millinear(50));

        Promise::new(instance_account_id.clone())
            .create_account()
            .transfer(account_creation_deposit)
            .use_global_contract(code_hash_bytes)
            .then(
                Promise::new(instance_account_id).function_call(
                    "new".to_string(),
                    near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                        "owner_id": owner_id,
                        "token_id": token_id
                    }))
                    .unwrap(),
                    NearToken::from_yoctonear(0),
                    Gas::from_tgas(30),
                ),
            )
    }

    // View methods
    pub fn get_instance(&self, name: String) -> Option<AccountId> {
        self.instances.get(&name).cloned()
    }

    pub fn get_instances(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(String, AccountId)> {
        let from = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(50).min(100);

        self.instances
            .iter()
            .skip(from as usize)
            .take(limit as usize)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn get_total_instances(&self) -> u64 {
        self.instances.len() as u64
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    pub fn get_code_hash(&self) -> String {
        SESSION_VAULT_CODE_HASH.to_string()
    }

    pub const fn is_global_contract_deployed(&self) -> bool {
        self.global_contract_deployed
    }

    pub fn get_global_deployer_account(&self) -> Option<AccountId> {
        self.global_deployer_account.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, NearToken, VMContext};

    fn get_context(is_view: bool) -> VMContext {
        VMContextBuilder::new()
            .current_account_id(accounts(0))
            .signer_account_id(accounts(1))
            .predecessor_account_id(accounts(1))
            .is_view(is_view)
            .build()
    }

    #[test]
    fn test_new() {
        let context = get_context(false);
        testing_env!(context);

        let contract = Contract::new(accounts(1));

        assert_eq!(contract.get_owner(), accounts(1));
        assert_eq!(contract.get_total_instances(), 0);
    }

    #[test]
    #[should_panic(expected = "Global contract must be deployed first")]
    fn test_create_instance_without_global_contract() {
        let mut context = get_context(false);
        context.attached_deposit = NearToken::from_near(1);
        testing_env!(context);

        let mut contract = Contract::new(accounts(1));

        // Try to create instance without deploying global contract first
        contract.create_instance("instance1".to_string(), accounts(1), accounts(2));
    }

    #[test]
    fn test_global_contract_deployment_tracking() {
        let context = get_context(false);
        testing_env!(context);

        let mut contract = Contract::new(accounts(1));

        // Initially no global contract should be deployed
        assert!(!contract.is_global_contract_deployed());
        assert!(contract.get_global_deployer_account().is_none());

        // After setting it (in real scenario this would be after deploy_global_contract)
        contract.global_contract_deployed = true;
        contract.global_deployer_account = Some(accounts(2));

        assert!(contract.is_global_contract_deployed());
        assert_eq!(contract.get_global_deployer_account(), Some(accounts(2)));
    }
}
