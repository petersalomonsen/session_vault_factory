use near_sdk::{
    env, log, near, store::IterableMap, AccountId, Promise,
    BorshStorageKey,
};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

// Hardcoded hash of the session_vault contract for security
const SESSION_VAULT_CODE_HASH: &str = "f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18";

#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    Instances,
}

#[near(contract_state)]
pub struct Contract {
    owner_id: AccountId,
    instances: IterableMap<String, AccountId>,
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
        }
    }

    #[payable]
    pub fn create_instance(
        &mut self,
        name: String,
    ) -> Promise {
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
            "Creating session vault instance: {}",
            instance_account_id
        );

        // Store the instance
        self.instances.insert(name.clone(), instance_account_id.clone());

        // Create the sub-account and deploy contract
        // NOTE: Global contract deployment using hash references is available in near-api
        // but not yet in near-sdk. The pattern would be:
        // Contract::deploy(instance_account_id)
        //     .use_global_hash(SESSION_VAULT_CODE_HASH)
        //     .without_init_call()
        // 
        // For now, we create the sub-account. When near-sdk supports global contracts,
        // we'll deploy using the hardcoded hash reference:
        Promise::new(instance_account_id.clone())
            .create_account()
            .transfer(attached_deposit)
            // TODO: Add .deploy_from_hash(SESSION_VAULT_CODE_HASH) when near-sdk supports it ( https://github.com/near/near-sdk-rs/pull/1369 )
    }


    // View methods
    pub fn get_instance(&self, name: String) -> Option<AccountId> {
        self.instances.get(&name).cloned()
    }

    pub fn get_instances(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<(String, AccountId)> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, VMContext, NearToken};

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
    #[should_panic(expected = "Invalid instance name")]
    fn test_create_instance_invalid_name() {
        let mut context = get_context(false);
        context.attached_deposit = NearToken::from_near(1);
        testing_env!(context);
        
        let mut contract = Contract::new(accounts(1));
        
        contract.create_instance("invalid.name".to_string());
    }
}