use blockchaintree::blockchaintree::BlockChainTree;
use primitive_types::U256;

#[tokio::test]
async fn test_amounts() {
    let tree = BlockChainTree::new("./BlockChainTree").unwrap();

    let address_a = [0; 33];
    let address_b = [1; 33];
    tree.add_amount(&address_a, U256::from_dec_str("10000000000").unwrap())
        .unwrap();
    let amount = tree.get_amount(&address_a).unwrap();
    assert_eq!(amount, U256::from_dec_str("10000000000").unwrap());

    tree.send_amount(&address_a, &address_b, U256::from_dec_str("100").unwrap())
        .unwrap();
    let amount_a = tree.get_amount(&address_a).unwrap();
    let amount_b = tree.get_amount(&address_b).unwrap();
    println!("{:?}", amount_a);
    println!("{:?}", amount_b);
    assert_eq!(
        amount_a,
        U256::from_dec_str("10000000000").unwrap() - U256::from_dec_str("100").unwrap()
    );
    assert_eq!(amount_b, U256::from_dec_str("100").unwrap());
}
