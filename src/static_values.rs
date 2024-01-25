use lazy_static::lazy_static;
use primitive_types::U256;

pub static BLOCKCHAIN_DIRECTORY: &str = "./BlockChainTree/";

pub static AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARY/";
pub static OLD_AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARYOLD/";

pub static MAIN_CHAIN_DIRECTORY: &str = "./BlockChainTree/MAIN/";

pub static DERIVATIVE_CHAINS_DIRECTORY: &str = "./BlockChainTree/DERIVATIVES/";
pub static CHAINS_FOLDER: &str = "CHAINS/";
//static DERIVATIVE_DB_DIRECTORY: BlockChainTreeError = "./BlockChainTree/DERIVATIVE/DB/";

pub static BLOCKS_FOLDER: &str = "BLOCKS/";
pub static REFERENCES_FOLDER: &str = "REF/";
pub static TRANSACTIONS_FOLDER: &str = "TRANSACTIONS/";

pub static CONFIG_FILE: &str = "Chain.config";
pub static LOOKUP_TABLE_FILE: &str = "LookUpTable.dat";
pub static TRANSACTIONS_POOL: &str = "TRXS_POOL.pool";

pub static BEGINNING_DIFFICULTY: [u8; 32] = [
    0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

pub static ROOT_PUBLIC_ADDRESS: [u8; 33] = [0; 33];

pub static INCEPTION_TIMESTAMP: u64 = 1597924800;

pub static BLOCKS_PER_ITERATION: usize = 12960;

pub static TIME_PER_BLOCK: u64 = 600;

lazy_static! {
    pub static ref COIN_FRACTIONS: U256 = U256::from_dec_str("1000000000000000000").unwrap();
    pub static ref INITIAL_FEE: U256 = U256::from_dec_str("25000000000000000").unwrap(); // 100_000_000//4
    pub static ref FEE_STEP: U256 = U256::from_dec_str("625000000000").unwrap(); // 100_000_000//255
    pub static ref MAIN_CHAIN_PAYMENT: U256 = *INITIAL_FEE;
    pub static ref COINS_PER_CYCLE: U256 = (*MAIN_CHAIN_PAYMENT*2000usize*BLOCKS_PER_ITERATION) + *COIN_FRACTIONS*10000usize;
}
