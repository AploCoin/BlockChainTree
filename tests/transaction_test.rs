use blockchaintree::transaction::{self, Transactionable};
use primitive_types::U256;
use secp256k1::Secp256k1;

#[test]
fn dump_parse_transaction() {
    let transaction = transaction::Transaction::new_signed(
        [10; 33],
        [20; 33],
        100,
        U256::from_dec_str("3627836287").unwrap(),
        U256::from_dec_str("3627836287").unwrap(),
        None,
        [33; 64],
    );

    let dump = transaction.dump().unwrap();

    let parsed_transaction = transaction::Transaction::parse(&dump[1..]).unwrap();

    assert_eq!(transaction.get_amount(), parsed_transaction.get_amount());
    assert_eq!(transaction.get_data(), parsed_transaction.get_data());
    assert_eq!(
        transaction.get_receiver(),
        parsed_transaction.get_receiver()
    );
    assert_eq!(transaction.get_sender(), parsed_transaction.get_sender());
    assert_eq!(
        transaction.get_signature(),
        parsed_transaction.get_signature()
    );
    assert_eq!(
        transaction.get_timestamp(),
        parsed_transaction.get_timestamp()
    );

    println!("{:?}", parsed_transaction);
}

#[test]
fn hash_transaction() {
    let transaction = transaction::Transaction::new_signed(
        [10; 33],
        [20; 33],
        100,
        U256::from_dec_str("3627836287").unwrap(),
        U256::from_dec_str("3627836287").unwrap(),
        None,
        [33; 64],
    );

    let hash = transaction.hash();

    println!("{:?}", hash);
}

#[test]
fn sign_verify_transaction() {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut rand::thread_rng());

    let transaction = transaction::Transaction::new(
        public_key.serialize(),
        public_key.serialize(),
        100,
        U256::from_dec_str("3627836287").unwrap(),
        U256::from_dec_str("3627836287").unwrap(),
        secret_key.secret_bytes(),
        Some(vec![1, 3, 3, 3, 3, 3, 3]),
    );

    assert!(transaction.verify().unwrap());
}
