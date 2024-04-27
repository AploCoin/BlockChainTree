use std::{collections::HashMap, path::Path};

use crate::{
    chain,
    errors::{BCTreeErrorKind, BlockChainTreeError},
    tools,
};
use error_stack::{Report, ResultExt};
use primitive_types::U256;
use sled::Db;

static BLOCKCHAIN_DIRECTORY: &str = "./BlockChainTree/";

static AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARY/";
static OLD_AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARYOLD/";

static GAS_SUMMARY: &str = "./BlockChainTree/GASSUMMARY/";
static OLD_GAS_SUMMARY: &str = "./BlockChainTree/GASSUMMARYOLD/";

static MAIN_CHAIN_DIRECTORY: &str = "./BlockChainTree/MAIN/";

static DERIVATIVE_CHAINS_DIRECTORY: &str = "./BlockChainTree/DERIVATIVES/";
static CHAINS_FOLDER: &str = "CHAINS/";

static BLOCKS_FOLDER: &str = "BLOCKS/";
static REFERENCES_FOLDER: &str = "REF/";
static TRANSACTIONS_FOLDER: &str = "TRANSACTIONS/";

static CONFIG_FILE: &str = "Chain.config";
static LOOKUP_TABLE_FILE: &str = "LookUpTable.dat";
static TRANSACTIONS_POOL: &str = "TRXS_POOL.pool";

pub static BEGINNING_DIFFICULTY: [u8; 32] = [
    0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
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

pub struct BlockChainTree {
    main_chain: chain::MainChain,
    derivative_chains: HashMap<[u8; 32], chain::DerivativeChain>,
    summary_db: Db,
    old_summary_db: Db,
    gas_db: Db,
    old_gas_db: Db,
}

impl BlockChainTree {
    pub async fn new() -> Result<Self, Report<BlockChainTreeError>> {
        let path_summary = Path::new(AMMOUNT_SUMMARY);
        let path_summary_old = Path::new(OLD_AMMOUNT_SUMMARY);
        let path_gas = Path::new(GAS_SUMMARY);
        let path_gas_old = Path::new(OLD_GAS_SUMMARY);

        // open summary DB
        let summary_db = sled::open(path_summary)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open summary db")?;

        // open old summary DB
        let old_summary_db = sled::open(path_summary_old)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open old summary db")?;

        // open gas DB
        let gas_db = sled::open(path_gas)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open gas db")?;

        let old_gas_db = sled::open(path_gas_old)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open old gas db")?;

        Ok(Self {
            main_chain: chain::MainChain::new().await?,
            derivative_chains: HashMap::new(),
            summary_db,
            old_summary_db,
            gas_db,
            old_gas_db,
        })
    }

    pub async fn add_amount(
        &self,
        owner: &[u8; 32],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.summary_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let prev_amount = match db.get(owner)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    let new_amount = prev_amount + amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }

    pub async fn sub_amount(
        &self,
        owner: &[u8; 32],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.summary_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let prev_amount = match db.get(owner)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    if prev_amount < amount {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(()));
                    }
                    let new_amount = prev_amount + amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }
    pub async fn get_amount(&self, owner: &[u8; 32]) -> Result<U256, Report<BlockChainTreeError>> {
        match self
            .summary_db
            .get(owner)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::GetFunds,
            ))
            .attach_printable("failed to read config")?
        {
            Some(v) => Ok(tools::load_u256(&v).unwrap().0),
            None => Ok(U256::zero()),
        }
    }

    pub async fn send_amount(
        &self,
        from: &[u8; 32],
        to: &[u8; 32],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.summary_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let mut from_amount = match db.get(from)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    let mut to_amount = match db.get(to)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    if from_amount < amount {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(()));
                    }

                    from_amount -= amount;
                    to_amount += amount;

                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&from_amount));
                    tools::dump_u256(&from_amount, &mut buf).unwrap();
                    db.insert(from, buf)?;

                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&to_amount));
                    tools::dump_u256(&from_amount, &mut buf).unwrap();
                    db.insert(to, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }
}
