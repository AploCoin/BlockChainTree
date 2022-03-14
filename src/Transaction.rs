use sha2::{Sha256, Digest};
use num_bigint::BigUint;
use std::convert::TryInto;
use base64;
use rsa::{RsaPublicKey, pkcs1::FromRsaPublicKey, PaddingScheme, hash::Hash::SHA2_256};
use rsa::PublicKey;
use std::mem::transmute;
use crate::Tools;

#[derive(Debug)]
pub struct Transaction{
    sender:String,
    receiver:String,
    timestamp:u64,
    signature:String,
    amount:BigUint
}

impl Transaction{
    pub fn new(sender:String,
               receiver:String,
               timestamp:u64,
               signature:String,
               amount:BigUint)->Transaction{
        let transaction:Transaction = Transaction{
            sender:sender,
            receiver:receiver,
            timestamp:timestamp,
            signature:signature,
            amount:amount
        };
        return transaction;
    }

    pub fn decode_current_owner(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.sender);
    }
    pub fn decode_previous_owner(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.receiver);
    }
    pub fn decode_signature(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.signature);
    }

    pub fn hash(&self,prev_hash:&[u8;32])->Box<[u8;32]>{ 
        let mut hasher = Sha256::new();

        let amount_as_bytes = self.amount.to_bytes_be();
        let calculated_size:usize = 32
                                    +self.sender.len()
                                    +self.receiver.len()
                                    +8
                                    +amount_as_bytes.len();
        let mut concatenated_input:Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in prev_hash.iter(){
            concatenated_input.push(*byte);
        }
        for byte in self.sender.as_bytes().iter(){
            concatenated_input.push(*byte);
        }
        for byte in self.receiver.as_bytes().iter(){
            concatenated_input.push(*byte);
        }
        for byte in self.signature.as_bytes().iter(){
            concatenated_input.push(*byte);
        }
        for byte in self.timestamp.to_be_bytes().iter(){
            concatenated_input.push(*byte);
        }
        for byte in amount_as_bytes.iter(){
            concatenated_input.push(*byte);
        }
        
        hasher.update(concatenated_input);
        let result:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();
        let to_return = Box::new(result);
        
        return to_return;
    }

    pub fn verify(&self,prev_hash:&[u8;32]) -> Result<bool,&'static str>{
        let signed_data_hash:Box<[u8;32]> = self.hash(prev_hash);

        let res = base64::decode(&self.signature);
        let decoded_signature:Vec<u8>;
        match res{
            Err(_) => {return Err(&"Signature decoding error")},
            Ok(r) => {decoded_signature = r;}
        }

        let res = base64::decode(&self.sender);
        let decoded_sender:Vec<u8>;
        match res{
            Err(_) => {return Err(&"Sender decoding error")},
            Ok(r) => {decoded_sender = r;}
        }

        let sender_public = RsaPublicKey::from_pkcs1_der(&decoded_sender);
        let decoded_sender_public:RsaPublicKey;
        match sender_public{
            Err(_) => {return Err(&"Public key decoding error")},
            Ok(key) => {decoded_sender_public = key;}
        }
        let padding_scheme = PaddingScheme::new_pkcs1v15_sign(Some(SHA2_256));
        let res = decoded_sender_public.verify(padding_scheme, 
                                    signed_data_hash.as_ref(), 
                                    &decoded_signature);
        match res{
            Err(_) => {return Ok(false);}
            Ok(_) => {return Ok(true);}
        }

    }


    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        let timestamp_as_bytes:[u8;8] = unsafe{transmute(self.timestamp.to_be())};

        let calculated_size:usize = self.get_dump_size();

        println!("{:?}",calculated_size);
        let mut transaction_dump:Vec<u8> = Vec::with_capacity(calculated_size);
        
        // header
        transaction_dump.push(0);

        // sender
        for byte in self.sender.as_bytes().iter(){
            transaction_dump.push(*byte);
        }
        transaction_dump.push(0);

        // receiver
        for byte in self.receiver.as_bytes().iter(){
            transaction_dump.push(*byte);
        }
        transaction_dump.push(0);

        // timestamp
        transaction_dump.extend(timestamp_as_bytes.iter());

        // signature
        for byte in self.signature.as_bytes().iter(){
            transaction_dump.push(*byte);
        }
        transaction_dump.push(0);
        
        // amount
        let res = Tools::dump_biguint(&self.amount, &mut transaction_dump);
        match res{
            Err(e)=>{return Err(e)}
            Ok(_)=>{}
        }
        
        return Ok(transaction_dump);
    }

    pub fn get_dump_size(&self) -> usize{
        let calculated_size:usize = 1
                                +self.sender.len()
                                +1
                                +self.receiver.len()
                                +1
                                +self.signature.len()
                                +1
                                +Tools::bigint_size(&self.amount)
                                +1
                                +8;
        return calculated_size;
    }

    pub fn parse_transaction(data:&[u8],transaction_size:u64) -> Result<Transaction,&'static str>{
        let mut index:usize = 0;

        // parsing sender address
        let mut new_index:usize = index;
        while data[new_index] != 0{
            new_index += 1;
            if new_index >= transaction_size as usize{
                return Err(&"Can't find ending \\x00 for sender field");
            }
        }     
        if new_index-index == 0{
            return Err("No sender address found");
        }
        let mut sender:String = String::with_capacity(new_index-index);
        for i in index..new_index{
            sender.push(data[i] as char);
        }
        index = new_index+1;

        // parsing receiver address
        let mut new_index:usize = index;
        while data[new_index] != 0{
            new_index += 1;
            if new_index >= transaction_size as usize{
                return Err(&"Can't find ending \\x00 for receiver field");
            }
        }   
        if new_index-index == 0{
            return Err("No reciever address found");
        }    
        let mut receiver:String = String::with_capacity(new_index-index);
        for i in index..new_index{
            receiver.push(data[i] as char);
        }
        index = new_index+1;

        // parsing timestamp
        new_index = index+8;
        let mut timestamp:u64 = 0;
        if new_index-index<8{
            return Err(&"Can't parse timestamp");
        }else{
            timestamp = u64::from_be_bytes(data[index..new_index].try_into().unwrap());
        }
        index = new_index;

        // parsing signature
        while data[new_index] != 0{
            new_index += 1;
            if new_index >= transaction_size as usize{
                return Err(&"Can't find ending \\x00 for receiver field");
            }
        } 
        if new_index-index == 0{
            return Err("No signature found");
        }    
        let mut signature:String = String::with_capacity(new_index-index);
        for i in index..new_index{
            signature.push(data[i] as char);
        }
        index = new_index+1;

        // parsing amount
        let res = Tools::load_biguint(&data[index..]);
        let amount:BigUint;
        match res{
            Err(e)=>{return Err(e);}
            Ok(a) => {amount = a.0;}
        }
        let transaction:Transaction = Transaction::new(
                                                    sender,
                                                    receiver,
                                                    timestamp,
                                                    signature,
                                                    amount);

        return Ok(transaction);
    }
}