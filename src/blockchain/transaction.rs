use crate::types::Bytes;
use ethereum_types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: H256,
    pub from: Address,
    pub to: Option<Address>, // None for contract creation
    pub value: U256,
    pub gas_limit: U256,
    pub gas_price: U256,
    pub data: Bytes,
    pub nonce: U256,
    pub v: u64,
    pub r: U256,
    pub s: U256,
    pub abby_fee: U256, // Fee paid in Abby tokens
}

impl Transaction {
    pub fn new(
        from: Address,
        to: Option<Address>,
        value: U256,
        gas_limit: U256,
        gas_price: U256,
        data: Bytes,
        nonce: U256,
    ) -> Self {
        let mut tx = Self {
            hash: H256::zero(),
            from,
            to,
            value,
            gas_limit,
            gas_price,
            data,
            nonce,
            v: 0,
            r: U256::zero(),
            s: U256::zero(),
            abby_fee: gas_limit * gas_price / U256::from(1000), // Convert to Abby tokens
        };
        tx.hash = tx.calculate_hash();
        tx
    }

    pub fn hash(&self) -> H256 {
        self.hash
    }

    fn calculate_hash(&self) -> H256 {
        let mut hasher = Keccak256::new();
        hasher.update(self.from.as_bytes());

        if let Some(to) = self.to {
            hasher.update(to.as_bytes());
        }

        let mut value_bytes = [0u8; 32];
        self.value.to_big_endian(&mut value_bytes);
        hasher.update(value_bytes);

        let mut gas_limit_bytes = [0u8; 32];
        self.gas_limit.to_big_endian(&mut gas_limit_bytes);
        hasher.update(gas_limit_bytes);

        let mut gas_price_bytes = [0u8; 32];
        self.gas_price.to_big_endian(&mut gas_price_bytes);
        hasher.update(gas_price_bytes);

        hasher.update(&self.data);

        let mut nonce_bytes = [0u8; 32];
        self.nonce.to_big_endian(&mut nonce_bytes);
        hasher.update(nonce_bytes);

        H256::from_slice(&hasher.finalize())
    }

    pub fn is_contract_creation(&self) -> bool {
        self.to.is_none()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.gas_limit == U256::zero() {
            return Err("Gas limit cannot be zero".to_string());
        }

        if self.gas_price == U256::zero() {
            return Err("Gas price cannot be zero".to_string());
        }

        if self.value > U256::from(u128::MAX) {
            return Err("Value too large".to_string());
        }

        if self.data.len() > 1024 * 1024 {
            return Err("Transaction data too large".to_string());
        }

        Ok(())
    }

    pub fn estimate_gas(&self) -> U256 {
        let mut gas = U256::from(21000); // Base transaction cost

        // Add cost for data
        for byte in &self.data {
            if *byte == 0 {
                gas += U256::from(4); // Zero byte cost
            } else {
                gas += U256::from(16); // Non-zero byte cost
            }
        }

        // Contract creation additional cost
        if self.is_contract_creation() {
            gas += U256::from(32000);
        }

        gas
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPool {
    pub pending: std::collections::HashMap<H256, Transaction>,
    pub queued: std::collections::HashMap<Address, Vec<Transaction>>,
}

impl Default for TransactionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionPool {
    pub fn new() -> Self {
        Self {
            pending: std::collections::HashMap::new(),
            queued: std::collections::HashMap::new(),
        }
    }

    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        tx.validate()?;

        self.pending.insert(tx.hash(), tx);
        Ok(())
    }

    pub fn get_transaction(&self, hash: &H256) -> Option<&Transaction> {
        self.pending.get(hash)
    }

    pub fn remove_transaction(&mut self, hash: &H256) -> Option<Transaction> {
        self.pending.remove(hash)
    }

    pub fn get_pending_transactions(&self) -> Vec<&Transaction> {
        self.pending.values().collect()
    }

    pub fn select_transactions_for_block(&self, gas_limit: U256) -> Vec<Transaction> {
        let mut selected = Vec::new();
        let mut total_gas = U256::zero();

        // Sort by gas price (highest first) for simple transaction selection
        let mut transactions: Vec<_> = self.pending.values().collect();
        transactions.sort_by(|a, b| b.gas_price.cmp(&a.gas_price));

        for tx in transactions {
            if total_gas + tx.gas_limit <= gas_limit {
                selected.push(tx.clone());
                total_gas += tx.gas_limit;
            }
        }

        selected
    }

    pub fn clear(&mut self) {
        self.pending.clear();
        self.queued.clear();
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}
