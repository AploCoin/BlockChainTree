// use crate::blockchaintree::{
//     BEGINNING_DIFFICULTY, GENESIS_BLOCK, INCEPTION_TIMESTAMP, ROOT_PUBLIC_ADDRESS,
// };
use crate::dump_headers::Headers;
use crate::errors::*;
use crate::tools;
use crate::types::{Address, Hash};
use byteorder::{BigEndian, ReadBytesExt};
use error_stack::{Report, Result, ResultExt};
use primitive_types::U256;
use std::cmp::Ordering;
use std::convert::TryInto;
use std::sync::Arc;

#[macro_export]
macro_rules! bytes_to_u64 {
    ($buffer:expr,$buffer_index:expr) => {
        (&$buffer[$buffer_index..$buffer_index + 8])
            .read_u64::<BigEndian>()
            .unwrap()
    };
}

#[derive(Debug, Clone)]
pub struct BasicInfo {
    pub timestamp: u64,
    pub pow: U256,
    pub previous_hash: Hash,
    pub height: U256,
    pub difficulty: Hash,
    pub founder: Address,
}

impl BasicInfo {
    pub fn new(
        timestamp: u64,
        pow: U256,
        previous_hash: Hash,
        height: U256,
        difficulty: Hash,
        founder: Address,
    ) -> BasicInfo {
        BasicInfo {
            timestamp,
            pow,
            previous_hash,
            height,
            difficulty,
            founder,
        }
    }

    pub fn get_dump_size(&self) -> usize {
        8 + tools::u256_size(&self.pow) + 32 + tools::u256_size(&self.height) + 32 + 33
    }
    pub fn dump(&self, buffer: &mut Vec<u8>) -> Result<(), BlockError> {
        // dumping timestamp
        for byte in self.timestamp.to_be_bytes().iter() {
            buffer.push(*byte);
        }

        // dumping previous hash
        for byte in self.previous_hash.iter() {
            buffer.push(*byte);
        }

        // dumping difficulty
        buffer.extend(self.difficulty);

        // dumping founder
        buffer.extend(self.founder);

        // dumping height
        tools::dump_u256(&self.height, buffer).unwrap();

        // dumping PoW
        tools::dump_u256(&self.pow, buffer).unwrap();

        Ok(())
    }

    pub fn parse(data: &[u8]) -> Result<BasicInfo, BlockError> {
        let mut index: usize = 0;

        if data.len() <= 105 {
            return Err(
                Report::new(BlockError::BasicInfo(BasicInfoErrorKind::Parse))
                    .attach_printable("data <= 105"),
            );
        }

        // parsing timestamp
        let timestamp = bytes_to_u64!(data, index);
        index += 8;

        // parsing previous hash
        let previous_hash: Hash = unsafe { data[index..index + 32].try_into().unwrap_unchecked() };
        index += 32;

        // parsing difficulty
        let difficulty: Hash = unsafe { data[index..index + 32].try_into().unwrap_unchecked() };
        index += 32;

        //parsing founder
        let founder: Address = unsafe { data[index..index + 33].try_into().unwrap_unchecked() };
        index += 33;

        // parsing height
        let (height, height_size) = tools::load_u256(&data[index..])
            .change_context(BlockError::BasicInfo(BasicInfoErrorKind::Parse))?;
        index += height_size + 1;

        // parsing POW
        let (pow, _) = tools::load_u256(&data[index..])
            .change_context(BlockError::BasicInfo(BasicInfoErrorKind::Parse))?;

        Ok(BasicInfo {
            timestamp,
            pow: pow,
            previous_hash,
            height,
            difficulty,
            founder,
        })
    }
}

#[derive(Debug)]
pub struct TransactionBlock {
    pub fee: U256,
    pub merkle_tree_root: Hash,
    pub default_info: BasicInfo,
}

impl TransactionBlock {
    pub fn new(fee: U256, default_info: BasicInfo, merkle_tree_root: Hash) -> TransactionBlock {
        TransactionBlock {
            fee,
            default_info,
            merkle_tree_root,
        }
    }

    pub fn get_dump_size(&self) -> usize {
        1 + tools::u256_size(&self.fee) + 32 + self.default_info.get_dump_size()
    }

    pub fn dump(&self) -> Result<Vec<u8>, BlockError> {
        let size = self.get_dump_size();

        let mut to_return = Vec::<u8>::with_capacity(size);

        // header
        to_return.push(Headers::TransactionBlock as u8);

        // merkle root
        to_return.extend(self.merkle_tree_root.iter());

        // default info
        self.default_info
            .dump(&mut to_return)
            .change_context(BlockError::TransactionBlock(TxBlockErrorKind::Dump))
            .attach_printable("Error dumping default info")?;

        // fee
        tools::dump_u256(&self.fee, &mut to_return)
            .change_context(BlockError::TransactionBlock(TxBlockErrorKind::Dump))
            .attach_printable("Error dumping fee")?;

        Ok(to_return)
    }

    pub fn parse(data: &[u8]) -> Result<Self, BlockError> {
        let mut index: usize = 0;

        let merkle_tree_root: Hash = unsafe { data[0..32].try_into().unwrap_unchecked() };
        index += 32;

        let default_info = BasicInfo::parse(&data[index..])
            .change_context(BlockError::TransactionBlock(TxBlockErrorKind::Parse))
            .attach_printable("Error parsing default data")?;
        index += default_info.get_dump_size();

        let (fee, _) = tools::load_u256(&data[index..])
            .change_context(BlockError::TransactionBlock(TxBlockErrorKind::Parse))
            .attach_printable("Error parsing fee")?;

        Ok(Self {
            fee,
            merkle_tree_root,
            default_info,
        })
    }

    pub fn hash(&self) -> Result<Hash, BlockError> {
        let dump: Vec<u8> = self.dump()?;

        Ok(tools::hash(&dump))
    }
}

pub trait MainChainBlock {
    fn hash(&self) -> Result<Hash, BlockError>;
    fn get_dump_size(&self) -> usize;
    fn dump(&self) -> Result<Vec<u8>, BlockError>;
    fn get_info(&self) -> BasicInfo;
    fn get_merkle_root(&self) -> Hash;
    fn verify_block(&self, prev_hash: &Hash) -> bool;
    fn get_founder(&self) -> &Address;
    fn get_fee(&self) -> U256;
    fn get_type(&self) -> Headers;
}

impl MainChainBlock for TransactionBlock {
    fn hash(&self) -> Result<Hash, BlockError> {
        self.hash()
    }
    fn get_dump_size(&self) -> usize {
        self.get_dump_size()
    }
    fn dump(&self) -> Result<Vec<u8>, BlockError> {
        self.dump()
    }
    fn get_info(&self) -> BasicInfo {
        self.default_info.clone()
    }
    fn get_merkle_root(&self) -> Hash {
        self.merkle_tree_root
    }
    fn verify_block(&self, prev_hash: &Hash) -> bool {
        self.default_info.previous_hash.eq(prev_hash)
    }
    fn get_founder(&self) -> &Address {
        &self.default_info.founder
    }
    fn get_fee(&self) -> U256 {
        self.fee.clone()
    }

    fn get_type(&self) -> Headers {
        Headers::TransactionBlock
    }
}

#[derive(Debug)]
pub struct SummarizeBlock {
    pub default_info: BasicInfo,
    pub merkle_tree_root: Hash,
}

impl SummarizeBlock {
    pub fn parse(data: &[u8]) -> Result<Self, BlockError> {
        if data.len() <= 32 {
            return Err(
                Report::new(BlockError::SummarizeBlock(SummarizeBlockErrorKind::Parse))
                    .attach_printable("data length <= 32"),
            );
        }

        let mut index = 0;

        let merkle_tree_root: Hash = unsafe { data[0..32].try_into().unwrap_unchecked() };
        index += 32;

        let default_info = BasicInfo::parse(&data[index..])
            .change_context(BlockError::SummarizeBlock(SummarizeBlockErrorKind::Parse))
            .attach_printable("Error parsing default data")?;

        Ok(Self {
            default_info,
            merkle_tree_root,
        })
    }
}

impl MainChainBlock for SummarizeBlock {
    fn get_type(&self) -> Headers {
        Headers::SummarizeBlock
    }
    fn hash(&self) -> Result<Hash, BlockError> {
        let result = self
            .dump()
            .change_context(BlockError::SummarizeBlock(SummarizeBlockErrorKind::Hash));

        let dump: Vec<u8> = unsafe { result.unwrap_unchecked() };

        Ok(tools::hash(&dump))
    }

    fn get_dump_size(&self) -> usize {
        1 + 32 + self.default_info.get_dump_size()
    }

    fn dump(&self) -> Result<Vec<u8>, BlockError> {
        let mut to_return: Vec<u8> = Vec::with_capacity(self.get_dump_size());

        // header
        to_return.push(Headers::SummarizeBlock as u8);

        // merkle tree
        to_return.extend(self.merkle_tree_root.iter());

        // default info
        self.default_info
            .dump(&mut to_return)
            .change_context(BlockError::SummarizeBlock(SummarizeBlockErrorKind::Dump))
            .attach_printable("Error dumping default info")?;

        Ok(to_return)
    }

    fn get_info(&self) -> BasicInfo {
        self.default_info.clone()
    }

    fn get_merkle_root(&self) -> Hash {
        self.merkle_tree_root
    }

    fn verify_block(&self, prev_hash: &Hash) -> bool {
        self.default_info.previous_hash.eq(prev_hash)
    }

    fn get_founder(&self) -> &Address {
        &self.default_info.founder
    }

    fn get_fee(&self) -> U256 {
        U256::zero()
    }
}

/// Deserializes block's dump into MainChainBlockArc
pub fn deserialize_main_chain_block(dump: &[u8]) -> Result<MainChainBlockArc, BlockError> {
    if dump.is_empty() {
        return Err(
            Report::new(BlockError::HeaderError(DumpHeadersErrorKind::WrongHeader))
                .attach_printable("The size of supplied data is 0"),
        );
    }

    let header = Headers::from_u8(*unsafe { dump.get_unchecked(0) })
        .change_context(BlockError::HeaderError(DumpHeadersErrorKind::UknownHeader))?;

    let block: MainChainBlockArc = match header {
        Headers::TransactionBlock => Arc::new(TransactionBlock::parse(&dump[1..])?),
        Headers::SummarizeBlock => Arc::new(SummarizeBlock::parse(&dump[1..])?),
        _ => {
            return Err(
                Report::new(BlockError::HeaderError(DumpHeadersErrorKind::WrongHeader))
                    .attach_printable("Not block header"),
            );
        }
    };

    Ok(block)
}

pub type MainChainBlockArc = Arc<dyn MainChainBlock + Send + Sync>;

impl Eq for dyn MainChainBlock + Send + Sync {}

impl PartialEq for dyn MainChainBlock + Send + Sync {
    fn eq(&self, other: &Self) -> bool {
        self.get_info().timestamp == other.get_info().timestamp
    }
}

impl PartialOrd for dyn MainChainBlock + Send + Sync {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.get_info().timestamp.cmp(&other.get_info().timestamp))
    }
}

impl Ord for dyn MainChainBlock + Send + Sync {
    fn cmp(&self, other: &Self) -> Ordering {
        self.get_info().timestamp.cmp(&other.get_info().timestamp)
    }
}
