use num_bigint::{ToBigUint, BigUint};
use num_traits::FromPrimitive;

use crate::{BlockChainTree::*, Block::{self, TransactionBlock, TransactionToken, BasicInfo, TokenBlock}, Transaction};

static sender:&[u8;33] = b"123456789012345678901234567890123";
static reciever:&[u8;33] = b"123456789012345678901234567890123";
static signature:&[u8;64] = b"1234567890123456789012345678901234567890123456789012345678901234";

#[test]
fn create_chain() {
    BlockChainTree::check_main_folders().unwrap();

    let mut blockchaintree = BlockChainTree::without_config().unwrap();

    let main_chain = blockchaintree.get_main_chain();
}



#[test]
fn dump_main_chain_config(){
    BlockChainTree::check_main_folders().unwrap();
    let mut blockchaintree = BlockChainTree::without_config().unwrap();
    blockchaintree.dump_pool();

    blockchaintree.get_main_chain().dump_config().unwrap();

    drop(blockchaintree);

    let mut blockchaintree = BlockChainTree::with_config().unwrap();
}



#[test]
fn add_funds(){
    BlockChainTree::check_main_folders().unwrap();
    let mut blockchaintree = BlockChainTree::without_config().unwrap();

    blockchaintree.add_funds(sender,&(1000u64.to_biguint().unwrap())).unwrap();

    let funds = blockchaintree.get_funds(sender).unwrap();
    assert_eq!(funds,1000u64.to_biguint().unwrap());
}



#[test]
fn decrease_funds(){
    BlockChainTree::check_main_folders().unwrap();
    let mut blockchaintree = BlockChainTree::without_config().unwrap();

    let current_funds = blockchaintree.get_funds(sender).unwrap(); 

    let result = blockchaintree.decrease_funds(sender,&(5u64.to_biguint().unwrap()));

    if result.is_ok() && current_funds < 5u64.to_biguint().unwrap(){
        assert_eq!(false,true);
    }

    let funds = blockchaintree.get_funds(sender).unwrap();
    assert_eq!(funds,current_funds-5u64.to_biguint().unwrap());
}



#[test]
fn move_database(){
    BlockChainTree::check_main_folders().unwrap();
    let mut blockchaintree = BlockChainTree::without_config().unwrap();

    let current_funds = blockchaintree.get_funds(sender).unwrap();
    blockchaintree.move_summary_database().unwrap();

    let funds = blockchaintree.get_old_funds(sender).unwrap();

    assert_eq!(current_funds,funds);

    let current_funds = blockchaintree.get_funds(sender).unwrap();
    assert_eq!(current_funds,0u64.to_biguint().unwrap());
}



#[test]
fn create_derivative_chain(){
    BlockChainTree::check_main_folders().unwrap();
    let mut blockchaintree = BlockChainTree::without_config().unwrap();

    let result = blockchaintree.get_derivative_chain(sender).unwrap();
    assert_eq!(result.is_none(),true);

    let result = BlockChainTree::create_derivative_chain(sender,
                                                &[0u8;32],
                                                1).unwrap();
    drop(result);

    let result = blockchaintree.get_derivative_chain(sender).unwrap();
    assert_eq!(result.is_none(),false);
}



#[test]
fn derivative_add_block(){
    BlockChainTree::check_main_folders().unwrap();
    let mut blockchaintree = BlockChainTree::without_config().unwrap();

    let result = blockchaintree.get_derivative_chain(sender).unwrap();
    assert_eq!(result.is_some(),true);

    let mut derivative_chain = result.unwrap();

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

    derivative_chain.add_block(&block).unwrap();

    let block_db = derivative_chain.find_by_height(0).unwrap().unwrap();
    assert_eq!(block_db.payment_transaction.get_sender(),sender);


}



#[test]
fn dump_empty_pool(){
    BlockChainTree::check_main_folders().unwrap();
    let mut blockchaintree = BlockChainTree::without_config().unwrap();
    blockchaintree.dump_pool().unwrap();

    blockchaintree.get_main_chain().dump_config();

    drop(blockchaintree);

    let mut blockchaintree = BlockChainTree::with_config().unwrap();
    assert_eq!(blockchaintree.get_pool().len(),0);

    // for i in 0..10{
    //     let mut tr = Transaction::Transaction::new(sender,reciever,228,signature,228u64.to_biguint().unwrap());
    //     let mut tr_tk = TransactionToken::new();
    //     tr_tk.set_transaction(tr);
    //     blockchaintree.new_transaction(tr_tk).unwrap();
    // }

    // blockchaintree.dump_pool().unwrap();
    // blockchaintree.get_main_chain().dump_config();

    // drop(blockchaintree);

    // let mut blockchaintree = BlockChainTree::with_config().unwrap();
    // assert_eq!(blockchaintree.get_pool().len(),10);

    // for tr_tk in blockchaintree.get_pool().iter(){
    //     assert_eq!(tr_tk.get_transaction().as_ref().unwrap().get_sender(),sender);
    //     assert_eq!(tr_tk.get_transaction().as_ref().unwrap().get_receiver(),reciever);
    // }
}

