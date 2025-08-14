use near_sdk::{
    env, log, near, store::IterableMap, AccountId, Promise,
    BorshStorageKey,
};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshDeserialize, BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    Instances,
}

#[near(contract_state)]
pub struct Contract {
    owner_id: AccountId,
    instances: IterableMap<String, AccountId>,
    session_vault_code_hash: String,
}

impl Default for Contract {
    fn default() -> Self {
        env::panic_str("Contract must be initialized with new()")
    }
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, session_vault_code_hash: String) -> Self {
        Self {
            owner_id,
            instances: IterableMap::new(StorageKey::Instances),
            session_vault_code_hash,
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

        // For now, we'll create a sub-account without deploying the actual contract
        // In a real implementation, we would deploy the session_vault contract here
        // using the code hash stored in the contract
        Promise::new(instance_account_id.clone())
            .create_account()
            .transfer(attached_deposit)
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
        self.session_vault_code_hash.clone()
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
        
        let contract = Contract::new(
            accounts(1),
            "f0b9a1ef2b68c7f258178e5e82a68374331e5abd3072aafb938adf010818bd18".to_string(),
        );
        
        assert_eq!(contract.get_owner(), accounts(1));
        assert_eq!(contract.get_total_instances(), 0);
    }

    #[test]
    #[should_panic(expected = "Invalid instance name")]
    fn test_create_instance_invalid_name() {
        let mut context = get_context(false);
        context.attached_deposit = NearToken::from_near(1);
        testing_env!(context);
        
        let mut contract = Contract::new(
            accounts(1),
            "test_hash".to_string(),
        );
        
        contract.create_instance("invalid.name".to_string());
    }
}