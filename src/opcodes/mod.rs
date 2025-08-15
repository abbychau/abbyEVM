use crate::evm::EvmState;
use ethereum_types::U256;
use sha3::{Digest, Keccak256};

// Helper function to decode bytes to a readable string
fn decode_string_from_bytes(data: &[u8]) -> String {
    // Since the data is now correctly loaded from memory,
    // we can simply convert the bytes to a UTF-8 string
    match String::from_utf8(data.to_vec()) {
        Ok(s) => s,
        Err(_) => {
            // If UTF-8 conversion fails, filter to ASCII printable chars
            let filtered: Vec<u8> = data.iter()
                .filter(|&&b| (32..=126).contains(&b)) // ASCII printable characters
                .copied()
                .collect();
            String::from_utf8_lossy(&filtered).to_string()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum OpCode {
    // Stop and Arithmetic Operations (0x00 - 0x0F)
    STOP,
    ADD,
    MUL,
    SUB,
    DIV,
    SDIV,
    MOD,
    SMOD,
    ADDMOD,
    MULMOD,
    EXP,
    SIGNEXTEND,
    
    // Comparison & Bitwise Logic Operations (0x10 - 0x1F)
    LT,
    GT,
    SLT,
    SGT,
    EQ,
    ISZERO,
    AND,
    OR,
    XOR,
    NOT,
    BYTE,
    SHL,
    SHR,
    SAR,
    
    // SHA3 (0x20)
    SHA3,
    
    // Environmental Information (0x30 - 0x3F)
    ADDRESS,
    BALANCE,
    ORIGIN,
    CALLER,
    CALLVALUE,
    CALLDATALOAD,
    CALLDATASIZE,
    CALLDATACOPY,
    CODESIZE,
    CODECOPY,
    GASPRICE,
    EXTCODESIZE,
    EXTCODECOPY,
    RETURNDATASIZE,
    RETURNDATACOPY,
    EXTCODEHASH,
    
    // Block Information (0x40 - 0x4F)
    BLOCKHASH,
    COINBASE,
    TIMESTAMP,
    NUMBER,
    DIFFICULTY,
    GASLIMIT,
    CHAINID,
    SELFBALANCE,
    BASEFEE,
    
    // Stack, Memory, Storage and Flow Operations (0x50 - 0x5F)
    POP,
    MLOAD,
    MSTORE,
    MSTORE8,
    SLOAD,
    SSTORE,
    JUMP,
    JUMPI,
    PC,
    MSIZE,
    GAS,
    JUMPDEST,
    
    // Push Operations (0x60 - 0x7F)
    PUSH1, PUSH2, PUSH3, PUSH4, PUSH5, PUSH6, PUSH7, PUSH8,
    PUSH9, PUSH10, PUSH11, PUSH12, PUSH13, PUSH14, PUSH15, PUSH16,
    PUSH17, PUSH18, PUSH19, PUSH20, PUSH21, PUSH22, PUSH23, PUSH24,
    PUSH25, PUSH26, PUSH27, PUSH28, PUSH29, PUSH30, PUSH31, PUSH32,
    
    // Duplication Operations (0x80 - 0x8F)
    DUP1, DUP2, DUP3, DUP4, DUP5, DUP6, DUP7, DUP8,
    DUP9, DUP10, DUP11, DUP12, DUP13, DUP14, DUP15, DUP16,
    
    // Exchange Operations (0x90 - 0x9F)
    SWAP1, SWAP2, SWAP3, SWAP4, SWAP5, SWAP6, SWAP7, SWAP8,
    SWAP9, SWAP10, SWAP11, SWAP12, SWAP13, SWAP14, SWAP15, SWAP16,
    
    // Logging Operations (0xA0 - 0xA4)
    LOG0, LOG1, LOG2, LOG3, LOG4,
    
    // System Operations (0xF0 - 0xFF)
    CREATE,
    CALL,
    CALLCODE,
    RETURN,
    DELEGATECALL,
    CREATE2,
    STATICCALL,
    REVERT,
    INVALID,
    SELFDESTRUCT,
    
    // Unknown/Invalid opcode
    UNKNOWN(u8),
}

impl OpCode {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => OpCode::STOP,
            0x01 => OpCode::ADD,
            0x02 => OpCode::MUL,
            0x03 => OpCode::SUB,
            0x04 => OpCode::DIV,
            0x05 => OpCode::SDIV,
            0x06 => OpCode::MOD,
            0x07 => OpCode::SMOD,
            0x08 => OpCode::ADDMOD,
            0x09 => OpCode::MULMOD,
            0x0a => OpCode::EXP,
            0x0b => OpCode::SIGNEXTEND,
            
            0x10 => OpCode::LT,
            0x11 => OpCode::GT,
            0x12 => OpCode::SLT,
            0x13 => OpCode::SGT,
            0x14 => OpCode::EQ,
            0x15 => OpCode::ISZERO,
            0x16 => OpCode::AND,
            0x17 => OpCode::OR,
            0x18 => OpCode::XOR,
            0x19 => OpCode::NOT,
            0x1a => OpCode::BYTE,
            0x1b => OpCode::SHL,
            0x1c => OpCode::SHR,
            0x1d => OpCode::SAR,
            
            0x20 => OpCode::SHA3,
            
            0x30 => OpCode::ADDRESS,
            0x31 => OpCode::BALANCE,
            0x32 => OpCode::ORIGIN,
            0x33 => OpCode::CALLER,
            0x34 => OpCode::CALLVALUE,
            0x35 => OpCode::CALLDATALOAD,
            0x36 => OpCode::CALLDATASIZE,
            0x37 => OpCode::CALLDATACOPY,
            0x38 => OpCode::CODESIZE,
            0x39 => OpCode::CODECOPY,
            0x3a => OpCode::GASPRICE,
            0x3b => OpCode::EXTCODESIZE,
            0x3c => OpCode::EXTCODECOPY,
            0x3d => OpCode::RETURNDATASIZE,
            0x3e => OpCode::RETURNDATACOPY,
            0x3f => OpCode::EXTCODEHASH,
            
            0x40 => OpCode::BLOCKHASH,
            0x41 => OpCode::COINBASE,
            0x42 => OpCode::TIMESTAMP,
            0x43 => OpCode::NUMBER,
            0x44 => OpCode::DIFFICULTY,
            0x45 => OpCode::GASLIMIT,
            0x46 => OpCode::CHAINID,
            0x47 => OpCode::SELFBALANCE,
            0x48 => OpCode::BASEFEE,
            
            0x50 => OpCode::POP,
            0x51 => OpCode::MLOAD,
            0x52 => OpCode::MSTORE,
            0x53 => OpCode::MSTORE8,
            0x54 => OpCode::SLOAD,
            0x55 => OpCode::SSTORE,
            0x56 => OpCode::JUMP,
            0x57 => OpCode::JUMPI,
            0x58 => OpCode::PC,
            0x59 => OpCode::MSIZE,
            0x5a => OpCode::GAS,
            0x5b => OpCode::JUMPDEST,
            
            0x60 => OpCode::PUSH1,
            0x61 => OpCode::PUSH2,
            0x62 => OpCode::PUSH3,
            0x63 => OpCode::PUSH4,
            0x64 => OpCode::PUSH5,
            0x65 => OpCode::PUSH6,
            0x66 => OpCode::PUSH7,
            0x67 => OpCode::PUSH8,
            0x68 => OpCode::PUSH9,
            0x69 => OpCode::PUSH10,
            0x6a => OpCode::PUSH11,
            0x6b => OpCode::PUSH12,
            0x6c => OpCode::PUSH13,
            0x6d => OpCode::PUSH14,
            0x6e => OpCode::PUSH15,
            0x6f => OpCode::PUSH16,
            0x70 => OpCode::PUSH17,
            0x71 => OpCode::PUSH18,
            0x72 => OpCode::PUSH19,
            0x73 => OpCode::PUSH20,
            0x74 => OpCode::PUSH21,
            0x75 => OpCode::PUSH22,
            0x76 => OpCode::PUSH23,
            0x77 => OpCode::PUSH24,
            0x78 => OpCode::PUSH25,
            0x79 => OpCode::PUSH26,
            0x7a => OpCode::PUSH27,
            0x7b => OpCode::PUSH28,
            0x7c => OpCode::PUSH29,
            0x7d => OpCode::PUSH30,
            0x7e => OpCode::PUSH31,
            0x7f => OpCode::PUSH32,
            
            0x80 => OpCode::DUP1,
            0x81 => OpCode::DUP2,
            0x82 => OpCode::DUP3,
            0x83 => OpCode::DUP4,
            0x84 => OpCode::DUP5,
            0x85 => OpCode::DUP6,
            0x86 => OpCode::DUP7,
            0x87 => OpCode::DUP8,
            0x88 => OpCode::DUP9,
            0x89 => OpCode::DUP10,
            0x8a => OpCode::DUP11,
            0x8b => OpCode::DUP12,
            0x8c => OpCode::DUP13,
            0x8d => OpCode::DUP14,
            0x8e => OpCode::DUP15,
            0x8f => OpCode::DUP16,
            
            0x90 => OpCode::SWAP1,
            0x91 => OpCode::SWAP2,
            0x92 => OpCode::SWAP3,
            0x93 => OpCode::SWAP4,
            0x94 => OpCode::SWAP5,
            0x95 => OpCode::SWAP6,
            0x96 => OpCode::SWAP7,
            0x97 => OpCode::SWAP8,
            0x98 => OpCode::SWAP9,
            0x99 => OpCode::SWAP10,
            0x9a => OpCode::SWAP11,
            0x9b => OpCode::SWAP12,
            0x9c => OpCode::SWAP13,
            0x9d => OpCode::SWAP14,
            0x9e => OpCode::SWAP15,
            0x9f => OpCode::SWAP16,
            
            0xa0 => OpCode::LOG0,
            0xa1 => OpCode::LOG1,
            0xa2 => OpCode::LOG2,
            0xa3 => OpCode::LOG3,
            0xa4 => OpCode::LOG4,
            
            0xf0 => OpCode::CREATE,
            0xf1 => OpCode::CALL,
            0xf2 => OpCode::CALLCODE,
            0xf3 => OpCode::RETURN,
            0xf4 => OpCode::DELEGATECALL,
            0xf5 => OpCode::CREATE2,
            0xfa => OpCode::STATICCALL,
            0xfd => OpCode::REVERT,
            0xfe => OpCode::INVALID,
            0xff => OpCode::SELFDESTRUCT,
            
            _ => OpCode::UNKNOWN(byte),
        }
    }

    pub fn gas_cost(&self) -> U256 {
        match self {
            OpCode::STOP => U256::from(0),
            OpCode::ADD | OpCode::SUB | OpCode::LT | OpCode::GT | OpCode::SLT | OpCode::SGT | 
            OpCode::EQ | OpCode::ISZERO | OpCode::AND | OpCode::OR | OpCode::XOR | OpCode::NOT |
            OpCode::BYTE | OpCode::SHL | OpCode::SHR | OpCode::SAR => U256::from(3),
            
            OpCode::MUL | OpCode::DIV | OpCode::SDIV | OpCode::MOD | OpCode::SMOD => U256::from(5),
            OpCode::ADDMOD | OpCode::MULMOD => U256::from(8),
            OpCode::SIGNEXTEND => U256::from(5),
            
            OpCode::SHA3 => U256::from(30),
            
            OpCode::ADDRESS | OpCode::ORIGIN | OpCode::CALLER | OpCode::CALLVALUE |
            OpCode::CALLDATASIZE | OpCode::CODESIZE | OpCode::GASPRICE | OpCode::COINBASE |
            OpCode::TIMESTAMP | OpCode::NUMBER | OpCode::DIFFICULTY | OpCode::GASLIMIT |
            OpCode::CHAINID | OpCode::SELFBALANCE | OpCode::BASEFEE => U256::from(2),
            
            OpCode::POP => U256::from(2),
            OpCode::MLOAD => U256::from(3),
            OpCode::MSTORE | OpCode::MSTORE8 => U256::from(3),
            OpCode::SLOAD => U256::from(200),
            OpCode::SSTORE => U256::from(5000), // Simplified, actual cost depends on state
            OpCode::JUMP => U256::from(8),
            OpCode::JUMPI => U256::from(10),
            OpCode::PC => U256::from(2),
            OpCode::MSIZE => U256::from(2),
            OpCode::GAS => U256::from(2),
            OpCode::JUMPDEST => U256::from(1),
            
            // PUSH operations
            OpCode::PUSH1 | OpCode::PUSH2 | OpCode::PUSH3 | OpCode::PUSH4 | OpCode::PUSH5 |
            OpCode::PUSH6 | OpCode::PUSH7 | OpCode::PUSH8 | OpCode::PUSH9 | OpCode::PUSH10 |
            OpCode::PUSH11 | OpCode::PUSH12 | OpCode::PUSH13 | OpCode::PUSH14 | OpCode::PUSH15 |
            OpCode::PUSH16 | OpCode::PUSH17 | OpCode::PUSH18 | OpCode::PUSH19 | OpCode::PUSH20 |
            OpCode::PUSH21 | OpCode::PUSH22 | OpCode::PUSH23 | OpCode::PUSH24 | OpCode::PUSH25 |
            OpCode::PUSH26 | OpCode::PUSH27 | OpCode::PUSH28 | OpCode::PUSH29 | OpCode::PUSH30 |
            OpCode::PUSH31 | OpCode::PUSH32 => U256::from(3),
            
            // DUP operations
            OpCode::DUP1 | OpCode::DUP2 | OpCode::DUP3 | OpCode::DUP4 | OpCode::DUP5 |
            OpCode::DUP6 | OpCode::DUP7 | OpCode::DUP8 | OpCode::DUP9 | OpCode::DUP10 |
            OpCode::DUP11 | OpCode::DUP12 | OpCode::DUP13 | OpCode::DUP14 | OpCode::DUP15 |
            OpCode::DUP16 => U256::from(3),
            
            // SWAP operations
            OpCode::SWAP1 | OpCode::SWAP2 | OpCode::SWAP3 | OpCode::SWAP4 | OpCode::SWAP5 |
            OpCode::SWAP6 | OpCode::SWAP7 | OpCode::SWAP8 | OpCode::SWAP9 | OpCode::SWAP10 |
            OpCode::SWAP11 | OpCode::SWAP12 | OpCode::SWAP13 | OpCode::SWAP14 | OpCode::SWAP15 |
            OpCode::SWAP16 => U256::from(3),
            
            OpCode::RETURN => U256::from(0),
            OpCode::REVERT => U256::from(0),
            
            _ => U256::from(1), // Default gas cost
        }
    }

    pub fn push_size(&self) -> Option<usize> {
        match self {
            OpCode::PUSH1 => Some(1),
            OpCode::PUSH2 => Some(2),
            OpCode::PUSH3 => Some(3),
            OpCode::PUSH4 => Some(4),
            OpCode::PUSH5 => Some(5),
            OpCode::PUSH6 => Some(6),
            OpCode::PUSH7 => Some(7),
            OpCode::PUSH8 => Some(8),
            OpCode::PUSH9 => Some(9),
            OpCode::PUSH10 => Some(10),
            OpCode::PUSH11 => Some(11),
            OpCode::PUSH12 => Some(12),
            OpCode::PUSH13 => Some(13),
            OpCode::PUSH14 => Some(14),
            OpCode::PUSH15 => Some(15),
            OpCode::PUSH16 => Some(16),
            OpCode::PUSH17 => Some(17),
            OpCode::PUSH18 => Some(18),
            OpCode::PUSH19 => Some(19),
            OpCode::PUSH20 => Some(20),
            OpCode::PUSH21 => Some(21),
            OpCode::PUSH22 => Some(22),
            OpCode::PUSH23 => Some(23),
            OpCode::PUSH24 => Some(24),
            OpCode::PUSH25 => Some(25),
            OpCode::PUSH26 => Some(26),
            OpCode::PUSH27 => Some(27),
            OpCode::PUSH28 => Some(28),
            OpCode::PUSH29 => Some(29),
            OpCode::PUSH30 => Some(30),
            OpCode::PUSH31 => Some(31),
            OpCode::PUSH32 => Some(32),
            _ => None,
        }
    }
}

pub fn execute_opcode(opcode: &OpCode, state: &mut EvmState, bytecode: &[u8]) -> Result<(), String> {
    // Consume gas
    let gas_cost = opcode.gas_cost();
    state.consume_gas(gas_cost)?;
    
    match opcode {
        // Stop and Arithmetic Operations
        OpCode::STOP => {
            state.halted = true;
        }
        
        OpCode::ADD => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = a.overflowing_add(b).0;
            state.push_stack(result)?;
        }
        
        OpCode::MUL => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = a.overflowing_mul(b).0;
            state.push_stack(result)?;
        }
        
        OpCode::SUB => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = a.overflowing_sub(b).0;
            state.push_stack(result)?;
        }
        
        OpCode::DIV => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = if b.is_zero() { U256::zero() } else { a / b };
            state.push_stack(result)?;
        }
        
        OpCode::MOD => {
            let b = state.pop_stack()?; // divisor (second operand)
            let a = state.pop_stack()?; // dividend (first operand)  
            let result = if b.is_zero() { U256::zero() } else { a % b };
            state.push_stack(result)?;
        }
        
        OpCode::EXP => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = a.overflowing_pow(b).0;
            state.push_stack(result)?;
        }
        
        // Comparison & Bitwise Logic Operations
        OpCode::LT => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = if a < b { U256::one() } else { U256::zero() };
            state.push_stack(result)?;
        }
        
        OpCode::GT => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = if a > b { U256::one() } else { U256::zero() };
            state.push_stack(result)?;
        }
        
        OpCode::EQ => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = if a == b { U256::one() } else { U256::zero() };
            state.push_stack(result)?;
        }
        
        OpCode::ISZERO => {
            let a = state.pop_stack()?;
            let result = if a.is_zero() { U256::one() } else { U256::zero() };
            state.push_stack(result)?;
        }
        
        OpCode::AND => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = a & b;
            state.push_stack(result)?;
        }
        
        OpCode::OR => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = a | b;
            state.push_stack(result)?;
        }
        
        OpCode::XOR => {
            let a = state.pop_stack()?;
            let b = state.pop_stack()?;
            let result = a ^ b;
            state.push_stack(result)?;
        }
        
        OpCode::NOT => {
            let a = state.pop_stack()?;
            let result = !a;
            state.push_stack(result)?;
        }
        
        // SHA3
        OpCode::SHA3 => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            let data = state.memory_load(offset, size)?;
            let mut hasher = Keccak256::new();
            hasher.update(&data);
            let hash = hasher.finalize();
            let hash_u256 = U256::from_big_endian(&hash);
            state.push_stack(hash_u256)?;
        }
        
        // Environmental Information
        OpCode::ADDRESS => {
            let addr_u256 = U256::from_big_endian(state.address.as_bytes());
            state.push_stack(addr_u256)?;
        }
        
        OpCode::CALLER => {
            let caller_u256 = U256::from_big_endian(state.caller.as_bytes());
            state.push_stack(caller_u256)?;
        }
        
        OpCode::CALLVALUE => {
            state.push_stack(state.value)?;
        }
        
        OpCode::CALLDATASIZE => {
            state.push_stack(U256::from(state.call_data.len()))?;
        }
        
        OpCode::CODESIZE => {
            state.push_stack(U256::from(bytecode.len()))?;
        }
        
        // Stack, Memory, Storage and Flow Operations
        OpCode::POP => {
            state.pop_stack()?;
        }
        
        OpCode::MLOAD => {
            let offset = state.pop_stack()?.as_usize();
            let data = state.memory_load(offset, 32)?;
            let mut bytes = [0u8; 32];
            bytes[..data.len().min(32)].copy_from_slice(&data[..data.len().min(32)]);
            let value = U256::from_big_endian(&bytes);
            state.push_stack(value)?;
        }
        
        OpCode::MSTORE => {
            let offset = state.pop_stack()?.as_usize();
            let value = state.pop_stack()?;
            let mut bytes = [0u8; 32];
            value.to_big_endian(&mut bytes);
            state.memory_store(offset, &bytes)?;
        }
        
        OpCode::MSTORE8 => {
            let offset = state.pop_stack()?.as_usize();
            let value = state.pop_stack()?;
            let byte = (value.low_u64() & 0xFF) as u8;
            state.memory_store(offset, &[byte])?;
        }
        
        OpCode::SLOAD => {
            let key = state.pop_stack()?;
            let value = state.storage_load(&key);
            state.push_stack(value)?;
        }
        
        OpCode::SSTORE => {
            let key = state.pop_stack()?;
            let value = state.pop_stack()?;
            state.storage_store(key, value);
        }
        
        OpCode::JUMP => {
            let dest = state.pop_stack()?.as_usize();
            if dest >= bytecode.len() || bytecode[dest] != 0x5b { // 0x5b is JUMPDEST
                return Err("Invalid jump destination".to_string());
            }
            state.pc = dest;
        }
        
        OpCode::JUMPI => {
            let dest = state.pop_stack()?.as_usize();
            let condition = state.pop_stack()?;
            if !condition.is_zero() {
                if dest >= bytecode.len() || bytecode[dest] != 0x5b { // 0x5b is JUMPDEST
                    return Err("Invalid jump destination".to_string());
                }
                state.pc = dest;
            } else {
                state.pc += 1; // Continue to next instruction
            }
        }
        
        OpCode::PC => {
            state.push_stack(U256::from(state.pc))?;
        }
        
        OpCode::MSIZE => {
            state.push_stack(U256::from(state.memory.len()))?;
        }
        
        OpCode::GAS => {
            state.push_stack(state.gas)?;
        }
        
        OpCode::JUMPDEST => {
            // JUMPDEST is a no-op, just marks valid jump destinations
        }
        
        // Push Operations
        push_op if push_op.push_size().is_some() => {
            let size = push_op.push_size().unwrap();
            if state.pc + size >= bytecode.len() {
                return Err("Push instruction exceeds bytecode length".to_string());
            }
            
            let mut bytes = vec![0u8; 32]; // U256 is 32 bytes
            let start_idx = 32 - size;
            bytes[start_idx..].copy_from_slice(&bytecode[state.pc + 1..state.pc + 1 + size]);
            
            let value = U256::from_big_endian(&bytes);
            state.push_stack(value)?;
            state.pc += size; // Skip the pushed bytes
        }
        
        // DUP Operations
        OpCode::DUP1 => state.dup_stack(1)?,
        OpCode::DUP2 => state.dup_stack(2)?,
        OpCode::DUP3 => state.dup_stack(3)?,
        OpCode::DUP4 => state.dup_stack(4)?,
        OpCode::DUP5 => state.dup_stack(5)?,
        OpCode::DUP6 => state.dup_stack(6)?,
        OpCode::DUP7 => state.dup_stack(7)?,
        OpCode::DUP8 => state.dup_stack(8)?,
        OpCode::DUP9 => state.dup_stack(9)?,
        OpCode::DUP10 => state.dup_stack(10)?,
        OpCode::DUP11 => state.dup_stack(11)?,
        OpCode::DUP12 => state.dup_stack(12)?,
        OpCode::DUP13 => state.dup_stack(13)?,
        OpCode::DUP14 => state.dup_stack(14)?,
        OpCode::DUP15 => state.dup_stack(15)?,
        OpCode::DUP16 => state.dup_stack(16)?,
        
        // SWAP Operations
        OpCode::SWAP1 => state.swap_stack(1)?,
        OpCode::SWAP2 => state.swap_stack(2)?,
        OpCode::SWAP3 => state.swap_stack(3)?,
        OpCode::SWAP4 => state.swap_stack(4)?,
        OpCode::SWAP5 => state.swap_stack(5)?,
        OpCode::SWAP6 => state.swap_stack(6)?,
        OpCode::SWAP7 => state.swap_stack(7)?,
        OpCode::SWAP8 => state.swap_stack(8)?,
        OpCode::SWAP9 => state.swap_stack(9)?,
        OpCode::SWAP10 => state.swap_stack(10)?,
        OpCode::SWAP11 => state.swap_stack(11)?,
        OpCode::SWAP12 => state.swap_stack(12)?,
        OpCode::SWAP13 => state.swap_stack(13)?,
        OpCode::SWAP14 => state.swap_stack(14)?,
        OpCode::SWAP15 => state.swap_stack(15)?,
        OpCode::SWAP16 => state.swap_stack(16)?,
        
        // System Operations
        OpCode::RETURN => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            state.return_data = state.memory_load(offset, size)?;
            state.halted = true;
        }
        
        OpCode::REVERT => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            state.return_data = state.memory_load(offset, size)?;
            state.reverted = true;
        }
        
        // Log Operations
        OpCode::LOG0 => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            let data = state.memory_load(offset, size)?;
            
            // Decode and display the string content
            let message = decode_string_from_bytes(&data);
            println!("console.log: {}", message);
        }
        
        OpCode::LOG1 => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            let topic1 = state.pop_stack()?;
            let data = state.memory_load(offset, size)?;
            
            // Decode the string content
            let message = decode_string_from_bytes(&data);
            
            // Different output based on topic (1=warn, 2=error)
            match topic1.as_u64() {
                1 => println!("console.warn: {}", message),
                2 => println!("console.error: {}", message),
                _ => println!("console (topic {}): {}", topic1, message),
            }
        }
        
        OpCode::LOG2 => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            let topic2 = state.pop_stack()?;
            let topic1 = state.pop_stack()?;
            let data = state.memory_load(offset, size)?;
            
            let message = decode_string_from_bytes(&data);
            println!("LOG2 (topics: {}, {}): {}", topic1, topic2, message);
        }
        
        OpCode::LOG3 => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            let topic3 = state.pop_stack()?;
            let topic2 = state.pop_stack()?;
            let topic1 = state.pop_stack()?;
            let data = state.memory_load(offset, size)?;
            
            let message = decode_string_from_bytes(&data);
            println!("LOG3 (topics: {}, {}, {}): {}", topic1, topic2, topic3, message);
        }
        
        OpCode::LOG4 => {
            let offset = state.pop_stack()?.as_usize();
            let size = state.pop_stack()?.as_usize();
            let topic4 = state.pop_stack()?;
            let topic3 = state.pop_stack()?;
            let topic2 = state.pop_stack()?;
            let topic1 = state.pop_stack()?;
            let data = state.memory_load(offset, size)?;
            
            let message = decode_string_from_bytes(&data);
            println!("LOG4 (topics: {}, {}, {}, {}): {}", topic1, topic2, topic3, topic4, message);
        }
        
        // Unimplemented opcodes
        _ => {
            return Err(format!("Unimplemented opcode: {:?}", opcode));
        }
    }
    
    Ok(())
}
