#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_snake_case)]
mod Transaction;
mod Token;
mod Tools;
use num_bigint::BigUint;
mod merkletree;
mod Block;

use rsa::PublicKey;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use num_bigint::{ToBigUint};
mod BlockChainTree;
mod DumpHeaders;

mod tests;




static PREVIOUS_HASH:[u8;32] = [1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1];

fn main() {
    let sender = b"123456789012345678901234567890123";
    let reciever = b"123456789012345678901234567890123";
    let signature = b"1234567890123456789012345678901234567890123456789012345678901234";
    
    
    
    BlockChainTree::BlockChainTree::check_main_folders().unwrap();

    let mut blockchaintree = BlockChainTree::BlockChainTree::without_config().unwrap();

    let mut main_chain = blockchaintree.get_main_chain();

    
}
