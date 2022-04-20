use num_bigint::{ToBigUint, BigUint};
use num_traits::FromPrimitive;

use crate::{BlockChainTree::*, Block::{self, TransactionBlock, TransactionToken, BasicInfo}, Transaction};

#[test]
fn create_chain() {
    check_main_folders();

    let chain = Chain::new("test_chain");
    chain.unwrap();
    //assert!(chain.is_ok()); 
}

#[test]
fn add_block() {
    let chain = Chain::new("test_chain");
    assert!(chain.is_ok());
    let mut chain = chain.unwrap();

    let txs = vec![
        TransactionToken::new(),
        TransactionToken::new(),
        TransactionToken::new(),                
    ];
    let fee = 1337u32.to_biguint().unwrap();
    let basic_info = BasicInfo::new(
        1650454369, BigUint::from_u64(4578475463736).unwrap(),
        *b"23wdwebr467fdshvft37ibwvefnauuj9", 46352344875u64, *b"23wdwebr467fdshvft3773428f9853j9");
    let block = TransactionBlock::new(txs, fee, basic_info, *b"23wdwebr467fdshvft37ibwvefnauuj9");

    let block = Block::SumTransactionBlock::new(Some(block), None);
    assert!(chain.add_block(&block).is_ok());

    assert_eq!(46352344876u64, chain.get_height());
    assert_eq!(chain.get_difficulty(), *b"23wdwebr467fdshvft3773428f9853j9");
}