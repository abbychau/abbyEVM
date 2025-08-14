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
}

#[cfg(test)]
mod tests;
