use std::convert::TryInto;

use crate::Transaction;
use num_bigint::{BigUint, ToBigUint};
use num_traits::FromPrimitive;
use secp256k1::ecdsa::Signature;
use secp256k1::rand::thread_rng;
use secp256k1::PublicKey;
use secp256k1::{Message, Secp256k1, SecretKey};

static sender: &[u8; 33] = b"123456789012345678901234567890123";
static reciever: &[u8; 33] = b"123456789012345678901234567890123";
static signature: &[u8; 64] = b"1234567890123456789012345678901234567890123456789012345678901234";
static prev_hash: &[u8; 32] = b"12345678901234567890123456789012";

#[test]
fn dump_load_transaction() {
    let transaction = Transaction::Transaction::new(
        sender,
        reciever,
        3388,
        signature,
        2222222288u64.to_biguint().unwrap(),
    );

    let dump = transaction.dump().unwrap();

    let tr_parsed = Transaction::Transaction::parse_transaction(
        &dump[1..],
        (transaction.get_dump_size() - 1) as u64,
    )
    .unwrap();

    assert_eq!(tr_parsed.get_sender(), sender);
    assert_eq!(tr_parsed.get_receiver(), reciever);
    assert_eq!(
        tr_parsed.get_amount().clone(),
        2222222288u64.to_biguint().unwrap()
    );
}

#[test]
fn sign_verify_transaction() {
    let secp = Secp256k1::new();

    let (sender_secret_key, sender_public_key) = secp.generate_keypair(&mut thread_rng());
    let (reciever_secret_key, reciever_public_key) = secp.generate_keypair(&mut thread_rng());

    let sign = Transaction::Transaction::sign(
        &sender_public_key.serialize(),
        &reciever_public_key.serialize(),
        3388,
        2222222288u64.to_biguint().unwrap(),
        prev_hash,
        &sender_secret_key.secret_bytes().try_into().unwrap(),
    );

    let transaction = Transaction::Transaction::new(
        &sender_public_key.serialize(),
        &reciever_public_key.serialize(),
        3388,
        &sign,
        2222222288u64.to_biguint().unwrap(),
    );

    let result = transaction.verify(prev_hash).unwrap();

    assert!(result);
}
