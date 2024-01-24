use std::{fs::File, io::Read, path::Path, sync::Arc};

use error_stack::{Report, ResultExt};
use primitive_types::U256;
use sled::Db;
use tokio::{fs::OpenOptions, io::AsyncWriteExt, sync::RwLock};

use crate::{
    block::{BasicInfo, MainChainBlock, SummarizeBlock, TransactionBlock},
    errors::{BlockChainTreeError, ChainErrorKind},
    merkletree::MerkleTree,
    tools,
    transaction::{Transaction, Transactionable},
};

static BLOCKCHAIN_DIRECTORY: &str = "./BlockChainTree/";

static AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARY/";
static OLD_AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARYOLD/";

static MAIN_CHAIN_DIRECTORY: &str = "./BlockChainTree/MAIN/";

static DERIVATIVE_CHAINS_DIRECTORY: &str = "./BlockChainTree/DERIVATIVES/";
static CHAINS_FOLDER: &str = "CHAINS/";
//static DERIVATIVE_DB_DIRECTORY: BlockChainTreeError = "./BlockChainTree/DERIVATIVE/DB/";

static BLOCKS_FOLDER: &str = "BLOCKS/";
static REFERENCES_FOLDER: &str = "REF/";
static TRANSACTIONS_FOLDER: &str = "TRANSACTIONS/";

static CONFIG_FILE: &str = "Chain.config";
static LOOKUP_TABLE_FILE: &str = "LookUpTable.dat";
static TRANSACTIONS_POOL: &str = "TRXS_POOL.pool";

pub static BEGINNING_DIFFICULTY: [u8; 32] = [
    0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

pub static ROOT_PRIVATE_ADDRESS: [u8; 32] = [1u8; 32];
pub static ROOT_PUBLIC_ADDRESS: [u8; 33] = [
    3, 27, 132, 197, 86, 123, 18, 100, 64, 153, 93, 62, 213, 170, 186, 5, 101, 215, 30, 24, 52, 96,
    72, 25, 255, 156, 23, 245, 233, 213, 221, 7, 143,
];

pub static INCEPTION_TIMESTAMP: u64 = 1597924800;

pub struct MainChain {
    blocks: Db,
    height_reference: Db,
    transactions: Db,
    height: Arc<RwLock<U256>>,
    difficulty: Arc<RwLock<[u8; 32]>>,
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
            let initial_transaction = Transaction::new(
                ROOT_PUBLIC_ADDRESS,
                ROOT_PUBLIC_ADDRESS,
                INCEPTION_TIMESTAMP,
                U256::zero(),
                ROOT_PRIVATE_ADDRESS,
                None,
            );
            let merkle_tree = MerkleTree::build_tree(&[initial_transaction.hash()]);
            chain
                .add_block_raw(&SummarizeBlock {
                    default_info: info,
                    merkle_tree_root: *merkle_tree.get_root(),
                })
                .await
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
                .attach_printable("Failed to insert inception block")?;
        }

        Ok(chain)
    }

    /// Dump config
    ///
    /// Dumps chain's config
    pub async fn dump_config(&self) -> Result<(), Report<BlockChainTreeError>> {
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

    /// Adds new block to the chain db, raw API function
    ///
    /// Adds block and sets heigh reference for it
    ///
    /// Doesn't check for blocks validity, just adds it directly to the end of the chain, checks only for the height
    pub async fn add_block_raw(
        &self,
        block: &impl MainChainBlock,
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

        //drop(height);

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
    pub async fn find_raw_by_height(
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
    pub async fn find_raw_by_hash(
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

    /// Get serialized last block of the chain
    pub async fn get_last_raw_block(&self) -> Result<Option<Vec<u8>>, Report<BlockChainTreeError>> {
        let height = self.height.read().await;
        let last_block_index = *height - 1;
        drop(height);

        self.find_raw_by_height(&last_block_index).await
    }
}
