use blockchaintree::{
    block,
    chain::{self, Chain, DerivativeChain},
    static_values::{BEGINNING_DIFFICULTY, INCEPTION_TIMESTAMP, ROOT_PUBLIC_ADDRESS},
};
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

#[tokio::test]
async fn init_flush_deriv_chain_test() {
    let deriv_chain = chain::DerivativeChain::new(
        "deadbeef",
        &[
            57, 26, 43, 126, 188, 137, 234, 205, 234, 97, 128, 221, 242, 186, 198, 206, 3, 25, 250,
            35, 169, 60, 208, 8, 94, 13, 60, 218, 72, 73, 207, 80,
        ],
    )
    .await
    .unwrap();

    deriv_chain.flush().await.unwrap();

    drop(deriv_chain);

    let _deriv_chain = chain::DerivativeChain::new(
        "deadbeef",
        &[
            57, 26, 43, 126, 188, 137, 234, 205, 234, 97, 128, 221, 242, 186, 198, 206, 3, 25, 250,
            35, 169, 60, 208, 8, 94, 13, 60, 218, 72, 73, 207, 80,
        ],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn init_flush_get_block_by_height_deriv_chain_test() {
    let deriv_chain = chain::DerivativeChain::new(
        "deadbeef",
        &[
            57, 26, 43, 126, 188, 137, 234, 205, 234, 97, 128, 221, 242, 186, 198, 206, 3, 25, 250,
            35, 169, 60, 208, 8, 94, 13, 60, 218, 72, 73, 207, 80,
        ],
    )
    .await
    .unwrap();

    deriv_chain.flush().await.unwrap();
    drop(deriv_chain);

    let deriv_chain = chain::DerivativeChain::new(
        "deadbeef",
        &[
            57, 26, 43, 126, 188, 137, 234, 205, 234, 97, 128, 221, 242, 186, 198, 206, 3, 25, 250,
            35, 169, 60, 208, 8, 94, 13, 60, 218, 72, 73, 207, 80,
        ],
    )
    .await
    .unwrap();

    // generate block
    let basic_data = block::BasicInfo {
        timestamp: 160000,
        pow: U256::from_dec_str("10000000000000000000001000000001").unwrap(),
        previous_hash: unsafe { [0; 32].try_into().unwrap_unchecked() },
        height: U256::from_dec_str("0").unwrap(),
        difficulty: [101; 32],
        founder: [6; 33],
    };
    let payment_transaction = [0; 32];
    let derivative_block = block::DerivativeBlock {
        default_info: basic_data,
        payment_transaction,
    };
    deriv_chain.add_block(&derivative_block).await.unwrap();

    let block = deriv_chain.find_by_height(&U256::zero()).await.unwrap();

    assert!(block.is_some());

    let block = block.unwrap();

    assert_eq!(
        derivative_block.default_info.timestamp,
        block.default_info.timestamp
    );
    assert_eq!(derivative_block.default_info.pow, block.default_info.pow);
    assert_eq!(
        derivative_block.default_info.previous_hash,
        block.default_info.previous_hash
    );
    assert_eq!(
        derivative_block.default_info.height,
        block.default_info.height
    );
    assert_eq!(
        derivative_block.default_info.difficulty,
        block.default_info.difficulty
    );
    assert_eq!(
        derivative_block.default_info.founder,
        block.default_info.founder
    );
    assert_eq!(
        derivative_block.payment_transaction,
        block.payment_transaction
    );
}
