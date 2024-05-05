use blockchaintree::block::Block as _;
use blockchaintree::tools;
use blockchaintree::{blockchaintree::BlockChainTree, static_values};
use primitive_types::U256;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut tree = BlockChainTree::new().unwrap();

    let wallet: [u8; 33] = [
        2, 178, 140, 81, 31, 206, 208, 171, 143, 240, 128, 134, 115, 82, 188, 63, 146, 189, 14, 59,
        85, 8, 11, 28, 137, 161, 145, 216, 251, 95, 93, 137, 159,
    ];

    let chain = tree.get_derivative_chain(&wallet).unwrap();

    loop {
        println!("Current height: {}", chain.get_height());
        println!(
            "Current miner gas amount: {}",
            tree.get_gas(&wallet).unwrap()
        );
        let mut nonce = U256::zero();
        let (prev_hash, difficulty, _prev_timestamp, _height) =
            if let Some(block) = chain.get_last_block().unwrap() {
                (
                    block.hash().unwrap(),
                    block.get_info().difficulty,
                    block.get_info().timestamp,
                    block.get_info().height,
                )
            } else {
                let block = tree
                    .get_main_chain()
                    .find_by_hash(&chain.genesis_hash)
                    .unwrap()
                    .unwrap();
                (
                    block.hash().unwrap(),
                    static_values::BEGINNING_DIFFICULTY,
                    block.get_info().timestamp,
                    U256::zero(),
                )
            };
        println!(
            "Current difficulty: {}",
            tools::count_leading_zeros(&difficulty)
        );
        while nonce < U256::MAX {
            let mut pow = [0u8; 32];
            nonce.to_big_endian(&mut pow);
            if tools::check_pow(&prev_hash, &difficulty, &pow) {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                println!("Found nonce! {}", nonce);

                let block = rt
                    .block_on(tree.emmit_new_derivative_block(&pow, &wallet, timestamp))
                    .unwrap();

                // Node should handle this
                tree.add_gas(&wallet, *static_values::MAIN_CHAIN_PAYMENT)
                    .unwrap();

                println!("Added new block! {:?}\n", block.hash().unwrap());

                rt.block_on(chain.flush()).unwrap();
                rt.block_on(tree.flush()).unwrap();
                break;
            }
            nonce += U256::one();
        }
    }
}
