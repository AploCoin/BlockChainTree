use blockchaintree::block;
use primitive_types::U256;

#[test]
fn dump_parse_basic_info() {
    let basic_data = block::BasicInfo {
        timestamp: 160000,
        pow: U256::from_dec_str("10000000000000000000001000000001").unwrap(),
        previous_hash: [5; 32],
        height: U256::from_dec_str("6378216378216387213672813821736").unwrap(),
        difficulty: [101; 32],
        founder: [6; 33],
    };

    let mut buffer: Vec<u8> = Vec::new();

    basic_data.dump(&mut buffer).unwrap();

    let basic_data_loaded = block::BasicInfo::parse(&buffer).unwrap();

    assert_eq!(basic_data.timestamp, basic_data_loaded.timestamp);
    assert_eq!(basic_data.pow, basic_data_loaded.pow);
    assert_eq!(basic_data.previous_hash, basic_data_loaded.previous_hash);
    assert_eq!(basic_data.height, basic_data_loaded.height);
    assert_eq!(basic_data.difficulty, basic_data_loaded.difficulty);
    assert_eq!(basic_data.founder, basic_data_loaded.founder);

    println!("{:?}", basic_data_loaded)
}

#[test]
fn dump_parse_block() {}
