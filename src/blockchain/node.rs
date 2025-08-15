use crate::blockchain::{
    Blockchain, Block, BlockHeader, Transaction, TransactionPool, 
    ConsensusState, StakingManager, 
    network::{NetworkManager, NetworkMessage, SyncManager}
};
use ethereum_types::{Address, H256, U256};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration};

pub struct AbbyNode {
    pub blockchain: Arc<RwLock<Blockchain>>,
    pub consensus: Arc<RwLock<ConsensusState>>,
    pub staking: Arc<RwLock<StakingManager>>,
    pub tx_pool: Arc<Mutex<TransactionPool>>,
    pub network: Arc<Mutex<NetworkManager>>,
    pub sync_manager: Arc<Mutex<SyncManager>>,
    pub validator_address: Option<Address>,
    pub is_mining: Arc<Mutex<bool>>,
    pub node_id: String,
}

impl AbbyNode {
    pub async fn new(
        validator_address: Option<Address>,
        network_port: u16,
        db_path: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize blockchain
        let blockchain = if let Some(path) = db_path {
            Arc::new(RwLock::new(Blockchain::new_with_persistence(path)?))
        } else {
            Arc::new(RwLock::new(Blockchain::new().map_err(|e| format!("Failed to create blockchain: {}", e))?))
        };
        
        // Initialize consensus
        let consensus = Arc::new(RwLock::new(ConsensusState::new()));
        
        // Initialize staking
        let staking = Arc::new(RwLock::new(StakingManager::new()));
        
        // Initialize transaction pool
        let tx_pool = Arc::new(Mutex::new(TransactionPool::new()));
        
        // Initialize network
        let mut network_manager = NetworkManager::new()?;
        network_manager.start_listening(network_port)?;
        let network = Arc::new(Mutex::new(network_manager));
        
        // Initialize sync manager
        let sync_manager = Arc::new(Mutex::new(SyncManager::new()));
        
        let node_id = format!("abby-node-{}", rand::random::<u32>());
        
        let node = Self {
            blockchain,
            consensus,
            staking,
            tx_pool,
            network,
            sync_manager,
            validator_address,
            is_mining: Arc::new(Mutex::new(false)),
            node_id,
        };
        
        // If we have a validator address, add it to consensus
        if let Some(addr) = validator_address {
            node.initialize_validator(addr).await?;
        }
        
        log::info!("AbbyNode {} initialized", node.node_id);
        Ok(node)
    }
    
    async fn initialize_validator(&self, address: Address) -> Result<(), Box<dyn std::error::Error>> {
        let initial_stake = U256::from_dec_str("32000000000000000000").unwrap(); // 32 Abby tokens
        
        let mut staking = self.staking.write().await;
        let mut consensus = self.consensus.write().await;
        
        staking.create_validator(address, initial_stake, &mut consensus)
            .map_err(|e| format!("Failed to create validator: {}", e))?;
        
        log::info!("Initialized validator {}", address);
        Ok(())
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting AbbyNode {}", self.node_id);
        
        // Start mining if we're a validator
        if self.validator_address.is_some() {
            self.start_mining().await;
        }
        
        // Start network event handler
        self.start_network_handler().await;
        
        // Start sync process
        self.start_sync_handler().await;
        
        // Start periodic tasks
        self.start_periodic_tasks().await;
        
        log::info!("AbbyNode {} started successfully", self.node_id);
        Ok(())
    }
    
    async fn start_mining(&self) {
        let blockchain = Arc::clone(&self.blockchain);
        let consensus = Arc::clone(&self.consensus);
        let tx_pool = Arc::clone(&self.tx_pool);
        let network = Arc::clone(&self.network);
        let is_mining = Arc::clone(&self.is_mining);
        let validator_address = self.validator_address;
        
        tokio::spawn(async move {
            let mut mining_interval = interval(Duration::from_secs(12)); // 12 second block time
            
            loop {
                mining_interval.tick().await;
                
                let mining_flag = is_mining.lock().await;
                if !*mining_flag {
                    continue;
                }
                drop(mining_flag);
                
                if let Some(validator_addr) = validator_address {
                    if let Err(e) = Self::mine_block(
                        &blockchain,
                        &consensus,
                        &tx_pool,
                        &network,
                        validator_addr,
                    ).await {
                        log::error!("Mining error: {}", e);
                    }
                }
            }
        });
        
        *self.is_mining.lock().await = true;
        log::info!("Mining started for validator {:?}", self.validator_address);
    }
    
    async fn mine_block(
        blockchain: &Arc<RwLock<Blockchain>>,
        consensus: &Arc<RwLock<ConsensusState>>,
        tx_pool: &Arc<Mutex<TransactionPool>>,
        network: &Arc<Mutex<NetworkManager>>,
        validator_address: Address,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let blockchain_read = blockchain.read().await;
        let consensus_read = consensus.read().await;
        
        // Check if we're the selected proposer for this slot
        let current_slot = consensus_read.current_slot;
        let randomness = blockchain_read.head_hash.as_bytes();
        let selected_proposer = consensus_read.select_proposer(current_slot, randomness);
        
        if selected_proposer != Some(validator_address) {
            return Ok(()); // Not our turn to propose
        }
        
        drop(consensus_read);
        
        // Get head block
        let head_block = blockchain_read.get_head_block()
            .ok_or("No head block found")?
            .clone();
        
        let head_hash = head_block.hash();
        let next_number = head_block.header.number + 1;
        drop(blockchain_read);
        
        // Select transactions from pool
        let tx_pool_lock = tx_pool.lock().await;
        let gas_limit = U256::from(10_000_000u64); // 10M gas limit
        let transactions = tx_pool_lock.select_transactions_for_block(gas_limit);
        drop(tx_pool_lock);
        
        // Create block header
        let header = BlockHeader::new(next_number, head_hash, validator_address, gas_limit);
        
        // Create block
        let block = Block::new(header, transactions.clone());
        
        // Validate with consensus
        let consensus_read = consensus.read().await;
        consensus_read.validate_proposal(&block, &validator_address)?;
        drop(consensus_read);
        
        // Add block to blockchain
        let mut blockchain_write = blockchain.write().await;
        blockchain_write.add_block(block.clone())?;
        drop(blockchain_write);
        
        // Remove processed transactions from pool
        let mut tx_pool_lock = tx_pool.lock().await;
        for tx in &transactions {
            tx_pool_lock.remove_transaction(&tx.hash());
        }
        drop(tx_pool_lock);
        
        // Broadcast block to network
        let mut network_lock = network.lock().await;
        network_lock.broadcast_block(block.clone())?;
        drop(network_lock);
        
        // Advance consensus slot
        let mut consensus_write = consensus.write().await;
        consensus_write.advance_slot();
        drop(consensus_write);
        
        log::info!("Mined block #{} with {} transactions", 
                  block.header.number, transactions.len());
        
        Ok(())
    }
    
    async fn start_network_handler(&self) {
        let blockchain = Arc::clone(&self.blockchain);
        let tx_pool = Arc::clone(&self.tx_pool);
        let sync_manager = Arc::clone(&self.sync_manager);
        let network: Arc<Mutex<NetworkManager>> = Arc::clone(&self.network);
        
        tokio::spawn(async move {
            let mut network_lock = network.lock().await;
            let mut message_receiver = network_lock.message_receiver.take()
                .expect("Message receiver should be available");
            drop(network_lock);
            
            while let Some(message) = message_receiver.recv().await {
                if let Err(e) = Self::handle_network_message(
                    message,
                    &blockchain,
                    &tx_pool,
                    &sync_manager,
                    &network,
                ).await {
                    log::error!("Error handling network message: {}", e);
                }
            }
        });
    }
    
    async fn handle_network_message(
        message: NetworkMessage,
        blockchain: &Arc<RwLock<Blockchain>>,
        tx_pool: &Arc<Mutex<TransactionPool>>,
        _sync_manager: &Arc<Mutex<SyncManager>>,
        _network: &Arc<Mutex<NetworkManager>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match message {
            NetworkMessage::NewBlock(block) => {
                log::info!("Received new block #{}", block.header.number);
                
                let mut blockchain_write = blockchain.write().await;
                if let Err(e) = blockchain_write.add_block(block) {
                    log::warn!("Failed to add received block: {}", e);
                }
            }
            
            NetworkMessage::NewTransaction(transaction) => {
                log::debug!("Received new transaction {}", transaction.hash());
                
                let mut tx_pool_lock = tx_pool.lock().await;
                if let Err(e) = tx_pool_lock.add_transaction(transaction) {
                    log::warn!("Failed to add received transaction: {}", e);
                }
            }
            
            NetworkMessage::BlockRequest { hash } => {
                let blockchain_read = blockchain.read().await;
                let block = blockchain_read.get_block(&hash).cloned();
                drop(blockchain_read);
                
                let _response = NetworkMessage::BlockResponse { block };
                // In simplified implementation, we'd send this back to the requesting peer
                log::info!("Would send block response for {} (simplified)", hash);
            }
            
            NetworkMessage::BlockResponse { block } => {
                if let Some(block) = block {
                    let mut blockchain_write = blockchain.write().await;
                    if let Err(e) = blockchain_write.add_block(block) {
                        log::warn!("Failed to add block from response: {}", e);
                    }
                }
            }
            
            NetworkMessage::PeerInfo { chain_head: _, chain_length: _ } => {
                // Update sync manager with peer info
                // This would need the peer ID, which isn't available in this context
                // In a real implementation, we'd need to modify the message structure
            }
            
            NetworkMessage::SyncRequest { from_block, to_block } => {
                let blockchain_read = blockchain.read().await;
                let mut blocks = Vec::new();
                
                for block_num in from_block..=to_block.min(blockchain_read.head_number) {
                    if let Some(block) = blockchain_read.get_block_by_number(block_num) {
                        blocks.push(block.clone());
                    }
                }
                drop(blockchain_read);
                
                let blocks_len = blocks.len();
                let _response = NetworkMessage::SyncResponse { blocks };
                // In simplified implementation, we'd send this back to the requesting peer
                log::info!("Would send sync response with {} blocks (simplified)", blocks_len);
            }
            
            NetworkMessage::SyncResponse { blocks } => {
                let mut blockchain_write = blockchain.write().await;
                for block in blocks {
                    if let Err(e) = blockchain_write.add_block(block) {
                        log::warn!("Failed to add sync block: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn start_sync_handler(&self) {
        let blockchain = Arc::clone(&self.blockchain);
        let sync_manager: Arc<Mutex<SyncManager>> = Arc::clone(&self.sync_manager);
        let network: Arc<Mutex<NetworkManager>> = Arc::clone(&self.network);
        
        tokio::spawn(async move {
            let mut sync_interval = interval(Duration::from_secs(30));
            
            loop {
                sync_interval.tick().await;
                
                let blockchain_read = blockchain.read().await;
                let our_length = blockchain_read.get_chain_length();
                let chain_head = blockchain_read.head_hash;
                drop(blockchain_read);
                
                let mut sync_manager_lock = sync_manager.lock().await;
                sync_manager_lock.cleanup_stale_peers(Duration::from_secs(300)); // 5 minutes
                
                if sync_manager_lock.should_sync(our_length) {
                    if let Some(best_peer) = sync_manager_lock.get_best_peer(our_length) {
                        let target_block = best_peer.chain_length;
                        sync_manager_lock.start_sync(target_block);
                        
                        let mut network_lock = network.lock().await;
                        if let Err(e) = network_lock.sync_request(our_length, target_block) {
                            log::error!("Failed to send sync request: {}", e);
                        }
                        drop(network_lock);
                    }
                }
                drop(sync_manager_lock);
                
                // Announce our chain info
                let mut network_lock = network.lock().await;
                if let Err(e) = network_lock.announce_peer_info(chain_head, our_length) {
                    log::error!("Failed to announce peer info: {}", e);
                }
            }
        });
    }
    
    async fn start_periodic_tasks(&self) {
        let staking = Arc::clone(&self.staking);
        let _consensus = Arc::clone(&self.consensus);
        
        tokio::spawn(async move {
            let mut reward_interval = interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                reward_interval.tick().await;
                
                // Distribute staking rewards
                let staking_lock = staking.write().await;
                // Process pending rewards, withdrawals, etc.
                // This would include more complex reward distribution logic
                drop(staking_lock);
                
                log::debug!("Processed periodic staking rewards");
            }
        });
    }
    
    pub async fn submit_transaction(&self, transaction: Transaction) -> Result<H256, String> {
        let mut tx_pool = self.tx_pool.lock().await;
        let tx_hash = transaction.hash();
        tx_pool.add_transaction(transaction.clone())?;
        drop(tx_pool);
        
        // Broadcast transaction to network
        let mut network = self.network.lock().await;
        network.broadcast_transaction(transaction)
            .map_err(|e| format!("Failed to broadcast transaction: {}", e))?;
        
        Ok(tx_hash)
    }
    
    pub async fn get_balance(&self, address: &Address) -> U256 {
        let blockchain = self.blockchain.read().await;
        blockchain.get_abby_balance(address)
    }
    
    pub async fn transfer_abby(&self, from: &Address, to: &Address, amount: U256) -> Result<H256, String> {
        // Create a transfer transaction
        let nonce = U256::zero(); // Simplified - should get actual nonce
        let gas_limit = U256::from(21000);
        let gas_price = U256::from(1_000_000_000u64);
        
        let transaction = Transaction::new(
            *from,
            Some(*to),
            amount,
            gas_limit,
            gas_price,
            Vec::new(),
            nonce,
        );
        
        self.submit_transaction(transaction).await
    }
    
    pub async fn stake_tokens(
        &self,
        staker: Address,
        validator: Address,
        amount: U256,
    ) -> Result<(), String> {
        let mut staking = self.staking.write().await;
        let mut consensus = self.consensus.write().await;
        
        staking.stake(staker, validator, amount, &mut consensus)
    }
    
    pub async fn get_validator_info(&self, address: &Address) -> Option<crate::blockchain::Validator> {
        let consensus = self.consensus.read().await;
        consensus.get_validator(address).cloned()
    }
    
    pub async fn get_blockchain_info(&self) -> (u64, H256, u64, U256) {
        let blockchain = self.blockchain.read().await;
        (
            blockchain.get_chain_length(),
            blockchain.head_hash,
            blockchain.blocks.len() as u64,
            blockchain.get_total_abby_supply(),
        )
    }
    
    pub async fn connect_to_peer(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut network = self.network.lock().await;
        network.dial_peer(address)
    }
    
    pub async fn get_peer_count(&self) -> usize {
        let network = self.network.lock().await;
        network.peer_count()
    }
}
