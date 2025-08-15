use crate::blockchain::Block;
use ethereum_types::{Address, U256};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub address: Address,
    pub stake: U256,
    pub is_active: bool,
    pub last_activity: u64,
    pub abby_tokens: U256,
    pub commission_rate: u32, // Percentage (0-10000, where 10000 = 100%)
}

impl Validator {
    pub fn new(address: Address, initial_stake: U256) -> Self {
        Self {
            address,
            stake: initial_stake,
            is_active: true,
            last_activity: 0,
            abby_tokens: U256::zero(),
            commission_rate: 1000, // 10% default commission
        }
    }

    pub fn add_stake(&mut self, amount: U256) {
        self.stake += amount;
        self.is_active = self.stake >= Self::minimum_stake();
    }

    pub fn remove_stake(&mut self, amount: U256) -> Result<(), String> {
        if amount > self.stake {
            return Err("Insufficient stake".to_string());
        }
        self.stake -= amount;
        self.is_active = self.stake >= Self::minimum_stake();
        Ok(())
    }

    pub fn minimum_stake() -> U256 {
        U256::from_dec_str("32000000000000000000").unwrap() // 32 Abby tokens
    }

    pub fn reward(&mut self, amount: U256) {
        self.abby_tokens += amount;
    }

    pub fn slash(&mut self, percentage: u32) -> U256 {
        let slash_amount = self.stake * U256::from(percentage) / U256::from(10000);
        self.stake = self.stake.saturating_sub(slash_amount);
        self.is_active = self.stake >= Self::minimum_stake();
        slash_amount
    }
}

#[derive(Debug, Clone)]
pub struct ConsensusState {
    pub validators: HashMap<Address, Validator>,
    pub current_epoch: u64,
    pub current_slot: u64,
    pub slots_per_epoch: u64,
    pub block_time: u64, // seconds
    pub total_stake: U256,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            current_epoch: 0,
            current_slot: 0,
            slots_per_epoch: 32,
            block_time: 12, // 12 seconds per block (like Ethereum 2.0)
            total_stake: U256::zero(),
        }
    }

    pub fn add_validator(&mut self, validator: Validator) -> Result<(), String> {
        if validator.stake < Validator::minimum_stake() {
            return Err("Insufficient stake to become validator".to_string());
        }

        self.total_stake += validator.stake;
        self.validators.insert(validator.address, validator);
        Ok(())
    }

    pub fn remove_validator(&mut self, address: &Address) -> Result<(), String> {
        if let Some(validator) = self.validators.remove(address) {
            self.total_stake = self.total_stake.saturating_sub(validator.stake);
            Ok(())
        } else {
            Err("Validator not found".to_string())
        }
    }

    pub fn select_proposer(&self, slot: u64, randomness: &[u8]) -> Option<Address> {
        let active_validators: Vec<&Validator> = self
            .validators
            .values()
            .filter(|v| v.is_active && v.stake >= Validator::minimum_stake())
            .collect();

        if active_validators.is_empty() {
            return None;
        }

        // Use randomness + slot to deterministically select proposer
        let mut hasher = Keccak256::new();
        hasher.update(randomness);
        hasher.update(slot.to_be_bytes());
        let hash = hasher.finalize();

        // Convert hash to seed for RNG
        let seed = u64::from_be_bytes([
            hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
        ]);

        let mut rng = StdRng::seed_from_u64(seed);

        // Use modular arithmetic to handle large stakes
        // We'll work with stake percentages rather than absolute values
        let total_stake_u64 = if self.total_stake > U256::from(u64::MAX) {
            // For very large stakes, use the last 64 bits
            (self.total_stake % U256::from(u64::MAX)).as_u64()
        } else {
            self.total_stake.as_u64()
        };

        let random_stake = rng.gen_range(0..total_stake_u64.max(1));

        let mut cumulative_stake = 0u64;
        for validator in &active_validators {
            let validator_stake = if validator.stake > U256::from(u64::MAX) {
                // For very large stakes, use the last 64 bits
                (validator.stake % U256::from(u64::MAX)).as_u64()
            } else {
                validator.stake.as_u64()
            };

            cumulative_stake += validator_stake;
            if cumulative_stake > random_stake {
                return Some(validator.address);
            }
        }

        // Fallback to first validator if something went wrong
        active_validators.first().map(|v| v.address)
    }

    pub fn validate_proposal(&self, block: &Block, proposer: &Address) -> Result<(), String> {
        let validator = self
            .validators
            .get(proposer)
            .ok_or("Proposer is not a validator")?;

        if !validator.is_active {
            return Err("Proposer is not active".to_string());
        }

        if validator.stake < Validator::minimum_stake() {
            return Err("Proposer has insufficient stake".to_string());
        }

        // Validate block contents
        block.validate()?;

        // Check if proposer is the expected one for this slot
        let expected_proposer =
            self.select_proposer(self.current_slot, &block.hash().as_bytes()[0..8]);

        if expected_proposer != Some(*proposer) {
            return Err("Unexpected proposer for this slot".to_string());
        }

        Ok(())
    }

    pub fn advance_slot(&mut self) {
        self.current_slot += 1;
        if self.current_slot.is_multiple_of(self.slots_per_epoch) {
            self.advance_epoch();
        }
    }

    pub fn advance_epoch(&mut self) {
        self.current_epoch += 1;

        // Distribute rewards at epoch end
        self.distribute_epoch_rewards();

        // Update validator activity
        for validator in self.validators.values_mut() {
            if self.current_epoch - validator.last_activity > 2 {
                // Validator inactive for 2+ epochs, reduce stake slightly
                let penalty = validator.stake / U256::from(1000); // 0.1% penalty
                validator.stake = validator.stake.saturating_sub(penalty);
                validator.is_active = validator.stake >= Validator::minimum_stake();
            }
        }
    }

    fn distribute_epoch_rewards(&mut self) {
        if self.total_stake == U256::zero() {
            return;
        }

        // Total rewards per epoch: 1000 Abby tokens
        let total_rewards = U256::from_dec_str("1000000000000000000000").unwrap(); // 1k rewards

        for validator in self.validators.values_mut() {
            if validator.is_active {
                // Reward proportional to stake
                let validator_reward = total_rewards * validator.stake / self.total_stake;
                validator.reward(validator_reward);
            }
        }
    }

    pub fn get_validator(&self, address: &Address) -> Option<&Validator> {
        self.validators.get(address)
    }

    pub fn get_validator_mut(&mut self, address: &Address) -> Option<&mut Validator> {
        self.validators.get_mut(address)
    }

    pub fn total_active_validators(&self) -> usize {
        self.validators
            .values()
            .filter(|v| v.is_active && v.stake >= Validator::minimum_stake())
            .count()
    }

    pub fn get_top_validators(&self, limit: usize) -> Vec<&Validator> {
        let mut validators: Vec<&Validator> =
            self.validators.values().filter(|v| v.is_active).collect();
        validators.sort_by(|a, b| b.stake.cmp(&a.stake));
        validators.into_iter().take(limit).collect()
    }
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub validator: Address,
    pub block_hash: ethereum_types::H256,
    pub slot: u64,
    pub signature: Vec<u8>,
}

impl Attestation {
    pub fn new(validator: Address, block_hash: ethereum_types::H256, slot: u64) -> Self {
        Self {
            validator,
            block_hash,
            slot,
            signature: Vec::new(), // TODO: Implement proper signature
        }
    }

    pub fn verify(&self, consensus: &ConsensusState) -> bool {
        // Check if validator exists and is active
        if let Some(validator) = consensus.get_validator(&self.validator) {
            validator.is_active && validator.stake >= Validator::minimum_stake()
        } else {
            false
        }
    }
}
