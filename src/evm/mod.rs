use crate::opcodes::{execute_opcode, OpCode};
use crate::types::{Account, Bytes, ExecutionResult, ExecutionStatus, Log, Word};
use ethereum_types::{Address, U256};
use std::collections::HashMap;

const MAX_STACK_SIZE: usize = 1024;
const MAX_MEMORY_SIZE: usize = 16 * 1024 * 1024; // 16MB

#[derive(Debug)]
pub struct EvmState {
    pub stack: Vec<Word>,
    pub memory: Vec<u8>,
    pub storage: HashMap<Word, Word>,
    pub accounts: HashMap<Address, Account>,
    pub logs: Vec<Log>,
    pub pc: usize, // Program counter
    pub gas: U256,
    pub value: U256,
    pub caller: Address,
    pub origin: Address,
    pub address: Address,
    pub call_data: Bytes,
    pub return_data: Bytes,
    pub halted: bool,
    pub reverted: bool,
    pub error: Option<String>,
}

impl EvmState {
    pub fn new(gas: U256, value: U256) -> Self {
        Self {
            stack: Vec::new(),
            memory: Vec::new(),
            storage: HashMap::new(),
            accounts: HashMap::new(),
            logs: Vec::new(),
            pc: 0,
            gas,
            value,
            caller: Address::zero(),
            origin: Address::zero(),
            address: Address::zero(),
            call_data: Vec::new(),
            return_data: Vec::new(),
            halted: false,
            reverted: false,
            error: None,
        }
    }

    pub fn push_stack(&mut self, value: Word) -> Result<(), String> {
        if self.stack.len() >= MAX_STACK_SIZE {
            return Err("Stack overflow".to_string());
        }
        self.stack.push(value);
        Ok(())
    }

    pub fn pop_stack(&mut self) -> Result<Word, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    pub fn peek_stack(&self, index: usize) -> Result<Word, String> {
        if index >= self.stack.len() {
            return Err("Stack underflow".to_string());
        }
        Ok(self.stack[self.stack.len() - 1 - index])
    }

    pub fn swap_stack(&mut self, n: usize) -> Result<(), String> {
        if self.stack.len() <= n {
            return Err("Stack underflow".to_string());
        }
        let len = self.stack.len();
        self.stack.swap(len - 1, len - 1 - n);
        Ok(())
    }

    pub fn dup_stack(&mut self, n: usize) -> Result<(), String> {
        if n == 0 || n > 16 {
            return Err("Invalid DUP parameter".to_string());
        }
        if self.stack.len() < n {
            return Err("Stack underflow".to_string());
        }
        let value = self.peek_stack(n - 1)?;
        self.push_stack(value)
    }

    pub fn memory_resize(&mut self, size: usize) -> Result<(), String> {
        if size > MAX_MEMORY_SIZE {
            return Err("Memory limit exceeded".to_string());
        }
        if size > self.memory.len() {
            self.memory.resize(size, 0);
        }
        Ok(())
    }

    pub fn memory_store(&mut self, offset: usize, data: &[u8]) -> Result<(), String> {
        let required_size = offset + data.len();
        self.memory_resize(required_size)?;
        self.memory[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn memory_load(&mut self, offset: usize, size: usize) -> Result<Vec<u8>, String> {
        let required_size = offset + size;
        self.memory_resize(required_size)?;
        Ok(self.memory[offset..offset + size].to_vec())
    }

    pub fn consume_gas(&mut self, amount: U256) -> Result<(), String> {
        if self.gas < amount {
            return Err("Out of gas".to_string());
        }
        self.gas -= amount;
        Ok(())
    }

    pub fn storage_load(&self, key: &Word) -> Word {
        self.storage.get(key).copied().unwrap_or_else(U256::zero)
    }

    pub fn storage_store(&mut self, key: Word, value: Word) {
        if value.is_zero() {
            self.storage.remove(&key);
        } else {
            self.storage.insert(key, value);
        }
    }
}

pub struct EvmExecutor {
    gas_limit: U256,
}

impl EvmExecutor {
    pub fn new(gas_limit: u64) -> Self {
        Self {
            gas_limit: U256::from(gas_limit),
        }
    }

    pub fn execute(&mut self, bytecode: &[u8], value: u64, verbose: bool) -> Result<ExecutionResult, anyhow::Error> {
        let mut state = EvmState::new(self.gas_limit, U256::from(value));
        let initial_gas = state.gas;

        if verbose {
            println!("🚀 Starting execution with {} bytes of bytecode", bytecode.len());
            println!("💰 Value: {} wei", value);
            println!("⛽ Gas limit: {}", self.gas_limit);
            println!();
        }

        let mut step_count = 0;
        while state.pc < bytecode.len() && !state.halted && !state.reverted && state.error.is_none() {
            if verbose {
                step_count += 1;
                println!("Step {}: PC={}, Gas={}", step_count, state.pc, state.gas);
            }

            let opcode_byte = bytecode[state.pc];
            let opcode = OpCode::from_byte(opcode_byte);

            if verbose {
                println!("  Opcode: {:?} (0x{:02x})", opcode, opcode_byte);
                println!("  Stack size: {}", state.stack.len());
                if !state.stack.is_empty() {
                    println!("  Stack top: {}", state.stack.last().unwrap());
                }
            }

            // Execute the opcode
            match execute_opcode(&opcode, &mut state, bytecode) {
                Ok(_) => {
                    if !matches!(opcode, OpCode::JUMP | OpCode::JUMPI) && !state.halted {
                        state.pc += 1;
                    }
                }
                Err(e) => {
                    state.error = Some(e);
                    break;
                }
            }

            if verbose {
                println!("  After execution: PC={}, Gas={}", state.pc, state.gas);
                println!();
            }

            // Safety check to prevent infinite loops
            if step_count > 10000 {
                state.error = Some("Execution limit exceeded (too many steps)".to_string());
                break;
            }
        }

        let gas_used = initial_gas - state.gas;

        let status = if let Some(error) = state.error {
            if error.contains("Out of gas") {
                ExecutionStatus::OutOfGas
            } else {
                ExecutionStatus::Error(error)
            }
        } else if state.reverted {
            ExecutionStatus::Revert("Execution reverted".to_string())
        } else {
            ExecutionStatus::Success
        };

        Ok(ExecutionResult {
            status,
            gas_used,
            gas_remaining: state.gas,
            return_data: state.return_data,
            logs: state.logs,
            state_changes: HashMap::new(), // TODO: Track state changes
        })
    }

    pub fn execute_transaction(
        &mut self,
        tx: &crate::types::Transaction,
        accounts: &mut HashMap<Address, Account>,
    ) -> Result<ExecutionResult, String> {
        // Get sender account
        let sender_account = accounts.entry(tx.from).or_default();
        
        // Check balance (simplified - in a real implementation, this would be more complex)
        if sender_account.balance < tx.value {
            return Err("Insufficient balance".to_string());
        }
        
        // Deduct value from sender
        sender_account.balance -= tx.value;
        sender_account.nonce += ethereum_types::U256::one();
        
        // Create EVM state
        let mut state = EvmState::new(tx.gas, tx.value);
        state.caller = tx.from;
        state.origin = tx.from;
        state.call_data = tx.data.clone();
        
        let initial_gas = state.gas;
        
        // Determine execution path
        let result = if let Some(to_address) = tx.to {
            // Call existing contract or transfer
            state.address = to_address;
            
            // Get recipient account
            let recipient_account = accounts.entry(to_address).or_default();
            recipient_account.balance += tx.value;
            
            // If recipient has code, execute it
            if !recipient_account.code.is_empty() {
                let bytecode = recipient_account.code.clone();
                self.execute_bytecode(&bytecode, &mut state)?
            } else {
                // Simple transfer
                ExecutionResult {
                    status: ExecutionStatus::Success,
                    gas_used: ethereum_types::U256::from(21000), // Base transaction cost
                    gas_remaining: state.gas - ethereum_types::U256::from(21000),
                    return_data: Vec::new(),
                    logs: Vec::new(),
                    state_changes: HashMap::new(),
                }
            }
        } else {
            // Contract creation
            let contract_address = self.create_contract_address(&tx.from, &sender_account.nonce);
            state.address = contract_address;
            
            // Execute constructor code
            let result = self.execute_bytecode(&tx.data, &mut state)?;
            
            // Store contract code if successful
            if matches!(result.status, ExecutionStatus::Success) {
                let contract_account = accounts.entry(contract_address).or_default();
                contract_account.code = result.return_data.clone();
                contract_account.balance += tx.value;
            }
            
            result
        };
        
        Ok(result)
    }
    
    fn execute_bytecode(
        &self,
        bytecode: &[u8],
        state: &mut EvmState,
    ) -> Result<ExecutionResult, String> {
        let initial_gas = state.gas;
        
        while state.pc < bytecode.len() && !state.halted && !state.reverted && state.error.is_none() {
            let opcode_byte = bytecode[state.pc];
            let opcode = crate::opcodes::OpCode::from_byte(opcode_byte);
            
            // Execute the opcode
            match crate::opcodes::execute_opcode(&opcode, state, bytecode) {
                Ok(_) => {
                    if !matches!(opcode, crate::opcodes::OpCode::JUMP | crate::opcodes::OpCode::JUMPI) && !state.halted {
                        state.pc += 1;
                    }
                }
                Err(e) => {
                    state.error = Some(e);
                    break;
                }
            }
        }
        
        let gas_used = initial_gas - state.gas;
        
        let status = if let Some(error) = &state.error {
            if error.contains("Out of gas") {
                ExecutionStatus::OutOfGas
            } else {
                ExecutionStatus::Error(error.clone())
            }
        } else if state.reverted {
            ExecutionStatus::Revert("Execution reverted".to_string())
        } else {
            ExecutionStatus::Success
        };
        
        Ok(ExecutionResult {
            status,
            gas_used,
            gas_remaining: state.gas,
            return_data: state.return_data.clone(),
            logs: state.logs.clone(),
            state_changes: HashMap::new(), // TODO: Track state changes
        })
    }
    
    fn create_contract_address(&self, sender: &Address, nonce: &ethereum_types::U256) -> Address {
        use sha3::{Digest, Keccak256};
        
        let mut hasher = Keccak256::new();
        hasher.update(sender.as_bytes());
        
        let mut nonce_bytes = [0u8; 32];
        nonce.to_big_endian(&mut nonce_bytes);
        hasher.update(nonce_bytes);
        
        let hash = hasher.finalize();
        Address::from_slice(&hash[12..])
    }
}

#[cfg(test)]
mod tests;
