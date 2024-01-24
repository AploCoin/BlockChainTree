use blockchaintree::chain;
use primitive_types::U256;

#[tokio::test]
async fn init_flush_chain_test() {
    let main_chain = chain::MainChain::new().await.unwrap();

    main_chain.flush().await.unwrap();

    drop(main_chain);

    let main_chain = chain::MainChain::new().await.unwrap();
}

#[tokio::test]
async fn init_flush_get_block_by_height_chain_test() {
    let main_chain = chain::MainChain::new().await.unwrap();

    main_chain.flush().await.unwrap();

    drop(main_chain);

    let main_chain = chain::MainChain::new().await.unwrap();

    let block = main_chain.find_raw_by_height(&U256::zero()).await.unwrap();

    assert!(block.is_some());
}
