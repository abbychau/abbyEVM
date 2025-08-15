# AbbyEVM & AbbyScript

<img width="971" height="616" alt="image" src="https://github.com/user-attachments/assets/9e360ba0-0bbd-400e-9dd2-c47ad9f52f0c" />


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

## License

MIT License
