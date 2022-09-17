#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_snake_case)]
mod Token;
mod Tools;
mod Transaction;
use num_bigint::BigUint;
mod Block;
mod merkletree;

use num_bigint::ToBigUint;
use rsa::PublicKey;
use sha2::{Digest, Sha256};
use std::convert::TryInto;
mod BlockChainTree;
mod DumpHeaders;
mod Errors;
mod tests;

use crate::Block::{BasicInfo, TokenBlock};
use num_traits::FromPrimitive;

static sender: &[u8; 33] = b"123456789012345678901234567890123";
static reciever: &[u8; 33] = b"123456789012345678901234567890123";
static signature: &[u8; 64] = b"1234567890123456789012345678901234567890123456789012345678901234";

fn main() {
    let default_info = BasicInfo::new(
        500,
        1000u64.to_biguint().unwrap(),
        [0u8; 32],
        [1u8; 32],
        0,
        [5u8; 32],
    );
    let transaction = Transaction::Transaction::new(
        sender,
        reciever,
        228,
        signature,
        1000u64.to_biguint().unwrap(),
    );
    let block = TokenBlock::new(default_info, String::new(), transaction);

    let dump = block.dump().unwrap();

    let block_parsed = TokenBlock::parse(&dump[1..], (dump.len() - 1) as u32).unwrap();
}
