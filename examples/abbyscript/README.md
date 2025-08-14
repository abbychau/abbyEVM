# AbbyScript Examples

This directory contains example programs demonstrating various features of AbbyScript.

## Console Logging Examples (NEW!)

### `console_logging.abs` 
Comprehensive demonstration of the new console.log functionality:
- Single variable logging: `console.log(variable)`
- String literals: `console.log("text")`  
- Mixed arguments: `console.log("text:", variable)`
- Multiple console.log calls in one program

### `variables_demo.abs`
Simple demonstration of variable assignment and console output:
- Variable declarations with single-digit numbers (0-9)
- Mixed string and variable console logging
- Multiple console.log statements

### `number_literals.abs` 
Tests different ways to output numbers:
- Direct number literals (compile-time conversion)
- Variable numbers (runtime conversion)
- Comparison between literal and variable output

## Memory and Storage Examples

### `memory_operations.abs`
Demonstrates memory and storage operations:
- Array-like syntax for storage operations
- Memory operations with array syntax
- Implicit memory assignment

## Language Features

### `arithmetic.abs`
Basic arithmetic operations

### `conditional.abs` 
If/else conditional statements

### `functions.abs`
Function declarations and calls

### `storage.abs`
Storage operations

### `simple_js_syntax.abs`
JavaScript-like syntax examples

### `simple_js_no_loops.abs`
JavaScript syntax without loops

## Usage

Compile and run any example:

```bash
# Compile only
./target/debug/abby_evm compile --file examples/abbyscript/console_logging.abs

# Compile and execute
./target/debug/abby_evm execute --bytecode $(./target/debug/abby_evm compile --file examples/abbyscript/variables_demo.abs 2>/dev/null | grep 'Bytecode:' | cut -d' ' -f2)
```

## Recent Updates

✅ **Console.log with Variables**: Console logging now supports both string literals and variables  
✅ **Multi-argument Support**: `console.log("text:", variable)` with automatic spacing  
✅ **Single-digit Numbers**: Full support for displaying numbers 0-9  
✅ **Mixed Output**: Can combine strings and variables in the same program  

## Current Limitations

- Number display currently supports single digits (0-9) 
- Multi-digit numbers will need extended conversion algorithms
- Complex expressions in console.log arguments have limited support
