use crate::types::ExecutionResult;
use chrono::{DateTime, Utc};
use ethereum_types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub number: u64,
    pub parent_hash: H256,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub timestamp: DateTime<Utc>,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub proposer: Address,
    pub difficulty: U256, // Used for PoS slot assignment
    pub extra_data: Vec<u8>,
    pub base_fee: U256,
    pub abby_reward: U256, // Reward in Abby tokens
}

impl BlockHeader {
    pub fn hash(&self) -> H256 {
        let serialized = serde_json::to_string(self).unwrap();
        let hash = Keccak256::digest(serialized.as_bytes());
        H256::from_slice(&hash)
    }
    
    pub fn new(
        number: u64,
        parent_hash: H256,
        proposer: Address,
        gas_limit: U256,
    ) -> Self {
        Self {
            number,
            parent_hash,
            state_root: H256::zero(),
            transactions_root: H256::zero(),
            receipts_root: H256::zero(),
            timestamp: Utc::now(),
            gas_limit,
            gas_used: U256::zero(),
            proposer,
            difficulty: U256::zero(),
            extra_data: Vec::new(),
            base_fee: U256::from(1_000_000_000u64), // 1 Gwei
            abby_reward: U256::from(1_000_000_000_000_000_000u64), // 1 Abby token
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<crate::blockchain::Transaction>,
    pub validators: Vec<ValidatorInfo>, // Validators that participated in consensus
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<crate::blockchain::Transaction>) -> Self {
        Self {
            header,
            transactions,
            validators: Vec::new(),
        }
    }
    
    pub fn hash(&self) -> H256 {
        self.header.hash()
    }
    
    pub fn calculate_merkle_root(transactions: &[crate::blockchain::Transaction]) -> H256 {
        if transactions.is_empty() {
            return H256::zero();
        }
        
        let mut hashes: Vec<H256> = transactions.iter().map(|tx| tx.hash()).collect();
        
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    [chunk[0].as_bytes(), chunk[1].as_bytes()].concat()
                } else {
                    [chunk[0].as_bytes(), chunk[0].as_bytes()].concat()
                };
                
                let hash = Keccak256::digest(&combined);
                next_level.push(H256::from_slice(&hash));
            }
            
            hashes = next_level;
        }
        
        hashes[0]
    }
    
    pub fn genesis() -> Self {
        let mut header = BlockHeader::new(0, H256::zero(), Address::zero(), U256::from(10_000_000u64));
        header.timestamp = DateTime::from_timestamp(1640995200, 0).unwrap_or_else(Utc::now); // Jan 1, 2022
        header.abby_reward = U256::from(10_000_000_000_000_000_000u64); // 10 Abby tokens for genesis
        
        Self::new(header, Vec::new())
    }
    
    pub fn validate(&self) -> Result<(), String> {
        // Basic validation
        if self.transactions.len() as u64 > 1000 {
            return Err("Too many transactions in block".to_string());
        }
        
        // Validate transaction root
        let calculated_root = Self::calculate_merkle_root(&self.transactions);
        if self.header.transactions_root != calculated_root && !self.transactions.is_empty() {
            return Err("Invalid transaction root".to_string());
        }
        
        // Validate gas usage
        let total_gas: U256 = self.transactions.iter()
            .map(|tx| tx.gas_limit)
            .fold(U256::zero(), |acc, gas| acc + gas);
            
        if total_gas > self.header.gas_limit {
            return Err("Block gas limit exceeded".to_string());
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub address: Address,
    pub stake: U256,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub transaction_hash: H256,
    pub transaction_index: u64,
    pub block_hash: H256,
    pub block_number: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub cumulative_gas_used: U256,
    pub gas_used: U256,
    pub contract_address: Option<Address>,
    pub logs: Vec<crate::types::Log>,
    pub status: bool, // true for success, false for failure
    pub abby_rewards: U256, // Abby tokens earned from this transaction
}

impl TransactionReceipt {
    pub fn new(
        tx: &crate::blockchain::Transaction,
        result: &ExecutionResult,
        block_hash: H256,
        block_number: u64,
        tx_index: u64,
        cumulative_gas: U256,
    ) -> Self {
        let abby_rewards = Self::calculate_abby_rewards(result.gas_used);
        
        Self {
            transaction_hash: tx.hash(),
            transaction_index: tx_index,
            block_hash,
            block_number,
            from: tx.from,
            to: tx.to,
            cumulative_gas_used: cumulative_gas,
            gas_used: result.gas_used,
            contract_address: None, // TODO: Calculate for contract deployments
            logs: result.logs.clone(),
            status: matches!(result.status, crate::types::ExecutionStatus::Success),
            abby_rewards,
        }
    }
    
    fn calculate_abby_rewards(gas_used: U256) -> U256 {
        // 1 Abby token for every 1000 gas used
        gas_used / U256::from(1000)
    }
}
