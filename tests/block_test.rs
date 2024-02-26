use std::sync::Arc;

use blockchaintree::block::{self, Block};
use primitive_types::U256;

#[test]
fn dump_parse_basic_info() {
    let basic_data = block::BasicInfo {
        timestamp: 160000,
        pow: U256::from_dec_str("10000000000000000000001000000001").unwrap(),
        previous_hash: [5; 32],
        height: U256::from_dec_str("6378216378216387213672813821736").unwrap(),
        difficulty: [101; 32],
        founder: [6; 33],
    };

    let mut buffer: Vec<u8> = Vec::new();

    basic_data.dump(&mut buffer).unwrap();

    let basic_data_loaded = block::BasicInfo::parse(&buffer).unwrap();

    assert_eq!(basic_data.timestamp, basic_data_loaded.timestamp);
    assert_eq!(basic_data.pow, basic_data_loaded.pow);
    assert_eq!(basic_data.previous_hash, basic_data_loaded.previous_hash);
    assert_eq!(basic_data.height, basic_data_loaded.height);
    assert_eq!(basic_data.difficulty, basic_data_loaded.difficulty);
    assert_eq!(basic_data.founder, basic_data_loaded.founder);

    println!("{:?}", basic_data_loaded)
}

#[test]
fn dump_parse_block() {
    let basic_data = block::BasicInfo {
        timestamp: 160000,
        pow: U256::from_dec_str("10000000000000000000001000000001").unwrap(),
        previous_hash: [5; 32],
        height: U256::from_dec_str("6378216378216387213672813821736").unwrap(),
        difficulty: [101; 32],
        founder: [6; 33],
    };
    let block = block::TransactionBlock::new(
        U256::from_dec_str("9089878746387246532").unwrap(),
        basic_data,
        [5; 32],
        vec![[1; 32], [2; 32], [3; 32]],
    );

    let dump = block.dump().unwrap();

    let block_loaded = block::TransactionBlock::parse(&dump[1..]).unwrap();

    assert_eq!(block.merkle_tree_root, block_loaded.merkle_tree_root);
    assert_eq!(block.fee, block_loaded.fee);
    assert_eq!(block.transactions, block_loaded.transactions);

    assert_eq!(
        block.default_info.timestamp,
        block_loaded.default_info.timestamp
    );
    assert_eq!(block.default_info.pow, block_loaded.default_info.pow);
    assert_eq!(
        block.default_info.previous_hash,
        block_loaded.default_info.previous_hash
    );
    assert_eq!(block.default_info.height, block_loaded.default_info.height);
    assert_eq!(
        block.default_info.difficulty,
        block_loaded.default_info.difficulty
    );
    assert_eq!(
        block.default_info.founder,
        block_loaded.default_info.founder
    );

    println!("{:?}", block_loaded);
}

#[test]
fn dump_parse_summarize_block() {
    let basic_data = block::BasicInfo {
        timestamp: 160000,
        pow: U256::from_dec_str("10000000000000000000001000000001").unwrap(),
        previous_hash: [5; 32],
        height: U256::from_dec_str("6378216378216387213672813821736").unwrap(),
        difficulty: [101; 32],
        founder: [6; 33],
    };
    let block = block::SummarizeBlock {
        default_info: basic_data,
        merkle_tree_root: [5; 32],
    };

    let dump = block.dump().unwrap();

    let block_loaded = block::SummarizeBlock::parse(&dump[1..]).unwrap();

    assert_eq!(block.merkle_tree_root, block_loaded.merkle_tree_root);

    assert_eq!(
        block.default_info.timestamp,
        block_loaded.default_info.timestamp
    );
    assert_eq!(block.default_info.pow, block_loaded.default_info.pow);
    assert_eq!(
        block.default_info.previous_hash,
        block_loaded.default_info.previous_hash
    );
    assert_eq!(block.default_info.height, block_loaded.default_info.height);
    assert_eq!(
        block.default_info.difficulty,
        block_loaded.default_info.difficulty
    );
    assert_eq!(
        block.default_info.founder,
        block_loaded.default_info.founder
    );

    println!("{:?}", block_loaded);
}

#[test]
fn validate_block_test() {
    let basic_data = block::BasicInfo {
        timestamp: 160000,
        pow: U256::from_dec_str("10000000000000000000001000000001").unwrap(),
        previous_hash: [5; 32],
        height: U256::from_dec_str("1").unwrap(),
        difficulty: [101; 32],
        founder: [6; 33],
    };
    let prev_block = block::TransactionBlock::new(
        U256::from_dec_str("9089878746387246532").unwrap(),
        basic_data,
        [5; 32],
        vec![[1; 32], [2; 32], [3; 32]],
    );

    let basic_data = block::BasicInfo {
        timestamp: 160000,
        pow: U256::from_dec_str("10000000000000000000001000000001").unwrap(),
        previous_hash: prev_block.hash().unwrap(),
        height: U256::from_dec_str("2").unwrap(),
        difficulty: [101; 32],
        founder: [6; 33],
    };
    let block = block::TransactionBlock::new(
        U256::from_dec_str("9089878746387246532").unwrap(),
        basic_data,
        [5; 32],
        vec![[1; 32], [2; 32], [3; 32]],
    );

    assert!(!block.validate(Some(Arc::new(prev_block))).unwrap());
}
