use crate::blockchain::{Block, TransactionReceipt};
use crate::types::{Account, ExecutionResult};
use ethereum_types::{Address, H256, U256};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Blockchain {
    pub blocks: HashMap<H256, Block>,
    pub block_by_number: HashMap<u64, H256>,
    pub receipts: HashMap<H256, TransactionReceipt>,
    pub accounts: HashMap<Address, Account>,
    pub head_hash: H256,
    pub head_number: u64,
    pub total_difficulty: U256,
    pub abby_balances: HashMap<Address, U256>, // Abby token balances
    pub db: Option<sled::Db>,
}

impl Blockchain {
    pub fn new() -> Result<Self, String> {
        let genesis = Block::genesis();
        let genesis_hash = genesis.hash();
        
        let mut blockchain = Self {
            blocks: HashMap::new(),
            block_by_number: HashMap::new(),
            receipts: HashMap::new(),
            accounts: HashMap::new(),
            head_hash: genesis_hash,
            head_number: 0,
            total_difficulty: U256::zero(),
            abby_balances: HashMap::new(),
            db: None,
        };
        
        blockchain.blocks.insert(genesis_hash, genesis);
        blockchain.block_by_number.insert(0, genesis_hash);
        
        // Initialize genesis Abby token distribution
        blockchain.initialize_abby_genesis();
        
        Ok(blockchain)
    }
    
    pub fn new_with_persistence(db_path: &str) -> Result<Self, String> {
        let db = sled::open(db_path).map_err(|e| format!("Failed to open database: {}", e))?;
        
        let mut blockchain = Self::new()?;
        blockchain.db = Some(db);
        blockchain.load_from_disk()?;
        
        Ok(blockchain)
    }
    
    fn initialize_abby_genesis(&mut self) {
        // Distribute initial Abby tokens to genesis addresses
        let genesis_distribution = vec![
            // Example addresses with initial Abby token allocations
            (Address::from_low_u64_be(1), U256::from_dec_str("100000000000000000000000").unwrap()), // 100k tokens
            (Address::from_low_u64_be(2), U256::from_dec_str("50000000000000000000000").unwrap()),  // 50k tokens
            (Address::from_low_u64_be(3), U256::from_dec_str("25000000000000000000000").unwrap()),  // 25k tokens
        ];
        
        for (address, balance) in genesis_distribution {
            self.abby_balances.insert(address, balance);
        }
        
        log::info!("Initialized Abby token genesis distribution");
    }
    
    pub fn add_block(&mut self, block: Block) -> Result<(), String> {
        // Validate block
        block.validate()?;
        
        // Check if parent exists
        if block.header.number > 0
            && !self.blocks.contains_key(&block.header.parent_hash) {
                return Err("Parent block not found".to_string());
            }
        
        // Check if block already exists
        let block_hash = block.hash();
        if self.blocks.contains_key(&block_hash) {
            return Err("Block already exists".to_string());
        }
        
        // Process block transactions and update state
        self.process_block(&block)?;
        
        // Add block to chain
        self.blocks.insert(block_hash, block.clone());
        self.block_by_number.insert(block.header.number, block_hash);
        
        // Update head if this block extends the chain
        if block.header.number > self.head_number {
            self.head_hash = block_hash;
            self.head_number = block.header.number;
            self.total_difficulty += block.header.difficulty;
        }
        
        // Persist to disk if enabled
        self.persist_block(&block)?;
        
        log::info!("Added block #{} with hash {}", block.header.number, block_hash);
        
        Ok(())
    }
    
    fn process_block(&mut self, block: &Block) -> Result<(), String> {
        let mut cumulative_gas = U256::zero();
        
        // Process each transaction in the block
        for (tx_index, tx) in block.transactions.iter().enumerate() {
            // Execute transaction on EVM
            let result = self.execute_transaction(tx)?;
            cumulative_gas += result.gas_used;
            
            // Create receipt
            let receipt = TransactionReceipt::new(
                tx,
                &result,
                block.hash(),
                block.header.number,
                tx_index as u64,
                cumulative_gas,
            );
            
            // Store receipt
            self.receipts.insert(tx.hash(), receipt.clone());
            
            // Update Abby token balances based on transaction fees and rewards
            self.update_abby_balances(tx, &receipt);
        }
        
        // Distribute block rewards to proposer
        self.distribute_block_reward(&block.header.proposer, block.header.abby_reward);
        
        Ok(())
    }
    
    fn execute_transaction(&mut self, tx: &crate::blockchain::Transaction) -> Result<ExecutionResult, String> {
        // Convert blockchain transaction to EVM transaction
        let evm_tx = crate::types::Transaction {
            from: tx.from,
            to: tx.to,
            value: tx.value,
            gas: tx.gas_limit,
            gas_price: tx.gas_price,
            data: tx.data.clone(),
        };
        
        // Create EVM executor
        let mut executor = crate::evm::EvmExecutor::new(1_000_000); // 1M gas limit
        
        // Execute transaction
        executor.execute_transaction(&evm_tx, &mut self.accounts)
    }
    
    fn update_abby_balances(&mut self, tx: &crate::blockchain::Transaction, receipt: &TransactionReceipt) {
        // Deduct Abby fee from sender
        let sender_balance = self.abby_balances.entry(tx.from).or_insert(U256::zero());
        *sender_balance = sender_balance.saturating_sub(tx.abby_fee);
        
        // Add Abby rewards to recipient (if transaction was successful)
        if receipt.status && receipt.abby_rewards > U256::zero() {
            if let Some(to) = tx.to {
                let recipient_balance = self.abby_balances.entry(to).or_insert(U256::zero());
                *recipient_balance += receipt.abby_rewards;
            }
        }
    }
    
    fn distribute_block_reward(&mut self, proposer: &Address, reward: U256) {
        let proposer_balance = self.abby_balances.entry(*proposer).or_insert(U256::zero());
        *proposer_balance += reward;
        
        log::info!("Distributed {} Abby tokens block reward to {}", 
                  self.format_abby_amount(reward), proposer);
    }
    
    pub fn get_block(&self, hash: &H256) -> Option<&Block> {
        self.blocks.get(hash)
    }
    
    pub fn get_block_by_number(&self, number: u64) -> Option<&Block> {
        self.block_by_number.get(&number)
            .and_then(|hash| self.blocks.get(hash))
    }
    
    pub fn get_receipt(&self, tx_hash: &H256) -> Option<&TransactionReceipt> {
        self.receipts.get(tx_hash)
    }
    
    pub fn get_account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }
    
    pub fn get_abby_balance(&self, address: &Address) -> U256 {
        self.abby_balances.get(address).copied().unwrap_or(U256::zero())
    }
    
    pub fn transfer_abby(&mut self, from: &Address, to: &Address, amount: U256) -> Result<(), String> {
        let from_balance = self.abby_balances.get(from).copied().unwrap_or(U256::zero());
        if from_balance < amount {
            return Err("Insufficient Abby token balance".to_string());
        }
        
        let from_balance_mut = self.abby_balances.entry(*from).or_insert(U256::zero());
        *from_balance_mut -= amount;
        
        let to_balance_mut = self.abby_balances.entry(*to).or_insert(U256::zero());
        *to_balance_mut += amount;
        
        Ok(())
    }
    
    pub fn get_head_block(&self) -> Option<&Block> {
        self.blocks.get(&self.head_hash)
    }
    
    pub fn get_chain_length(&self) -> u64 {
        self.head_number + 1
    }
    
    pub fn get_total_abby_supply(&self) -> U256 {
        self.abby_balances.values().fold(U256::zero(), |acc, balance| acc + balance)
    }
    
    fn persist_block(&self, block: &Block) -> Result<(), String> {
        if let Some(ref db) = self.db {
            let serialized = serde_json::to_vec(block)
                .map_err(|e| format!("Failed to serialize block: {}", e))?;
            
            db.insert(format!("block_{}", block.hash()), serialized)
                .map_err(|e| format!("Failed to persist block: {}", e))?;
                
            db.flush()
                .map_err(|e| format!("Failed to flush database: {}", e))?;
        }
        Ok(())
    }
    
    fn load_from_disk(&mut self) -> Result<(), String> {
        if let Some(ref db) = self.db {
            for result in db.scan_prefix("block_") {
                let (key, value) = result.map_err(|e| format!("Database scan error: {}", e))?;
                
                let block: Block = serde_json::from_slice(&value)
                    .map_err(|e| format!("Failed to deserialize block: {}", e))?;
                
                let block_hash = block.hash();
                self.blocks.insert(block_hash, block.clone());
                self.block_by_number.insert(block.header.number, block_hash);
                
                if block.header.number > self.head_number {
                    self.head_hash = block_hash;
                    self.head_number = block.header.number;
                }
            }
        }
        Ok(())
    }
    
    pub fn validate_chain(&self) -> Result<(), String> {
        let mut current_number = 0u64;
        let mut current_hash = self.block_by_number.get(&0)
            .copied()
            .ok_or("Genesis block not found")?;
        
        while current_number <= self.head_number {
            let block = self.blocks.get(&current_hash)
                .ok_or(format!("Block {} not found", current_hash))?;
                
            // Validate block
            block.validate()?;
            
            // Check parent hash (except for genesis)
            if current_number > 0 {
                let expected_parent = self.block_by_number.get(&(current_number - 1))
                    .copied()
                    .ok_or(format!("Parent block {} not found", current_number - 1))?;
                    
                if block.header.parent_hash != expected_parent {
                    return Err(format!("Invalid parent hash for block {}", current_number));
                }
            }
            
            // Move to next block
            current_number += 1;
            if current_number <= self.head_number {
                current_hash = self.block_by_number.get(&current_number)
                    .copied()
                    .ok_or(format!("Block {} not found", current_number))?;
            }
        }
        
        log::info!("Blockchain validation completed successfully");
        Ok(())
    }
    
    pub fn get_abby_rich_list(&self, limit: usize) -> Vec<(Address, U256)> {
        let mut balances: Vec<_> = self.abby_balances.iter()
            .map(|(&addr, &balance)| (addr, balance))
            .collect();
        balances.sort_by(|a, b| b.1.cmp(&a.1));
        balances.into_iter().take(limit).collect()
    }
    
    fn format_abby_amount(&self, amount: U256) -> String {
        let decimals = U256::from(1_000_000_000_000_000_000u64); // 18 decimals
        let whole = amount / decimals;
        let fractional = (amount % decimals) / U256::from(1_000_000_000_000u64); // Show 6 decimal places
        
        format!("{}.{:06}", whole, fractional.as_u64())
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new().expect("Failed to create default blockchain")
    }
}
