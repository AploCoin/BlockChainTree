#![allow(non_snake_case)]
use crate::block::{
    self, BasicInfo, GenesisBlock, MainChainBlock, MainChainBlockArc, SummarizeBlock, TokenBlock,
    TransactionBlock,
};
use crate::merkletree::MerkleTree;
use crate::tools::{self, check_pow};
use crate::transaction::{Transaction, Transactionable, TransactionableItem};
use num_bigint::BigUint;
use std::cmp::Ordering;
use std::collections::binary_heap::Iter;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::convert::TryInto;

use crate::summary_db::SummaryDB;

use crate::dump_headers::Headers;
use hex::ToHex;
use lazy_static::lazy_static;
use sled::Db;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::str::{self};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockWriteGuard};

use crate::errors::*;
use error_stack::{IntoReport, Report, Result, ResultExt};

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
pub static GENESIS_BLOCK: [u8; 32] = [
    0x77, 0xe6, 0xd9, 0x52, 0x67, 0x57, 0x8e, 0x85, 0x39, 0xa9, 0xcf, 0xe0, 0x03, 0xf4, 0xf7, 0xfe,
    0x7d, 0x6a, 0x29, 0x0d, 0xaf, 0xa7, 0x73, 0xa6, 0x5c, 0x0f, 0x01, 0x9d, 0x5c, 0xbc, 0x0a, 0x7c,
]; // God is dead, noone will stop anarchy
pub static BEGINNING_DIFFICULTY: [u8; 32] = [
    0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];
static MAX_DIFFICULTY: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 128,
];

pub static ROOT_PRIVATE_ADDRESS: [u8; 32] = [1u8; 32];
pub static ROOT_PUBLIC_ADDRESS: [u8; 33] = [
    3, 27, 132, 197, 86, 123, 18, 100, 64, 153, 93, 62, 213, 170, 186, 5, 101, 215, 30, 24, 52, 96,
    72, 25, 255, 156, 23, 245, 233, 213, 221, 7, 143,
];

pub static INCEPTION_TIMESTAMP: u64 = 1597924800;

lazy_static! {
    // one coin is 100_000_000 smol coins
    static ref COIN_FRACTIONS: BigUint = BigUint::from(100_000_000usize);
    static ref INITIAL_FEE: BigUint = BigUint::from(16666666usize); // 100_000_000//4
    static ref FEE_STEP: BigUint = BigUint::from(392156usize); // 100_000_000//255
    static ref MAIN_CHAIN_PAYMENT: BigUint = INITIAL_FEE.clone();
    static ref COINS_PER_CYCLE:BigUint = (MAIN_CHAIN_PAYMENT.clone()*2000usize*BLOCKS_PER_ITERATION) + COIN_FRACTIONS.clone()*10000usize;
}

//static MAX_TRANSACTIONS_PER_BLOCK: usize = 3000;
static BLOCKS_PER_ITERATION: usize = 12960;

type TrxsPool = BinaryHeap<TransactionableItem>;

type DerivativesCell = Arc<RwLock<DerivativeChain>>;
type Derivatives = Arc<RwLock<HashMap<[u8; 33], DerivativesCell>>>;

#[derive(Default)]
pub struct TransactionsPool {
    pool: TrxsPool,
    hashes: HashSet<[u8; 32]>,
}

impl TransactionsPool {
    pub fn new() -> TransactionsPool {
        TransactionsPool::default()
    }
    pub fn with_capacity(capacity: usize) -> TransactionsPool {
        TransactionsPool {
            pool: BinaryHeap::with_capacity(capacity),
            hashes: HashSet::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, transaction: TransactionableItem) -> bool {
        if !self.hashes.insert(transaction.hash()) {
            return false;
        }
        self.pool.push(transaction);
        true
    }

    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn transactions_iter(&self) -> Iter<'_, TransactionableItem> {
        self.pool.iter()
    }

    pub fn pop(&mut self) -> Option<([u8; 32], TransactionableItem)> {
        let tr = self.pool.pop()?;
        let hash = tr.hash();
        self.hashes.remove(&hash);
        Some((hash, tr))
    }

    pub fn transaction_exists(&self, hash: &[u8; 32]) -> bool {
        self.hashes.contains(hash)
    }
}

#[derive(Clone)]
pub struct Chain {
    db: Db,
    height_reference: Db,
    transactions: Db,
    height: Arc<RwLock<u64>>,
    genesis_hash: [u8; 32],
    difficulty: Arc<RwLock<[u8; 32]>>,
}

impl Chain {
    /// Open chain with config
    pub fn new() -> Result<Chain, BlockChainTreeError> {
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
        let db = sled::open(path_blocks)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = sled::open(path_reference)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open references db")?;

        // open transactions DB
        let transactions_db = sled::open(path_transactions)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open transactions db")?;

        let mut file = File::open(path_height)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))?;

        // read height from config
        let mut height_bytes: [u8; 8] = [0; 8];

        file.read_exact(&mut height_bytes)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to read config")?;

        let height: u64 = u64::from_be_bytes(height_bytes);

        // read genesis hash
        let mut genesis_hash: [u8; 32] = [0; 32];
        file.read_exact(&mut genesis_hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to read genesis hash")?;

        // read difficulty
        let mut difficulty: [u8; 32] = [0; 32];
        file.read_exact(&mut difficulty)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to read difficulty")?;

        Ok(Chain {
            db,
            height_reference,
            transactions: transactions_db,
            height: Arc::new(RwLock::new(height)),
            genesis_hash,
            difficulty: Arc::new(RwLock::new(difficulty)),
        })
    }

    /// Remove heigh reference for supplied hash
    async fn remove_height_reference(&self, hash: &[u8; 32]) -> Result<(), BlockChainTreeError> {
        self.height_reference
            .remove(hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::FailedToRemoveHeighReference,
            ))
            .attach_printable("Hash: {hash:?}")?;

        self.height_reference
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::FailedToRemoveHeighReference,
            ))
            .attach_printable("Hash: {hash:?}")?;

        Ok(())
    }

    /// Remove all transactions for supplied transactions hashes
    ///
    /// fee should be same for the supplied transactions
    ///
    /// Transactions should be rotated newer - older
    ///
    /// will update amounts in summary db
    async fn remove_transactions<'a, I>(
        &self,
        transactions: I,
        fee: BigUint,
        summary_db: &SummaryDB,
    ) -> Result<(), BlockChainTreeError>
    where
        I: Iterator<Item = &'a [u8; 32]>,
    {
        for transaction_hash in transactions {
            let transaction_dump = self
                .transactions
                .remove(transaction_hash)
                .into_report()
                .change_context(BlockChainTreeError::Chain(
                    ChainErrorKind::FailedToRemoveTransaction,
                ))
                .attach_printable(format!("Hash: {transaction_hash:?}"))?
                .ok_or(BlockChainTreeError::Chain(
                    ChainErrorKind::FailedToRemoveTransaction,
                ))
                .into_report()
                .attach_printable(format!("Transaction with hash: {transaction_hash:?}"))?;

            // TODO: rewrite transaction parsing
            let transaction =
                Transaction::parse(&transaction_dump[1..], (transaction_dump.len() - 1) as u64)
                    .change_context(BlockChainTreeError::Chain(
                        ChainErrorKind::FailedToRemoveTransaction,
                    ))
                    .attach_printable(format!(
                        "Error parsing transaction with hash: {transaction_hash:?}"
                    ))?;

            summary_db
                .add_funds(transaction.get_sender(), transaction.get_amount())
                .await?;
            summary_db
                .decrease_funds(
                    transaction.get_receiver(),
                    &(transaction.get_amount() - &fee),
                )
                .await?;
        }
        Ok(())
    }

    /// Removes blocks references and associated transactions
    ///
    /// end_height > start_height
    ///
    /// removes all blocks from start_height to end_height
    ///
    /// utilizes remove_height_reference() and remove_transactions()
    pub async fn remove_blocks(
        &self,
        start_height: u64,
        end_height: u64,
        summary_db: &SummaryDB,
    ) -> Result<(), BlockChainTreeError> {
        for height in end_height - 1..start_height {
            let block = self.find_by_height(height).await?.unwrap(); // fatal error

            let hash = block.hash().change_context(BlockChainTreeError::Chain(
                ChainErrorKind::FailedToHashBlock,
            ))?;

            self.remove_height_reference(&hash).await.unwrap(); // fatal error

            self.remove_transactions(
                block.get_transactions().iter().rev(),
                block.get_fee(),
                summary_db,
            )
            .await
            .unwrap(); // fatal error
        }
        Ok(())
    }

    /// Overwrite block with same height
    ///
    /// Adds a block to db under it's height
    ///
    /// Removes higher blocks references and removes associated transactions
    ///
    /// uses remove_blocks() to remove higher blocks and transactions
    ///
    /// sets current height to the block's height + 1
    ///
    /// Doesn't change difficulty
    pub async fn block_overwrite(
        &self,
        block: &MainChainBlockArc,
        summary_db: &SummaryDB,
    ) -> Result<(), BlockChainTreeError> {
        let mut height = self.height.write().await;

        let dump = block
            .dump()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        let hash = tools::hash(&dump);

        let height_block = block.get_info().height;
        let height_bytes = height.to_be_bytes();

        self.remove_blocks(height_block, *height, &summary_db)
            .await?;

        self.db
            .insert(height_bytes, dump)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        self.height_reference
            .insert(hash, &height_bytes)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        *height = height_block + 1;

        self.db
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        self.height_reference
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        Ok(())
    }

    /// Adds new block to the chain db, raw API function
    ///
    /// Adds block and sets heigh reference for it
    ///
    /// Doesn't check for blocks validity, just adds it directly to the end of the chain
    pub async fn add_block_raw(
        &self,
        block: &impl MainChainBlock,
    ) -> Result<(), BlockChainTreeError> {
        let dump = block
            .dump()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        let hash = tools::hash(&dump);

        let mut height = self.height.write().await;
        let height_bytes = height.to_be_bytes();

        self.db
            .insert(height_bytes, dump)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        self.height_reference
            .insert(hash, &height_bytes)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        *height += 1;

        //drop(height);

        self.db
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        self.height_reference
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        Ok(())
    }

    /// Add new transaction to the chain, raw API function
    ///
    /// Adds transaction into db of transactions, transaction should be also registered in the block
    ///
    /// Doesn't validate transaction
    pub async fn add_transaction_raw(
        &self,
        transaction: impl Transactionable,
    ) -> Result<(), BlockChainTreeError> {
        self.transactions
            .insert(
                transaction.hash(),
                transaction
                    .dump()
                    .map_err(|e| {
                        e.change_context(BlockChainTreeError::Chain(
                            ChainErrorKind::AddingTransaction,
                        ))
                    })
                    .attach_printable("failed to dump transaction")?,
            )
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::AddingTransaction,
            ))
            .attach_printable("failed to add transaction to database")?;

        self.transactions
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::AddingTransaction,
            ))?;

        Ok(())
    }

    /// Add a batch of transactions
    pub async fn add_transactions_raw(
        &self,
        transactions: Vec<Box<dyn Transactionable + Send + Sync>>,
    ) -> Result<(), BlockChainTreeError> {
        let mut batch = sled::Batch::default();
        for transaction in transactions {
            batch.insert(
                &transaction.hash(),
                transaction
                    .dump()
                    .change_context(BlockChainTreeError::Chain(
                        ChainErrorKind::AddingTransaction,
                    ))?,
            );
        }

        self.transactions
            .apply_batch(batch)
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::AddingTransaction,
            ))?;

        self.transactions
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::AddingTransaction,
            ))?;

        Ok(())
    }

    /// Get deserialized transaction by it's hash
    pub async fn find_transaction(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Transaction>, BlockChainTreeError> {
        let dump = if let Some(dump) = self
            .transactions
            .get(hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindTransaction))
            .attach_printable("Error getting transaction from database")?
            .take()
        {
            dump
        } else {
            return Ok(None);
        };

        let transaction = if dump[0] == Headers::Transaction as u8 {
            Transaction::parse(&dump[1..], (dump.len() - 1) as u64)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindTransaction))
                .attach_printable("Error parsing transaction")
        } else {
            Err(
                Report::new(BlockChainTreeError::Chain(ChainErrorKind::FindTransaction))
                    .attach_printable("Unknown header"),
            )
        }?;

        Ok(Some(transaction))
    }

    /// Check whether transaction exists in the chain
    pub async fn transaction_exists(&self, hash: &[u8; 32]) -> Result<bool, BlockChainTreeError> {
        Ok(self
            .transactions
            .get(hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindTransaction))
            .attach_printable("Error getting transaction from database")?
            .is_some())
    }

    /// Get current chain's height
    pub async fn get_height(&self) -> u64 {
        *self.height.read().await
    }

    pub async fn get_locked_height(&self) -> RwLockWriteGuard<u64> {
        self.height.write().await
    }

    /// Get current chain's difficulty
    pub async fn get_difficulty(&self) -> [u8; 32] {
        *self.difficulty.read().await
    }

    pub async fn get_locked_difficulty(&self) -> RwLockWriteGuard<[u8; 32]> {
        self.difficulty.write().await
    }

    /// Get serialized block by it's height
    pub async fn find_raw_by_height(
        &self,
        height: u64,
    ) -> Result<Option<Vec<u8>>, BlockChainTreeError> {
        if height == 0 {
            return Ok(None);
        }
        let chain_height = self.height.read().await;
        if height > *chain_height {
            return Ok(None);
        }
        drop(chain_height);
        let mut dump = self
            .db
            .get(height.to_be_bytes())
            .into_report()
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
    ) -> Result<Option<Vec<u8>>, BlockChainTreeError> {
        let height = match self
            .height_reference
            .get(hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?
        {
            None => {
                return Ok(None);
            }
            Some(h) => {
                u64::from_be_bytes(h.iter().copied().collect::<Vec<u8>>().try_into().unwrap())
            }
        };

        let block = self
            .find_raw_by_height(height)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?;

        Ok(block)
    }

    /// Get deserialized block by height
    pub async fn find_by_height(
        &self,
        height: u64,
    ) -> Result<Option<Box<dyn MainChainBlock + Send + Sync>>, BlockChainTreeError> {
        if height == 0 {
            return Ok(Some(Box::new(GenesisBlock {})));
        }
        let chain_height = self.height.read().await;
        if height > *chain_height {
            return Ok(None);
        }
        drop(chain_height);
        let dump = self
            .db
            .get(height.to_be_bytes())
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?;

        if dump.is_none() {
            return Ok(None);
        }

        let dump = dump.unwrap();

        Ok(Some(
            block::deserialize_main_chain_block(&dump)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?,
        ))
    }

    /// Get deserialized block by it's hash
    pub async fn find_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<Box<dyn MainChainBlock + Send + Sync>>, BlockChainTreeError> {
        let height = match self
            .height_reference
            .get(hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?
        {
            None => {
                return Ok(None);
            }
            Some(h) => {
                u64::from_be_bytes(h.iter().copied().collect::<Vec<u8>>().try_into().unwrap())
            }
        };

        let block = self
            .find_by_height(height)
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?;

        Ok(block)
    }

    /// Dump config
    ///
    /// Dumps chain's config
    pub async fn dump_config(&self) -> Result<(), BlockChainTreeError> {
        let root = String::from(MAIN_CHAIN_DIRECTORY);
        let path_config = root + CONFIG_FILE;

        let mut file = File::create(path_config)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))?;

        file.write_all(&self.height.read().await.to_be_bytes())
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write height")?;

        file.write_all(&self.genesis_hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to write genesis block")?;

        file.write_all(self.difficulty.read().await.as_ref())
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failes to write difficulty")?;

        Ok(())
    }

    /// Create new chain
    ///
    /// Creates new chain without config, creates necessary folders
    pub fn new_without_config(
        root_path: &str,
        genesis_hash: &[u8; 32],
    ) -> Result<Chain, BlockChainTreeError> {
        let root = String::from(root_path);
        let path_blocks_st = root.clone() + BLOCKS_FOLDER;
        let path_references_st = root.clone() + REFERENCES_FOLDER;
        let path_transactions_st = root + TRANSACTIONS_FOLDER;

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);
        let path_transactions = Path::new(&path_transactions_st);

        // open blocks DB
        let db = sled::open(path_blocks)
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = sled::open(path_reference)
            .into_report()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open references db")?;

        // open transactions DB
        let transactions_db = sled::open(path_transactions)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::Init))
            .attach_printable("failed to open transactions db")?;

        Ok(Chain {
            db,
            height_reference,
            transactions: transactions_db,
            height: Arc::new(RwLock::new(1)),
            genesis_hash: *genesis_hash,
            difficulty: Arc::new(RwLock::new(BEGINNING_DIFFICULTY)),
        })
    }

    /// Get serialized last block if the chain
    pub async fn get_last_raw_block(&self) -> Result<Option<Vec<u8>>, BlockChainTreeError> {
        let height = self.height.read().await;
        let last_block_index = *height - 1;
        drop(height);

        self.find_raw_by_height(last_block_index).await
    }

    /// Get deserialized last block of the chain
    pub async fn get_last_block(
        &self,
    ) -> Result<Option<Box<dyn MainChainBlock + Send + Sync>>, BlockChainTreeError> {
        let height = self.height.read().await;
        let last_block_index = *height - 1;
        drop(height);

        self.find_by_height(last_block_index).await
    }

    /// Get hash of the last block in chain
    ///
    /// Gets hash from the last record in height reference db
    pub async fn get_last_hash(&self) -> Result<[u8; 32], BlockChainTreeError> {
        if self.get_height().await == 0 {
            return Ok(GENESIS_BLOCK);
        }
        Ok(self
            .height_reference
            .last()
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?
            .map(|(hash, _)| {
                let mut hash_arr = [0u8; 32];
                hash.iter()
                    .zip(hash_arr.iter_mut())
                    .for_each(|(val, cell)| *cell = *val);
                hash_arr
            })
            .unwrap_or(GENESIS_BLOCK))
    }

    /// Checks if the supplied pow is correct
    ///
    /// Takes hash of the last block for current time and checks against it
    ///
    /// Since this function checks data only in current time, it should not be used alone when adding new block,
    ///
    /// because of the way this implementation built it should be used with additional thread safety, such as locking `height` to ensure,
    ///
    /// that this function will get latest info
    ///
    /// P.S. it was made into separate function only because of mudularity and to provide raw API(later)
    async fn check_pow_validity(&self, pow: BigUint) -> Result<bool, BlockChainTreeError> {
        let last_hash = self.get_last_hash().await?;

        let difficulty = self.get_difficulty().await;
        Ok(tools::check_pow(&last_hash, &difficulty, &pow))
    }

    /// Calculate fee for the difficulty
    ///
    /// takes difficulty and calculates fee for it
    ///
    /// TODO: Change the way fee calculated
    pub fn calculate_fee(difficulty: &[u8; 32]) -> BigUint {
        let mut leading_zeroes = 0;
        for byte in difficulty {
            let bytes_leading_zeroes = byte.count_zeros() as usize;
            leading_zeroes += bytes_leading_zeroes;
            if bytes_leading_zeroes < 8 {
                break;
            }
        }

        INITIAL_FEE.clone() + (FEE_STEP.clone() * (leading_zeroes - 1))
    }

    /// Goes trough all the blocks in main chain and verifies each of them
    pub async fn verify_chain(&self) -> Result<(), BlockChainTreeError> {
        let height = *self.height.read().await;

        let prev_hash = self.genesis_hash;
        for i in 0..height {
            let block = match self.find_by_height(i).await? {
                None => {
                    return Err(Report::new(BlockChainTreeError::Chain(
                        ChainErrorKind::FindByHeight,
                    ))
                    .attach_printable(format!("Block height: {:?}", i)))
                }
                Some(block) => block,
            };

            if !block.verify_block(&prev_hash) {
                return Err(Report::new(BlockChainTreeError::Chain(
                    ChainErrorKind::FailedToVerify,
                ))
                .attach_printable(format!(
                    "Block hash: {:?}",
                    block.hash().change_context(BlockChainTreeError::Chain(
                        ChainErrorKind::FailedToVerify,
                    ))?
                )));
            }
        }

        Ok(())
    }
}

pub struct DerivativeChain {
    db: Db,
    height_reference: Db,
    height: u64,
    global_height: u64,
    genesis_hash: [u8; 32],
    difficulty: [u8; 32],
}

impl DerivativeChain {
    /// Open chain with config
    pub fn new(root_path: &str) -> Result<DerivativeChain, BlockChainTreeError> {
        let root = String::from(root_path);
        let path_blocks_st = root.clone() + BLOCKS_FOLDER;
        let path_references_st = root.clone() + REFERENCES_FOLDER;
        let path_height_st = root + CONFIG_FILE;

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);
        let path_height = Path::new(&path_height_st);

        // open blocks DB
        let db = sled::open(path_blocks)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = sled::open(path_reference)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to open references db")?;

        let mut file = File::open(path_height)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to open config")?;

        // read height from config
        let mut height_bytes: [u8; 8] = [0; 8];
        file.read_exact(&mut height_bytes)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to read config")?;

        let height: u64 = u64::from_be_bytes(height_bytes);

        // read genesis hash
        let mut genesis_hash: [u8; 32] = [0; 32];
        file.read_exact(&mut genesis_hash)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to open genesis hash from config")?;

        // read difficulty
        let mut difficulty: [u8; 32] = [0; 32];
        file.read_exact(&mut difficulty)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to read difficulty from config")?;

        // read global height
        let mut global_height: [u8; 8] = [0; 8];
        file.read_exact(&mut global_height)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to read global height from config")?;

        let global_height: u64 = u64::from_be_bytes(global_height);

        Ok(DerivativeChain {
            db,
            height_reference,
            height,
            genesis_hash,
            difficulty,
            global_height,
        })
    }

    /// Adds block to the chain, sets heigh reference
    pub async fn add_block(&mut self, block: &TokenBlock) -> Result<(), BlockChainTreeError> {
        let dump = block
            .dump()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::AddingBlock,
            ))?;

        let hash = tools::hash(&dump);

        self.db
            .insert(self.height.to_be_bytes(), dump)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to add block to db")?;

        self.height_reference
            .insert(hash, &self.height.to_be_bytes())
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::Init,
            ))
            .attach_printable("failed to add reference to db")?;

        self.height += 1;

        self.db
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        self.height_reference
            .flush_async()
            .await
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::AddingBlock))?;

        Ok(())
    }

    /// Get current height of the chain
    pub fn get_height(&self) -> u64 {
        self.height
    }

    /// Get current difficulty of the chain
    pub fn get_difficulty(&self) -> [u8; 32] {
        self.difficulty
    }

    /// Get global height of the chain
    pub fn get_global_height(&self) -> u64 {
        self.global_height
    }

    /// Get deserialized block by it's height
    pub fn find_by_height(&self, height: u64) -> Result<Option<TokenBlock>, BlockChainTreeError> {
        if height > self.height {
            return Ok(None);
        }
        let dump = self
            .db
            .get(height.to_be_bytes())
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::FindByHeight,
            ))
            .attach_printable("failed to read block")?;

        if dump.is_none() {
            return Ok(None);
        }
        let dump = dump.unwrap();

        if dump[0] != Headers::TokenBlock as u8 {
            return Err(Report::new(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::FindByHeight,
            ))
            .attach_printable("wrong header"));
        }
        let block = TokenBlock::parse(&dump[1..], (dump.len() - 1) as u32).change_context(
            BlockChainTreeError::DerivativeChain(DerivChainErrorKind::FindByHeight),
        )?;

        Ok(Some(block))
    }

    /// Get deserialized block by it's hash
    pub fn find_by_hash(&self, hash: &[u8; 32]) -> Result<Option<TokenBlock>, BlockChainTreeError> {
        let height = match self
            .height_reference
            .get(hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?
        {
            None => {
                return Ok(None);
            }
            Some(h) => {
                u64::from_be_bytes(h.iter().copied().collect::<Vec<u8>>().try_into().unwrap())
            }
        };

        let block =
            self.find_by_height(height)
                .change_context(BlockChainTreeError::DerivativeChain(
                    DerivChainErrorKind::FindByHash,
                ))?;

        Ok(block)
    }

    /// Dump config of the chain
    pub fn dump_config(&self, root_path: &str) -> Result<(), BlockChainTreeError> {
        let root = String::from(root_path);
        let path_config = root + CONFIG_FILE;

        let mut file = File::create(path_config)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::DumpConfig,
            ))
            .attach_printable("failed to open config")?;

        file.write_all(&self.height.to_be_bytes())
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::DumpConfig,
            ))
            .attach_printable("failed to write height")?;

        file.write_all(&self.genesis_hash)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::DumpConfig,
            ))
            .attach_printable("failed to write genesis block")?;

        file.write_all(&self.difficulty)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::DumpConfig,
            ))
            .attach_printable("failed to write difficulty")?;

        file.write_all(&self.global_height.to_be_bytes())
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::DumpConfig,
            ))
            .attach_printable("failed to write global height")?;

        Ok(())
    }

    /// Open chain without config, sets up all directories
    pub fn without_config(
        root_path: &str,
        genesis_hash: &[u8; 32],
        global_height: u64,
    ) -> Result<DerivativeChain, BlockChainTreeError> {
        let root = String::from(root_path);
        let path_blocks_st = root.clone() + BLOCKS_FOLDER;
        let path_references_st = root + REFERENCES_FOLDER;

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);

        // open blocks DB
        let db = sled::open(path_blocks)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = sled::open(path_reference)
            .into_report()
            .change_context(BlockChainTreeError::DerivativeChain(
                DerivChainErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open references db")?;

        Ok(DerivativeChain {
            db,
            height_reference,
            height: 0,
            genesis_hash: *genesis_hash,
            difficulty: BEGINNING_DIFFICULTY,
            global_height,
        })
    }

    /// Get deserialized last block of the chain
    pub fn get_last_block(&self) -> Result<Option<TokenBlock>, BlockChainTreeError> {
        self.find_by_height(self.height - 1)
    }
}

#[derive(Clone)]
pub struct BlockChainTree {
    trxs_pool: Arc<RwLock<TransactionsPool>>,
    summary_db: Arc<RwLock<SummaryDB>>,
    old_summary_db: Arc<RwLock<SummaryDB>>,
    main_chain: Arc<Chain>,
    deratives: Derivatives,
}

impl BlockChainTree {
    /// Open BlockChainTree
    ///
    /// opens blockchain tree with existing config
    pub fn with_config() -> Result<BlockChainTree, BlockChainTreeError> {
        let summary_db_path = Path::new(&AMMOUNT_SUMMARY);

        // open summary db
        let summary_db = sled::open(summary_db_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open summary db")?;

        let old_summary_db_path = Path::new(&OLD_AMMOUNT_SUMMARY);

        // open old summary db
        let old_summary_db = sled::open(old_summary_db_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open old summary db")?;

        // read transactions pool
        let pool_path = String::from(BLOCKCHAIN_DIRECTORY) + TRANSACTIONS_POOL;
        let pool_path = Path::new(&pool_path);

        let mut file = File::open(pool_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open transactions pool")?;

        // read amount of transactions
        let mut buf: [u8; 8] = [0; 8];
        file.read_exact(&mut buf)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to read amount of transactions")?;

        let trxs_amount = u64::from_be_bytes(buf);

        let mut buf: [u8; 4] = [0; 4];

        // allocate VecDeque
        let mut trxs_pool = TransactionsPool::with_capacity(10000);

        // parsing transactions
        for _ in 0..trxs_amount {
            file.read_exact(&mut buf)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
                .attach_printable("failed to read transaction size")?;

            let tr_size = u32::from_be_bytes(buf);

            let mut transaction_buffer = vec![0u8; (tr_size - 1) as usize];

            file.read_exact(&mut transaction_buffer)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
                .attach_printable("failed to read transaction")?;

            if transaction_buffer[0] == 0 {
                let transaction =
                    Transaction::parse(&transaction_buffer[1..], (tr_size - 1) as u64)
                        .change_context(BlockChainTreeError::BlockChainTree(
                            BCTreeErrorKind::Init,
                        ))?;

                trxs_pool.push(Box::new(transaction));
            } else {
                return Err(Report::new(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::Init,
                ))
                .attach_printable("Not implemented yet"));
            }
        }

        // opening main chain
        let main_chain = Chain::new()
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))?;

        Ok(BlockChainTree {
            trxs_pool: Arc::new(RwLock::new(trxs_pool)),
            summary_db: Arc::new(RwLock::new(SummaryDB::new(summary_db))),
            main_chain: Arc::new(main_chain),
            old_summary_db: Arc::new(RwLock::new(SummaryDB::new(old_summary_db))),
            deratives: Arc::default(),
        })
    }

    /// Open BlockChainTree
    ///
    /// opens blockchain tree without config
    pub fn without_config() -> Result<BlockChainTree, BlockChainTreeError> {
        let summary_db_path = Path::new(&AMMOUNT_SUMMARY);

        // open summary db
        let summary_db = sled::open(summary_db_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open summary db")?;

        // set initial value for the root address
        if summary_db
            .get(ROOT_PUBLIC_ADDRESS)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::InitWithoutConfig,
            ))
            .attach_printable(
                "failed to get amount of coins in the summary db for the root address",
            )?
            .is_none()
        {
            let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&COINS_PER_CYCLE));
            tools::dump_biguint(&COINS_PER_CYCLE, &mut dump).change_context(
                BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
            )?;
            summary_db
                .insert(ROOT_PUBLIC_ADDRESS, dump)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::InitWithoutConfig,
                ))
                .attach_printable(
                    "failed to set amount of coins in the summary db for the root address",
                )?;
        }

        let old_summary_db_path = Path::new(&OLD_AMMOUNT_SUMMARY);

        // open old summary db
        let old_summary_db = sled::open(old_summary_db_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open old summary db")?;

        // allocate VecDeque
        let trxs_pool = TransactionsPool::with_capacity(10000);

        // opening main chain
        let main_chain = Chain::new_without_config(MAIN_CHAIN_DIRECTORY, &GENESIS_BLOCK)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open main chain")?;

        let _ = fs::create_dir(Path::new(DERIVATIVE_CHAINS_DIRECTORY));
        // .into_report()
        // .change_context(BlockChainTreeError::BlockChainTree(
        //     BCTreeErrorKind::CreateDerivChain,
        // ))
        // .attach_printable("failed to create root folder for derivatives")?;

        Ok(BlockChainTree {
            trxs_pool: Arc::new(RwLock::new(trxs_pool)),
            summary_db: Arc::new(RwLock::new(SummaryDB::new(summary_db))),
            main_chain: Arc::new(main_chain),
            old_summary_db: Arc::new(RwLock::new(SummaryDB::new(old_summary_db))),
            deratives: Arc::default(),
        })
    }

    /// Dump Transactions pool
    ///
    /// Dumps Transactions pool into folder specified as static
    pub async fn dump_pool(&self) -> Result<(), BlockChainTreeError> {
        let pool_path = String::from(BLOCKCHAIN_DIRECTORY) + TRANSACTIONS_POOL;
        let pool_path = Path::new(&pool_path);

        // open file
        let mut file = File::create(pool_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::DumpPool,
            ))
            .attach_printable("failed to open config file")?;

        let trxs_pool = self.trxs_pool.read().await;

        // write transactions amount
        file.write_all(&(trxs_pool.len() as u64).to_be_bytes())
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::DumpPool,
            ))
            .attach_printable("failed to write amount of transactions")?;

        //write transactions
        for transaction in trxs_pool.transactions_iter() {
            // get dump
            let dump = transaction
                .dump()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::DumpPool,
                ))?;

            // write transaction size
            file.write_all(&(dump.len() as u32).to_be_bytes())
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::DumpPool,
                ))
                .attach_printable("failed to write transaction size")?;

            // write transaction dump
            file.write_all(&dump)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::DumpPool,
                ))
                .attach_printable("failed to write transaction dump")?;
        }

        Ok(())
    }

    /// Get derivative chain
    ///
    /// Gets existing derivative chain(checks by path), places into inner field `derivatives`, returnes pointer to chain
    pub async fn get_derivative_chain(
        &self,
        addr: &[u8; 33],
    ) -> Result<Option<Arc<RwLock<DerivativeChain>>>, BlockChainTreeError> {
        let mut path_string = String::from(DERIVATIVE_CHAINS_DIRECTORY);
        let hex_addr: String = addr.encode_hex::<String>();
        path_string += &hex_addr;
        path_string += "/";

        let path = Path::new(&path_string);
        if path.exists() {
            let result = DerivativeChain::new(&path_string).change_context(
                BlockChainTreeError::BlockChainTree(BCTreeErrorKind::GetDerivChain),
            )?;

            return Ok(Some(
                self.deratives
                    .write()
                    .await
                    .entry(*addr)
                    .or_insert_with(|| Arc::new(RwLock::new(result)))
                    .clone(),
            ));
        }

        Ok(None)
    }

    pub fn get_main_chain(&self) -> Arc<Chain> {
        self.main_chain.clone()
    }

    /// Creates derivative chain
    ///
    /// Creates neccessary folders for derivative chain, creates chain, places into inner field `derivatives`, returns pointer to chain
    pub async fn create_derivative_chain(
        &self,
        addr: &[u8; 33],
        genesis_hash: &[u8; 32],
        global_height: u64,
    ) -> Result<Arc<RwLock<DerivativeChain>>, BlockChainTreeError> {
        let mut root_path = String::from(DERIVATIVE_CHAINS_DIRECTORY);
        let hex_addr: String = addr.encode_hex::<String>();
        root_path += &hex_addr;
        root_path += "/";

        fs::create_dir(Path::new(&root_path))
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::CreateDerivChain,
            ))
            .attach_printable("failed to create root folder")?;

        let blocks_path = root_path.clone() + BLOCKS_FOLDER;
        fs::create_dir(Path::new(&blocks_path))
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::CreateDerivChain,
            ))
            .attach_printable("failed to create blocks folder")?;

        let references_path = root_path.clone() + REFERENCES_FOLDER;
        fs::create_dir(Path::new(&references_path))
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::CreateDerivChain,
            ))
            .attach_printable("failed to create references folder")?;

        let chain = DerivativeChain::without_config(&root_path, genesis_hash, global_height)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::CreateDerivChain,
            ))?;

        chain
            .dump_config(&root_path)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::CreateDerivChain,
            ))?;

        return Ok(self
            .deratives
            .write()
            .await
            .entry(*addr)
            .or_insert_with(|| Arc::new(RwLock::new(chain)))
            .clone());
    }

    /// Check main folders for BlockChainTree
    ///
    /// Checks for required folders, if some not found will create them
    pub fn check_main_folders() -> Result<(), BlockChainTreeError> {
        let root = Path::new(BLOCKCHAIN_DIRECTORY);
        if !root.exists() {
            fs::create_dir(root)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create blockchain root")?;
        }

        let main_path = Path::new(MAIN_CHAIN_DIRECTORY);
        if !main_path.exists() {
            fs::create_dir(main_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create main chain folder")?;
        }

        let summary_path = Path::new(AMMOUNT_SUMMARY);
        if !summary_path.exists() {
            fs::create_dir(summary_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create summary folder")?;
        }

        let old_summary_path = Path::new(OLD_AMMOUNT_SUMMARY);
        if !old_summary_path.exists() {
            fs::create_dir(old_summary_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create old summary folder")?;
        }

        let blocks_path = String::from(MAIN_CHAIN_DIRECTORY) + BLOCKS_FOLDER;
        let blocks_path = Path::new(&blocks_path);
        if !blocks_path.exists() {
            fs::create_dir(blocks_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create blocks path")?;
        }

        let references_path = String::from(MAIN_CHAIN_DIRECTORY) + REFERENCES_FOLDER;
        let references_path = Path::new(&references_path);
        if !references_path.exists() {
            fs::create_dir(references_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create references paths")?;
        }

        let transactions_path = String::from(MAIN_CHAIN_DIRECTORY) + TRANSACTIONS_FOLDER;
        let transactions_path = Path::new(&transactions_path);
        if !transactions_path.exists() {
            fs::create_dir(references_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create transactions paths")?;
        }

        let derivatives_path = String::from(DERIVATIVE_CHAINS_DIRECTORY);
        let derivatives_path = Path::new(&derivatives_path);
        if !derivatives_path.exists() {
            fs::create_dir(derivatives_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create derivatives chains path")?;
        }

        let derivative_chains_path = String::from(DERIVATIVE_CHAINS_DIRECTORY) + CHAINS_FOLDER;
        let derivative_chains_path = Path::new(&derivative_chains_path);
        if !derivative_chains_path.exists() {
            fs::create_dir(derivative_chains_path)
                .into_report()
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::CheckMainFolders,
                ))
                .attach_printable("failed to create derivative chains folder")?;
        }

        Ok(())
    }

    // summary data bases functions

    /// Add funds for address
    ///
    /// Adds funs for specified address in the summary db
    pub async fn add_funds(
        &self,
        addr: &[u8; 33],
        funds: &BigUint,
    ) -> Result<(), BlockChainTreeError> {
        self.summary_db.write().await.add_funds(addr, funds).await
    }

    /// Decrease funds
    ///
    /// Decreases funds for specified address in the summary db
    pub async fn decrease_funds(
        &self,
        addr: &[u8; 33],
        funds: &BigUint,
    ) -> Result<(), BlockChainTreeError> {
        self.summary_db
            .write()
            .await
            .decrease_funds(addr, funds)
            .await
    }

    /// Get funds
    ///
    /// Gets funds for specified address from summary db
    pub async fn get_funds(&self, addr: &[u8; 33]) -> Result<BigUint, BlockChainTreeError> {
        self.summary_db.read().await.get_funds(addr)
    }

    /// Get old funds
    ///
    /// Gets old funds for specified address from previous summary db
    pub async fn get_old_funds(&self, addr: &[u8; 33]) -> Result<BigUint, BlockChainTreeError> {
        self.old_summary_db.read().await.get_funds(addr)
    }

    /// Move current summary database to old database
    ///
    /// Removes old summary database and places current summary db on it's place
    pub fn move_summary_database(&self) -> Result<(Db, Db), BlockChainTreeError> {
        let old_sum_path = Path::new(OLD_AMMOUNT_SUMMARY);
        let sum_path = Path::new(AMMOUNT_SUMMARY);

        //self.old_summary_db = Arc::new(None);
        //self.summary_db = Arc::new(None);

        fs::remove_dir_all(old_sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to remove previous database")?;

        fs::create_dir(old_sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to create folder for an old summarize db")?;

        tools::copy_dir_all(sum_path, old_sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to copy current db into old db")?;

        let summary_db = sled::open(sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to open summary db")?;

        //self.summary_db = Arc::new(Some(result));

        let old_summary_db = sled::open(old_sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to open old summary db")?;

        //self.old_summary_db = Arc::new(Some(result));

        Ok((summary_db, old_summary_db))
    }

    // Check whether transaction with same hash exists
    //
    // First check in trxs_hashes then in main chain references
    //
    // Blocks trxs pool for reading for the whole duration of the function
    pub async fn transaction_exists(&self, hash: &[u8; 32]) -> Result<bool, BlockChainTreeError> {
        let trxs_pool = self.trxs_pool.read().await;
        if trxs_pool.transaction_exists(hash) {
            return Ok(true);
        }

        if self
            .get_main_chain()
            .transaction_exists(hash)
            .await
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))?
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Add new transaction
    ///
    /// Adds new transaction to the transaction pool
    ///
    /// If trxs_pool.len() < MAX_TRANSACTIONS_PER_BLOCK and it's not the last block of epoch transaction will be immediately processed
    ///
    /// If transaction with same hash exists will return error
    pub async fn new_transaction(&self, tr: Transaction) -> Result<(), BlockChainTreeError> {
        let mut trxs_pool = self.trxs_pool.write().await;

        let tr_hash = tr.hash();
        if trxs_pool.transaction_exists(&tr_hash)
            || self
                .get_main_chain()
                .transaction_exists(&tr_hash)
                .await
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::NewTransaction,
                ))?
        {
            return Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))
            .attach_printable("Transaction with same hash found"));
        }

        let difficulty = self.main_chain.difficulty.read().await;
        let fee = Chain::calculate_fee(&difficulty);
        drop(difficulty);

        let amount = tr.get_amount();

        if amount <= &fee {
            return Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))
            .attach_printable("Amount sent in transaction is smaller, than the fee"));
        }

        trxs_pool.push(Box::new(tr.clone()));

        self.decrease_funds(tr.get_sender(), amount)
            .await
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))?;

        self.add_funds(tr.get_sender(), &(amount - &fee))
            .await
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))?;

        Ok(())
    }

    /// Create transaction block
    ///
    /// This function validates pow, pops transactions from trxs_pool, then
    ///
    /// adds new transactions block and poped transactions to the main chain
    async fn emit_transaction_block(
        &self,
        pow: BigUint,
        addr: [u8; 33],
        timestamp: u64,
        difficulty: [u8; 32],
    ) -> Result<TransactionBlock, BlockChainTreeError> {
        let mut trxs_pool = self.trxs_pool.write().await;

        let last_hash = self.main_chain.get_last_hash().await.change_context(
            BlockChainTreeError::BlockChainTree(BCTreeErrorKind::CreateMainChainBlock),
        )?;

        if !tools::check_pow(&last_hash, &difficulty, &pow) {
            // if pow is bad
            return Err(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::WrongPow,
            ))
            .into_report();
        }

        let fee = Chain::calculate_fee(&difficulty);

        let transactions_amount = trxs_pool.len();

        // get transactions
        let mut transactions: Vec<Box<dyn Transactionable + Send + Sync>> =
            Vec::with_capacity(transactions_amount + 1);

        // founder transaction
        let founder_transaction_amount = (transactions_amount * &fee)
            + if self.get_funds(&ROOT_PUBLIC_ADDRESS).await? >= *MAIN_CHAIN_PAYMENT {
                // if there is enough coins left in the root address make payment transaction
                self.decrease_funds(&ROOT_PUBLIC_ADDRESS, &MAIN_CHAIN_PAYMENT)
                    .await?;
                MAIN_CHAIN_PAYMENT.clone()
            } else {
                0usize.into()
            };

        transactions.push(Box::new(Transaction::new(
            ROOT_PUBLIC_ADDRESS,
            addr,
            timestamp,
            founder_transaction_amount.clone(),
            ROOT_PRIVATE_ADDRESS,
        )));

        self.add_funds(&addr, &founder_transaction_amount).await?;

        transactions.extend(
            (0..transactions_amount).map(|_| unsafe { trxs_pool.pop().unwrap_unchecked().1 }),
        );

        // get hashes & remove transaction references
        let transactions_hashes: Vec<_> = transactions.iter().map(|trx| trx.hash()).collect();

        // build merkle tree & get root
        let mut merkle_tree = MerkleTree::new();
        merkle_tree.add_objects(&transactions_hashes);
        let merkle_tree_root = *merkle_tree.get_root();

        let basic_info = BasicInfo::new(
            timestamp,
            pow,
            last_hash,
            self.main_chain.get_height().await,
            difficulty,
            addr,
        );

        // add block to the main chain
        let block = TransactionBlock::new(transactions_hashes, fee, basic_info, merkle_tree_root);
        self.main_chain.add_block_raw(&block).await?;

        // add transactions to the main chain
        self.main_chain.add_transactions_raw(transactions).await?;

        Ok(block)
    }

    async fn emit_summarize_block(
        &self,
        pow: BigUint,
        addr: [u8; 33],
        timestamp: u64,
        difficulty: [u8; 32],
    ) -> Result<SummarizeBlock, BlockChainTreeError> {
        let last_hash = self.main_chain.get_last_hash().await.change_context(
            BlockChainTreeError::BlockChainTree(BCTreeErrorKind::CreateMainChainBlock),
        )?;

        if !tools::check_pow(&last_hash, &difficulty, &pow) {
            // if pow is bad
            return Err(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::WrongPow,
            ))
            .into_report();
        }

        let basic_info = BasicInfo::new(
            timestamp,
            pow,
            last_hash,
            self.main_chain.get_height().await,
            difficulty,
            addr,
        );

        let founder_transaction = Transaction::new(
            ROOT_PUBLIC_ADDRESS,
            addr,
            timestamp,
            MAIN_CHAIN_PAYMENT.clone(),
            ROOT_PRIVATE_ADDRESS,
        );

        self.add_funds(&addr, &MAIN_CHAIN_PAYMENT).await?;

        let block = SummarizeBlock::new(basic_info, founder_transaction.hash());

        self.main_chain.add_block_raw(&block).await?;
        self.main_chain
            .add_transaction_raw(founder_transaction)
            .await?;

        Ok(block)
    }

    /// Set new difficulty for the chain
    pub async fn new_main_chain_difficulty(
        &self,
        timestamp: u64,
        difficulty: &mut [u8; 32],
        height: u64,
    ) -> Result<(), BlockChainTreeError> {
        // TODO: rewrite the way difficulty calculated
        if *difficulty != MAX_DIFFICULTY {
            let last_block = self.main_chain.find_by_height(height - 1).await?;
            if let Some(last_block) = last_block {
                let last_block_timestamp = last_block.get_info().timestamp;
                match (timestamp - last_block_timestamp).cmp(&600) {
                    std::cmp::Ordering::Less => {
                        for byte in difficulty.iter_mut() {
                            if *byte > 0 {
                                *byte <<= 1;
                                break;
                            }
                        }
                    }
                    std::cmp::Ordering::Equal => {}
                    std::cmp::Ordering::Greater => {
                        let mut index: usize = 0;
                        for (ind, byte) in difficulty.iter().enumerate() {
                            let byte = *byte;
                            if byte > 0 {
                                if byte == 0xFF && ind > 0 {
                                    index = ind - 1;
                                    break;
                                }
                                index = ind;
                                break;
                            }
                        }

                        difficulty[index] = (difficulty[index] >> 1) | 0b10000000;
                    }
                }
            }
        }
        Ok(())
    }

    /// Create main chain block and add it to the main chain
    ///
    /// Verifies POW and creates new main chain block
    ///
    /// Does not verify timestamp
    ///
    /// returns emmited block
    pub async fn emit_main_chain_block(
        &self,
        pow: BigUint,
        addr: [u8; 33],
        timestamp: u64,
    ) -> Result<Box<dyn MainChainBlock + Send + Sync>, BlockChainTreeError> {
        let mut difficulty = self.main_chain.get_locked_difficulty().await;
        let height = self.main_chain.get_height().await as usize;
        let block: Box<dyn MainChainBlock + Send + Sync> =
            if height % BLOCKS_PER_ITERATION == 0 && height > 0 {
                // new cycle
                let block = self
                    .emit_summarize_block(pow, addr, timestamp, *difficulty)
                    .await?;

                let mut summary_db_lock = self.summary_db.write().await;
                let mut old_summary_db_lock = self.old_summary_db.write().await;

                let (summary_db, old_summary_db) = self.move_summary_database()?;

                *summary_db_lock = SummaryDB::new(summary_db);
                *old_summary_db_lock = SummaryDB::new(old_summary_db);

                Box::new(block)
            } else {
                let block = self
                    .emit_transaction_block(pow, addr, timestamp, *difficulty)
                    .await?;

                Box::new(block)
            };

        self.new_main_chain_difficulty(timestamp, &mut difficulty, height as u64)
            .await?;

        Ok(block)
    }

    /// Adds new block, checks for block's validity
    ///
    /// returns true is block was added/already present
    ///
    /// returns false if the block is valid, but there is already a block there or it has diverging transactions
    ///
    /// returns error if the block couldn't be verified
    pub async fn new_main_chain_block(
        &self,
        new_block: &MainChainBlockArc,
    ) -> Result<bool, BlockChainTreeError> {
        let mut difficulty = self.main_chain.difficulty.write().await;
        let mut trxs_pool = self.trxs_pool.write().await;

        let height = *self.main_chain.height.read().await;
        let new_block_height = new_block.get_info().height;
        let new_block_hash = new_block.hash().change_context(BlockChainTreeError::Chain(
            ChainErrorKind::FailedToHashBlock,
        ))?;

        if new_block_height == 0 {
            return Err(
                Report::new(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                    .attach_printable("Tried to add block with height 0"),
            );
        }

        match new_block_height.cmp(&height) {
            Ordering::Less => {
                // not the last block
                let current_block = match self.main_chain.find_by_height(new_block_height).await? {
                    Some(block) => block,
                    None => {
                        return Err(Report::new(BlockChainTreeError::Chain(
                            ChainErrorKind::FailedToVerify,
                        )));
                    }
                };
                let current_block_hash =
                    current_block
                        .hash()
                        .change_context(BlockChainTreeError::Chain(
                            ChainErrorKind::FailedToHashBlock,
                        ))?;

                if current_block_hash == new_block_hash {
                    return Ok(true);
                }

                let prev_block = match self.main_chain.find_by_height(new_block_height - 1).await? {
                    Some(block) => block,
                    None => {
                        return Err(Report::new(BlockChainTreeError::Chain(
                            ChainErrorKind::FailedToVerify,
                        )));
                    }
                };
                let prev_block_hash =
                    prev_block
                        .hash()
                        .change_context(BlockChainTreeError::Chain(
                            ChainErrorKind::FailedToHashBlock,
                        ))?;

                if !new_block.verify_block(&prev_block.hash().change_context(
                    BlockChainTreeError::Chain(ChainErrorKind::FailedToHashBlock),
                )?) {
                    return Err(Report::new(BlockChainTreeError::Chain(
                        ChainErrorKind::FailedToVerify,
                    ))
                    .attach_printable("Wrong previous hash"));
                }

                if !check_pow(
                    &prev_block_hash,
                    &current_block.get_info().difficulty,
                    &new_block.get_info().pow,
                ) {
                    return Err(Report::new(BlockChainTreeError::Chain(
                        ChainErrorKind::FailedToVerify,
                    ))
                    .attach_printable("Bad POW"));
                }

                return Ok(false);
            }
            Ordering::Equal => {
                // the last block
                let last_hash = self
                    .main_chain
                    .get_last_hash()
                    .await
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                    .attach_printable("Couldn't find last hash")?;

                // verify new block with prev hash
                if !new_block.verify_block(&last_hash) {
                    return Err(Report::new(BlockChainTreeError::Chain(
                        ChainErrorKind::FailedToVerify,
                    ))
                    .attach_printable("Wrong previous hash"));
                }

                // verify new blck's pow
                if !tools::check_pow(&last_hash, &difficulty, &new_block.get_info().pow) {
                    // if pow is bad
                    return Err(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::WrongPow,
                    ))
                    .into_report();
                }

                // get last block of the chain
                let last_block = self
                    .main_chain
                    .get_last_block()
                    .await
                    .change_context(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                    .attach_printable("Couldn't find last block")?
                    .expect(
                        "Something went horribly wrong, couldn't find last block in main chain",
                    );

                // check new block's timestamp
                match new_block
                    .get_info()
                    .timestamp
                    .cmp(&last_block.get_info().timestamp)
                {
                    Ordering::Less | Ordering::Equal => {
                        return Err(Report::new(BlockChainTreeError::Chain(
                            ChainErrorKind::FailedToVerify,
                        ))
                        .attach_printable("The block is older, than the last block"));
                    }
                    _ => {}
                }

                if height as usize % BLOCKS_PER_ITERATION == 0 {
                    // summarize block
                    if new_block.get_transactions().len() != 1 {
                        return Err(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                            .into_report();
                    }
                    let founder_transaction = Transaction::new(
                        ROOT_PUBLIC_ADDRESS,
                        *new_block.get_founder(),
                        new_block.get_info().timestamp,
                        MAIN_CHAIN_PAYMENT.clone(),
                        ROOT_PRIVATE_ADDRESS,
                    );
                    let constructed_block = SummarizeBlock::new(
                        BasicInfo::new(
                            new_block.get_info().timestamp,
                            new_block.get_info().pow,
                            last_hash,
                            height,
                            *difficulty,
                            *new_block.get_founder(),
                        ),
                        founder_transaction.hash(),
                    );

                    if !new_block
                        .get_merkle_root()
                        .eq(&constructed_block.get_merkle_root())
                    {
                        return Err(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                            .into_report()
                            .attach_printable("The merkle root is wrong");
                    }

                    self.add_funds(new_block.get_founder(), &MAIN_CHAIN_PAYMENT)
                        .await?;

                    self.main_chain.add_block_raw(&constructed_block).await?;
                    self.main_chain
                        .add_transaction_raw(founder_transaction)
                        .await?;
                } else {
                    let transactions_amount = trxs_pool.len();

                    //let new_block_transactions = new_block.get_transactions();

                    let new_block_info = new_block.get_info();

                    let mut transactions_hashes: Vec<[u8; 32]> =
                        Vec::with_capacity(transactions_amount + 1);

                    let fee = Chain::calculate_fee(&difficulty);

                    let mut decrease_root_funds = false;

                    // founder transaction
                    let founder_transaction_amount = (transactions_amount * &fee)
                        + if self.get_funds(&ROOT_PUBLIC_ADDRESS).await? >= *MAIN_CHAIN_PAYMENT {
                            // if there is enough coins left in the root address make payment transaction
                            decrease_root_funds = true;
                            MAIN_CHAIN_PAYMENT.clone()
                        } else {
                            0usize.into()
                        };

                    let founder_transaction = Transaction::new(
                        ROOT_PUBLIC_ADDRESS,
                        new_block_info.founder,
                        new_block_info.timestamp,
                        founder_transaction_amount.clone(),
                        ROOT_PRIVATE_ADDRESS,
                    );

                    transactions_hashes.push(founder_transaction.hash());

                    // get sorted transactions
                    let mut transactions: Vec<_> = trxs_pool.pool.iter().collect();
                    transactions.sort();
                    transactions.reverse();
                    transactions_hashes.extend(transactions.iter().map(|tr| tr.hash()));

                    drop(transactions); // drop cuz not needed anymore

                    // construct new block from new_block data
                    let mut constructed_block = TransactionBlock::new(
                        transactions_hashes,
                        fee,
                        BasicInfo::new(
                            new_block_info.timestamp,
                            new_block_info.pow,
                            last_hash,
                            height,
                            *difficulty,
                            new_block_info.founder,
                        ),
                        new_block.get_merkle_root(),
                    );

                    // verify transactions
                    if !constructed_block.check_merkle_tree().map_err(|e| {
                        e.change_context(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                    })? {
                        return Ok(false);
                    }

                    // all checks passed, proceed to add block
                    if decrease_root_funds {
                        self.decrease_funds(&ROOT_PUBLIC_ADDRESS, &MAIN_CHAIN_PAYMENT)
                            .await?;
                    }

                    self.add_funds(&new_block_info.founder, &founder_transaction_amount)
                        .await?;

                    self.main_chain
                        .add_transaction_raw(founder_transaction)
                        .await?;

                    let transactions: Vec<_> = (0..transactions_amount)
                        .map(|_| unsafe { trxs_pool.pop().unwrap_unchecked().1 })
                        .collect();

                    self.main_chain.add_transactions_raw(transactions).await?;

                    self.main_chain.add_block_raw(&constructed_block).await?;
                }

                self.new_main_chain_difficulty(
                    new_block.get_info().timestamp,
                    &mut difficulty,
                    height,
                )
                .await?;
            }
            Ordering::Greater => {
                return Err(Report::new(BlockChainTreeError::Chain(
                    ChainErrorKind::FailedToVerify,
                ))
                .attach_printable("The block has bigger height, than the current chains height"));
            }
        }

        Ok(true)
    }

    /// Overwrites the block with same heigh if it existed
    ///
    /// also removes all higher blocks, linked transactions and derivative chains
    ///
    /// clears transactions pool
    pub async fn overwrite_main_chain_block<'a, I>(
        &self,
        new_block: &MainChainBlockArc,
        transactions: I,
    ) -> Result<(), BlockChainTreeError>
    where
        I: Iterator<Item = &'a Transaction>,
    {
        let mut difficulty = self.main_chain.difficulty.write().await;
        let mut trxs_pool = self.trxs_pool.write().await;

        let new_block_height = new_block.get_info().height;
        let new_block_hash = new_block.hash().change_context(BlockChainTreeError::Chain(
            ChainErrorKind::FailedToHashBlock,
        ))?;

        if new_block_height == 0 {
            return Err(
                Report::new(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                    .attach_printable("Tried to add block with height 0"),
            );
        }

        let current_block = match self.main_chain.find_by_height(new_block_height).await? {
            Some(block) => block,
            None => {
                return Err(Report::new(BlockChainTreeError::Chain(
                    ChainErrorKind::FailedToVerify,
                )));
            }
        };
        let current_block_hash =
            current_block
                .hash()
                .change_context(BlockChainTreeError::Chain(
                    ChainErrorKind::FailedToHashBlock,
                ))?;

        if current_block_hash == new_block_hash {
            return Ok(());
        }

        let prev_block = match self.main_chain.find_by_height(new_block_height - 1).await? {
            Some(block) => block,
            None => {
                return Err(Report::new(BlockChainTreeError::Chain(
                    ChainErrorKind::FailedToVerify,
                )));
            }
        };
        let prev_block_hash = prev_block
            .hash()
            .change_context(BlockChainTreeError::Chain(
                ChainErrorKind::FailedToHashBlock,
            ))?;

        if !new_block.verify_block(&prev_block.hash().change_context(
            BlockChainTreeError::Chain(ChainErrorKind::FailedToHashBlock),
        )?) {
            return Err(
                Report::new(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                    .attach_printable("Wrong previous hash"),
            );
        }

        if !check_pow(
            &prev_block_hash,
            &current_block.get_info().difficulty,
            &new_block.get_info().pow,
        ) {
            return Err(
                Report::new(BlockChainTreeError::Chain(ChainErrorKind::FailedToVerify))
                    .attach_printable("Bad POW"),
            );
        }

        let summary_db = self.summary_db.read().await;
        self.main_chain
            .block_overwrite(new_block, &summary_db)
            .await?;

        // TODO: add transations

        Ok(())
    }
}
