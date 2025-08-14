# Example Smart Contracts for AbbyEVM

This directory contains example bytecode and their corresponding Solidity source code (for reference).

## Simple Addition (`simple_add.bin`)

**Solidity Source:**
```solidity
// Simple addition: returns 1 + 2 = 3
// PUSH1 0x01
// PUSH1 0x02  
// ADD
// STOP
```

**Bytecode:** `6001600201`

## Simple Multiplication (`simple_mul.bin`)

**Solidity Source:**
```solidity
// Simple multiplication: returns 2 * 3 = 6
// PUSH1 0x02
// PUSH1 0x03
// MUL
// STOP
```

**Bytecode:** `6002600302`

## Storage Example (`storage.bin`)

**Solidity Source:**
```solidity
// Store value 1 at key 0, then load it
// PUSH1 0x01    // value to store
// PUSH1 0x00    // storage key
// SSTORE        // store value at key
// PUSH1 0x00    // storage key  
// SLOAD         // load value from key
// STOP
```

**Bytecode:** `6001600055600054`

## Running Examples

You can run these examples using:

```bash
# Execute simple addition
cargo run -- execute --example simple-add

# Execute from file
cargo run -- execute --file examples/simple_add.bin

# Execute with verbose output
cargo run -- execute --bytecode "6001600201" --verbose

# Analyze bytecode
cargo run -- analyze --bytecode "6001600201"
```
