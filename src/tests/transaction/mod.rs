use crate::Transaction;
use num_bigint::{ToBigUint, BigUint};
use num_traits::FromPrimitive;


static sender:&[u8;33] = b"123456789012345678901234567890123";
static reciever:&[u8;33] = b"123456789012345678901234567890123";
static signature:&[u8;64] = b"1234567890123456789012345678901234567890123456789012345678901234";


#[test]
fn dump_load_transaction(){
    let transaction = Transaction::Transaction::new(sender,
                                                reciever,
                                                3388,
                                                signature,
                                                2222222288u64.to_biguint().unwrap());

    let dump = transaction.dump().unwrap();

    let tr_parsed = Transaction::Transaction::parse_transaction(&dump[1..], (transaction.get_dump_size()-1)as u64).unwrap();
    
    assert_eq!(tr_parsed.get_sender(),sender);
    assert_eq!(tr_parsed.get_receiver(),reciever);
    assert_eq!(tr_parsed.get_amount().clone(),2222222288u64.to_biguint().unwrap());
}