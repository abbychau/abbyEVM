# AbbyEVM & AbbyScript

<img width="930" height="613" alt="image" src="https://github.com/user-attachments/assets/cd9c52ab-658b-428c-b855-7fbb259c0f96" />


AbbyEVM, A simple Ethereum Virtual Machine implementation in Rust with a JavaScript-like programming language.

[AbbyScript](./examples/abbyscript/) is a high-level language designed for writing smart contracts on the Ethereum blockchain. It provides a familiar syntax for JavaScript developers while abstracting away the complexities of EVM bytecode.

## AbbyScript Language

AbbyScript looks like JavaScript but compiles to EVM bytecode:

```javascript
// Variables and arithmetic
let x = 42;
let y = x + 10;
let result = x * y;

// Storage operations  
storage[0] = result;
let value = storage[0];

// Memory operations
memory[64] = 0xdeadbeef;
let data = memory[64];

// Control flow
if (x > 50) {
    storage[1] = x * 2;
} else {
    storage[1] = 0;
}

// Functions
function add(a, b) {
    return a + b;
}

let sum = add(5, 3);
```

## Try

```bash
# Build
cargo build

# Run tests  
cargo test

# Debug mode
RUST_LOG=debug cargo run -- execute --example simple-add
```

## How it works

1. **Lexer** breaks source code into tokens
2. **Parser** builds an abstract syntax tree (AST)  
3. **Code generator** converts AST to EVM bytecode
4. **EVM executor** runs the bytecode with proper gas metering

The EVM implementation supports most opcodes including arithmetic, logic, memory, storage, and control flow operations.

## Blockchain

AbbyEVM includes a simple blockchain implementation that allows users to deploy and interact with smart contracts. The blockchain is designed to be lightweight and easy to use, making it ideal for educational purposes.

[Documentation](./BLOCKCHAIN.md)

## License

MIT License
