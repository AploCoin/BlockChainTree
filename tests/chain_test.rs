use blockchaintree::chain::{self, BEGINNING_DIFFICULTY, INCEPTION_TIMESTAMP, ROOT_PUBLIC_ADDRESS};
use primitive_types::U256;

#[tokio::test]
async fn init_flush_chain_test() {
    let main_chain = chain::MainChain::new().await.unwrap();

    main_chain.flush().await.unwrap();

    drop(main_chain);

    chain::MainChain::new().await.unwrap();
}

#[tokio::test]
async fn init_flush_get_block_by_height_chain_test() {
    let main_chain = chain::MainChain::new().await.unwrap();

    main_chain.flush().await.unwrap();

    drop(main_chain);

    let main_chain = chain::MainChain::new().await.unwrap();

    let block = main_chain.find_by_height(&U256::zero()).await.unwrap();

    assert!(block.is_some());

    let block = block.unwrap();

    assert_eq!(ROOT_PUBLIC_ADDRESS, *block.get_founder());
    assert_eq!(INCEPTION_TIMESTAMP, block.get_info().timestamp);
    assert_eq!(U256::zero(), block.get_info().pow);
    assert_eq!(U256::zero(), block.get_info().height);
    assert_eq!(BEGINNING_DIFFICULTY, block.get_info().difficulty);
    assert_eq!(U256::zero(), block.get_fee());
    assert_eq!(
        [
            57, 26, 43, 126, 188, 137, 234, 205, 234, 97, 128, 221, 242, 186, 198, 206, 3, 25, 250,
            35, 169, 60, 208, 8, 94, 13, 60, 218, 72, 73, 207, 80
        ],
        block.get_merkle_root()
    );
}
