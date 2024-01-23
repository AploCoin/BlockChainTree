use std::{fs::File, io::Read, path::Path, sync::Arc};

use error_stack::{Report, ResultExt};
use primitive_types::U256;
use sled::Db;
use tokio::sync::RwLock;

use crate::errors::{BlockChainTreeError, ChainErrorKind};

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

pub struct MainChain {
    blocks: Db,
    height_reference: Db,
    transactions: Db,
    height: Arc<RwLock<U256>>,
    difficulty: Arc<RwLock<[u8; 32]>>,
}

impl MainChain {
    pub fn new() -> Result<Self, Report<BlockChainTreeError>> {
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
            (U256::one(), BEGINNING_DIFFICULTY)
        };

        Ok(Self {
            blocks,
            height_reference,
            transactions,
            height: Arc::new(RwLock::new(height)),
            difficulty: Arc::new(RwLock::new(difficulty)),
        })
    }
}
