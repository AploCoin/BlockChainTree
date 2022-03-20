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




static PREVIOUS_HASH:[u8;32] = [1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1,
                            1,1,1,1,1,1,1,1];

fn main() {
    let sender = b"123456789012345678901234567890123";
    let reciever = b"123456789012345678901234567890123";
    let signature = b"1234567890123456789012345678901234567890123456789012345678901234";
    
    
    
    let transaction = Transaction::Transaction::new(sender,
                                                reciever,
                                                228,
                                                signature,
                                                1337u32.to_biguint().unwrap());

    println!("{:?}",transaction);

    let transaction_dump = transaction.dump().unwrap();

    println!("{}",transaction_dump.len());

    let parsed_transaction = Transaction::Transaction::parse_transaction(&transaction_dump[1..], transaction_dump.len() as u64-1).unwrap();

    println!("{:?}",parsed_transaction);


}
