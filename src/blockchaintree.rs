#![allow(non_snake_case)]
use crate::block::{SumTransactionBlock, SummarizeBlock, TokenBlock, TransactionBlock};
use crate::tools;
use crate::transaction::{Transaction, Transactionable};
use num_bigint::BigUint;
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::convert::TryInto;
use std::mem::transmute_copy;

use crate::dump_headers::Headers;
use hex::ToHex;
use num_traits::Zero;
use rocksdb::{DBWithThreadMode as DB, MultiThreaded, Options};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::str;

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

pub struct Chain {
    db: DB<MultiThreaded>,
    height_reference: DB<MultiThreaded>,
    height: u64,
    genesis_hash: [u8; 32],
    difficulty: [u8; 32],
}

impl Chain {
    pub fn new() -> Result<Chain, BlockChainTreeError> {
        let root = String::from(MAIN_CHAIN_DIRECTORY);
        let path_blocks_st = root.clone() + BLOCKS_FOLDER;
        let path_references_st = root.clone() + REFERENCES_FOLDER;
        let path_height_st = root + CONFIG_FILE;

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);
        let path_height = Path::new(&path_height_st);

        // open blocks DB
        let db = DB::<MultiThreaded>::open_default(path_blocks)
            .report()
            .change_context(BlockChainTreeError::ChainError(ChainErrorKind::InitError))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = DB::<MultiThreaded>::open_default(path_reference)
            .report()
            .change_context(BlockChainTreeError::ChainError(ChainErrorKind::InitError))
            .attach_printable("failed to open references db")?;

        let mut file = File::open(path_height)
            .report()
            .change_context(BlockChainTreeError::ChainError(ChainErrorKind::InitError))?;

        // read height from config
        let mut height_bytes: [u8; 8] = [0; 8];

        file.read_exact(&mut height_bytes)
            .report()
            .change_context(BlockChainTreeError::ChainError(ChainErrorKind::InitError))
            .attach_printable("failed to read config")?;

        let height: u64 = u64::from_be_bytes(height_bytes);

        // read genesis hash
        let mut genesis_hash: [u8; 32] = [0; 32];
        file.read_exact(&mut genesis_hash)
            .report()
            .change_context(BlockChainTreeError::ChainError(ChainErrorKind::InitError))
            .attach_printable("failed to read genesis hash")?;

        // read difficulty
        let mut difficulty: [u8; 32] = [0; 32];
        file.read_exact(&mut difficulty)
            .report()
            .change_context(BlockChainTreeError::ChainError(ChainErrorKind::InitError))
            .attach_printable("failed to read difficulty")?;

        return Ok(Chain {
            db,
            height_reference,
            height,
            genesis_hash,
            difficulty,
        });
    }

    pub fn add_block(&mut self, block: &SumTransactionBlock) -> Result<(), BlockChainTreeError> {
        let dump = block
            .dump()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::AddingBlockError,
            ))?;

        let hash = tools::hash(&dump);

        self.db
            .put(hash, dump)
            .report()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::AddingBlockError,
            ))?;

        self.height_reference
            .put(hash, self.height.to_be_bytes())
            .report()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::AddingBlockError,
            ));

        self.height += 1;

        return Ok(());
    }

    pub fn get_height(&self) -> u64 {
        return self.height;
    }

    pub fn get_difficulty(&self) -> [u8; 32] {
        return self.difficulty;
    }

    pub fn find_by_height(
        &self,
        height: u64,
    ) -> Result<Option<SumTransactionBlock>, BlockChainTreeError> {
        if height > self.height {
            return Ok(None);
        }
        let dump = self.db.get(height.to_be_bytes()).report().change_context(
            BlockChainTreeError::ChainError(ChainErrorKind::FindByHeightError),
        )?;

        if dump.is_none() {
            return Ok(None);
        }

        let dump = dump.unwrap();

        if dump[0] == Headers::TransactionBlock as u8 {
            let result = TransactionBlock::parse(&dump[1..], (dump.len() - 1) as u32)
                .change_context(BlockChainTreeError::ChainError(
                    ChainErrorKind::FindByHeightError,
                ))?;

            let block = SumTransactionBlock::new(Some(result), None);

            return Ok(Some(block));
        } else if dump[0] == Headers::SummarizeBlock as u8 {
            let result = SummarizeBlock::parse(&dump[1..]).change_context(
                BlockChainTreeError::ChainError(ChainErrorKind::FindByHeightError),
            )?;

            let block = SumTransactionBlock::new(None, Some(result));
            return Ok(Some(block));
        }

        return Err(Report::new(BlockChainTreeError::ChainError(
            ChainErrorKind::FindByHeightError,
        ))
        .attach_printable("block type not found"));
    }

    pub fn find_by_hash(
        &self,
        hash: &[u8; 32],
    ) -> Result<Option<SumTransactionBlock>, BlockChainTreeError> {
        let result = self.height_reference.get(hash).report().change_context(
            BlockChainTreeError::ChainError(ChainErrorKind::FindByHashError),
        )?;

        if result.is_none() {
            return Ok(None);
        }
        let height = u64::from_be_bytes(result.unwrap().try_into().unwrap());

        let block = self
            .find_by_height(height)
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::FindByHashError,
            ))?;

        return Ok(block);
    }

    pub fn dump_config(&self) -> Result<(), BlockChainTreeError> {
        let root = String::from(MAIN_CHAIN_DIRECTORY);
        let path_config = root + CONFIG_FILE;

        let mut file =
            File::create(path_config)
                .report()
                .change_context(BlockChainTreeError::ChainError(
                    ChainErrorKind::DumpConfigError,
                ))?;

        let result = file
            .write_all(&self.height.to_be_bytes())
            .report()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failed to write height")?;

        file.write_all(&self.genesis_hash)
            .report()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failed to write genesis block")?;

        file.write_all(&self.difficulty)
            .report()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failes to write difficulty")?;

        return Ok(());
    }

    pub fn new_without_config(
        root_path: &str,
        genesis_hash: &[u8; 32],
    ) -> Result<Chain, BlockChainTreeError> {
        let root = String::from(root_path);
        let path_blocks_st = root.clone() + BLOCKS_FOLDER;
        let path_references_st = root.clone() + REFERENCES_FOLDER;

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);

        // open blocks DB
        let db = DB::<MultiThreaded>::open_default(path_blocks)
            .report()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::InitWithoutConfigError,
            ))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = DB::<MultiThreaded>::open_default(path_reference)
            .report()
            .change_context(BlockChainTreeError::ChainError(
                ChainErrorKind::InitWithoutConfigError,
            ))
            .attach_printable("failed to open references db")?;

        return Ok(Chain {
            db,
            height_reference,
            height: 0,
            genesis_hash: *genesis_hash,
            difficulty: BEGINNING_DIFFICULTY,
        });
    }

    pub fn get_last_block(&self) -> Result<Option<SumTransactionBlock>, BlockChainTreeError> {
        return self.find_by_height(self.height - 1);
    }
}

pub struct DerivativeChain {
    db: DB<MultiThreaded>,
    height_reference: DB<MultiThreaded>,
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
        let db = DB::<MultiThreaded>::open_default(path_blocks)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = DB::<MultiThreaded>::open_default(path_reference)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to open references db")?;

        let mut file = File::open(path_height)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to open config")?;

        // read height from config
        let mut height_bytes: [u8; 8] = [0; 8];
        file.read_exact(&mut height_bytes)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to read config")?;

        let height: u64 = u64::from_be_bytes(height_bytes);

        // read genesis hash
        let mut genesis_hash: [u8; 32] = [0; 32];
        file.read_exact(&mut genesis_hash)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to open genesis hash from config")?;

        // read difficulty
        let mut difficulty: [u8; 32] = [0; 32];
        file.read_exact(&mut difficulty)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to read difficulty from config")?;

        // read global height
        let mut global_height: [u8; 8] = [0; 8];
        file.read_exact(&mut global_height)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to read global height from config")?;

        let global_height: u64 = u64::from_be_bytes(global_height);

        return Ok(DerivativeChain {
            db,
            height_reference,
            height,
            genesis_hash,
            difficulty,
            global_height,
        });
    }

    pub fn add_block(&mut self, block: &TokenBlock) -> Result<(), BlockChainTreeError> {
        let dump = block
            .dump()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::AddingBlockError,
            ))?;

        let hash = tools::hash(&dump);

        self.db
            .put(self.height.to_be_bytes(), dump)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to add block to db")?;

        let result = self
            .height_reference
            .put(hash, self.height.to_be_bytes())
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitError,
            ))
            .attach_printable("failed to add reference to db")?;

        self.height += 1;

        return Ok(());
    }

    pub fn get_height(&self) -> u64 {
        return self.height;
    }

    pub fn get_difficulty(&self) -> [u8; 32] {
        return self.difficulty;
    }

    pub fn get_global_height(&self) -> u64 {
        return self.global_height;
    }

    pub fn find_by_height(&self, height: u64) -> Result<Option<TokenBlock>, BlockChainTreeError> {
        if height > self.height {
            return Ok(None);
        }
        let dump = self
            .db
            .get(height.to_be_bytes())
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::FindByHeightError,
            ))
            .attach_printable("failed to read block")?;

        if dump.is_none() {
            return Ok(None);
        }
        let dump = dump.unwrap();

        if dump[0] != Headers::TokenBlock as u8 {
            return Err(Report::new(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::FindByHeightError,
            ))
            .attach_printable("wrong header"));
        }
        let block = TokenBlock::parse(&dump[1..], (dump.len() - 1) as u32).change_context(
            BlockChainTreeError::DerivativeChainError(DerivChainErrorKind::FindByHeightError),
        )?;

        return Ok(Some(block));
    }

    pub fn find_by_hash(&self, hash: &[u8; 32]) -> Result<Option<TokenBlock>, BlockChainTreeError> {
        let result = self
            .height_reference
            .get(hash)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::FindByHashError,
            ))
            .attach_printable("failed to get height")?;

        if result.is_none() {
            return Ok(None);
        }
        let height = u64::from_be_bytes(result.unwrap().try_into().unwrap());

        let block = self.find_by_height(height).change_context(
            BlockChainTreeError::DerivativeChainError(DerivChainErrorKind::FindByHashError),
        )?;

        return Ok(block);
    }

    pub fn dump_config(&self, root_path: &str) -> Result<(), BlockChainTreeError> {
        let root = String::from(root_path);
        let path_config = root + CONFIG_FILE;

        let mut file = File::create(path_config)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failed to open config")?;

        file.write_all(&self.height.to_be_bytes())
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failed to write height")?;

        file.write_all(&self.genesis_hash)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failed to write genesis block")?;

        file.write_all(&self.difficulty)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failed to write difficulty")?;

        file.write_all(&self.global_height.to_be_bytes())
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::DumpConfigError,
            ))
            .attach_printable("failed to write global height")?;

        return Ok(());
    }

    pub fn without_config(
        root_path: &str,
        genesis_hash: &[u8; 32],
        global_height: u64,
    ) -> Result<DerivativeChain, BlockChainTreeError> {
        let root = String::from(root_path);
        let path_blocks_st = root.clone() + BLOCKS_FOLDER;
        let path_references_st = root.clone() + REFERENCES_FOLDER;

        let path_blocks = Path::new(&path_blocks_st);
        let path_reference = Path::new(&path_references_st);

        // open blocks DB
        let db = DB::<MultiThreaded>::open_default(path_blocks)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitWithoutConfigError,
            ))
            .attach_printable("failed to open blocks db")?;

        // open height references DB
        let height_reference = DB::<MultiThreaded>::open_default(path_reference)
            .report()
            .change_context(BlockChainTreeError::DerivativeChainError(
                DerivChainErrorKind::InitWithoutConfigError,
            ))
            .attach_printable("failed to open references db")?;

        return Ok(DerivativeChain {
            db,
            height_reference,
            height: 0,
            genesis_hash: *genesis_hash,
            difficulty: BEGINNING_DIFFICULTY,
            global_height,
        });
    }

    pub fn get_last_block(&self) -> Result<Option<TokenBlock>, BlockChainTreeError> {
        return self.find_by_height(self.height - 1);
    }
}

pub struct BlockChainTree {
    trxs_pool: VecDeque<Box<dyn Transactionable>>,
    summary_db: Option<DB<MultiThreaded>>,
    old_summary_db: Option<DB<MultiThreaded>>,
    main_chain: Chain,
}

impl BlockChainTree {
    pub fn with_config() -> Result<BlockChainTree, BlockChainTreeError> {
        let summary_db_path = Path::new(&AMMOUNT_SUMMARY);

        // open summary db
        let summary_db = DB::<MultiThreaded>::open_default(summary_db_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::InitError,
            ))
            .attach_printable("failed to open summary db")?;

        let old_summary_db_path = Path::new(&OLD_AMMOUNT_SUMMARY);

        // open old summary db
        let old_summary_db = DB::<MultiThreaded>::open_default(old_summary_db_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::InitError,
            ))
            .attach_printable("failed to open old summary db")?;

        // read transactions pool
        let pool_path = String::from(BLOCKCHAIN_DIRECTORY) + TRANSACTIONS_POOL;
        let pool_path = Path::new(&pool_path);

        let mut file = File::open(pool_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::InitError,
            ))
            .attach_printable("failed to open transactions pool")?;

        // read amount of transactions
        let mut buf: [u8; 8] = [0; 8];
        file.read_exact(&mut buf)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::InitError,
            ))
            .attach_printable("failed to read amount of transactions")?;

        let trxs_amount = u64::from_be_bytes(buf);

        let mut buf: [u8; 4] = [0; 4];

        // allocate VecDeque
        let mut trxs_pool =
            VecDeque::<Box<dyn Transactionable>>::with_capacity(trxs_amount as usize);

        // parsing transactions
        for _ in 0..trxs_amount {
            file.read_exact(&mut buf)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::InitError,
                ))
                .attach_printable("failed to read transaction size")?;

            let tr_size = u32::from_be_bytes(buf);

            let mut transaction_buffer = vec![0u8; (tr_size - 1) as usize];

            file.read_exact(&mut transaction_buffer)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::InitError,
                ))
                .attach_printable("failed to read transaction")?;

            if transaction_buffer[0] == 0 {
                let transaction =
                    Transaction::parse(&transaction_buffer[1..], (tr_size - 1) as u64)
                        .change_context(BlockChainTreeError::BlockChainTreeError(
                            BCTreeErrorKind::InitError,
                        ))?;

                trxs_pool.push_back(Box::new(transaction));
            } else {
                return Err(Report::new(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::InitError,
                ))
                .attach_printable("Not implemented yet"));
            }
        }

        // opening main chain
        let main_chain = Chain::new().change_context(BlockChainTreeError::BlockChainTreeError(
            BCTreeErrorKind::InitError,
        ))?;

        return Ok(BlockChainTree {
            trxs_pool,
            summary_db: Some(summary_db),
            main_chain,
            old_summary_db: Some(old_summary_db),
        });
    }

    pub fn without_config() -> Result<BlockChainTree, BlockChainTreeError> {
        let summary_db_path = Path::new(&AMMOUNT_SUMMARY);

        // open summary db
        let summary_db = DB::<MultiThreaded>::open_default(summary_db_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::InitWithoutConfigError,
            ))
            .attach_printable("failed to open summary db")?;

        let old_summary_db_path = Path::new(&OLD_AMMOUNT_SUMMARY);

        // open old summary db
        let old_summary_db = DB::<MultiThreaded>::open_default(old_summary_db_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::InitWithoutConfigError,
            ))
            .attach_printable("failed to open old summary db")?;

        // allocate VecDeque
        let mut trxs_pool = VecDeque::<Box<dyn Transactionable>>::new();

        // opening main chain
        let main_chain = Chain::new_without_config(MAIN_CHAIN_DIRECTORY, &GENESIS_BLOCK)
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::InitWithoutConfigError,
            ))
            .attach_printable("failed to open main chain")?;

        return Ok(BlockChainTree {
            trxs_pool,
            summary_db: Some(summary_db),
            main_chain,
            old_summary_db: Some(old_summary_db),
        });
    }

    pub fn dump_pool(&self) -> Result<(), BlockChainTreeError> {
        let pool_path = String::from(BLOCKCHAIN_DIRECTORY) + TRANSACTIONS_POOL;
        let pool_path = Path::new(&pool_path);

        // open file
        let mut file = File::create(pool_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::DumpPoolError,
            ))
            .attach_printable("failed to open config file")?;

        // write transactions amount
        file.write_all(&(self.trxs_pool.len() as u64).to_be_bytes())
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::DumpPoolError,
            ))
            .attach_printable("failed to write amount of transactions")?;

        //write transactions
        for transaction in self.trxs_pool.iter() {
            // get dump
            let dump =
                transaction
                    .dump()
                    .change_context(BlockChainTreeError::BlockChainTreeError(
                        BCTreeErrorKind::DumpPoolError,
                    ))?;

            // write transaction size
            file.write_all(&(dump.len() as u32).to_be_bytes())
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::DumpPoolError,
                ))
                .attach_printable("failed to write transaction size")?;

            // write transaction dump
            file.write_all(&dump)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::DumpPoolError,
                ))
                .attach_printable("failed to write transaction dump")?;
        }

        return Ok(());
    }

    pub fn get_derivative_chain(
        &mut self,
        addr: &[u8; 33],
    ) -> Result<Option<Box<DerivativeChain>>, BlockChainTreeError> {
        let mut path_string = String::from(DERIVATIVE_CHAINS_DIRECTORY);
        let hex_addr: String = addr.encode_hex::<String>();
        path_string += &hex_addr;
        path_string += "/";

        let path = Path::new(&path_string);
        if path.exists() {
            let result = DerivativeChain::new(&path_string).change_context(
                BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::GetDerivChainError),
            )?;

            let chain = Box::new(result);

            return Ok(Some(chain));
        }

        return Ok(None);
    }

    pub fn get_main_chain(&mut self) -> &mut Chain {
        return &mut self.main_chain;
    }

    pub fn create_derivative_chain(
        addr: &[u8; 33],
        genesis_hash: &[u8; 32],
        global_height: u64,
    ) -> Result<DerivativeChain, BlockChainTreeError> {
        let mut root_path = String::from(DERIVATIVE_CHAINS_DIRECTORY);
        let hex_addr: String = addr.encode_hex::<String>();
        root_path += &hex_addr;
        root_path += "/";

        fs::create_dir(Path::new(&root_path))
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::CreateDerivChainError,
            ))
            .attach_printable("failed to create root folder")?;

        let blocks_path = root_path.clone() + BLOCKS_FOLDER;
        fs::create_dir(Path::new(&blocks_path))
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::CreateDerivChainError,
            ))
            .attach_printable("failed to create blocks folder")?;

        let references_path = root_path.clone() + REFERENCES_FOLDER;
        fs::create_dir(Path::new(&references_path))
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::CreateDerivChainError,
            ))
            .attach_printable("failed to create references folder")?;

        let chain = DerivativeChain::without_config(&root_path, genesis_hash, global_height)
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::CreateDerivChainError,
            ))?;

        chain
            .dump_config(&root_path)
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::CreateDerivChainError,
            ))?;

        return Ok(chain);
    }

    pub fn check_main_folders() -> Result<(), BlockChainTreeError> {
        let root = Path::new(BLOCKCHAIN_DIRECTORY);
        if !root.exists() {
            fs::create_dir(root)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create blockchain root")?;
        }

        let main_path = Path::new(MAIN_CHAIN_DIRECTORY);
        if !main_path.exists() {
            fs::create_dir(main_path)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create main chain folder")?;
        }

        let summary_path = Path::new(AMMOUNT_SUMMARY);
        if !summary_path.exists() {
            fs::create_dir(summary_path)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create summary folder")?;
        }

        let old_summary_path = Path::new(OLD_AMMOUNT_SUMMARY);
        if !old_summary_path.exists() {
            fs::create_dir(old_summary_path)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create old summary folder")?;
        }

        let blocks_path = String::from(MAIN_CHAIN_DIRECTORY) + BLOCKS_FOLDER;
        let blocks_path = Path::new(&blocks_path);
        if !blocks_path.exists() {
            fs::create_dir(blocks_path)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create blocks path")?;
        }

        let references_path = String::from(MAIN_CHAIN_DIRECTORY) + REFERENCES_FOLDER;
        let references_path = Path::new(&references_path);
        if !references_path.exists() {
            fs::create_dir(references_path)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create references paths")?;
        }

        let derivatives_path = String::from(DERIVATIVE_CHAINS_DIRECTORY);
        let derivatives_path = Path::new(&derivatives_path);
        if !derivatives_path.exists() {
            fs::create_dir(derivatives_path)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create derivatives chains path")?;
        }

        let derivative_chains_path = String::from(DERIVATIVE_CHAINS_DIRECTORY) + CHAINS_FOLDER;
        let derivative_chains_path = Path::new(&derivative_chains_path);
        if !derivative_chains_path.exists() {
            fs::create_dir(derivative_chains_path)
                .report()
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::CheckMainFoldersError,
                ))
                .attach_printable("failed to create derivative chains folder")?;
        }

        return Ok(());
    }

    // summary data bases functions

    pub fn add_funds(
        &mut self,
        addr: &[u8; 33],
        funds: &BigUint,
    ) -> Result<(), BlockChainTreeError> {
        let result = self.summary_db.as_mut().unwrap().get(addr);
        match result {
            Ok(None) => {
                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&funds));
                tools::dump_biguint(funds, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::AddFundsError),
                )?;

                self.summary_db
                    .as_mut()
                    .unwrap()
                    .put(addr, &dump)
                    .report()
                    .change_context(BlockChainTreeError::BlockChainTreeError(
                        BCTreeErrorKind::AddFundsError,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                return Ok(());
            }
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::AddFundsError),
                )?;

                let mut previous = res.0;
                previous += funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&previous));
                tools::dump_biguint(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::AddFundsError),
                )?;

                let res = self
                    .summary_db
                    .as_mut()
                    .unwrap()
                    .put(addr, &dump)
                    .report()
                    .change_context(BlockChainTreeError::BlockChainTreeError(
                        BCTreeErrorKind::AddFundsError,
                    ))
                    .attach_printable(format!(
                        "failed to put funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                return Ok(());
            }
            Err(_) => {
                return Err(Report::new(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::AddFundsError,
                ))
                .attach_printable(format!(
                    "failed to get data from address: {}",
                    std::str::from_utf8(addr).unwrap()
                )));
            }
        }
    }

    pub fn decrease_funds(
        &mut self,
        addr: &[u8; 33],
        funds: &BigUint,
    ) -> Result<(), BlockChainTreeError> {
        let result = self.summary_db.as_mut().unwrap().get(addr);
        match result {
            Ok(None) => {
                return Err(Report::new(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::DecreaseFundsError,
                ))
                .attach_printable(format!(
                    "address: {} doesn't have any coins",
                    std::str::from_utf8(addr).unwrap()
                )));
            }
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::DecreaseFundsError),
                )?;

                let mut previous = res.0;
                if previous < *funds {
                    return Err(Report::new(BlockChainTreeError::BlockChainTreeError(
                        BCTreeErrorKind::DecreaseFundsError,
                    ))
                    .attach_printable("insufficient balance"));
                }
                previous -= funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&previous));
                tools::dump_biguint(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::DecreaseFundsError),
                )?;

                self.summary_db
                    .as_mut()
                    .unwrap()
                    .put(addr, &dump)
                    .report()
                    .change_context(BlockChainTreeError::BlockChainTreeError(
                        BCTreeErrorKind::DecreaseFundsError,
                    ))
                    .attach_printable(format!(
                        "failed to put funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                return Ok(());
            }
            Err(_) => {
                return Err(Report::new(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::DecreaseFundsError,
                ))
                .attach_printable(format!(
                    "failed to get data from address: {}",
                    std::str::from_utf8(addr).unwrap()
                )));
            }
        }
    }

    pub fn get_funds(&mut self, addr: &[u8; 33]) -> Result<BigUint, BlockChainTreeError> {
        let result = self.summary_db.as_mut().unwrap().get(addr);
        match result {
            Ok(None) => {
                return Ok(Zero::zero());
            }
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::GetFundsError),
                )?;

                let previous = res.0;
                return Ok(previous);
            }
            Err(_) => {
                return Err(Report::new(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::GetDerivChainError,
                ))
                .attach_printable(format!(
                    "failed to get data from summary db at address: {}",
                    std::str::from_utf8(addr).unwrap()
                )));
            }
        }
    }

    pub fn get_old_funds(&mut self, addr: &[u8; 33]) -> Result<BigUint, BlockChainTreeError> {
        let result = self.old_summary_db.as_mut().unwrap().get(addr);
        match result {
            Ok(None) => {
                return Ok(Zero::zero());
            }
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTreeError(BCTreeErrorKind::GetOldFundsError),
                )?;
                let previous = res.0;
                return Ok(previous);
            }
            Err(_) => {
                return Err(Report::new(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::GetOldFundsError,
                )));
            }
        }
    }

    pub fn move_summary_database(&mut self) -> Result<(), BlockChainTreeError> {
        let old_sum_path = Path::new(OLD_AMMOUNT_SUMMARY);
        let sum_path = Path::new(AMMOUNT_SUMMARY);

        self.old_summary_db = None;
        self.summary_db = None;

        let result = DB::<MultiThreaded>::destroy(&Options::default(), old_sum_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::MoveSummaryDBError,
            ))
            .attach_printable("failed to remove previous database")?;

        fs::rename(sum_path, old_sum_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::MoveSummaryDBError,
            ))
            .attach_printable("failed to rename folder for summary db")?;

        fs::create_dir(sum_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::MoveSummaryDBError,
            ))
            .attach_printable("failed to create folder for an old summarize db")?;

        let result = DB::<MultiThreaded>::open_default(sum_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::MoveSummaryDBError,
            ))
            .attach_printable("failed to open summary db")?;

        self.summary_db = Some(result);

        let result = DB::<MultiThreaded>::open_default(old_sum_path)
            .report()
            .change_context(BlockChainTreeError::BlockChainTreeError(
                BCTreeErrorKind::MoveSummaryDBError,
            ))
            .attach_printable("failed to open old summary db")?;

        self.old_summary_db = Some(result);

        return Ok(());
    }

    pub fn new_transaction(&mut self, tr: Transaction) -> Result<(), BlockChainTreeError> {
        // if it is in first bunch of transactions
        // to be added to blockchain.
        // AND if it is not a last block
        // that is pending.
        if self.trxs_pool.len() < MAX_TRANSACTIONS_PER_BLOCK
            && self.main_chain.get_height() as usize + 1 % BLOCKS_PER_ITERATION != 0
        {
            self.decrease_funds(tr.get_sender(), tr.get_amount())
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::NewTransactionError,
                ))?;

            self.add_funds(tr.get_sender(), tr.get_amount())
                .change_context(BlockChainTreeError::BlockChainTreeError(
                    BCTreeErrorKind::NewTransactionError,
                ))?;
        }

        self.trxs_pool.push_front(Box::new(tr));
        return Ok(());
    }

    pub fn pop_last_transactions(&mut self) -> Option<Vec<Box<dyn Transactionable>>> {
        if self.trxs_pool.is_empty() {
            return None;
        }

        let mut transactions_amount = MAX_TRANSACTIONS_PER_BLOCK;
        if transactions_amount > self.trxs_pool.len() {
            transactions_amount = self.trxs_pool.len();
        }
        let mut to_return: Vec<Box<dyn Transactionable>> = Vec::with_capacity(transactions_amount);

        let mut counter = 0;

        while counter < transactions_amount {
            let result = self.trxs_pool.pop_back();
            if result.is_none() {
                break;
            }
            let tr = result.unwrap();

            to_return.push(tr);
            counter += 1;
        }
        return Some(to_return);
    }

    pub fn get_pool(&mut self) -> &VecDeque<Box<dyn Transactionable>> {
        return &self.trxs_pool;
    }
}
