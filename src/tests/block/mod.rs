use num_bigint::{ToBigUint, BigUint};
use num_traits::FromPrimitive;

use crate::{BlockChainTree::*, Block::{self, TransactionBlock, TransactionToken, BasicInfo, TokenBlock}, Transaction};
use crate::merkletree;

static sender:&[u8;33] = b"123456789012345678901234567890123";
static reciever:&[u8;33] = b"123456789012345678901234567890123";
static signature:&[u8;64] = b"1234567890123456789012345678901234567890123456789012345678901234";
static prev_hash:&[u8;32] = b"12345678901234567890123456789012";

#[test]
fn dump_parse_tokenblock(){
    let default_info = BasicInfo::new(500,
                            1000u64.to_biguint().unwrap(),
                            [0u8;32],
                            [1u8;32],
                            0,
                            [5u8;32]);
    let transaction = Transaction::Transaction::new(sender,
                                            reciever,
                                            228,
                                            signature,
                                            1000u64.to_biguint().unwrap());
    let block = TokenBlock::new(default_info,
                            String::new(),
                            transaction);

    let dump = block.dump().unwrap();

    let block_parsed = TokenBlock::parse(&dump[1..],(dump.len()-1) as u32).unwrap();
}

#[test]
fn dump_parse_transactionblock(){
    let default_info = BasicInfo::new(500,
                                    1000u64.to_biguint().unwrap(),
                                    [0u8;32],
                                    [1u8;32],
                                    0,
                                    [5u8;32]);
                                        

    let mut hashes:Vec<&[u8;32]> = Vec::with_capacity(5);
    
    let transaction = Transaction::Transaction::new(sender,
        reciever,
        228,
        signature,
        1000u64.to_biguint().unwrap());

    let hash = transaction.hash(prev_hash);

    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);

    let mut mk_tree = merkletree::MerkleTree::new();

    mk_tree.add_objects(hashes);

    let mut transactions:Vec<TransactionToken> = Vec::new();

    for i in 0..5{
        let transaction = Transaction::Transaction::new(sender,
            reciever,
            228,
            signature,
            1000u64.to_biguint().unwrap());
        let tr_tk = TransactionToken::new(Some(transaction),None);
        transactions.push(tr_tk);
    }

    let block = TransactionBlock::new(transactions,
                        1000u64.to_biguint().unwrap(),
                        default_info,
                        *mk_tree.get_root());

    let dump = block.dump().unwrap();

    let block = TransactionBlock::parse(&dump[1..], (dump.len()-1) as u32).unwrap();

}

#[test]
fn check_merkle_tree(){
    let default_info = BasicInfo::new(500,
        1000u64.to_biguint().unwrap(),
        prev_hash.clone(),
        [1u8;32],
        0,
        [5u8;32]);
            

    let mut hashes:Vec<&[u8;32]> = Vec::with_capacity(5);

    let transaction = Transaction::Transaction::new(sender,
    reciever,
    228,
    signature,
    1000u64.to_biguint().unwrap());

    let hash = transaction.hash(prev_hash);

    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);

    let mut mk_tree = merkletree::MerkleTree::new();

    mk_tree.add_objects(hashes);

    let mut transactions:Vec<TransactionToken> = Vec::new();

    for i in 0..5{
    let transaction = Transaction::Transaction::new(sender,
                            reciever,
                            228,
                            signature,
                            1000u64.to_biguint().unwrap());
    let tr_tk = TransactionToken::new(Some(transaction),None);
    transactions.push(tr_tk);
    }

    let mut block = TransactionBlock::new(transactions,
                                1000u64.to_biguint().unwrap(),
                                default_info,
                                *mk_tree.get_root());

    let res = block.check_merkle_tree().unwrap();
    assert!(res);
    
}