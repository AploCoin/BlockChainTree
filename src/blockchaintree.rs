#![allow(non_snake_case)]
use crate::block::{SumTransactionBlock, SummarizeBlock, TokenBlock, TransactionBlock};
use crate::tools;
use crate::transaction::{Transaction, Transactionable, TransactionableItem};
use num_bigint::BigUint;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::convert::TryInto;

use crate::dump_headers::Headers;
use hex::ToHex;
use num_traits::Zero;
use sled::Db;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::str;
use std::sync::Arc;
use tokio::sync::RwLock;

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
static GENESIS_BLOCK: [u8; 32] = [
    0x77, 0xe6, 0xd9, 0x52, 0x67, 0x57, 0x8e, 0x85, 0x39, 0xa9, 0xcf, 0xe0, 0x03, 0xf4, 0xf7, 0xfe,
    0x7d, 0x6a, 0x29, 0x0d, 0xaf, 0xa7, 0x73, 0xa6, 0x5c, 0x0f, 0x01, 0x9d, 0x5c, 0xbc, 0x0a, 0x7c,
];
static BEGINNING_DIFFICULTY: [u8; 32] = [
    0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];
// God is dead, noone will stop anarchy

static MAX_TRANSACTIONS_PER_BLOCK: usize = 3000;
static BLOCKS_PER_ITERATION: usize = 12960;

type TrxsPool = Arc<RwLock<BinaryHeap<TransactionableItem>>>;

type DerivativesCell = Arc<RwLock<DerivativeChain>>;
type Derivatives = Arc<RwLock<HashMap<[u8; 33], DerivativesCell>>>;

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

    pub async fn add_block(&self, block: &SumTransactionBlock) -> Result<(), BlockChainTreeError> {
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

        drop(height);

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

    /// dumps only transactions as of now, fix later
    pub async fn add_transaction(
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

    pub async fn transaction_exists(&self, hash: &[u8; 32]) -> Result<bool, BlockChainTreeError> {
        Ok(self
            .transactions
            .get(hash)
            .into_report()
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindTransaction))
            .attach_printable("Error getting transaction from database")?
            .is_some())
    }

    pub async fn get_height(&self) -> u64 {
        *self.height.read().await
    }

    pub async fn get_difficulty(&self) -> [u8; 32] {
        *self.difficulty.read().await
    }

    pub async fn find_by_height(
        &self,
        height: u64,
    ) -> Result<Option<SumTransactionBlock>, BlockChainTreeError> {
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

        if dump[0] == Headers::TransactionBlock as u8 {
            let result = TransactionBlock::parse(&dump[1..], (dump.len() - 1) as u32)
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?;

            let block = SumTransactionBlock::new(Some(result), None);

            return Ok(Some(block));
        } else if dump[0] == Headers::SummarizeBlock as u8 {
            let result = SummarizeBlock::parse(&dump[1..])
                .change_context(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))?;

            let block = SumTransactionBlock::new(None, Some(result));
            return Ok(Some(block));
        }

        Err(
            Report::new(BlockChainTreeError::Chain(ChainErrorKind::FindByHeight))
                .attach_printable("block type not found"),
        )
    }

    pub async fn find_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<SumTransactionBlock>, BlockChainTreeError> {
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
            height: Arc::new(RwLock::new(0)),
            genesis_hash: *genesis_hash,
            difficulty: Arc::new(RwLock::new(BEGINNING_DIFFICULTY)),
        })
    }

    pub async fn get_last_block(&self) -> Result<Option<SumTransactionBlock>, BlockChainTreeError> {
        let height = self.height.read().await;
        let last_block_index = *height - 1;
        drop(height);

        self.find_by_height(last_block_index).await
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

    pub fn get_height(&self) -> u64 {
        self.height
    }

    pub fn get_difficulty(&self) -> [u8; 32] {
        self.difficulty
    }

    pub fn get_global_height(&self) -> u64 {
        self.global_height
    }

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

    pub fn get_last_block(&self) -> Result<Option<TokenBlock>, BlockChainTreeError> {
        self.find_by_height(self.height - 1)
    }
}

#[derive(Clone)]
pub struct BlockChainTree {
    trxs_pool: TrxsPool,
    trxs_hashes: Arc<RwLock<HashSet<[u8; 32]>>>,
    summary_db: Arc<Option<Db>>,
    old_summary_db: Arc<Option<Db>>,
    main_chain: Arc<Chain>,
    deratives: Derivatives,
}

impl BlockChainTree {
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
        let mut trxs_pool = BinaryHeap::<TransactionableItem>::with_capacity(trxs_amount as usize);

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
            summary_db: Arc::new(Some(summary_db)),
            main_chain: Arc::new(main_chain),
            old_summary_db: Arc::new(Some(old_summary_db)),
            deratives: Arc::default(),
            trxs_hashes: Arc::default(),
        })
    }

    pub fn without_config() -> Result<BlockChainTree, BlockChainTreeError> {
        let summary_db_path = Path::new(&AMMOUNT_SUMMARY);

        // open summary db
        let summary_db = sled::open(summary_db_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open summary db")?;

        let old_summary_db_path = Path::new(&OLD_AMMOUNT_SUMMARY);

        // open old summary db
        let old_summary_db = sled::open(old_summary_db_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::InitWithoutConfig,
            ))
            .attach_printable("failed to open old summary db")?;

        // allocate VecDeque
        let trxs_pool = BinaryHeap::<TransactionableItem>::new();

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
            summary_db: Arc::new(Some(summary_db)),
            main_chain: Arc::new(main_chain),
            old_summary_db: Arc::new(Some(old_summary_db)),
            deratives: Arc::default(),
            trxs_hashes: Arc::default(),
        })
    }

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
        for transaction in trxs_pool.iter() {
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

    pub async fn add_funds(
        &self,
        addr: &[u8; 33],
        funds: &BigUint,
    ) -> Result<(), BlockChainTreeError> {
        let result = self.summary_db.as_ref().as_ref().unwrap().get(addr);
        match result {
            Ok(None) => {
                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(funds));
                tools::dump_biguint(funds, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                let mut db_ref = Option::as_ref(&self.summary_db);
                let db = db_ref.as_mut().unwrap();

                db.insert(addr, dump)
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                db.flush_async()
                    .await
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                Ok(())
            }
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                let mut previous = res.0;
                previous += funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&previous));
                tools::dump_biguint(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                let mut db_ref = Option::as_ref(&self.summary_db);
                let db = db_ref.as_mut().unwrap();

                db.insert(addr, dump)
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to put funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                db.flush_async()
                    .await
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                Ok(())
            }
            Err(_) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::AddFunds,
            ))
            .attach_printable(format!(
                "failed to get data from address: {}",
                std::str::from_utf8(addr).unwrap()
            ))),
        }
    }

    pub async fn decrease_funds(
        &self,
        addr: &[u8; 33],
        funds: &BigUint,
    ) -> Result<(), BlockChainTreeError> {
        let mut db_ref = Option::as_ref(&self.summary_db);
        let db = db_ref.as_mut().unwrap();

        let result = db.get(addr);
        match result {
            Ok(None) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::DecreaseFunds,
            ))
            .attach_printable(format!(
                "address: {} doesn't have any coins",
                std::str::from_utf8(addr).unwrap()
            ))),
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DecreaseFunds),
                )?;

                let mut previous = res.0;
                if previous < *funds {
                    return Err(Report::new(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::DecreaseFunds,
                    ))
                    .attach_printable("insufficient balance"));
                }
                previous -= funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&previous));
                tools::dump_biguint(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DecreaseFunds),
                )?;

                db.insert(addr, dump)
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::DecreaseFunds,
                    ))
                    .attach_printable(format!(
                        "failed to put funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                db.flush_async()
                    .await
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                Ok(())
            }
            Err(_) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::DecreaseFunds,
            ))
            .attach_printable(format!(
                "failed to get data from address: {}",
                std::str::from_utf8(addr).unwrap()
            ))),
        }
    }

    pub fn get_funds(&self, addr: &[u8; 33]) -> Result<BigUint, BlockChainTreeError> {
        match Option::as_ref(&self.summary_db).as_ref().unwrap().get(addr) {
            Ok(None) => Ok(Zero::zero()),
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::GetFunds),
                )?;

                let previous = res.0;
                Ok(previous)
            }
            Err(_) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::GetDerivChain,
            ))
            .attach_printable(format!(
                "failed to get data from summary db at address: {}",
                std::str::from_utf8(addr).unwrap()
            ))),
        }
    }

    pub fn get_old_funds(&self, addr: &[u8; 33]) -> Result<BigUint, BlockChainTreeError> {
        match Option::as_ref(&self.old_summary_db)
            .as_ref()
            .unwrap()
            .get(addr)
        {
            Ok(None) => Ok(Zero::zero()),
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::GetOldFunds),
                )?;
                let previous = res.0;
                Ok(previous)
            }
            Err(_) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::GetOldFunds,
            ))),
        }
    }

    pub fn move_summary_database(&mut self) -> Result<(), BlockChainTreeError> {
        let old_sum_path = Path::new(OLD_AMMOUNT_SUMMARY);
        let sum_path = Path::new(AMMOUNT_SUMMARY);

        self.old_summary_db = Arc::new(None);
        self.summary_db = Arc::new(None);

        fs::remove_dir_all(old_sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to remove previous database")?;

        fs::rename(sum_path, old_sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to rename folder for summary db")?;

        fs::create_dir(sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to create folder for an old summarize db")?;

        let result = sled::open(sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to open summary db")?;

        self.summary_db = Arc::new(Some(result));

        let result = sled::open(old_sum_path)
            .into_report()
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to open old summary db")?;

        self.old_summary_db = Arc::new(Some(result));

        Ok(())
    }

    /// Check whether transaction with same hash exists
    ///
    ///
    pub async fn transaction_exists(&self, hash: &[u8; 32]) -> Result<bool, BlockChainTreeError> {
        if self.trxs_hashes.read().await.get(hash).is_some() {
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
        if self.transaction_exists(&tr.hash()).await? {
            return Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))
            .attach_printable("Transaction with same hash found"));
        }

        let trxs_pool_len = self.trxs_pool.read().await.len() + 1;
        self.trxs_pool.write().await.push(Box::new(tr.clone()));

        self.trxs_hashes.write().await.insert(tr.hash());

        // if it is in first bunch of transactions
        // to be added to blockchain.
        // AND if it is not a last block
        // that is pending.
        if trxs_pool_len < MAX_TRANSACTIONS_PER_BLOCK
            && self.main_chain.get_height().await as usize + 1 % BLOCKS_PER_ITERATION != 0
        {
            self.decrease_funds(tr.get_sender(), tr.get_amount())
                .await
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::NewTransaction,
                ))?;

            self.add_funds(tr.get_sender(), tr.get_amount())
                .await
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::NewTransaction,
                ))?;
        }

        Ok(())
    }

    pub async fn pop_last_transactions(&mut self) -> Option<Vec<TransactionableItem>> {
        let trxs_pool = self.trxs_pool.read().await;
        if trxs_pool.is_empty() {
            return None;
        }

        let transactions_amount = if MAX_TRANSACTIONS_PER_BLOCK > trxs_pool.len() {
            trxs_pool.len()
        } else {
            MAX_TRANSACTIONS_PER_BLOCK
        };

        let mut to_return: Vec<TransactionableItem> = Vec::with_capacity(transactions_amount);

        let mut counter = 0;

        while counter < transactions_amount {
            if let Some(tr) = self.trxs_pool.write().await.pop() {
                to_return.push(tr);
            } else {
                break;
            }

            counter += 1;
        }
        Some(to_return)
    }

    // pub fn get_pool(&mut self) -> &VecDeque<Box<dyn Transactionable>> {
    //     &self.trxs_pool
    // }
}
