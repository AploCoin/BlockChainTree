use sled::Db;

pub struct MainChain {
    blocks: Db,
    height_reference: Db,
}
