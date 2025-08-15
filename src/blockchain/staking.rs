use crate::blockchain::{ConsensusState, Validator};
use ethereum_types::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeInfo {
    pub staker: Address,
    pub validator: Address,
    pub amount: U256,
    pub rewards_earned: U256,
    pub delegation_time: u64,
    pub withdrawal_time: Option<u64>,
}

impl StakeInfo {
    pub fn new(staker: Address, validator: Address, amount: U256) -> Self {
        Self {
            staker,
            validator,
            amount,
            rewards_earned: U256::zero(),
            delegation_time: chrono::Utc::now().timestamp() as u64,
            withdrawal_time: None,
        }
    }

    pub fn calculate_rewards(&self, annual_rate: u32, time_staked: u64) -> U256 {
        // Calculate rewards based on annual percentage rate
        // annual_rate is in basis points (e.g., 800 = 8%)
        let annual_reward = self.amount * U256::from(annual_rate) / U256::from(10000);
        let seconds_per_year = 365 * 24 * 3600;

        annual_reward * U256::from(time_staked) / U256::from(seconds_per_year)
    }
}

#[derive(Debug, Clone)]
pub struct StakingManager {
    pub stakes: HashMap<Address, Vec<StakeInfo>>, // staker -> stakes
    pub validator_delegations: HashMap<Address, Vec<StakeInfo>>, // validator -> delegated stakes
    pub total_staked: U256,
    pub abby_token_supply: U256,
    pub annual_reward_rate: u32, // basis points (e.g., 800 = 8%)
    pub withdrawal_delay: u64,   // seconds before withdrawal is available
}

impl StakingManager {
    pub fn new() -> Self {
        Self {
            stakes: HashMap::new(),
            validator_delegations: HashMap::new(),
            total_staked: U256::zero(),
            abby_token_supply: U256::from_dec_str("1000000000000000000000000").unwrap(), // 1 million Abby tokens
            annual_reward_rate: 800,         // 8% annual reward
            withdrawal_delay: 7 * 24 * 3600, // 7 days
        }
    }

    pub fn stake(
        &mut self,
        staker: Address,
        validator: Address,
        amount: U256,
        consensus: &mut ConsensusState,
    ) -> Result<(), String> {
        if amount < U256::from(1_000_000_000_000_000_000u64) {
            return Err("Minimum stake is 1 Abby token".to_string());
        }

        // Check if validator exists
        if !consensus.validators.contains_key(&validator) {
            return Err("Validator does not exist".to_string());
        }

        let stake_info = StakeInfo::new(staker, validator, amount);

        // Add to staker's stakes
        self.stakes
            .entry(staker)
            .or_default()
            .push(stake_info.clone());

        // Add to validator's delegations
        self.validator_delegations
            .entry(validator)
            .or_default()
            .push(stake_info);

        // Update validator's total stake
        if let Some(val) = consensus.validators.get_mut(&validator) {
            val.add_stake(amount);
        }

        self.total_staked += amount;

        log::info!(
            "Staked {} Abby tokens from {} to validator {}",
            self.format_abby_amount(amount),
            staker,
            validator
        );

        Ok(())
    }

    pub fn unstake(
        &mut self,
        staker: Address,
        validator: Address,
        amount: U256,
        consensus: &mut ConsensusState,
    ) -> Result<(), String> {
        let staker_stakes = self
            .stakes
            .get_mut(&staker)
            .ok_or("No stakes found for staker")?;

        let mut remaining_amount = amount;
        let mut stakes_to_remove = Vec::new();

        for (i, stake) in staker_stakes.iter_mut().enumerate() {
            if stake.validator == validator && remaining_amount > U256::zero() {
                if stake.amount <= remaining_amount {
                    remaining_amount -= stake.amount;
                    stake.withdrawal_time = Some(chrono::Utc::now().timestamp() as u64);
                    stakes_to_remove.push(i);
                } else {
                    stake.amount -= remaining_amount;
                    remaining_amount = U256::zero();
                }
            }
        }

        if remaining_amount > U256::zero() {
            return Err("Insufficient staked amount".to_string());
        }

        // Remove fully unstaked stakes
        for &i in stakes_to_remove.iter().rev() {
            staker_stakes.remove(i);
        }

        // Update validator's stake
        if let Some(val) = consensus.validators.get_mut(&validator) {
            val.remove_stake(amount)?;
        }

        self.total_staked = self.total_staked.saturating_sub(amount);

        log::info!(
            "Unstaked {} Abby tokens from validator {} by {}",
            self.format_abby_amount(amount),
            validator,
            staker
        );

        Ok(())
    }

    pub fn withdraw(&mut self, staker: Address, validator: Address) -> Result<U256, String> {
        let current_time = chrono::Utc::now().timestamp() as u64;
        let staker_stakes = self
            .stakes
            .get_mut(&staker)
            .ok_or("No stakes found for staker")?;

        let mut withdrawn_amount = U256::zero();
        let mut stakes_to_remove = Vec::new();

        for (i, stake) in staker_stakes.iter().enumerate() {
            if stake.validator == validator {
                if let Some(withdrawal_time) = stake.withdrawal_time {
                    if current_time >= withdrawal_time + self.withdrawal_delay {
                        withdrawn_amount += stake.amount;
                        stakes_to_remove.push(i);
                    }
                }
            }
        }

        // Remove withdrawn stakes
        for &i in stakes_to_remove.iter().rev() {
            staker_stakes.remove(i);
        }

        if withdrawn_amount == U256::zero() {
            return Err("No funds available for withdrawal yet".to_string());
        }

        log::info!(
            "Withdrew {} Abby tokens for {} from validator {}",
            self.format_abby_amount(withdrawn_amount),
            staker,
            validator
        );

        Ok(withdrawn_amount)
    }

    pub fn claim_rewards(&mut self, staker: Address, validator: Address) -> Result<U256, String> {
        let current_time = chrono::Utc::now().timestamp() as u64;
        let staker_stakes = self
            .stakes
            .get_mut(&staker)
            .ok_or("No stakes found for staker")?;

        let mut total_rewards = U256::zero();

        for stake in staker_stakes.iter_mut() {
            if stake.validator == validator {
                let time_staked = current_time.saturating_sub(stake.delegation_time);
                let pending_rewards = stake.calculate_rewards(self.annual_reward_rate, time_staked);
                let new_rewards = pending_rewards.saturating_sub(stake.rewards_earned);

                total_rewards += new_rewards;
                stake.rewards_earned = pending_rewards;
                stake.delegation_time = current_time; // Reset for next calculation
            }
        }

        if total_rewards > U256::zero() {
            self.mint_abby_tokens(total_rewards)?;
            log::info!(
                "Claimed {} Abby token rewards for {} from validator {}",
                self.format_abby_amount(total_rewards),
                staker,
                validator
            );
        }

        Ok(total_rewards)
    }

    pub fn get_staker_info(&self, staker: &Address) -> Vec<&StakeInfo> {
        self.stakes
            .get(staker)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_validator_delegations(&self, validator: &Address) -> Vec<&StakeInfo> {
        self.validator_delegations
            .get(validator)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_total_staked_to_validator(&self, validator: &Address) -> U256 {
        self.validator_delegations
            .get(validator)
            .map(|stakes| {
                stakes
                    .iter()
                    .map(|s| s.amount)
                    .fold(U256::zero(), |acc, x| acc + x)
            })
            .unwrap_or(U256::zero())
    }

    pub fn create_validator(
        &mut self,
        validator_address: Address,
        initial_stake: U256,
        consensus: &mut ConsensusState,
    ) -> Result<(), String> {
        if initial_stake < Validator::minimum_stake() {
            return Err(format!(
                "Minimum stake for validator is {} Abby tokens",
                self.format_abby_amount(Validator::minimum_stake())
            ));
        }

        let validator = Validator::new(validator_address, initial_stake);
        consensus.add_validator(validator)?;

        // Self-stake
        self.stake(
            validator_address,
            validator_address,
            initial_stake,
            consensus,
        )?;

        log::info!(
            "Created validator {} with {} Abby tokens stake",
            validator_address,
            self.format_abby_amount(initial_stake)
        );

        Ok(())
    }

    fn mint_abby_tokens(&mut self, amount: U256) -> Result<(), String> {
        // Simple inflation model - for a real implementation,
        // this would need proper monetary policy
        self.abby_token_supply += amount;
        Ok(())
    }

    fn format_abby_amount(&self, amount: U256) -> String {
        let decimals = U256::from(1_000_000_000_000_000_000u64); // 18 decimals
        let whole = amount / decimals;
        let fractional = (amount % decimals) / U256::from(1_000_000_000_000u64); // Show 6 decimal places

        format!("{}.{:06}", whole, fractional.as_u64())
    }

    pub fn get_staking_apy(&self) -> f64 {
        self.annual_reward_rate as f64 / 100.0
    }

    pub fn get_total_rewards_distributed(&self) -> U256 {
        self.stakes
            .values()
            .flatten()
            .map(|stake| stake.rewards_earned)
            .fold(U256::zero(), |acc, x| acc + x)
    }
}

impl Default for StakingManager {
    fn default() -> Self {
        Self::new()
    }
}
