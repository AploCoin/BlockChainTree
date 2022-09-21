use crate::Transaction;
use num_bigint::{BigUint, ToBigUint};
use num_traits::FromPrimitive;
use secp256k1::ecdsa::Signature;
use secp256k1::rand::thread_rng;
use secp256k1::PublicKey;
use secp256k1::{Message, Secp256k1, SecretKey};
use std::convert::TryInto;

use crate::merkletree;

static sender: &[u8; 33] = b"123456789012345678901234567890123";
static reciever: &[u8; 33] = b"123456789012345678901234567890123";
static signature: &[u8; 64] = b"1234567890123456789012345678901234567890123456789012345678901234";
static prev_hash: &[u8; 32] = b"12345678901234567890123456789012";

#[test]
fn populate_tree() {
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

    let hash = transaction.hash(prev_hash);

    let mut hashes: Vec<&[u8; 32]> = Vec::new();
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);

    let mut mr_tree = merkletree::MerkleTree::new();

    let res = mr_tree.add_objects(hashes);
    assert!(res);
}

#[test]
fn get_proof() {
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

    let hash = transaction.hash(prev_hash);

    let mut hashes: Vec<&[u8; 32]> = Vec::new();
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);

    let mut mr_tree = merkletree::MerkleTree::new();

    let res = mr_tree.add_objects(hashes);

    let proof = mr_tree.get_proof(&hash).unwrap();
}

#[test]
fn verify_proof() {
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

    let hash = transaction.hash(prev_hash);

    let mut hashes: Vec<&[u8; 32]> = Vec::new();
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);
    hashes.push(&hash);

    let mut mr_tree = merkletree::MerkleTree::new();

    let res = mr_tree.add_objects(hashes);

    let proof = mr_tree.get_proof(&hash).unwrap();
    let root = mr_tree.get_root();

    let res = merkletree::verify_proof(&hash, root, proof.clone());
    assert!(res);

    let res = merkletree::verify_proof(&prev_hash, root, proof);
    assert!(!res);
}
