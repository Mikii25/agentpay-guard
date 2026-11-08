use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId, Balance, NearToken, PanicOnDefault};
use std::collections::HashMap;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SpendPolicy {
    pub max_per_tx: Balance,
    pub daily_limit: Balance,
    pub spent_today: Balance,
    pub last_reset_timestamp: u64,
    pub is_active: bool,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct AgentPayGuard {
    pub owner_id: AccountId,
    pub agents: HashMap<AccountId, SpendPolicy>,
    pub whitelist: HashMap<AccountId, Vec<AccountId>>,
}

#[near_bindgen]
impl AgentPayGuard {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            agents: HashMap::new(),
            whitelist: HashMap::new(),
        }
    }

    pub fn register_agent(&mut self, agent_id: AccountId, max_per_tx: Balance, daily_limit: Balance) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Unauthorized");
        let policy = SpendPolicy {
            max_per_tx,
            daily_limit,
            spent_today: 0,
            last_reset_timestamp: env::block_timestamp(),
            is_active: true,
        };
        self.agents.insert(agent_id, policy);
    }

    pub fn execute_agent_payment(&mut self, recipient: AccountId, amount: Balance) {
        let agent_id = env::predecessor_account_id();
        let mut policy = self.agents.get(&agent_id).expect("Agent not registered").clone();

        assert!(policy.is_active, "Agent execution paused by Emergency Kill-Switch");
        assert!(amount <= policy.max_per_tx, "Exceeds maximum per-transaction limit!");

        let current_time = env::block_timestamp();
        if current_time - policy.last_reset_timestamp >= 86_400_000_000_000 {
            policy.spent_today = 0;
            policy.last_reset_timestamp = current_time;
        }

        assert!(policy.spent_today + amount <= policy.daily_limit, "Exceeds daily spend limit!");

        policy.spent_today += amount;
        self.agents.insert(agent_id, policy);

        near_sdk::Promise::new(recipient).transfer(NearToken::from_yoctonear(amount));
    }

    pub fn toggle_agent_status(&mut self, agent_id: AccountId, status: bool) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Unauthorized");
        if let Some(policy) = self.agents.get_mut(&agent_id) {
            policy.is_active = status;
        }
    }
}
