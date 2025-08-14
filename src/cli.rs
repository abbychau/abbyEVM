use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

pub fn interactive_mode() -> Result<()> {
    println!("{}", "🎮 Interactive EVM Mode".bright_cyan().bold());
    println!("{}", "Type 'help' for available commands, 'quit' to exit".bright_yellow());
    println!("{}", "─".repeat(50).bright_blue());

    loop {
        print!("{} ", "evm>".bright_green().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match input {
            "quit" | "exit" | "q" => {
                println!("{}", "Goodbye! 👋".bright_cyan());
                break;
            }
            "help" | "h" => {
                print_help();
            }
            "examples" | "ex" => {
                list_examples();
            }
            input if input.starts_with("execute ") || input.starts_with("exec ") => {
                let bytecode = input.split_whitespace().nth(1).unwrap_or("");
                if !bytecode.is_empty() {
                    if let Err(e) = execute_interactive_bytecode(bytecode) {
                        println!("{}: {}", "Error".bright_red().bold(), e);
                    }
                } else {
                    println!("{}: Please provide bytecode to execute", "Error".bright_red().bold());
                }
            }
            input if input.starts_with("analyze ") => {
                let bytecode = input.split_whitespace().nth(1).unwrap_or("");
                if !bytecode.is_empty() {
                    if let Err(e) = analyze_interactive_bytecode(bytecode) {
                        println!("{}: {}", "Error".bright_red().bold(), e);
                    }
                } else {
                    println!("{}: Please provide bytecode to analyze", "Error".bright_red().bold());
                }
            }
            _ => {
                println!("{}: Unknown command. Type 'help' for available commands.", "Error".bright_red().bold());
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("{}", "Available Commands:".bright_cyan().bold());
    println!("  {} - Execute bytecode", "execute <bytecode>".bright_green());
    println!("  {} - Analyze bytecode without execution", "analyze <bytecode>".bright_green());
    println!("  {} - List available examples", "examples".bright_green());
    println!("  {} - Show this help message", "help".bright_green());
    println!("  {} - Exit the interactive mode", "quit".bright_green());
    println!();
    println!("{}", "Examples:".bright_yellow().bold());
    println!("  execute 6001600201    # Execute simple addition");
    println!("  analyze 6001600201    # Analyze without execution");
    println!("  examples              # Show example contracts");
}

fn execute_interactive_bytecode(bytecode_hex: &str) -> Result<()> {
    use crate::evm::EvmExecutor;
    use crate::display_execution_result;

    let bytecode = hex::decode(bytecode_hex.trim_start_matches("0x"))?;
    let mut executor = EvmExecutor::new(1000000);
    
    println!("🔄 {}", "Executing...".bright_green());
    let result = executor.execute(&bytecode, 0, false)?;
    display_execution_result(&result);
    
    Ok(())
}

fn analyze_interactive_bytecode(bytecode_hex: &str) -> Result<()> {
    let bytecode = hex::decode(bytecode_hex.trim_start_matches("0x"))?;
    
    println!("{}", "📋 Bytecode Analysis".bright_cyan().bold());
    println!("{}", "─".repeat(30).bright_cyan());
    println!("Length: {} bytes", bytecode.len().to_string().bright_yellow());
    println!("Raw: 0x{}", hex::encode(&bytecode).bright_blue());
    println!();
    
    println!("{}", "Disassembly:".bright_green().bold());
    disassemble_bytecode(&bytecode);
    
    Ok(())
}

fn disassemble_bytecode(bytecode: &[u8]) {
    use crate::opcodes::OpCode;
    
    let mut pc = 0;
    while pc < bytecode.len() {
        let opcode = OpCode::from_byte(bytecode[pc]);
        print!("{:04x}: {:02x} {:?}", pc, bytecode[pc], opcode);
        
        if let Some(size) = opcode.push_size() {
            if pc + size < bytecode.len() {
                let data = &bytecode[pc + 1..pc + 1 + size];
                print!(" 0x{}", hex::encode(data));
                pc += size;
            }
        }
        
        println!();
        pc += 1;
    }
}

pub fn examples_command(list: bool) -> Result<()> {
    if list {
        list_examples();
    } else {
        println!("{}", "🧪 Running Example Contracts".bright_cyan().bold());
        println!("{}", "─".repeat(40).bright_cyan());
        
        // Run all examples
        run_example("simple-add", "Simple Addition (1 + 2)")?;
        run_example("simple-mul", "Simple Multiplication (2 * 3)")?;
        run_example("storage", "Storage Operations")?;
    }
    
    Ok(())
}

fn list_examples() {
    println!("{}", "📚 Available Examples:".bright_cyan().bold());
    println!("  {} - Simple addition (1 + 2)", "simple-add".bright_green());
    println!("  {} - Simple multiplication (2 * 3)", "simple-mul".bright_green());
    println!("  {} - Storage read/write operations", "storage".bright_green());
    println!();
    println!("{}", "Usage:".bright_yellow().bold());
    println!("  cargo run -- execute --example simple-add");
    println!("  cargo run -- execute --example simple-mul");
    println!("  cargo run -- execute --example storage");
}

fn run_example(example: &str, description: &str) -> Result<()> {
    use crate::{get_example_bytecode, display_execution_result};
    use crate::evm::EvmExecutor;
    
    println!("\n{}: {}", "Example".bright_yellow().bold(), description);
    
    let bytecode_hex = get_example_bytecode(example)?;
    println!("Bytecode: {}", bytecode_hex.bright_blue());
    
    let bytecode = hex::decode(&bytecode_hex)?;
    let mut executor = EvmExecutor::new(1000000);
    
    let result = executor.execute(&bytecode, 0, false)?;
    display_execution_result(&result);
    
    Ok(())
}

pub fn analyze_command(bytecode_hex: String) -> Result<()> {
    println!("{}", "🔍 Bytecode Analysis".bright_cyan().bold());
    println!("{}", "═".repeat(50).bright_blue());
    
    let bytecode = hex::decode(bytecode_hex.trim_start_matches("0x"))?;
    
    println!("📊 {}", "Basic Information:".bright_yellow().bold());
    println!("  Length: {} bytes", bytecode.len());
    println!("  Hex: 0x{}", hex::encode(&bytecode));
    println!();
    
    println!("🔧 {}", "Disassembly:".bright_green().bold());
    disassemble_with_details(&bytecode);
    
    println!("\n⛽ {}", "Gas Analysis:".bright_magenta().bold());
    analyze_gas_usage(&bytecode);
    
    Ok(())
}

fn disassemble_with_details(bytecode: &[u8]) {
    use crate::opcodes::OpCode;
    
    let mut pc = 0;
    let mut total_gas = ethereum_types::U256::zero();
    
    while pc < bytecode.len() {
        let opcode = OpCode::from_byte(bytecode[pc]);
        let gas_cost = opcode.gas_cost();
        total_gas += gas_cost;
        
        print!("  {:04x}: ", pc);
        print!("{:02x} ", bytecode[pc]);
        print!("{:12} ", format!("{:?}", opcode));
        
        if let Some(size) = opcode.push_size() {
            if pc + size < bytecode.len() {
                let data = &bytecode[pc + 1..pc + 1 + size];
                print!("0x{:20} ", hex::encode(data));
                pc += size;
            } else {
                print!("{:22} ", "");
            }
        } else {
            print!("{:22} ", "");
        }
        
        println!("(gas: {})", gas_cost);
        pc += 1;
    }
    
    println!("\nEstimated minimum gas: {}", total_gas);
}

fn analyze_gas_usage(bytecode: &[u8]) {
    use crate::opcodes::OpCode;
    use std::collections::HashMap;
    
    let mut gas_by_opcode: HashMap<String, (u64, u64)> = HashMap::new(); // (count, total_gas)
    let mut pc = 0;
    let mut total_gas = 0u64;
    
    while pc < bytecode.len() {
        let opcode = OpCode::from_byte(bytecode[pc]);
        let gas_cost = opcode.gas_cost().low_u64();
        total_gas += gas_cost;
        
        let opcode_str = format!("{:?}", opcode);
        let entry = gas_by_opcode.entry(opcode_str).or_insert((0, 0));
        entry.0 += 1; // count
        entry.1 += gas_cost; // total gas
        
        if opcode.push_size().is_some() {
            pc += opcode.push_size().unwrap();
        }
        pc += 1;
    }
    
    println!("  Total estimated gas: {}", total_gas);
    println!("  Gas breakdown by opcode:");
    
    let mut sorted_opcodes: Vec<_> = gas_by_opcode.iter().collect();
    sorted_opcodes.sort_by(|a, b| b.1.1.cmp(&a.1.1)); // Sort by total gas desc
    
    for (opcode, (count, gas)) in sorted_opcodes.into_iter().take(10) {
        println!("    {:12}: {} uses, {} gas total", opcode, count, gas);
    }
}
