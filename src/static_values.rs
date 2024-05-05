use lazy_static::lazy_static;
use primitive_types::U256;

pub static BLOCKCHAIN_DIRECTORY: &str = "./BlockChainTree/";

pub static AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARY/";
pub static OLD_AMMOUNT_SUMMARY: &str = "./BlockChainTree/SUMMARYOLD/";

pub static GAS_SUMMARY: &str = "./BlockChainTree/GASSUMMARY/";
pub static OLD_GAS_SUMMARY: &str = "./BlockChainTree/GASSUMMARYOLD/";

pub static MAIN_CHAIN_DIRECTORY: &str = "./BlockChainTree/MAIN/";

pub static DERIVATIVE_CHAINS_DIRECTORY: &str = "./BlockChainTree/DERIVATIVES/";
pub static CHAINS_FOLDER: &str = "CHAINS/";

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

pub static MAX_DIFFICULTY: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 128,
];

pub static ROOT_PRIVATE_ADDRESS: [u8; 32] = [1u8; 32];
pub static ROOT_PUBLIC_ADDRESS: [u8; 33] = [
    3, 27, 132, 197, 86, 123, 18, 100, 64, 153, 93, 62, 213, 170, 186, 5, 101, 215, 30, 24, 52, 96,
    72, 25, 255, 156, 23, 245, 233, 213, 221, 7, 143,
];

pub static INCEPTION_TIMESTAMP: u64 = 1597924800;

pub static BLOCKS_PER_EPOCH: usize = 1000000;

pub static TIME_PER_BLOCK: u64 = 600;

lazy_static! {
    pub static ref COIN_FRACTIONS: U256 = U256::from_dec_str("1000000000000000000").unwrap();
    pub static ref INITIAL_FEE: U256 = U256::from_dec_str("25000000000000000").unwrap(); // 100_000_000//4
    pub static ref FEE_STEP: U256 = U256::from_dec_str("62500").unwrap(); // 100_000_000//255
    pub static ref MAIN_CHAIN_PAYMENT: U256 = *INITIAL_FEE;
    pub static ref COINS_PER_CYCLE: U256 = (*MAIN_CHAIN_PAYMENT*2000usize*BLOCKS_PER_EPOCH) + *COIN_FRACTIONS*10000usize;
    pub static ref BYTE_GAS_PRICE: U256 = U256::from_dec_str("625000000000").unwrap();
}
