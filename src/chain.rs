use std::{fs::File, io::Read, path::Path, sync::Arc};

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use primitive_types::U256;
use sled::Db;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::RwLock};

use crate::static_values::*;
use crate::{
    block::{self, BasicInfo, Block, SummarizeBlock, TransactionBlock},
    errors::{BlockChainTreeError, ChainErrorKind},
    errors::DerivChainErrorKind,
    merkletree::MerkleTree,
    tools,
    transaction::{Transaction, Transactionable},
};

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

pub struct MainChain {
    blocks: Db,
    height_reference: Db,
    transactions: Db,
    height: Arc<RwLock<U256>>,
    difficulty: Arc<RwLock<[u8; 32]>>,
}

pub struct DerivativeChain {
    db: Db,
    height_reference: Db,
    height: u64,
    global_height: u64,
    genesis_hash: Arc<RwLock<[u8; 32]>>,
    difficulty: Arc<RwLock<[u8; 32]>>,
}

#[async_trait]
impl Chain for MainChain {
    /// Dump config
    ///
    /// Dumps chain's config
    async fn dump_config(&self) -> Result<(), Report<BlockChainTreeError>> {
        let root = String::from(MAIN_CHAIN_DIRECTORY);
        let path_config = root + CONFIG_FILE;

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path_config)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))?;
        let mut buffer_32_bytes: [u8; 32] = [0; 32];
        self.height.read().await.to_big_endian(&mut buffer_32_bytes);
        file.write_all(&buffer_32_bytes)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write height")?;

        file.write_all(self.difficulty.read().await.as_ref())
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write difficulty")?;

        Ok(())
    }

    /// Flushes all DBs and config
    async fn flush(&self) -> Result<(), Report<BlockChainTreeError>> {
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

    // /// Adds new block to the chain db, raw API function
    // ///
    // /// Adds block and sets heigh reference for it
    // ///
    // /// Doesn't check for blocks validity, just adds it directly to the end of the chain, checks only for the height
    // pub async fn add_block_raw(
    //     &self,
    //     block: &impl MainChainBlock,
    // ) -> Result<(), Report<BlockChainTreeError>> {
    //     let dump = block
    //         .dump()
    //         .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

    //     let hash = tools::hash(&dump);

    //     let mut height = self.height.write().await;

    //     if block.get_info().height != *height {
    //         return Err(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock)).attach_printable(
    //             "The height of the chain is different from the height of the block",
    //         );
    //     }

    //     let mut height_bytes = [0u8; 32];
    //     height.to_big_endian(&mut height_bytes);

    //     self.blocks
    //         .insert(height_bytes, dump)
    //         .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
    //         .attach_printable("Failed to insert block to blocks db")?;

    //     self.height_reference
    //         .insert(hash, &height_bytes)
    //         .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
    //         .attach_printable("Failed to insert height reference for the block")?;

    //     *height += U256::one();

    //     self.blocks
    //         .flush_async()
    //         .await
    //         .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
    //         .attach_printable("Failed to flush blocks db")?;

    //     self.height_reference
    //         .flush_async()
    //         .await
    //         .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
    //         .attach_printable("Failed to flush height reference db")?;

    //     Ok(())
    // }

    /// Adds new block to the chain db
    ///
    /// Adds block and sets heigh reference for it
    ///
    /// Checks for blocks validity, adds it directly to the end of the chain
    async fn add_block(
        &self,
        block: &(impl Block + Sync),
    ) -> Result<(), Report<BlockChainTreeError>> {
        let dump = block
            .dump()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        let hash = tools::hash(&dump);

        let mut height = self.height.write().await;

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

        self.blocks
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
            .attach_printable("Failed to flush blocks db")?;

        self.height_reference
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))
            .attach_printable("Failed to flush height reference db")?;

        Ok(())
    }

    /// Get serialized block by it's height
    async fn find_raw_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let chain_height = self.height.read().await;
        if height > &chain_height {
            return Ok(None);
        }
        drop(chain_height);

        let mut height_serialized = [0u8; 32];
        height.to_big_endian(&mut height_serialized);
        let mut dump = self
            .blocks
            .get(&height_serialized)
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?;

        if let Some(dump) = dump.take() {
            return Ok(Some(dump.to_vec()));
        }
        Ok(None)
    }

    /// Get serialized block by it's hash
    async fn find_raw_by_hash(
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
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?;

        Ok(block)
    }

    async fn find_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>> {
        let dump = self.find_raw_by_hash(hash).await?;

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
    async fn get_last_raw_block(&self) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let height = self.height.read().await;
        let last_block_index = *height - 1;
        drop(height);

        self.find_raw_by_height(&last_block_index).await
    }

    /// Get deserialized latest block
    async fn get_last_block(
        &self,
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>> {
        let dump = self.get_last_raw_block().await?;

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
    async fn find_by_height(
        &self,
        height: &U256,
    ) -> Result<Option<Arc<dyn Block + Send + Sync>>, Report<BlockChainTreeError>> {
        let dump = self.find_raw_by_height(height).await?;

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

impl MainChain {
    pub async fn new() -> Result<Self, Report<BlockChainTreeError>> {
        let root = String::from(MAIN_CHAIN_DIRECTORY);

        let path_blocks_st = root.clone() + BLOCKS_FOLDER;
        let path_references_st = root.clone() + REFERENCES_FOLDER;
        let path_transactions_st = root.clone() + TRANSACTIONS_FOLDER;
        let path_height_st = root + CONFIG_FILE;

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
        };
        if height.is_zero() {
            let info = BasicInfo::new(
                INCEPTION_TIMESTAMP,
                U256::zero(),
                [0u8; 32],
                U256::zero(),
                BEGINNING_DIFFICULTY,
                ROOT_PUBLIC_ADDRESS,
            );
            let mut initial_amount = Vec::<u8>::new();
            initial_amount.extend(ROOT_PUBLIC_ADDRESS.iter());
            initial_amount.extend([0u8; 32]);
            COINS_PER_CYCLE.to_big_endian(&mut initial_amount[33..]);

            let merkle_tree = MerkleTree::build_tree(&[tools::hash(&initial_amount)]);
            chain
                .add_block(&SummarizeBlock {
                    default_info: info,
                    merkle_tree_root: *merkle_tree.get_root(),
                })
                .await
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("Failed to insert inception block")?;
        }

        Ok(chain)
    }
}
