use blockchaintree::static_values::BLOCKS_PER_EPOCH;
use blockchaintree::tools;
use blockchaintree::{blockchaintree::BlockChainTree, static_values};
use primitive_types::U256;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut tree = BlockChainTree::new("./BlockChainTree").unwrap();

    let main_chain = tree.get_main_chain();

    let wallet: [u8; 33] = [
        2, 178, 140, 81, 31, 206, 208, 171, 143, 240, 128, 134, 115, 82, 188, 63, 146, 189, 14, 59,
        85, 8, 11, 28, 137, 161, 145, 216, 251, 95, 93, 137, 159,
    ];

    loop {
        println!("Current height: {}", main_chain.get_height());
        println!(
            "Current miner balance: {}",
            tree.get_amount(&wallet).unwrap()
        );
        println!(
            "Current root balance: {}",
            tree.get_amount(&static_values::ROOT_PUBLIC_ADDRESS)
                .unwrap()
        );
        let mut nonce = U256::zero();
        let last_block = main_chain.get_last_block().unwrap().unwrap();
        let prev_hash = last_block.hash().unwrap();
        let difficulty = last_block.get_info().difficulty;
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

                let transactions: &[[u8; 32]] =
                    if ((last_block.get_info().height + 1) % BLOCKS_PER_EPOCH).is_zero() {
                        println!("Cycle ended!");
                        &[]
                    } else {
                        &[[25u8; 32]]
                    };

                let block = rt
                    .block_on(tree.emmit_new_main_block(&pow, &wallet, transactions, timestamp))
                    .unwrap();

                // Node should handle this
                tree.send_amount(
                    &static_values::ROOT_PUBLIC_ADDRESS,
                    &wallet,
                    *static_values::MAIN_CHAIN_PAYMENT,
                )
                .unwrap();

                let fee = tools::recalculate_fee(&last_block.get_info().difficulty);
                for _ in transactions {
                    tree.add_amount(&wallet, fee).unwrap();
                }

                println!("Added new block! {:?}\n", block.hash().unwrap());

                rt.block_on(tree.flush()).unwrap();
                break;
            }
            nonce += U256::one();
        }
    }
}
