use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use crate::Tools;
use crate::merkletree::MerkleTree;
use crate::Transaction;
use crate::Token;
use base64;
use byteorder::{BigEndian, ReadBytesExt};
use std::mem::transmute;
use std::mem::transmute_copy;


#[macro_export]
macro_rules! bytes_to_u64  {
    ($buffer:expr,$buffer_index:expr) => {
       (&$buffer[$buffer_index..$buffer_index+8]).read_u64::<BigEndian>().unwrap()
    };
}

static ALREADY_SET:&str = "data is already set";

pub struct BasicInfo{
    miner:[u8;33],
    timestamp:u64,
    PoW:BigUint,
    previous_hash:[u8;32],
    height:u64,
    difficulty:u8
}


impl BasicInfo{
    pub fn new(miner:[u8;33],
                timestamp:u64,
                PoW:BigUint,
                previous_hash:[u8;32],
                height:u64,
                difficulty:u8) -> BasicInfo{
        return BasicInfo{miner:miner,
                        timestamp:timestamp,
                        PoW:PoW,
                        previous_hash:previous_hash,
                        height:height,
                        difficulty:difficulty};
    }

    pub fn get_dump_size(&self) -> usize{
        let to_return = 33
                    + 8
                    + Tools::bigint_size(&self.PoW)
                    + 32
                    + 8;
        return to_return;
    }
    pub fn dump(&self,buffer:&mut Vec<u8>) -> Result<(),&'static str>{

        // dumping miner
        for byte in self.miner.iter(){
            buffer.push(*byte);
        }

        // dumping timestamp
        for byte in self.timestamp.to_be_bytes().iter(){
            buffer.push(*byte);
        }

        // dumping previous hash
        for byte in self.previous_hash.iter(){
            buffer.push(*byte);
        }

        // dumping height
        for byte in self.height.to_be_bytes().iter(){
            buffer.push(*byte);
        }

        // dumping difficulty
        buffer.push(self.difficulty);

        // dumping PoW
        let result = Tools::dump_biguint(&self.PoW, buffer);
        if result.is_err(){
            return Err("could not dump PoW");
        }

        return Ok(());
    }

    pub fn parse(data:&[u8]) -> Result<BasicInfo,&'static str>{
        let mut index:usize = 0;
        
        // parsing miner
        let miner:[u8;33] = unsafe{transmute_copy(&data[index])};
        index += 33;

        // parsing timestamp
        if data.len() - index < 8{
            return Err("Could not parse timestamp");
        }
        let timestamp = bytes_to_u64!(data,index);
        index += 8;

        // parsing previous hash
        if data.len() - index < 32{
            return Err("Could not parse previous hash");
        }
        let mut previous_hash:[u8;32] = [0;32];
        for i in 0..32{
            previous_hash[i] = data[index+i];
        }
        index += 32;

        // parsing height
        if data.len() - index < 8{
            return Err("Could not parse height");
        }
        let height:u64 = bytes_to_u64!(data,index);
        index += 8;

        // parsing difficulty
        if data.len() - index < 1{
            return Err("Could not parse timestamp");
        }
        let difficulty:u8 = data[index];
        
        // parsing PoW
        let result = Tools::load_biguint(&data[index..]);
        if result.is_err(){
            return Err("Error loading PoW");
        }
        let PoW: BigUint;
        match result{
            Err(e)=>{return Err(e);}
            Ok(a) => {PoW = a.0;}
        }

        return Ok(BasicInfo{miner:miner,
                        timestamp:timestamp,
                        PoW:PoW,
                        previous_hash:previous_hash,
                        height:height,
                        difficulty:difficulty});
    } 
}

pub struct TransactionToken{
    transaction:Option<Transaction::Transaction>,
    token:Option<Token::TokenAction>
}
impl TransactionToken{
    pub fn new()->TransactionToken{
        return TransactionToken{transaction:None,
                                token:None};
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

    pub fn set_transaction(&mut self, transaction:Transaction::Transaction) 
                            -> Result<(),&'static str>{
        if !self.is_empty(){
            return Err(ALREADY_SET);
        }

        self.transaction = Some(transaction);

        return Ok(());
    }
    pub fn set_token(&mut self, token:Token::TokenAction) 
                            -> Result<(),&'static str>{
        if !self.is_empty(){
            return Err(ALREADY_SET);
        }

        self.token = Some(token);

        return Ok(());
    }

    pub fn get_transaction(&self) -> &Option<Transaction::Transaction>{
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
    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        if self.is_transaction(){
            return self.transaction.as_ref().unwrap().dump();
        }else{
            return self.token.as_ref().unwrap().dump();
        }
    }   
}


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

    pub fn build_merkle_tree(&mut self) ->Result<(),&'static str>{
        let mut new_merkle_tree = MerkleTree::new();
        let mut hashes:Vec<&[u8;32]> = Vec::with_capacity(self.transactions.len());

        for TT in self.transactions.iter(){
            let res = TT.get_hash(&self.default_info.previous_hash);
            hashes.push(Box::leak(res));      
        }

        new_merkle_tree.add_objects(hashes);
        self.merkle_tree = Some(new_merkle_tree);
        return Ok(());
    }

    pub fn check_merkle_tree(&mut self) -> Result<bool,&'static str>{
        if !self.merkle_tree_is_built(){
            let res = self.build_merkle_tree();
            if res.is_err(){
                return Err(res.err().unwrap());
            }
        }
        let constructed_tree_root_raw = self.merkle_tree.as_ref().unwrap().get_root();
        
        let constructed_tree_root_raw_root:&[u64;4] = unsafe{
                                transmute(constructed_tree_root_raw)};
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

    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        let size:usize = self.get_dump_size();

        let mut to_return:Vec<u8> = Vec::with_capacity(size);

        //header
        to_return.push(2);

        // merkle tree root
        to_return.extend(self.merkle_tree_root.iter());

        // default info
        let result = self.default_info.dump(&mut to_return);
        if result.is_err(){
            return Err("Error dumping default info");
        }

        // fee
        let result = Tools::dump_biguint(&self.fee, &mut to_return);
        if result.is_err(){
            return Err("Error dumping BigUInt");
        }
        
        // amount of transactions
        let amount_of_transactions:u16;
        if self.transactions.len() > 0xFFFF{
            return Err("Too much transactions");
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

    pub fn parse(data:&[u8],block_size:u32) -> Result<TransactionBlock,&'static str>{
        let mut offset:usize = 0;

        // merkle tree root
        let merkle_tree_root:[u8;32] = data[..32].try_into().unwrap();
        offset += 32; // inc offset

        
        // default info
        let result = BasicInfo::parse(&data[offset..]);
        if result.is_err(){
            return Err("Bad BasicInfo");
        }
        let default_info:BasicInfo = result.unwrap();
        offset += default_info.get_dump_size(); // inc offset

        // fee
        let result = Tools::load_biguint(&data[offset..]);
        if result.is_err(){
            return Err("Error in parsing fee");
        }
        let result_enum = result.unwrap();
        let fee = result_enum.0;

        offset += result_enum.1; // inc offset

        // transactions
        let amount_of_transactions:u16 = u16::from_be_bytes(
                                data[offset..offset+2].try_into().unwrap());
        offset += 2; // inc offset
        
        let mut transactions:Vec<TransactionToken> = Vec::with_capacity(amount_of_transactions as usize);

        for _ in 0..amount_of_transactions{
            let transaction_size:u32 = u32::from_be_bytes(data[offset..offset+4].try_into().unwrap());
            
            offset += 4; // inc offset

            let trtk_type:u8 = data[offset];
            let mut trtk:TransactionToken = TransactionToken::new();

            if trtk_type == 0{
                // if transaction
                let result = Transaction::Transaction::parse_transaction(
                    &data[offset..offset+(transaction_size as usize)],transaction_size as u64);
                if result.is_err(){
                    return Err("Error parsing token");
                }
                let transaction = result.unwrap();
                let result = trtk.set_transaction(transaction);
                if result.is_err(){
                    return Err("Error setting transaction");
                }
            } else if trtk_type == 1{
                // if token action
                let result = Token::TokenAction::parse(
                    &data[offset..offset+(transaction_size as usize)],transaction_size as u64);
                if result.is_err(){
                    return Err("Error parsing token");
                }
                let token = result.unwrap();
                let result = trtk.set_token(token);
                if result.is_err(){
                    return Err("Error setting token");
                }  
            }else{
                return Err("Not existant type");
            }
            offset += transaction_size as usize; // inc offset
            
            transactions.push(trtk); 
        }

        if offset != block_size as usize{
            return Err("Could not parse block");
        }

        let transaction_block = TransactionBlock::new(transactions,
                                            fee,
                                            default_info,
                                            merkle_tree_root);

        return Ok(transaction_block);

    }
    pub fn get_hash(&self) -> [u8;32]{
        let mut hasher = Sha256::new();
        let dump = self.dump().unwrap();

        hasher.update(dump);
        let result:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();
        return result;
    }

}

pub struct TokenBlock{
    default_info:BasicInfo,
    token_signature:String,
    payment_transaction:Transaction::Transaction
}

impl TokenBlock{
    pub fn new(default_info:BasicInfo,
                token_signature:String,
                payment_transaction:Transaction::Transaction) -> TokenBlock{

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

    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        let dump_size:usize = self.get_dump_size();
        
        let mut dump:Vec<u8> = Vec::with_capacity(dump_size);

        dump.push(3);

        for byte in self.token_signature.as_bytes().iter(){
            dump.push(*byte);
        }
        dump.push(0);

        let transaction_len:u32 = self.payment_transaction.get_dump_size() as u32;
        dump.extend(transaction_len.to_be_bytes().iter());

        let result = self.payment_transaction.dump();
        if result.is_err(){
            return Err("Error dumping payment transaction");
        }

        dump.extend(result.unwrap());

        let result = self.default_info.dump(&mut dump);
        if result.is_err(){
            return Err("Error dumping default info");
        }

        return Ok(dump);
    }

    pub fn parse(data:&[u8],block_size:u32) -> Result<TokenBlock,&'static str>{
        
        let mut offset:usize = 0;
        let mut token_signature:String = String::new();
        for byte in data{
            offset += 1;
            if *byte == 0{
                break;
            }
            token_signature.push(*byte as char);
        }

        let transaction_size:u32 = u32::from_be_bytes(data[offset..offset+4].try_into().unwrap());
        offset += 4;
        let result = Transaction::Transaction::parse_transaction(&data[offset..offset+transaction_size as usize], transaction_size as u64);
        if result.is_err(){
            return Err("Parsing transaction error");
        }

        let transaction = result.unwrap();
        offset += transaction_size as usize;

        let result = BasicInfo::parse(&data[offset..block_size as usize]);
        if result.is_err(){
            return Err("Parsing basic info error");
        }
        let default_info = result.unwrap();

        offset += default_info.get_dump_size();

        if offset != block_size as usize{
            return Err("Error parsing token block");
        }


        return Ok(TokenBlock{default_info:default_info,
                            token_signature:token_signature,
                            payment_transaction:transaction});
    }
}
