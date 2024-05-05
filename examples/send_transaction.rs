use blockchaintree::static_values::BLOCKS_PER_EPOCH;
use blockchaintree::transaction::Transactionable;
use blockchaintree::{blockchaintree::BlockChainTree, static_values};
use blockchaintree::{tools, transaction};
use primitive_types::U256;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut tree = BlockChainTree::new().unwrap();

    let main_chain = tree.get_main_chain();

    let wallet_private: [u8; 32] = [
        25, 53, 50, 224, 180, 250, 177, 186, 87, 47, 28, 80, 183, 208, 219, 119, 101, 60, 173, 157,
        190, 29, 208, 231, 98, 69, 82, 211, 107, 185, 192, 224,
    ];
    let wallet = [
        2, 178, 140, 81, 31, 206, 208, 171, 143, 240, 128, 134, 115, 82, 188, 63, 146, 189, 14, 59,
        85, 8, 11, 28, 137, 161, 145, 216, 251, 95, 93, 137, 159,
    ];
    let receiver = static_values::ROOT_PUBLIC_ADDRESS;

    println!("Sender amount: {}", tree.get_amount(&wallet).unwrap());
    println!("Sender gas amount: {}", tree.get_gas(&wallet).unwrap());
    println!("Receiver amount: {}", tree.get_amount(&receiver).unwrap());

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let transaction = transaction::Transaction::new(
        wallet.clone(),
        receiver,
        timestamp,
        U256::from_str_radix("228", 10).unwrap(),
        wallet_private,
        None,
    );
    let transaction_hash = transaction.hash();
    tree.send_transaction(&transaction).unwrap();

    println!("Transaction created: {:?}", &transaction_hash);
    println!("Sender amount: {}", tree.get_amount(&wallet).unwrap());
    println!("Sender gas amount: {}", tree.get_gas(&wallet).unwrap());
    println!("Receiver amount: {}", tree.get_amount(&receiver).unwrap());

    // MINING
    let mut nonce = U256::zero();
    let last_block = main_chain.get_last_block().unwrap().unwrap();
    let prev_hash = last_block.hash().unwrap();
    let difficulty = last_block.get_info().difficulty;
    while nonce < U256::MAX {
        let mut pow = [0u8; 32];
        nonce.to_big_endian(&mut pow);
        if tools::check_pow(&prev_hash, &difficulty, &pow) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let transactions: Vec<[u8; 32]> =
                if ((last_block.get_info().height + 1) % BLOCKS_PER_EPOCH).is_zero() {
                    Vec::with_capacity(0)
                } else {
                    vec![transaction_hash]
                };

            let block = rt
                .block_on(tree.emmit_new_main_block(&pow, &wallet, &transactions, timestamp))
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
