# AbbyEVM & AbbyScript

A simple Ethereum Virtual Machine implementation in Rust with a JavaScript-like programming language.

[AbbyScript](./examples/abbyscript/) is a high-level language designed for writing smart contracts on the Ethereum blockchain. It provides a familiar syntax for JavaScript developers while abstracting away the complexities of EVM bytecode.

## What is this?

AbbyEVM lets you run Ethereum bytecode and write smart contract logic using AbbyScript, a language that looks like JavaScript but compiles to EVM bytecode.

## Features

- **EVM execution**: Run bytecode with proper gas metering and state management
- **AbbyScript compiler**: Write code that looks like JavaScript, get EVM bytecode
- **Interactive CLI**: Test bytecode and explore the EVM interactively
- **Verbose tracing**: See exactly what happens during execution

## Getting Started

Build and run:
```bash
git clone https://github.com/user/abbyevm.git
cd abbyevm
cargo build --release
```

Run some bytecode:
```bash
# Execute bytecode directly
cargo run -- execute --bytecode "6001600201" --verbose

# Run an example
cargo run -- execute --example simple-add
```

Compile and run AbbyScript:
```bash
# From file
cargo run -- compile --file examples/abbyscript/arithmetic.abs --run

# From expression
cargo run -- compile --expression "let x = 42; storage[0] = x;" --run
```

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

## Examples

Check out the `examples/` directory:

- **EVM bytecode examples**: `simple_add.bin`, `simple_mul.bin`, `storage.bin`
- **AbbyScript examples**: 
  - `arithmetic.abs` - Basic math
  - `storage.abs` - Storage operations  
  - `conditional.abs` - If/else logic
  - `functions.abs` - Function definitions

## Development

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

MIT License - see [LICENSE](LICENSE) file.
