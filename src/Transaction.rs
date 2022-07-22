use sha2::{Sha256, Digest};
use num_bigint::BigUint;
use std::convert::TryInto;
use std::mem::transmute_copy;
use crate::Tools;

use secp256k1::{Secp256k1, Message};
use secp256k1::PublicKey;
use secp256k1::ecdsa::Signature;
use secp256k1::hashes::sha256;
use crate::DumpHeaders::Headers;

#[derive(Debug)]
pub struct Transaction{
    sender:[u8;33],
    receiver:[u8;33],
    timestamp:u64,
    signature:[u8;64],
    amount:BigUint
}

impl Transaction{
    pub fn new(sender:&[u8;33],
               receiver:&[u8;33],
               timestamp:u64,
               signature:&[u8;64],
               amount:BigUint)->Transaction{
        let transaction:Transaction = Transaction{
            sender:*sender,
            receiver:*receiver,
            timestamp:timestamp,
            signature:*signature,
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
                                    +33
                                    +33
                                    +8
                                    +amount_as_bytes.len();
        
        let mut concatenated_input:Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in prev_hash.iter(){
            concatenated_input.push(*byte);
        }
        for byte in self.sender.iter(){
            concatenated_input.push(*byte);
        }
        for byte in self.receiver.iter(){
            concatenated_input.push(*byte);
        }
        for byte in self.signature.iter(){
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

        // load sender
        let result = PublicKey::from_slice(&self.sender);
        if result.is_err(){
            return Err("Error loading sender");
        }
        let sender = result.unwrap();

        // creating verifier
        let verifier = Secp256k1::verification_only();

        // load message
        let result = Message::from_slice(Box::leak(signed_data_hash));
        if result.is_err(){
            return Err("Error loading message");
        }
        let message = result.unwrap();

        // load signature
        let result = Signature::from_compact(&self.signature);
        if result.is_err(){
            return Err("Error loading signature");
        }
        let signature = result.unwrap();

        // verifying hashed data with public key
        let result = verifier.verify_ecdsa(&message,
                                            &signature,
                                            &sender);
        match result{
            Err(_) => {return Ok(false);}
            Ok(_) => {return Ok(true);}
        }

    }


    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        let timestamp_as_bytes:[u8;8] = self.timestamp.to_be_bytes();

        let calculated_size:usize = self.get_dump_size();

        let mut transaction_dump:Vec<u8> = Vec::with_capacity(calculated_size);
        
        // header
        transaction_dump.push(Headers::Transaction as u8);

        // sender
        for byte in self.sender.iter(){
            transaction_dump.push(*byte);
        }

        // receiver
        for byte in self.receiver.iter(){
            transaction_dump.push(*byte);
        }

        // timestamp
        transaction_dump.extend(timestamp_as_bytes.iter());

        // signature
        for byte in self.signature.iter(){
            transaction_dump.push(*byte);
        }
        
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
                                +33
                                +33
                                +8
                                +64
                                +Tools::bigint_size(&self.amount);
        return calculated_size;
    }

    pub fn parse_transaction(data:&[u8],transaction_size:u64) -> Result<Transaction,&'static str>{
        let mut index:usize = 0;

        if data.len() <= 138{
            return Err("Bad transaction size");
        }

        // parsing sender address
        let sender:[u8;33] = unsafe{transmute_copy(&data[index])};
        index += 33;

        // parsing receiver address
        let receiver:[u8;33] = unsafe{transmute_copy(&data[index])};
        index += 33;

        // parsing timestamp
        let timestamp:u64 = u64::from_be_bytes(data[index..index+8].try_into().unwrap());
        index += 8;

        // parsing signature
        let signature:[u8;64] = unsafe{transmute_copy(&data[index])};
        index += 64;


        // parsing amount
        let res = Tools::load_biguint(&data[index..]);
        let amount:BigUint;
        match res{
            Err(e)=>{return Err(e);}
            Ok(a) => {amount = a.0; 
                    index += a.1;}
        }
        if index != transaction_size as usize{
            return Err("Error parsing transaction")
        }

        let transaction:Transaction = Transaction::new(
                                                    &sender,
                                                    &receiver,
                                                    timestamp,
                                                    &signature,
                                                    amount);

        return Ok(transaction);
    }

    pub fn get_sender(&self) -> &[u8;33]{
        return &self.sender;
    }
    
    pub fn get_receiver(&self) -> &[u8;33]{
        return &self.receiver;
    }

    pub fn get_timestamp(&self) -> u64{
        return self.timestamp;
    }

    pub fn get_signature(&self) -> &[u8;64]{
        return &self.signature;
    }

    pub fn get_amount(&self) -> &BigUint{
        return &self.amount;
    }

}