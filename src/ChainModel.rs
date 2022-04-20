trait BaseChain {
    type BlockType;

    fn new(root_path: &str) -> Result<Self, &'static str>;
    fn new_without_config(root_path: &str, genesis_hash: &[u8;32], global_height: Option<u64>) -> Result<Self, &'static str>;

    fn get_last_block(&self) -> Result<Option<BlockType>, &'static str>;
    fn add_block(&mut self, block: &BlockType) -> Result<(), &'static str>;

    fn dump_config(&self, root_path: &str) -> Result<(), &'static str>;

    fn get_height(&self) -> u64;
    fn get_global_height(&self) -> u64;
    fn get_difficulty(&self) -> [u8; 32];

    fn find_by_height(&self, height: u64) -> Result<Option<BlockType>, &'static str>;
    fn find_by_hash(&self, hash: &[u8; 32]) -> Result<Option<BlockType>, &'static str>;

}