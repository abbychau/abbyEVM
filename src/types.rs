use ethereum_types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Bytes = Vec<u8>;
pub type Word = U256;

#[derive(Debug, Clone)]
pub struct Account {
    pub balance: U256,
    pub nonce: U256,
    pub code: Bytes,
    pub storage: HashMap<U256, U256>,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            balance: U256::zero(),
            nonce: U256::zero(),
            code: Vec::new(),
            storage: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas: U256,
    pub gas_price: U256,
    pub data: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<H256>,
    pub data: Bytes,
}

impl std::fmt::Display for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Log{{ address: {}, topics: [{}], data: 0x{} }}",
            self.address,
            self.topics.iter().map(|t| format!("{}", t)).collect::<Vec<_>>().join(", "),
            hex::encode(&self.data)
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    Success,
    Revert(String),
    OutOfGas,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub gas_used: U256,
    pub gas_remaining: U256,
    pub return_data: Bytes,
    pub logs: Vec<Log>,
    pub state_changes: HashMap<Address, Account>,
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Success,
            gas_used: U256::zero(),
            gas_remaining: U256::zero(),
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: HashMap::new(),
        }
    }
}
