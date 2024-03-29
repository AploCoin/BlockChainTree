use std::str::FromStr;

use blockchaintree::block::{self, BasicInfo, GenesisBlock, MainChainBlock, TransactionBlock};
use blockchaintree::blockchaintree::{BlockChainTree, INCEPTION_TIMESTAMP, ROOT_PUBLIC_ADDRESS};
use blockchaintree::tools::{self, check_pow};
use blockchaintree::{self, blockchaintree::ROOT_PRIVATE_ADDRESS, transaction::Transactionable};
use num_bigint::{BigUint, ToBigUint};
use secp256k1::{PublicKey, Secp256k1, SecretKey};

static SENDER: &[u8; 33] = b"123456789012345678901234567890123";
static RECIEVER: &[u8; 33] = b"123456689012345678901234567890123";
//static SIGNATURE: &[u8; 64] = b"1234567890123456789012345678901234567890123456789012345678901234";
static PREV_HASH: &[u8; 32] = b"12345678901234567890123456789012";

#[tokio::test]
async fn chain_test() {
    let blockchain = blockchaintree::blockchaintree::BlockChainTree::without_config().unwrap();

    let default_info = BasicInfo::new(
        500,
        vec![3, 232],
        [0u8; 32],
        //[1u8; 32],
        0,
        [5u8; 32],
        *SENDER,
    );
    let tr = blockchaintree::transaction::Transaction::new(
        *SENDER,
        *RECIEVER,
        121212,
        2222222288u64.to_biguint().unwrap(),
        *PREV_HASH,
    );

    let block = block::TokenBlock::new(default_info.clone(), String::new(), tr.clone());

    let derivative_chain =
        if let Some(chain) = blockchain.get_derivative_chain(SENDER).await.unwrap() {
            chain
        } else {
            blockchain
                .create_derivative_chain(SENDER, PREV_HASH, 0)
                .await
                .unwrap()
        }
        .clone();

    derivative_chain
        .write()
        .await
        .add_block(&block)
        .await
        .unwrap();

    let block_db = derivative_chain
        .read()
        .await
        .find_by_height(0)
        .unwrap()
        .unwrap();
    assert_eq!(block_db.payment_transaction.get_sender(), SENDER);

    let chain = blockchain.get_main_chain();
    let block = TransactionBlock::new(
        vec![tr.hash()],
        50.to_biguint().unwrap(),
        default_info,
        [0u8; 32],
    );
    chain.add_block_raw(&block).await.unwrap();

    chain.add_transaction_raw(&tr).await.unwrap();

    let loaded_transaction = chain.find_transaction(&tr.hash()).await.unwrap().unwrap();
    assert_eq!(loaded_transaction.get_sender(), SENDER);
}

#[test]
fn generate_public_root_key() {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&ROOT_PRIVATE_ADDRESS).unwrap();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    println!("{:?}", public_key.serialize());
}

#[tokio::test]
async fn mine_main_chain() {
    let blockchain = match BlockChainTree::with_config() {
        Err(e) => {
            println!("Failed to load blockchain with config {:?}", e.to_string());
            //info!("Trying to load blockchain without config");
            BlockChainTree::without_config().unwrap()
        }
        Ok(tree) => tree,
    };

    let chain = blockchain.get_main_chain();

    println!("Difficulty: {:?}", chain.get_difficulty().await);

    let res = blockchain
        .emit_main_chain_block(&[0], *SENDER, INCEPTION_TIMESTAMP + 10)
        .await
        .unwrap();

    assert_eq!(
        chain
            .get_last_block()
            .await
            .unwrap()
            .unwrap()
            .hash()
            .unwrap(),
        res.hash().unwrap()
    );

    assert_ne!(
        blockchain.get_funds(SENDER).await.unwrap(),
        BigUint::from(0u64)
    );

    println!("Difficulty: {:?}", chain.get_difficulty().await);

    println!(
        "Funds for address: {:?} {:?}",
        SENDER,
        blockchain.get_funds(SENDER).await.unwrap()
    );

    println!(
        "Funds for address: {:?} {:?}",
        ROOT_PUBLIC_ADDRESS,
        blockchain.get_funds(&ROOT_PUBLIC_ADDRESS).await.unwrap()
    );

    chain.dump_config().await.unwrap();
    blockchain.dump_pool().await.unwrap();
}

#[test]
fn biguint_test() {
    let num = BigUint::from_str("17239872183291832718372614872678146291748972189471829748921748")
        .unwrap();
    let mut dump: Vec<u8> = Vec::new();
    tools::dump_biguint(&num, &mut dump).unwrap();

    let loaded_num = tools::load_biguint(&dump).unwrap();

    assert_eq!(loaded_num.0, num);

    let num = BigUint::from_str("0").unwrap();
    let mut dump: Vec<u8> = Vec::new();
    tools::dump_biguint(&num, &mut dump).unwrap();

    let loaded_num = tools::load_biguint(&dump).unwrap();

    assert_eq!(loaded_num.0, num);
}

#[test]
fn transaction_block_test() {
    let default_info = BasicInfo::new(500, vec![0], [1u8; 32], 0, [5u8; 32], *SENDER);
    let tr = blockchaintree::transaction::Transaction::new(
        *SENDER,
        *RECIEVER,
        121212,
        2222222288u64.to_biguint().unwrap(),
        *PREV_HASH,
    );
    let block = TransactionBlock::new(
        vec![tr.hash()],
        50.to_biguint().unwrap(),
        default_info,
        [1u8; 32],
    );

    let dump = block.dump().unwrap();

    let loaded_block = TransactionBlock::parse(&dump[1..]).unwrap();

    assert_eq!(block.hash().unwrap(), loaded_block.hash().unwrap());
}

#[test]
fn check_pow_test() {
    check_pow(&[0u8; 32], &[1u8; 32], &[1]);
}

#[test]
fn dump_genesis_block() {
    let genesis_block = GenesisBlock {};

    println!("{:?}", genesis_block.hash());
}
