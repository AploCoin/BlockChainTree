use blockchaintree::block::{self, BasicInfo, TransactionBlock};
use blockchaintree::{self, transaction::Transactionable};
use num_bigint::ToBigUint;

static SENDER: &[u8; 33] = b"123456789012345678901234567890123";
static RECIEVER: &[u8; 33] = b"123456789012345678901234567890123";
//static SIGNATURE: &[u8; 64] = b"1234567890123456789012345678901234567890123456789012345678901234";
static PREV_HASH: &[u8; 32] = b"12345678901234567890123456789012";

#[tokio::test]
async fn chain_test() {
    let blockchain = blockchaintree::blockchaintree::BlockChainTree::without_config().unwrap();

    let default_info = BasicInfo::new(
        500,
        1000u64.to_biguint().unwrap(),
        [0u8; 32],
        [1u8; 32],
        0,
        [5u8; 32],
    );
    let tr = blockchaintree::transaction::Transaction::new(
        SENDER.clone(),
        RECIEVER.clone(),
        121212,
        2222222288u64.to_biguint().unwrap(),
        PREV_HASH.clone(),
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
    chain.add_block(&block).await.unwrap();

    chain.add_transaction(tr.clone()).await.unwrap();

    let loaded_transaction = chain.find_transaction(&tr.hash()).await.unwrap().unwrap();
    assert_eq!(loaded_transaction.get_sender(), SENDER);
}
