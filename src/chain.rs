use std::{fs::File, io::Read, path::Path, sync::Arc};

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use parking_lot::RwLock;
use primitive_types::U256;
use sled::Db;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use crate::block::{BlockArc, DerivativeBlock};
use crate::dump_headers::Headers;
use crate::{
    block::{self, BasicInfo, Block, SummarizeBlock},
    errors::{BlockChainTreeError, ChainErrorKind},
    merkletree::MerkleTree,
    tools,
    transaction::Transactionable,
};
use crate::{static_values::*, transaction};

#[async_trait]
pub trait Chain {
    async fn dump_config(&self) -> Result<(), Report<BlockChainTreeError>>;
    async fn flush(&self) -> Result<(), Report<BlockChainTreeError>>;
    async fn add_block(
        &self,
        block: &(impl Block + Sync),
    ) -> Result<(), Report<BlockChainTreeError>>;
    async fn find_raw_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>>;
    async fn find_raw_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>>;
    async fn get_last_raw_block(&self) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>>;
    async fn get_last_block(
        &self,
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>>;
    async fn find_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>>;
    async fn find_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>>;
}

#[derive(Clone)]
pub struct MainChain {
    blocks: Db,
    height_reference: Db,
    transactions: Db,
    height: Arc<RwLock<U256>>,
    difficulty: Arc<RwLock<[u8; 32]>>,
    root: String,
}

impl MainChain {
    pub fn new(root: &str) -> Result<Self, Report<BlockChainTreeError>> {
        let root = Path::new(root).join(MAIN_CHAIN_DIRECTORY);

        let path_blocks_st = root.join(BLOCKS_FOLDER);
        let path_references_st = root.join(REFERENCES_FOLDER);
        let path_transactions_st = root.join(TRANSACTIONS_FOLDER);
        let path_height_st = root.join(CONFIG_FILE);

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);
        let path_transactions = Path::new(&path_transactions_st);
        let path_height = Path::new(&path_height_st);

        // open blocks DB
        let blocks = sled::open(path_blocks)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = sled::open(path_reference)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open references db")?;

        // open transactions DB
        let transactions = sled::open(path_transactions)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open transactions db")?;

        let file = File::open(path_height);

        let (height, difficulty) = if let Ok(mut file) = file {
            let mut height_bytes: [u8; 32] = [0; 32];
            file.read_exact(&mut height_bytes)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("failed to read config")?;

            // read difficulty
            let mut difficulty: [u8; 32] = [0; 32];
            file.read_exact(&mut difficulty)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("failed to read difficulty")?;

            (U256::from_big_endian(&height_bytes), difficulty)
        } else {
            (U256::zero(), BEGINNING_DIFFICULTY)
        };

        let chain = Self {
            blocks,
            height_reference,
            transactions,
            height: Arc::new(RwLock::new(height)),
            difficulty: Arc::new(RwLock::new(difficulty)),
            root: root.to_str().unwrap().to_string(),
        };
        if height.is_zero() {
            let info = BasicInfo::new(
                INCEPTION_TIMESTAMP,
                [0; 32],
                [0u8; 32],
                U256::zero(),
                BEGINNING_DIFFICULTY,
                ROOT_PUBLIC_ADDRESS,
            );
            let mut initial_amount = Vec::<u8>::new();
            initial_amount.extend(ROOT_PUBLIC_ADDRESS.iter());
            initial_amount.push(b'|');
            initial_amount.extend(COINS_PER_CYCLE.to_string().as_bytes().iter());
            initial_amount.push(b'|');
            initial_amount.push(b'0');

            let merkle_tree = MerkleTree::build_tree(&[tools::hash(&initial_amount)]);
            chain
                .add_block(Arc::new(SummarizeBlock {
                    default_info: info,
                    merkle_tree_root: *merkle_tree.get_root(),
                }))
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("Failed to insert inception block")?;
        }

        Ok(chain)
    }
    /// Dump config
    ///
    /// Dumps chain's config
    async fn dump_config(&self) -> Result<(), Report<BlockChainTreeError>> {
        let path_config = Path::new(&self.root).join(CONFIG_FILE);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path_config)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))?;
        let mut buffer_32_bytes: [u8; 32] = [0; 32];
        self.height.read().to_big_endian(&mut buffer_32_bytes);
        file.write_all(&buffer_32_bytes)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write height")?;

        let difficulty = *self.difficulty.read();
        file.write_all(&difficulty)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write difficulty")?;

        Ok(())
    }

    pub fn get_height(&self) -> U256 {
        *self.height.read()
    }

    /// Flushes all DBs and config
    pub async fn flush(&self) -> Result<(), Report<BlockChainTreeError>> {
        self.dump_config().await?;

        self.blocks
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to flush db")?;

        self.height_reference
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to flush height references")?;

        self.transactions
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to flush transactions")?;

        Ok(())
    }

    pub fn add_transactions(
        &self,
        transactions: &[impl transaction::Transactionable],
    ) -> Result<(), Report<BlockChainTreeError>> {
        for transaction in transactions {
            let dump = transaction
                .dump()
                .change_context(BlockChainTreeError::Chain(
                    ChainErrorKind::AddingTransaction,
                ))?;
            self.transactions
                .insert(tools::hash(&dump), dump)
                .change_context(BlockChainTreeError::Chain(
                    ChainErrorKind::AddingTransaction,
                ))
                .attach_printable("Failed to insert transaction")?;
        }
        Ok(())
    }

    pub fn add_transaction(
        &self,
        transaction: &dyn transaction::Transactionable,
    ) -> Result<(), Report<BlockChainTreeError>> {
        let dump = transaction
            .dump()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::AddingTransaction,
            ))?;
        self.transactions
            .insert(tools::hash(&dump), dump)
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::AddingTransaction,
            ))
            .attach_printable("Failed to insert transaction")?;
        Ok(())
    }

    pub fn transaction_exists(
        &self,
        transaction_hash: &[u8; 32],
    ) -> Result<bool, Report<BlockChainTreeError>> {
        self.transactions
            .contains_key(transaction_hash)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))
    }

    pub fn get_transaction_raw(
        &self,
        transaction_hash: &[u8; 32],
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let transaction = self
            .transactions
            .get(transaction_hash)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?;
        Ok(transaction.map(|v| v.to_vec()))
    }

    pub fn get_transaction(
        &self,
        transaction_hash: &[u8; 32],
    ) -> Result<Option<transaction::Transaction>, Report<BlockChainTreeError>> {
        let raw_transaction = self.get_transaction_raw(transaction_hash)?;

        if let Some(tr) = raw_transaction {
            if !tr.first().unwrap_or(&10).eq(&(Headers::Transaction as u8)) {
                return Err(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE).into());
            }
            return Ok(Some(
                transaction::Transaction::parse(&tr[1..])
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?,
            ));
        }

        Ok(None)
    }

    /// Adds new block to the chain db
    ///
    /// Adds block and sets height reference for it
    ///
    /// Checks for blocks validity, adds it directly to the end of the chain
    pub fn add_block(&self, block: BlockArc) -> Result<(), Report<BlockChainTreeError>> {
        let dump = block
            .dump()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        let hash = tools::hash(&dump);

        let mut height = self.height.write();

        if block.get_info().height != *height {
            return Err(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock)).attach_printable(
                "The height of the chain is different from the height of the block",
            );
        }

        let mut height_bytes = [0u8; 32];
        height.to_big_endian(&mut height_bytes);

        self.blocks
            .insert(height_bytes, dump)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
            .attach_printable("Failed to insert block to blocks db")?;

        self.height_reference
            .insert(hash, &height_bytes)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
            .attach_printable("Failed to insert height reference for the block")?;

        *height += U256::one();

        Ok(())
    }

    /// Get serialized block by it's height
    pub fn find_raw_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let chain_height = self.height.read();
        if height > &chain_height {
            return Ok(None);
        }
        drop(chain_height);

        let mut height_serialized = [0u8; 32];
        height.to_big_endian(&mut height_serialized);
        let mut dump = self
            .blocks
            .get(height_serialized)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?;

        if let Some(dump) = dump.take() {
            return Ok(Some(dump.to_vec()));
        }
        Ok(None)
    }

    /// Get serialized block by it's hash
    pub fn find_raw_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let height = match self
            .height_reference
            .get(hash)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?
        {
            None => {
                return Ok(None);
            }
            Some(h) => U256::from_big_endian(&h.iter().copied().collect::<Vec<u8>>()),
        };

        let block = self
            .find_raw_by_height(&height)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?;

        Ok(block)
    }

    pub fn find_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>> {
        let dump = self.find_raw_by_hash(hash)?;

        let deserialized = if let Some(data) = dump {
            Some(
                block::deserialize_main_chain_block(&data)
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))
                    .attach_printable(format!(
                        "Failed to deserialize latest main chain block with hash {:?}",
                        hash
                    ))?,
            )
        } else {
            None
        };

        Ok(deserialized)
    }

    /// Get serialized last block of the chain
    pub fn get_last_raw_block(&self) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let height = self.height.read();
        let last_block_index = *height - 1;
        drop(height);

        self.find_raw_by_height(&last_block_index)
    }

    /// Get deserialized latest block
    pub fn get_last_block(
        &self,
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>> {
        let dump = self.get_last_raw_block()?;

        let deserialized = if let Some(data) = dump {
            Some(
                block::deserialize_main_chain_block(&data)
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))
                    .attach_printable("Failed to deserialize latest main chain block")?,
            )
        } else {
            None
        };

        Ok(deserialized)
    }

    /// Get deserialized block by height
    pub fn find_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>> {
        let dump = self.find_raw_by_height(height)?;

        let deserialized = if let Some(data) = dump {
            Some(
                block::deserialize_main_chain_block(&data)
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))
                    .attach_printable(format!(
                        "Failed to deserialize main chain block with height {}",
                        height
                    ))?,
            )
        } else {
            None
        };

        Ok(deserialized)
    }
}

#[derive(Clone)]
pub struct DerivativeChain {
    blocks: Db,
    height_reference: Db,
    height: Arc<RwLock<U256>>,
    pub genesis_hash: Arc<[u8; 32]>,
    difficulty: Arc<RwLock<[u8; 32]>>,
    chain_owner: String,
    root: String,
}

impl DerivativeChain {
    pub fn new(
        root: &str,
        chain_owner: &str,
        provided_genesis_hash: &[u8; 32],
    ) -> Result<Self, Report<BlockChainTreeError>> {
        let root = Path::new(root)
            .join(DERIVATIVE_CHAINS_DIRECTORY)
            .join(chain_owner);

        let path_blocks_st = root.join(BLOCKS_FOLDER);
        let path_references_st = root.join(REFERENCES_FOLDER);
        let path_height_st = root.join(CONFIG_FILE);

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);
        let path_height = Path::new(&path_height_st);

        // open blocks DB
        let blocks = sled::open(path_blocks)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = sled::open(path_reference)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open references db")?;

        let file = File::open(path_height);

        let (height, difficulty, genesis_hash) = if let Ok(mut file) = file {
            let mut height_bytes: [u8; 32] = [0; 32];
            file.read_exact(&mut height_bytes)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("failed to read config")?;

            // read difficulty
            let mut difficulty: [u8; 32] = [0; 32];
            file.read_exact(&mut difficulty)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("failed to read difficulty")?;

            // read difficulty
            let mut genesis_hash: [u8; 32] = [0; 32];
            file.read_exact(&mut genesis_hash)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("failed to read genesis hash")?;

            (
                U256::from_big_endian(&height_bytes),
                difficulty,
                genesis_hash,
            )
        } else {
            (U256::zero(), BEGINNING_DIFFICULTY, *provided_genesis_hash)
        };

        let chain = Self {
            blocks,
            height_reference,
            height: Arc::new(RwLock::new(height)),
            difficulty: Arc::new(RwLock::new(difficulty)),
            genesis_hash: Arc::new(genesis_hash),
            chain_owner: chain_owner.to_string(),
            root: root.to_str().unwrap().to_string(),
        };

        Ok(chain)
    }

    pub fn get_height(&self) -> U256 {
        *self.height.read()
    }

    /// Dump config
    ///
    /// Dumps chain's config
    async fn dump_config(&self) -> Result<(), Report<BlockChainTreeError>> {
        let path_config = Path::new(&self.root).join(CONFIG_FILE);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path_config)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))?;
        let mut buffer_32_bytes: [u8; 32] = [0; 32];
        self.height.read().to_big_endian(&mut buffer_32_bytes);
        file.write_all(&buffer_32_bytes)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write height")?;

        let difficulty = *self.difficulty.read();
        file.write_all(&difficulty)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write difficulty")?;

        file.write_all(self.genesis_hash.as_ref())
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write genesis hash")?;

        Ok(())
    }

    /// Flushes all DBs and config
    pub async fn flush(&self) -> Result<(), Report<BlockChainTreeError>> {
        self.dump_config().await?;

        self.blocks
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to flush db")?;

        self.height_reference
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to flush height references")?;

        Ok(())
    }

    /// Adds new block to the chain db
    ///
    /// Adds block and sets heigh reference for it
    ///
    /// Checks for blocks validity, adds it directly to the end of the chain
    pub fn add_block(&self, block: &DerivativeBlock) -> Result<(), Report<BlockChainTreeError>> {
        let dump = block
            .dump()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        let hash = tools::hash(&dump);

        let mut height = self.height.write();

        if block.get_info().height != *height {
            println!("{} {}", block.get_info().height, *height);
            return Err(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock)).attach_printable(
                "The height of the chain is different from the height of the block",
            );
        }

        let mut height_bytes = [0u8; 32];
        height.to_big_endian(&mut height_bytes);

        self.blocks
            .insert(height_bytes, dump)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
            .attach_printable("Failed to insert block to blocks db")?;

        self.height_reference
            .insert(hash, &height_bytes)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
            .attach_printable("Failed to insert height reference for the block")?;

        *height += U256::one();

        Ok(())
    }

    /// Get serialized block by it's height
    pub fn find_raw_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let chain_height = self.height.read();
        if height > &chain_height {
            return Ok(None);
        }
        drop(chain_height);

        let mut height_serialized = [0u8; 32];
        height.to_big_endian(&mut height_serialized);
        let mut dump = self
            .blocks
            .get(height_serialized)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?;

        if let Some(dump) = dump.take() {
            return Ok(Some(dump.to_vec()));
        }
        Ok(None)
    }

    /// Get serialized block by it's hash
    pub fn find_raw_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let height = match self
            .height_reference
            .get(hash)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?
        {
            None => {
                return Ok(None);
            }
            Some(h) => U256::from_big_endian(&h.iter().copied().collect::<Vec<u8>>()),
        };

        let block = self
            .find_raw_by_height(&height)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?;

        Ok(block)
    }

    pub fn find_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Arc<DerivativeBlock>>, Report<BlockChainTreeError>> {
        let dump = self.find_raw_by_hash(hash)?;

        let deserialized = if let Some(data) = dump {
            Some(Arc::new(
                block::DerivativeBlock::parse(&data[1..])
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))
                    .attach_printable(format!(
                        "Failed to deserialize latest main chain block with hash {:?}",
                        hash
                    ))?,
            ))
        } else {
            None
        };

        Ok(deserialized)
    }

    /// Get serialized last block of the chain
    pub fn get_last_raw_block(&self) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let height = self.height.read();
        if height.is_zero() {
            return Ok(None);
        }
        let last_block_index = *height - 1;
        drop(height);

        self.find_raw_by_height(&last_block_index)
    }

    /// Get deserialized latest block
    pub fn get_last_block(&self) -> Result<Option<DerivativeBlock>, Report<BlockChainTreeError>> {
        let dump = self.get_last_raw_block()?;

        let deserialized = if let Some(data) = dump {
            Some(
                block::DerivativeBlock::parse(&data[1..])
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))
                    .attach_printable(
                        "Failed to deserialize latest main chain block".to_string(),
                    )?,
            )
        } else {
            None
        };

        Ok(deserialized)
    }

    /// Get deserialized block by height
    pub fn find_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Arc<DerivativeBlock>>, Report<BlockChainTreeError>> {
        let dump = self.find_raw_by_height(height)?;

        let deserialized = if let Some(data) = dump {
            Some(Arc::new(
                block::DerivativeBlock::parse(&data[1..])
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))
                    .attach_printable(format!(
                        "Failed to deserialize deriv chain block with height {}",
                        height
                    ))?,
            ))
        } else {
            None
        };

        Ok(deserialized)
    }
}
