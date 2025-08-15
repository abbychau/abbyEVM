use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod blockchain;
mod cli;
mod compiler;
mod evm;
mod opcodes;
mod types;
mod utils;

use blockchain::AbbyNode;
use cli::*;
use compiler::Compiler;
use evm::{EvmExecutor};
use types::{ExecutionResult, ExecutionStatus};

#[derive(Parser)]
#[command(name = "abby_evm")]
#[command(about = "A user-friendly Ethereum Virtual Machine implementation in Rust")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute EVM bytecode
    Execute {
        /// Bytecode to execute (hex string)
        #[arg(short, long, conflicts_with_all = ["file", "example"])]
        bytecode: Option<String>,
        
        /// File containing bytecode
        #[arg(short, long, conflicts_with_all = ["bytecode", "example"])]
        file: Option<PathBuf>,
        
        /// Run a predefined example
        #[arg(short, long, conflicts_with_all = ["bytecode", "file"])]
        example: Option<String>,
        
        /// Gas limit for execution
        #[arg(short, long, default_value = "1000000")]
        gas_limit: u64,
        
        /// Initial value in wei
        #[arg(long, default_value = "0")]
        value: u64,
        
        /// Enable verbose output for this command
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Compile AbbyScript source code to EVM bytecode
    Compile {
        /// AbbyScript source file
        #[arg(short, long, conflicts_with_all = ["source", "expression"])]
        file: Option<PathBuf>,
        
        /// AbbyScript source code as string
        #[arg(short, long, conflicts_with_all = ["file", "expression"])]
        source: Option<String>,
        
        /// Compile a single expression
        #[arg(short, long, conflicts_with_all = ["file", "source"])]
        expression: Option<String>,
        
        /// Output file for bytecode
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Enable debug output
        #[arg(short, long)]
        debug: bool,
        
        /// Execute the compiled bytecode immediately
        #[arg(short = 'r', long)]
        run: bool,
        
        /// Gas limit for execution (if --run is specified)
        #[arg(long, default_value = "1000000")]
        gas_limit: u64,
    },
    
    /// Start interactive EVM shell
    Interactive {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Start AbbyEVM blockchain node
    Node {
        /// Network port for P2P communication
        #[arg(short, long, default_value = "30303")]
        port: u16,
        
        /// Validator address (if this node should participate in consensus)
        #[arg(short, long)]
        validator: Option<String>,
        
        /// Peer addresses to connect to
        #[arg(short = 'c', long)]
        connect: Vec<String>,
        
        /// Database path for persistent storage (default: ~/.ABBYCHAIN)
        #[arg(short, long)]
        db_path: Option<PathBuf>,
        
        /// Enable mining (validator mode)
        #[arg(short, long)]
        mine: bool,
    },
    
    /// List and run example contracts
    Examples {
        /// List available examples
        #[arg(short, long)]
        list: bool,
    },
    
    /// Analyze bytecode without executing
    Analyze {
        /// Bytecode to analyze (hex string)
        #[arg(short, long)]
        bytecode: String,
    },
}

fn main() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();
    
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }

    println!("{}", "🚀 AbbyEVM - User-Friendly Ethereum Virtual Machine".bright_cyan().bold());
    println!("{}", "═".repeat(50).bright_blue());
    
    match cli.command {
        Commands::Execute { bytecode, file, example, gas_limit, value, verbose } => {
            let final_verbose = cli.verbose || verbose;
            execute_command(bytecode, file, example, gas_limit, value, final_verbose)?;
        }
        Commands::Compile { file, source, expression, output, debug, run, gas_limit } => {
            compile_command(file, source, expression, output, debug, run, gas_limit)?;
        }
        Commands::Interactive { verbose } => {
            let _final_verbose = cli.verbose || verbose;
            interactive_mode()?;
        }
        Commands::Node { port, validator, connect, db_path, mine } => {
            node_command(port, validator, connect, db_path, mine).await?;
        }
        Commands::Examples { list } => {
            examples_command(list)?;
        }
        Commands::Analyze { bytecode } => {
            analyze_command(bytecode)?;
        }
    }
    
    Ok(())
}

fn execute_command(
    bytecode: Option<String>,
    file: Option<PathBuf>,
    example: Option<String>,
    gas_limit: u64,
    value: u64,
    verbose: bool,
) -> Result<()> {
    let bytecode_hex = if let Some(bc) = bytecode {
        bc
    } else if let Some(path) = file {
        std::fs::read_to_string(path)?
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else if let Some(ex) = example {
        get_example_bytecode(&ex)?
    } else {
        anyhow::bail!("Must provide either --bytecode, --file, or --example");
    };

    println!("📋 {}: {}", "Bytecode".bright_yellow(), bytecode_hex);
    println!("⛽ {}: {}", "Gas Limit".bright_yellow(), gas_limit);
    println!("💰 {}: {} wei", "Value".bright_yellow(), value);
    println!();

    let bytecode = hex::decode(bytecode_hex.trim_start_matches("0x"))?;
    let mut executor = EvmExecutor::new(gas_limit);
    
    println!("{}", "🔄 Executing...".bright_green());
    let result = executor.execute(&bytecode, value, verbose)?;
    
    display_execution_result(&result);
    
    Ok(())
}

fn display_execution_result(result: &ExecutionResult) {
    println!("{}", "✨ Execution Results".bright_green().bold());
    println!("{}", "─".repeat(30).bright_green());
    
    match &result.status {
        ExecutionStatus::Success => {
            println!("Status: {}", "SUCCESS".bright_green().bold());
        }
        ExecutionStatus::Revert(reason) => {
            println!("Status: {}", "REVERTED".bright_red().bold());
            if !reason.is_empty() {
                println!("Reason: {}", reason.bright_red());
            }
        }
        ExecutionStatus::OutOfGas => {
            println!("Status: {}", "OUT OF GAS".bright_red().bold());
        }
        ExecutionStatus::Error(err) => {
            println!("Status: {}", "ERROR".bright_red().bold());
            println!("Error: {}", err.bright_red());
        }
    }
    
    println!("Gas Used: {}", result.gas_used.to_string().bright_cyan());
    println!("Gas Remaining: {}", result.gas_remaining.to_string().bright_cyan());
    
    if !result.return_data.is_empty() {
        println!("Return Data: 0x{}", hex::encode(&result.return_data).bright_blue());
    }
    
    if !result.logs.is_empty() {
        println!("\n📋 Logs:");
        for (i, log) in result.logs.iter().enumerate() {
            println!("  Log {}: {}", i, format!("{}", log).bright_magenta());
        }
    }
}

fn get_example_bytecode(example: &str) -> Result<String> {
    match example {
        "simple-add" => Ok("6001600201".to_string()), // PUSH1 1, PUSH1 2, ADD
        "simple-mul" => Ok("6002600302".to_string()), // PUSH1 2, PUSH1 3, MUL
        "storage" => Ok("6001600055600054".to_string()), // Simple storage example
        _ => anyhow::bail!("Unknown example: {}", example),
    }
}

fn compile_command(
    file: Option<PathBuf>,
    source: Option<String>,
    expression: Option<String>,
    output: Option<PathBuf>,
    debug: bool,
    run: bool,
    gas_limit: u64,
) -> Result<()> {
    println!("{}", "🔧 AbbyScript Compiler".bright_magenta().bold());
    println!("{}", "─".repeat(20).bright_blue());
    
    // Get source code
    let source_code = if let Some(file) = file {
        println!("Reading from file: {}", file.display().to_string().bright_cyan());
        std::fs::read_to_string(file)?
    } else if let Some(source) = source {
        source
    } else if let Some(expr) = &expression {
        expr.clone()
    } else {
        anyhow::bail!("Must provide either --file, --source, or --expression");
    };
    
    // Create compiler
    let compiler = Compiler::new().with_debug(debug);
    
    // Compile the code
    let bytecode = if expression.is_some() {
        println!("Compiling expression...");
        compiler.compile_expression(&source_code)
    } else {
        println!("Compiling program...");
        compiler.compile(&source_code)
    };
    
    let bytecode = match bytecode {
        Ok(bytecode) => bytecode,
        Err(e) => {
            eprintln!("{}", format!("Compilation failed: {}", e).bright_red().bold());
            return Ok(());
        }
    };
    
    // Display compilation results
    let hex_bytecode = hex::encode(&bytecode);
    println!("{}", "✨ Compilation successful!".bright_green().bold());
    println!("Generated {} bytes of bytecode", bytecode.len().to_string().bright_cyan());
    println!("Bytecode: 0x{}", hex_bytecode.bright_blue());
    
    // Save to output file if specified
    if let Some(output_path) = output {
        println!("Saving bytecode to: {}", output_path.display().to_string().bright_cyan());
        std::fs::write(&output_path, hex_bytecode.as_bytes())?;
        println!("{}", "Bytecode saved successfully!".bright_green());
    }
    
    // Execute if requested
    if run {
        println!("\n{}", "🚀 Executing compiled bytecode...".bright_yellow().bold());
        println!("{}", "─".repeat(35).bright_blue());
        
        let mut executor = EvmExecutor::new(gas_limit);
        let result = executor.execute(&bytecode, 0, debug)?;
        
        display_execution_result(&result);
    }
    
    Ok(())
}

async fn node_command(
    port: u16,
    validator: Option<String>,
    connect_peers: Vec<String>,
    db_path: Option<PathBuf>,
    mine: bool,
) -> Result<()> {
    use ethereum_types::Address;
    
    println!("{}", "🌐 Starting AbbyEVM Blockchain Node".bright_cyan().bold());
    println!("{}", "═".repeat(35).bright_blue());
    
    // Parse validator address if provided
    let validator_address = if let Some(addr_str) = validator {
        if addr_str.starts_with("0x") {
            Some(addr_str.parse::<Address>().map_err(|_| {
                anyhow::anyhow!("Invalid validator address format")
            })?)
        } else {
            Some(format!("0x{}", addr_str).parse::<Address>().map_err(|_| {
                anyhow::anyhow!("Invalid validator address format")
            })?)
        }
    } else if mine {
        // Generate a random validator address for mining
        Some(Address::random())
    } else {
        None
    };
    
    // Handle database path with default to ~/.ABBYCHAIN
    let db_path_str = if let Some(path) = db_path {
        path.to_str().unwrap_or("~/.ABBYCHAIN").to_string()
    } else {
        // Default to ~/.ABBYCHAIN
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.ABBYCHAIN", home_dir)
    };
    
    // Create the database directory if it doesn't exist
    if let Ok(expanded_path) = std::fs::canonicalize(&db_path_str) {
        // Path exists, use it as-is
    } else {
        // Path doesn't exist, create it
        if let Err(e) = std::fs::create_dir_all(&db_path_str) {
            log::warn!("Failed to create database directory {}: {}", db_path_str, e);
        }
    }
    
    // Initialize node
    println!("Initializing node on port {}...", port);
    println!("Database path: {}", db_path_str);
    if let Some(addr) = validator_address {
        println!("Validator address: {}", addr);
    }
    
    let node = AbbyNode::new(validator_address, port, Some(&db_path_str)).await
        .map_err(|e| anyhow::anyhow!("Failed to create node: {}", e))?;
    
    // Connect to peers
    for peer_addr in connect_peers {
        println!("Connecting to peer: {}", peer_addr);
        if let Err(e) = node.connect_to_peer(&peer_addr).await {
            log::warn!("Failed to connect to peer {}: {}", peer_addr, e);
        }
    }
    
    // Start the node
    println!("Starting blockchain node...");
    node.start().await
        .map_err(|e| anyhow::anyhow!("Failed to start node: {}", e))?;
    
    // Display node info
    let (chain_length, head_hash, block_count, abby_supply) = node.get_blockchain_info().await;
    println!("\n{}", "📊 Node Status".bright_green().bold());
    println!("Chain length: {} blocks", chain_length);
    println!("Head hash: {}", head_hash);
    println!("Total Abby supply: {} tokens", format_abby_amount(abby_supply));
    println!("Connected peers: {}", node.get_peer_count().await);
    
    if mine && validator_address.is_some() {
        println!("\n{}", "⛏️  Mining enabled".bright_yellow().bold());
    }
    
    // Keep the node running
    println!("\n{}", "Node is running... Press Ctrl+C to stop".bright_green());
    
    // Set up signal handling
    tokio::signal::ctrl_c().await
        .map_err(|e| anyhow::anyhow!("Failed to listen for ctrl-c: {}", e))?;
    
    println!("\nShutting down node...");
    Ok(())
}

fn format_abby_amount(amount: ethereum_types::U256) -> String {
    let decimals = ethereum_types::U256::from(1_000_000_000_000_000_000u64); // 18 decimals
    let whole = amount / decimals;
    let fractional = (amount % decimals) / ethereum_types::U256::from(1_000_000_000_000u64); // Show 6 decimal places
    
    format!("{}.{:06}", whole, fractional.as_u64())
}
