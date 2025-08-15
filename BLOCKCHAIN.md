# AbbyEVM Blockchain Node

AbbyEVM now includes a full Proof-of-Stake blockchain implementation with the "Abby" token for staking and rewards.

## Quick Overview

| Property | Value |
|----------|-------|
| Consensus | Proof-of-Stake |
| Storage | Persistent blockchain storage |
| Mining | Gas limit enforcement |
| State Management | Account balance tracking |
| Transaction Pool | Mempool management |
| Token Symbol | ABY |
| Token Decimals | 18 |
| Initial Supply | 1,000,000 tokens |
| Staking Rewards | 8% annual rate |
| Transaction Fees | Paid in Abby tokens |
| Minimum Stake | 32 Abby tokens |
| Validator Selection | Stake-weighted random |
| Reward Distribution | Automatic epoch-based |
| Slashing Protection | Validator misbehavior prevention |
| Delegation Support | Non-validator participation |
| Network Protocol | libp2p |
| Block Propagation | gossipsub |
| Chain Sync | Multi-node synchronization |
| Peer Discovery | Automatic management |

## Quick Start

### 1. Run a Single Node (Development)

```bash
# Start a node with mining enabled
cargo run -- node --mine --port 30303

# Start with a specific validator address
cargo run -- node --mine --validator 0x742d35Cc6C09b73C31342413B0d0a1a1C7a2b5C8

# Start with persistent storage (uses ~/.ABBYCHAIN by default)
cargo run -- node --mine

# Start with custom database path  
cargo run -- node --mine --db-path ./blockchain_data
```

### 2. Run Multiple Nodes (Network)

Terminal 1 (Bootstrap node):
```bash
cargo run -- node --mine --port 30303
```

Terminal 2 (Connect to bootstrap):
```bash
cargo run -- node --mine --port 30304 --connect /ip4/127.0.0.1/tcp/30303
```

Terminal 3 (Observer node):
```bash
cargo run -- node --port 30305 --connect /ip4/127.0.0.1/tcp/30303
```

### 3. Deploy Staking Contract

```bash
# Compile and deploy the Abby staking contract
cargo run -- compile --file examples/abbyscript/abby_staking.abs --run
```

## Staking Guide

### Become a Validator

1. **Acquire Abby tokens** (minimum 32 tokens)
2. **Stake your tokens** to become eligible
3. **Start your node** with validator mode
4. **Earn rewards** automatically each epoch

### Example Staking Flow

```javascript
// In AbbyScript
let stakeAmount = 50; // 50 Abby tokens

// Stake tokens
stake(stakeAmount);

// Become validator (requires minimum stake)
becomeValidator();

// Calculate potential rewards
let rewards = calculateRewards(myAddress);

// Claim rewards
claimRewards();
```

## Network Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Validator A    │    │  Validator B    │    │  Observer Node  │
│  (Mining)       │◄──►│  (Mining)       │◄──►│  (Non-mining)   │
│  Port: 30303    │    │  Port: 30304    │    │  Port: 30305    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   P2P Network   │
                    │   (Gossipsub)   │
                    └─────────────────┘
```

## Block Structure

```rust
BlockHeader {
    number: u64,           // Block number
    parent_hash: H256,     // Previous block hash
    timestamp: DateTime,   // When block was created
    proposer: Address,     // Validator who proposed block
    gas_limit: U256,       // Max gas for all transactions
    gas_used: U256,        // Actual gas consumed
    abby_reward: U256,     // Abby tokens rewarded to proposer
}

Block {
    header: BlockHeader,
    transactions: Vec<Transaction>,
    validators: Vec<ValidatorInfo>, // Consensus participants
}
```

## Transaction Types

### 1. Simple Transfer
```rust
Transaction {
    from: Address,
    to: Some(Address),
    value: U256,        // Amount in wei
    gas_limit: U256,
    gas_price: U256,
    data: Vec::new(),   // Empty for transfers
    abby_fee: U256,     // Fee in Abby tokens
}
```

### 2. Smart Contract Call
```rust
Transaction {
    from: Address,
    to: Some(contract_address),
    value: U256,
    gas_limit: U256,
    gas_price: U256,
    data: Vec<u8>,      // Contract call data
    abby_fee: U256,
}
```

### 3. Contract Deployment
```rust
Transaction {
    from: Address,
    to: None,           // None for contract creation
    value: U256,
    gas_limit: U256,
    gas_price: U256,
    data: Vec<u8>,      // Contract bytecode
    abby_fee: U256,
}
```

## Consensus Algorithm

### Slot-Based PoS
1. **Time divided into slots** (12 seconds each)
2. **Epoch = 32 slots** (6.4 minutes)
3. **Proposer selection** based on stake weight and randomness
4. **Block validation** by other validators
5. **Rewards distributed** at epoch boundaries

### Validator Selection
```rust
fn select_proposer(slot: u64, randomness: &[u8]) -> Address {
    // Deterministic selection based on:
    // - Current slot number
    // - Block hash randomness
    // - Validator stake weights
}
```

## Token Economics

### Abby Token (ABY)
- **Symbol**: ABY
- **Decimals**: 18
- **Total Supply**: 1,000,000 ABY (initially)
- **Inflation**: ~8% annual through staking rewards

### Reward Distribution
- **Block Reward**: 1 ABY per block
- **Staking Rewards**: 8% APY for validators
- **Transaction Fees**: Paid in ABY tokens
- **Gas Rewards**: 1 ABY per 1000 gas used

### Economic Model
```
Genesis Distribution:
├── 100,000 ABY → Early validators
├── 50,000 ABY  → Development fund  
├── 25,000 ABY  → Community rewards
└── 825,000 ABY → Public distribution

Ongoing Inflation:
├── 60% → Validator rewards
├── 20% → Delegator rewards
├── 15% → Development fund
└── 5%  → Governance treasury
```

## API Reference

### Node Commands
```bash
# Start node
cargo run -- node [OPTIONS]

Options:
  -p, --port <PORT>           Network port [default: 30303]
  -v, --validator <ADDRESS>   Validator address
  -c, --connect <PEERS>       Peer addresses to connect to
  -d, --db-path <PATH>        Database path for persistence
  -m, --mine                  Enable mining mode
```

### Node Information
```bash
# Check node status
curl http://localhost:8545/status

# Get blockchain info
curl http://localhost:8545/blockchain

# Get validator info
curl http://localhost:8545/validators
```

## Development

### Running Tests
```bash
# Run all tests
cargo test

# Run blockchain-specific tests
cargo test blockchain

# Run network tests
cargo test network
```

### Building
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build with networking features
cargo build --features network
```

## Configuration

### Genesis Configuration
```rust
// Initial validator set
validators: [
    {
        address: "0x742d35Cc6C09b73C31342413B0d0a1a1C7a2b5C8",
        stake: "32000000000000000000", // 32 ABY
    }
],

// Chain parameters  
consensus: {
    block_time: 12,        // seconds
    slots_per_epoch: 32,   // ~6.4 minutes
    reward_rate: 800,      // 8% (basis points)
}
```

### Network Configuration
```toml
[network]
listen_port = 30303
bootstrap_peers = [
    "/ip4/127.0.0.1/tcp/30303",
]
max_peers = 50
```

## Security Considerations

### Validator Security
- **Key Management**: Secure validator private keys
- **Slashing Conditions**: Double signing, long-range attacks
- **Network Security**: DDoS protection, peer filtering

### Smart Contract Security
- **Gas Limits**: Prevent infinite loops
- **Reentrancy**: Guard against recursive calls  
- **Integer Overflow**: Safe math operations

## Roadmap

### Phase 1: Core Blockchain ✅
- [x] Basic blockchain implementation
- [x] Proof-of-Stake consensus
- [x] Abby token integration
- [x] P2P networking

### Phase 2: Advanced Features 🔄
- [ ] Light client support
- [ ] State pruning
- [ ] Advanced cryptography (BLS signatures)
- [ ] Cross-chain bridges

### Phase 3: Ecosystem 📋
- [ ] Block explorer
- [ ] Wallet integration
- [ ] DEX implementation
- [ ] Governance system


## License

MIT License