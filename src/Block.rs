use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use crate::Tools;
use crate::merkletree::MerkleTree;
use crate::Transaction::Transaction;
use crate::Token;
use base64;
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::transmute;
use std::mem::transmute_copy;
use crate::DumpHeaders::Headers;
use crate::Errors::*;

use error_stack::{Report, Result, ResultExt, IntoReport};


#[macro_export]

macro_rules! bytes_to_u64  {
    ($buffer:expr,$buffer_index:expr) => {
       (&$buffer[$buffer_index..$buffer_index+8]).read_u64::<BigEndian>().unwrap()
    };
}

static ALREADY_SET:&str = "data is already set";

#[derive(Debug)]
pub struct BasicInfo{
    timestamp:u64,
    PoW:BigUint,
    previous_hash:[u8;32],
    current_hash:[u8;32],
    height:u64,
    difficulty:[u8;32]
}


impl BasicInfo{
    pub fn new(//miner:[u8;33],
                timestamp:u64,
                PoW:BigUint,
                previous_hash:[u8;32],
                current_hash:[u8;32],
                height:u64,
                difficulty:[u8;32]) -> BasicInfo{
        return BasicInfo{//miner:miner,
                        timestamp:timestamp,
                        PoW:PoW,
                        previous_hash:previous_hash,
                        current_hash:current_hash,
                        height:height,
                        difficulty:difficulty};
    }

    pub fn get_dump_size(&self) -> usize{
        let to_return = 8
                    + Tools::bigint_size(&self.PoW)
                    + 32
                    + 32
                    + 8
                    + 32;
        return to_return;
    }
    pub fn dump(&self,buffer:&mut Vec<u8>) -> Result<(), BlockError>{

        // dumping timestamp
        for byte in self.timestamp.to_be_bytes().iter(){
            buffer.push(*byte);
        }

        // dumping previous hash
        for byte in self.previous_hash.iter(){
            buffer.push(*byte);
        }

        // dumping current hash
        for byte in self.current_hash.iter(){
            buffer.push(*byte);
        }

        // dumping height
        for byte in self.height.to_be_bytes().iter(){
            buffer.push(*byte);
        }

        // dumping difficulty
        buffer.extend(self.difficulty);

        // dumping PoW
        Tools::dump_biguint(&self.PoW, buffer)
        .change_context(BlockError::BasicInfoError(BasicInfoErrorKind::DumpError));

        return Ok(());
    }

    pub fn parse(data:&[u8]) -> Result<BasicInfo, BlockError>{
        let mut index:usize = 0;

        if data.len() <= 112{
            return Err(
                Report::new(BlockError::BasicInfoError(BasicInfoErrorKind::ParseError))
                .attach_printable("data <= 112")
            );
        }

        // parsing timestamp
        let timestamp = bytes_to_u64!(data,index);
        index += 8;

        // parsing previous hash
        let previous_hash:[u8;32] = unsafe{transmute_copy(&data[index])};
        index += 32;

        // parsing current hash
        let current_hash:[u8;32] = unsafe{transmute_copy(&data[index])};
        index += 32;

        // parsing height
        let height:u64 = bytes_to_u64!(data,index);
        index += 8;

        // parsing difficulty
        let difficulty:[u8;32] = unsafe{transmute_copy(&data[index])};
        index += 32;
        
        // parsing PoW
        let (PoW, _) = Tools::load_biguint(&data[index..])
        .change_context(BlockError::BasicInfoError(BasicInfoErrorKind::ParseError))
        .attach_printable("failed to parse PoW")?;

        return Ok(BasicInfo{timestamp:timestamp,
                        PoW:PoW,
                        previous_hash:previous_hash,
                        current_hash:current_hash,
                        height:height,
                        difficulty:difficulty});
    } 
}

#[derive(Debug)]
pub struct TransactionToken{
    transaction:Option<Transaction>,
    token:Option<Token::TokenAction>
}
impl TransactionToken{
    pub fn new(tr:Option<Transaction>,tk:Option<Token::TokenAction>)->TransactionToken{
        return TransactionToken{transaction:tr,
                                token:tk};
    }
    pub fn is_empty(&self) -> bool{
        return self.transaction.is_none() && self.token.is_none();  
    }

    pub fn is_transaction(&self) -> bool{
        return !self.transaction.is_none();
    }
    pub fn is_token(&self) -> bool{
        return !self.token.is_none();
    }

    pub fn set_transaction(&mut self, 
                            transaction:Transaction) 
                            -> Result<(), BlockError>{
        if !self.is_empty(){
            return Err(
                Report::new(BlockError::TransactionTokenError(TxTokenErrorKind::SettingTxError))
            );
        }

        self.transaction = Some(transaction);

        return Ok(());
    }
    pub fn set_token(&mut self, token:Token::TokenAction) 
                            -> Result<(), BlockError>{
        if !self.is_empty(){
            return Err(
                Report::new(BlockError::TransactionTokenError(TxTokenErrorKind::SettingTokenError))
            );
        }

        self.token = Some(token);

        return Ok(());
    }

    pub fn get_transaction(&self) -> &Option<Transaction>{
        return &self.transaction;
    }
    pub fn get_token(&self) -> &Option<Token::TokenAction>{
        return &self.token;
    }
    pub fn get_hash(&self,previous_hash:&[u8;32]) -> Box<[u8;32]>{
        if self.is_transaction(){
            return self.transaction.as_ref().unwrap().hash(previous_hash);
        }else{
            return self.token.as_ref().unwrap().hash(previous_hash);
        }
    }
    pub fn get_dump_size(&self) -> usize{
        if self.is_transaction(){
            return self.transaction.as_ref().unwrap().get_dump_size();
        }
        else{
            return self.token.as_ref().unwrap().get_dump_size();
        }
    }

    pub fn dump(&self) -> Result<Vec<u8>,BlockError>{
        if self.is_transaction(){
            return self.transaction.as_ref().unwrap().dump()
            .change_context(BlockError::TransactionTokenError(TxTokenErrorKind::DumpError));
        }else{
            return self.token.as_ref().unwrap().dump()
            .change_context(BlockError::TransactionTokenError(TxTokenErrorKind::DumpError));
        }
    }   
}

#[derive(Debug)]
pub struct TransactionBlock{
    transactions:Vec<TransactionToken>,
    fee:BigUint,
    merkle_tree:Option<MerkleTree>,
    merkle_tree_root:[u8;32],
    default_info:BasicInfo
}

impl TransactionBlock{
    pub fn new(transactions:Vec<TransactionToken>,
                fee:BigUint,
                default_info:BasicInfo,
                merkle_tree_root:[u8;32]) -> TransactionBlock{
        return TransactionBlock{transactions:transactions,
                                fee:fee,merkle_tree:None,
                                default_info:default_info,
                                merkle_tree_root:merkle_tree_root};
    }

    pub fn merkle_tree_is_built(&self) -> bool{
        return !self.merkle_tree.is_none();
    }

    pub fn build_merkle_tree(&mut self) ->Result<(), BlockError>{
        let mut new_merkle_tree = MerkleTree::new();
        let mut hashes:Vec<&[u8;32]> = Vec::with_capacity(self.transactions.len());

        for TT in self.transactions.iter(){
            let res = TT.get_hash(&self.default_info.previous_hash);
            hashes.push(Box::leak(res));      
        }

        let res = new_merkle_tree.add_objects(hashes);
        if !res{
            return Err(
                Report::new(BlockError::TransactionBlockError(TxBlockErrorKind::BuildingMerkleTreeError))
            );
        }
        self.merkle_tree = Some(new_merkle_tree);
        return Ok(());
    }

    pub fn check_merkle_tree(&mut self) -> Result<bool, BlockError>{
        // build merkle tree if not built
        if !self.merkle_tree_is_built(){
            self.build_merkle_tree()?;
        }

        // transmute computed root into 4 u64 bytes 
        let constructed_tree_root_raw = self.merkle_tree.as_ref().unwrap().get_root(); 
        let constructed_tree_root_raw_root:&[u64;4] = unsafe{
                                transmute(constructed_tree_root_raw)};
        
        // transmute root into 4 u64 bytes 
        let root:&[u64;4] = unsafe{transmute(&self.merkle_tree_root)};

        for (a,b) in root.iter().zip(
                            constructed_tree_root_raw_root.iter()){
            if *a != *b{
                return Ok(false);
            }
        }
        return Ok(true);
    }

    pub fn get_dump_size(&self) -> usize{
        let mut size:usize = 1;
        for transaction in self.transactions.iter(){
            size += transaction.get_dump_size();
        }
        size += Tools::bigint_size(&self.fee);
        size += 32;
        size += self.default_info.get_dump_size();

        return size;
    }

    pub fn dump(&self) -> Result<Vec<u8>, BlockError>{
        let size:usize = self.get_dump_size();

        let mut to_return:Vec<u8> = Vec::with_capacity(size);

        //header
        to_return.push(Headers::TransactionBlock as u8);

        // merkle tree root
        to_return.extend(self.merkle_tree_root.iter());

        // default info
        self.default_info.dump(&mut to_return)
        .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::DumpError))?;

        // fee
        Tools::dump_biguint(&self.fee, &mut to_return)
        .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::DumpError))?;
        
        // amount of transactions
        let amount_of_transactions:u16;
        if self.transactions.len() > 0xFFFF{
            return Err(
                Report::new(BlockError::TransactionBlockError(TxBlockErrorKind::DumpError))
                .attach_printable(format!("transactions: {}", self.transactions.len()))
            );
        }else{
            amount_of_transactions = self.transactions.len() as u16
        }

        to_return.extend(amount_of_transactions.to_be_bytes().iter());

        // transactions/tokens
        for transaction in self.transactions.iter(){
            // size of transaction
            let size_of_transaction:u32 = transaction.get_dump_size() as u32;
            to_return.extend(size_of_transaction.to_be_bytes().iter());

            for byte in transaction.dump().unwrap().iter(){
                to_return.push(*byte);
            }
        }

        return Ok(to_return);
    }

    pub fn parse(data:&[u8],block_size:u32) -> Result<TransactionBlock, BlockError>{
        let mut offset:usize = 0;

        // merkle tree root
        let merkle_tree_root:[u8;32] = data[..32].try_into().unwrap();
        offset += 32; // inc offset

        
        // default info
        let default_info = BasicInfo::parse(&data[offset..])
        .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))?;

        offset += default_info.get_dump_size(); // inc offset

        // fee
        let (fee, _offset) = Tools::load_biguint(&data[offset..])
        .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))?;
  

        offset += _offset; // inc offset

        // transactions
        let amount_of_transactions:u16 = u16::from_be_bytes(
                                data[offset..offset+2].try_into().unwrap());
        offset += 2; // inc offset
        
        let mut transactions:Vec<TransactionToken> = Vec::with_capacity(amount_of_transactions as usize);

        for _ in 0..amount_of_transactions{
            let transaction_size:u32 = u32::from_be_bytes(data[offset..offset+4].try_into().unwrap())-1;
            
            offset += 4; // inc offset

            let trtk_type:u8 = data[offset];
            offset += 1;

            let mut trtk:TransactionToken = TransactionToken::new(None,None);

            if trtk_type == Headers::Transaction as u8{
                // if transaction
                let transaction = Transaction::parse_transaction(
                    &data[offset..offset+(transaction_size as usize)],transaction_size as u64)
                    .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))?;
                
                trtk.set_transaction(transaction)
                .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))?;

            } else if trtk_type == Headers::Token as u8{
                // if token action
                //TODO
                let token = Token::TokenAction::parse(
                    &data[offset..offset+(transaction_size as usize)],transaction_size as u64)
                    .attach_printable("Error parsing transaction block: couldn't parse token")
                    .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))?;
                

                trtk.set_token(token)
                .attach_printable("Error parsing transaction block: couldn't set token")
                .change_context(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))?;
            }else{
                return Err(
                    Report::new(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))
                );
            }
            offset += transaction_size as usize; // inc offset
            
            transactions.push(trtk); 
        }

        if offset != block_size as usize{
            return Err(
                Report::new(BlockError::TransactionBlockError(TxBlockErrorKind::ParseError))
            );
        }

        let transaction_block = TransactionBlock::new(transactions,
                                            fee,
                                            default_info,
                                            merkle_tree_root);

        return Ok(transaction_block);

    }

    pub fn hash(&self) -> Result<[u8;32],BlockError>{
        let dump:Vec<u8> = self.dump().unwrap();

        return Ok(Tools::hash(&dump));
    }

}

pub struct TokenBlock{
    pub default_info:BasicInfo,
    pub token_signature:String,
    pub payment_transaction:Transaction
}

impl TokenBlock{
    pub fn new(default_info:BasicInfo,
                token_signature:String,
                payment_transaction:Transaction) -> TokenBlock{

        return TokenBlock{default_info:default_info,
                        token_signature:token_signature,
                        payment_transaction:payment_transaction}
    }

    pub fn get_dump_size(&self) -> usize{
        return self.default_info.get_dump_size()
                +self.token_signature.len()
                +1
                +self.payment_transaction.get_dump_size();
    }

    pub fn dump(&self) -> Result<Vec<u8>, BlockError>{
        let dump_size:usize = self.get_dump_size();
        
        let mut dump:Vec<u8> = Vec::with_capacity(dump_size);

        // header
        dump.push(Headers::TokenBlock as u8);

        // // dumping token signature
        // for byte in self.token_signature.as_bytes().iter(){
        //     dump.push(*byte);
        // }
        // dump.push(0);

        // dumping payment transaction
        let transaction_len:u32 = self.payment_transaction.get_dump_size() as u32;
        dump.extend(transaction_len.to_be_bytes().iter());
        
        let result = self.payment_transaction.dump()
        .change_context(BlockError::TokenBlockError(TokenBlockErrorKind::DumpError))?;

        dump.extend(result);

        // dumping default info
        self.default_info.dump(&mut dump)
        .change_context(BlockError::TokenBlockError(TokenBlockErrorKind::DumpError))?;

        return Ok(dump);
    }

    pub fn parse(data:&[u8],block_size:u32) -> Result<TokenBlock,BlockError>{
        
        let mut offset:usize = 0;
        // parsing token signature
        let mut token_signature:String = String::new();
        // for byte in data{
        //     offset += 1;
        //     if *byte == 0{
        //         break;
        //     }
        //     token_signature.push(*byte as char);
        // }

        // parsing transaction
        let transaction_size:u32 = u32::from_be_bytes(data[offset..offset+4].try_into().unwrap());
        offset += 4;
        
        if data[offset] != Headers::Transaction as u8{
            return Err(
                Report::new(BlockError::TokenBlockError(TokenBlockErrorKind::ParseError))
            );
        }
        offset += 1;

        let transaction = Transaction::parse_transaction(&data[offset..offset+transaction_size as usize], (transaction_size-1) as u64)
        .attach_printable("Error parsing token block: couldn't parse transaction")
        .change_context(BlockError::TokenBlockError(TokenBlockErrorKind::ParseError))?;

        offset += (transaction_size-1) as usize;

        // parsing basic info 
        let default_info = BasicInfo::parse(&data[offset..block_size as usize])
        .attach_printable("Error parsing token block: couldn't parse basic info")
        .change_context(BlockError::TokenBlockError(TokenBlockErrorKind::ParseError))?;

        offset += default_info.get_dump_size();

        if offset != block_size as usize{
            return Err(
                Report::new(BlockError::TokenBlockError(TokenBlockErrorKind::ParseError))
            );
        }

        return Ok(TokenBlock{default_info:default_info,
                            token_signature:token_signature,
                            payment_transaction:transaction});
    }

    pub fn hash(&self) -> Result<[u8;32],BlockError>{
        let dump:Vec<u8> = self.dump().unwrap();

        return Ok(Tools::hash(&dump));
    }

}



pub struct SummarizeBlock{
    default_info:BasicInfo,
    founder_transaction:Transaction

}

impl SummarizeBlock{
    pub fn new(default_info:BasicInfo,
                founder_transaction:Transaction) -> SummarizeBlock{

        return SummarizeBlock{default_info:default_info,
                    founder_transaction:founder_transaction};
    }

    pub fn get_dump_size(&self) -> usize{
        return 1 // header
                +self.default_info.get_dump_size()
                +self.founder_transaction.get_dump_size();
    }
    
    pub fn dump(&self) -> Result<Vec<u8>, BlockError>{

        let mut to_return:Vec<u8> = Vec::with_capacity(self.get_dump_size());

        // header
        to_return.push(Headers::SummarizeBlock as u8);

        // dump transaction
        let mut transaction_dump = self.founder_transaction.dump()
        .change_context(BlockError::SummarizeBlockError(SummarizeBlockErrorKind::DumpError))?;
        
        to_return.extend((transaction_dump.len() as u64).to_be_bytes());
        to_return.append(&mut transaction_dump);

        // dump basic info
        self.default_info.dump(&mut to_return)?;

        return Ok(to_return);
    }

    pub fn parse(data:&[u8]) -> Result<SummarizeBlock, BlockError>{
        if data.len() <= 8{
            return Err(
                Report::new(BlockError::SummarizeBlockError(SummarizeBlockErrorKind::ParseError))
                .attach_printable("data length <= 8")
            );
        }
        let mut offset:usize = 0;
        
        // parse transaction
        let transaction_size:usize = u64::from_be_bytes(data[0..8].try_into().unwrap()) as usize - 1;
        offset += 8;
        if data.len()<transaction_size+8{
            return Err(
                Report::new(BlockError::SummarizeBlockError(SummarizeBlockErrorKind::ParseError))
                .attach_printable("data length < tx size + 8")
            );
        }
        if data[offset] != Headers::Transaction as u8{
            return Err(
                Report::new(BlockError::SummarizeBlockError(SummarizeBlockErrorKind::ParseError))
                .attach_printable("headers not found")
            );
        }
        offset += 1;

        let transaction = Transaction::parse_transaction(&data[offset..offset+transaction_size], transaction_size as u64)
        .change_context(BlockError::SummarizeBlockError(SummarizeBlockErrorKind::ParseError))?;

        offset += transaction_size;

        // parse default info
        let default_info = BasicInfo::parse(&data[offset..])
        .change_context(BlockError::SummarizeBlockError(SummarizeBlockErrorKind::ParseError))?;

        return Ok(SummarizeBlock{default_info:default_info,
                        founder_transaction:transaction});

    }

    pub fn hash(&self) -> Result<[u8;32],BlockError>{
        let result = self.dump()
        .change_context(BlockError::SummarizeBlockError(SummarizeBlockErrorKind::HashError));

        let dump:Vec<u8> = unsafe{result.unwrap_unchecked()}; 

        return Ok(Tools::hash(&dump));
    }

}


pub struct SumTransactionBlock{
    transaction_block: Option<TransactionBlock>,
    summarize_block: Option<SummarizeBlock>
}

impl SumTransactionBlock{
    pub fn new(transaction_block:Option<TransactionBlock>,
                summarize_block:Option<SummarizeBlock>)
                ->SumTransactionBlock{
                                       
        return SumTransactionBlock{transaction_block:transaction_block,
                                summarize_block:summarize_block};
    }
    
    pub fn is_empty(&self) -> bool{
        return self.summarize_block.is_none() && 
                self.transaction_block.is_none();
    }

    pub fn is_transaction_block(&self) -> bool{
        return self.transaction_block.is_none();
    }
    pub fn is_summarize_block(&self) -> bool{
        return self.summarize_block.is_none();
    }
    pub fn hash(&self) -> Result<[u8;32],BlockError>{
        if self.is_transaction_block(){
            return self.transaction_block.as_ref().unwrap().hash();
        }else{
            return self.summarize_block.as_ref().unwrap().hash();
        }
    }

    pub fn get_dump_size(&self) -> usize{
        if self.is_transaction_block(){
            return self.transaction_block.as_ref().unwrap().get_dump_size();
        }else{
            return self.summarize_block.as_ref().unwrap().get_dump_size();
        }
    }

    pub fn dump(&self) -> Result<Vec<u8>, BlockError>{
        if self.is_transaction_block(){
            return self.transaction_block.as_ref().unwrap().dump();
        }else{
            return self.summarize_block.as_ref().unwrap().dump();
        }
    }
}

