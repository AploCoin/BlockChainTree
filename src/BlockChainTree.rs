#![allow(non_snake_case)]
use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use crate::Tools;
use crate::Transaction;
use crate::Token;
use crate::Block;
use std::mem::transmute_copy;
use zstd;

use std::env;
use std::fs;
use std::io;
use std::io::Write;
use std::io::Read;
use std::collections::HashMap;

static BLOCKS_IN_FILE:usize = 4;
static BLOCKS_DIRECTORY:&'static str = "./BlockChainTree/"; 
static MAIN_BLOCKS_DIRECTORY:&'static str = "./BlockChainTree/MAIN/";
static DERIVATIVE_BLOCKS_DIRECTORY:&'static str = "./BlockChainTree/DERIVATIVE/";
static CONFIG_FILE:&'static str = "BlockChainTree.config";
static LOOKUP_TABLE_FILE:&'static str = "LookUPTable.dat"; 
static GENESIS_BLOCK:[u8;32] = [0x77,0xe6,0xd9,0x52,
                                0x67,0x57,0x8e,0x85,
                                0x39,0xa9,0xcf,0xe0,
                                0x03,0xf4,0xf7,0xfe,
                                0x7d,0x6a,0x29,0x0d,
                                0xaf,0xa7,0x73,0xa6,
                                0x5c,0x0f,0x01,0x9d,
                                0x5c,0xbc,0x0a,0x7c];
// God is dead, noone will stop anarchy

pub fn compress_to_file(output_file:String,data:&[u8])->Result<(),&'static str>{
    let result = fs::File::create(output_file);
    if result.is_err(){
        return Err("Error creating file");
    }
    let target = result.unwrap();


    let result = zstd::Encoder::new(target,1);
    if result.is_err(){
        return Err("Error creating encoder");
    }
    let mut encoder = result.unwrap(); 
    
    let result = encoder.write_all(data);
    if result.is_err(){
        return Err("Error encoding data");
    }

    let result = encoder.finish();
    if result.is_err(){
        return Err("Error closing file");
    }

    return Ok(());
}

pub fn decompress_from_file(filename:String) -> Result<Vec<u8>,&'static str>{
    let mut decoded_data:Vec<u8> = Vec::new();

    let result = fs::File::open(filename);
    if result.is_err(){
        return Err("Error opening file");
    }
    let file = result.unwrap();
    
    let result = zstd::Decoder::new(file);
    if result.is_err(){
        return Err("Error creating decoder");
    }
    let mut decoder = result.unwrap();

    let result = decoder.read_to_end(&mut decoded_data);
    if result.is_err(){
        return Err("Error reading file");
    }

    return Ok(decoded_data);
}

pub fn read_lookup_table(file_path:String) -> 
                    Result<HashMap<[u8;32],u64>,&'static str>{

    let result = decompress_from_file(file_path);
    

    let decompressed_data = result.unwrap();

    if decompressed_data.len()%40 != 0{
        return Err("Bad Data");
    }

    let mut lookup_table_files:HashMap<[u8;32],u64> = HashMap::new(); 

    let mut offset = 0;
    while offset < decompressed_data.len(){
        let key:[u8;32] = unsafe{ transmute_copy::<u8,[u8;32]>(&decompressed_data[offset])};
        let value:u64 = unsafe{ transmute_copy::<u8,u64>(&decompressed_data[offset+32])};
            
        lookup_table_files.insert(key,value);
        offset += 40;
    }

    return Ok(lookup_table_files); 
}

pub fn dump_lookup_table(table:&HashMap<[u8;32],u64>,
                    file_path:String) -> Result<(),&'static str>{
    
    let mut data_to_compress:Vec<u8> = Vec::with_capacity(table.len()*40);
    for (key,value) in table.iter(){
        for byte in key{
            data_to_compress.push(*byte);
        }
        for byte in value.to_be_bytes(){
            data_to_compress.push(byte);
        }
    }

    let result = compress_to_file(String::from(file_path), 
                                        &data_to_compress);
    if result.is_err(){
        return Err("Error compressing and dumping lookup table");
    }
                                
    return Ok(());
}

pub struct Config{
    genesis_block:[u8;32],
    amount_of_files:u64,
}

impl Config{
    pub fn read(file_path:String)->Result<Config,&'static str>{
        let result = fs::File::open(file_path);
        if result.is_err(){
            return Err("Error opening config file");
        }
    
        let mut file = result.unwrap();
    
        let mut genesis_block:[u8;32] = [0;32];
    
        let result = file.read_exact(&mut genesis_block);
        if result.is_err(){
            return Err("Error in reading genesis block");
        }
    
        let mut amount_of_files_bytes:[u8;8] = [0;8];
    
        let result = file.read_exact(&mut amount_of_files_bytes);
        if result.is_err(){
            return Err("Error reading amount of files");
        }
            
        let amount_of_files = u64::from_be_bytes(amount_of_files_bytes);
    
        return Ok(Config{genesis_block:genesis_block,
                amount_of_files:amount_of_files});
    }

    pub fn dump(&self,file_path:String) -> Result<(),&'static str>{
        let result = fs::File::create(file_path);
        if result.is_err(){
            return Err("Error creating file");
        }
        let mut file = result.unwrap();
        
        let result = file.write_all(&self.genesis_block);
        if result.is_err(){
            return Err("Error writing genesis block");
        }

        let result = file.write_all(&self.amount_of_files.to_be_bytes());
        if result.is_err(){
            return Err("Error writing amount of files");
        }

        return Ok(());
    }
}

pub struct MainChain{
    config:Config,
    lookup_table:HashMap<[u8;32],u64>
}

impl MainChain{

    pub fn dump_blocks(&mut self, 
                    blocks:&[Block::TransactionBlock])->
                    Result<(),&'static str>{
        
        let blocks_amount = blocks.len();
        
        if blocks_amount > BLOCKS_IN_FILE 
            || blocks_amount == 0{
            return Err("Wrong");
        }

        let mut dump_data_size:usize = 1 + (blocks_amount*4);
        for block in blocks.iter(){
            let dump_size = block.get_dump_size();
            dump_data_size += dump_size;
        }

        let mut to_dump:Vec<u8> = Vec::with_capacity(dump_data_size);
        
        to_dump.push(blocks_amount as u8);

        for block in blocks.iter(){
            let result = block.dump();
            if result.is_err(){
                return Err("Error dumping block");
            }
            
            let mut dump = result.unwrap();

            to_dump.extend((dump.len() as u32).to_be_bytes().iter());

            to_dump.append(&mut dump);
        }

        let first_block_hash = &blocks[0].get_hash();
        let result = self.lookup_table.get(first_block_hash);
        let mut filename = self.config.amount_of_files;
        let mut file_was_found:bool = false;
        if result.is_some(){
            filename = *result.unwrap();
            file_was_found = true;
        }

        let path = String::from(MAIN_BLOCKS_DIRECTORY) + &filename.to_string();

        let result = compress_to_file(path, &to_dump);
        if result.is_err(){
            return Err("Could not save file");
        }

        if !file_was_found{
            self.config.amount_of_files += 1;
        }
        for block in blocks.iter(){
            self.lookup_table.insert(block.get_hash(), 
                                            filename);
        }

        return Ok(());
    }
    
    pub fn parse_blocks(&self,hash:&[u8;32]) -> Result<Vec<Block::TransactionBlock>,&'static str>{
        let result = self.lookup_table.get(hash);
        if result.is_none(){
            return Err("Could not locate file");
        }
        let filename = result.unwrap();

        let result = self.get_blocks_by_index(*filename);
        if result.is_err(){
            return Err("Error parsing blocks");
        }

        return Ok(result.unwrap());

    }

    pub fn get_blocks_by_index(&self,index:u64) -> Result<Vec<Block::TransactionBlock>,&'static str>{
        if index >= self.config.amount_of_files{
            return Err("Bad index");
        }
        let path = String::from(MAIN_BLOCKS_DIRECTORY) + &index.to_string();

        let result = decompress_from_file(path);
        if result.is_err(){
            return Err("Could not decompress file");
        }

        let decompressed_data = result.unwrap();
        let size_of_data = decompressed_data.len();
        if size_of_data == 0{
            return Err("Empty file");
        }
        let amount:usize = decompressed_data[0] as usize;

        let mut to_return:Vec<Block::TransactionBlock> = Vec::with_capacity(amount);
        let mut offset:usize = 1;
        for _ in 0..amount{
            if size_of_data - offset < 4{
                return Err("Could not parse size of block");
            }
            let mut block_size:u32 = (decompressed_data[offset] as u32)<<24;
            offset += 1;
            block_size += (decompressed_data[offset] as u32)<<16;
            offset += 1;
            block_size += (decompressed_data[offset] as u32)<<8;
            offset += 1;
            block_size += decompressed_data[offset] as u32;
            offset += 1;

            if size_of_data - offset < block_size as usize{
                return Err("Could not parse block");
            }

            let result = Block::TransactionBlock::parse(&decompressed_data[offset..offset+block_size as usize],
                                                        block_size);
            
            if result.is_err(){
                return Err("could not parse block");
            }
            offset += block_size as usize;
            to_return.push(result.unwrap());

        }

        return Ok(to_return);
    }

    pub fn get_blocks_by_height(&self,height:u64) -> Result<Vec<Block::TransactionBlock>,&'static str>{
        let index = height % 4;
        let result = self.get_blocks_by_index(index);
        if result.is_err(){
            return Err(result.err().unwrap());
        }
        return Ok(result.unwrap());
    }

    pub fn dump_config(&self) -> Result<(),&'static str>{
        let result = self.config.dump(String::from(CONFIG_FILE));
        if result.is_err(){
            return Err("Error dumping config");
        }

        let result = dump_lookup_table(&self.lookup_table,
                        String::from(LOOKUP_TABLE_FILE));
        if result.is_err(){
            return Err("Error dumping lookup table");
        }

        return Ok(());
    }

    pub fn with_config() -> Result<MainChain,&'static str>{

        let result = Config::read(String::from(CONFIG_FILE));
        if result.is_err(){
            return Err("Error reading config");
        }
        let config = result.unwrap();

        let result = read_lookup_table(String::from(LOOKUP_TABLE_FILE));
        if result.is_err(){
            return Err("Error reading lookup table");
        }
        let lookup_table = result.unwrap();

        return Ok(MainChain{config:config,
                            lookup_table:lookup_table});

    }
}


pub struct DerivativeChain{
    config:Config,
    lookup_table:HashMap<[u8;32],u64>,
    pathname:u64
}

impl DerivativeChain{

    pub fn with_config(pathname:u64) -> Result<DerivativeChain,
                                        &'static str>{

        let root_path = String::from(DERIVATIVE_BLOCKS_DIRECTORY) 
                    + &pathname.to_string(); 
        
        let config_path = (root_path.clone() + "/") + &CONFIG_FILE;
        
        let result = Config::read(config_path);
        if result.is_err(){
            return Err("Error reading config");
        } 
        let config = result.unwrap();

        let lookup_table_path = (root_path+"/")+&LOOKUP_TABLE_FILE;

        let result = read_lookup_table(lookup_table_path);
        if result.is_err(){
            return Err("Error reading lookup table");
        }
        let lookup_table = result.unwrap();


        return Ok(DerivativeChain{config:config,
                                lookup_table:lookup_table,
                                pathname:pathname});        
    }

    pub fn dump_config(&self) -> Result<(),&'static str>{
        let root_path = String::from(DERIVATIVE_BLOCKS_DIRECTORY) 
                    + &self.pathname.to_string(); 
        
        let config_path = (root_path.clone() + "/") + &CONFIG_FILE;
        let lookup_table_path = (root_path+"/")+&LOOKUP_TABLE_FILE;

        let result = self.config.dump(config_path);
        if result.is_err(){
            return Err("Error dumping config");
        }

        let result = dump_lookup_table(&self.lookup_table,
                                    lookup_table_path);
        if result.is_err(){
            return Err("Error dumping lookup table");
        }

        return Ok(()); 
    }

    pub fn dump_blocks(&mut self, 
            blocks:&[Block::TransactionBlock])->
            Result<(),&'static str>{

        let blocks_amount = blocks.len();

        if blocks_amount > BLOCKS_IN_FILE 
                || blocks_amount == 0{
            return Err("Wrong");
        }

        let mut dump_data_size:usize = 1 + (blocks_amount*4);
        for block in blocks.iter(){
            let dump_size = block.get_dump_size();
            dump_data_size += dump_size;
        }

        let mut to_dump:Vec<u8> = Vec::with_capacity(dump_data_size);

        to_dump.push(blocks_amount as u8);

        for block in blocks.iter(){
            let result = block.dump();
            if result.is_err(){
                return Err("Error dumping block");
            }

            let mut dump = result.unwrap();

            to_dump.extend((dump.len() as u32).to_be_bytes().iter());

            to_dump.append(&mut dump);
        }

        let first_block_hash = &blocks[0].get_hash();
        let result = self.lookup_table.get(first_block_hash);
        let mut filename = self.config.amount_of_files;
        let mut file_was_found:bool = false;
        if result.is_some(){
            filename = *result.unwrap();
            file_was_found = true;
        }

        let path = String::from(DERIVATIVE_BLOCKS_DIRECTORY) + 
                        &self.pathname.to_string() + "/" + 
                        &filename.to_string();


        let result = compress_to_file(path, &to_dump);
        if result.is_err(){
            return Err("Could not save file");
        }

        if !file_was_found{
            self.config.amount_of_files += 1;
        }

        for block in blocks.iter(){
            self.lookup_table.insert(block.get_hash(), 
                                            filename);
        }

        return Ok(());
    }

    pub fn parse_blocks(&self,hash:&[u8;32]) -> Result<Vec<Block::TokenBlock>,&'static str>{
        let result = self.lookup_table.get(hash);
        if result.is_none(){
            return Err("Could not locate file");
        }
        let filename = result.unwrap();

        let result = self.get_blocks_by_index(*filename);
        if result.is_err(){
            return Err("Error parsing blocks");
        }

        return Ok(result.unwrap());

    }

    pub fn get_blocks_by_height(&self,height:u64) -> Result<Vec<Block::TokenBlock>,&'static str>{
        let index = height % 4;
        let result = self.get_blocks_by_index(index);
        if result.is_err(){
            return Err(result.err().unwrap());
        }
        return Ok(result.unwrap());
    }

    pub fn get_blocks_by_index(&self,index:u64) -> Result<Vec<Block::TokenBlock>,&'static str>{
        if index >= self.config.amount_of_files{
            return Err("Bad index");
        }

        let path = String::from(DERIVATIVE_BLOCKS_DIRECTORY) + 
                        &self.pathname.to_string() + "/" + 
                        &index.to_string();

        let result = decompress_from_file(path);
        if result.is_err(){
            return Err("Could not decompress file");
        }

        let decompressed_data = result.unwrap();
        let size_of_data = decompressed_data.len();
        if size_of_data == 0{
            return Err("Empty file");
        }
        let amount:usize = decompressed_data[0] as usize;

        let mut to_return:Vec<Block::TokenBlock> = Vec::with_capacity(amount);
        let mut offset:usize = 1;
        for _ in 0..amount{
            if size_of_data - offset < 4{
                return Err("Could not parse size of block");
            }
            let mut block_size:u32 = (decompressed_data[offset] as u32)<<24;
            offset += 1;
            block_size += (decompressed_data[offset] as u32)<<16;
            offset += 1;
            block_size += (decompressed_data[offset] as u32)<<8;
            offset += 1;
            block_size += decompressed_data[offset] as u32;
            offset += 1;

            if size_of_data - offset < block_size as usize{
                return Err("Could not parse block");
            }

            let result = Block::TokenBlock::parse(&decompressed_data[offset..offset+block_size as usize],
                                                        block_size);
            
            if result.is_err(){
                return Err("could not parse block");
            }

            offset += block_size as usize;
            to_return.push(result.unwrap());
        }

        return Ok(to_return);
    }

    
}