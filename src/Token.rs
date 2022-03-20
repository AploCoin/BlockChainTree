use num_bigint::BigUint;
use num_traits::identities::Zero;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use secp256k1::{Secp256k1, Message};
use secp256k1::PublicKey;
use secp256k1::ecdsa::Signature;
use secp256k1::hashes::sha256;
use crate::Tools;
use std::mem::transmute_copy;




/*
    Token dumping protocol

    Header - 1 byte

*/

#[derive(Debug)]
pub struct Token{
    current_owner:[u8;33],

    signature:[u8;64],
    token_hash:[u8;32],

    token_data:String,
    smol_contract:String,

    coin_supply:BigUint,
    transfer_fee:BigUint,

    assigned:bool
}


impl Token{
    pub fn new(current_owner:[u8;33],
                signature:[u8;64],
                token_hash:[u8;32],
                token_data:String,
                smol_contract:String,
                coin_supply:BigUint,
                transfer_fee:BigUint,
                assigned:bool)->Result<Token,&'static str>{
        
        if !assigned{
            if token_data.len() != 0{
                return Err("Token data is already set");
            }
            if !coin_supply.is_zero(){
                return Err("Coin supply is already set");
            }
            if smol_contract.len() != 0{
                return Err("Token is not assigned, but contract is set");
            }
            if !transfer_fee.is_zero(){
                return Err("Token is not assigned, but fee is set");
            }
        }

        let token:Token = Token{
            current_owner: current_owner,
            signature: signature,
            token_hash: token_hash,
            token_data: token_data,
            smol_contract: smol_contract,
            coin_supply: coin_supply,
            transfer_fee: transfer_fee,
            assigned: assigned
        };

        return Ok(token);
    }
    pub fn is_fee_static(&self)->bool{
        if self.transfer_fee.is_zero(){
            return false
        }else{
            return true;
        }
    }

    pub fn decode_current_owner(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.current_owner);
    }
    pub fn decode_signature(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.signature);
    }
    pub fn decode_token_data(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.token_data);
    }
    pub fn decode_smol_contract(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.smol_contract);
    }

    pub fn hash(&self)->Box<[u8;32]>{
        // converting BigUInts -> string -> bytes
        let coin_supply_as_string:String = self.coin_supply.to_str_radix(10);
        let coin_supply_as_bytes:&[u8] = coin_supply_as_string.as_bytes();

        let transfer_fee_as_string:String = self.transfer_fee.to_str_radix(10);
        let transfer_fee_as_bytes:&[u8] = transfer_fee_as_string.as_bytes();

        // getting overall size
        let mut concatenated_size:u64 = 1;

        concatenated_size += self.current_owner.len() as u64
                            + self.token_data.len() as u64
                            + self.smol_contract.len() as u64
                            + coin_supply_as_bytes.len() as u64
                            + transfer_fee_as_bytes.len() as u64
                            + self.signature.len() as u64
                            + self.token_hash.len() as u64;
        
        // getting concatenated input

        let mut concatenated:Vec<u8> = Vec::with_capacity(concatenated_size as usize);
        
        concatenated.push(self.assigned as u8);

        for byte in self.current_owner.iter(){
            concatenated.push(*byte);
        }
        for byte in self.token_data.as_bytes().iter(){
            concatenated.push(*byte);
        }
        for byte in self.smol_contract.as_bytes().iter(){
            concatenated.push(*byte);
        }
        for byte in coin_supply_as_bytes{
            concatenated.push(*byte);
        }
        for byte in transfer_fee_as_bytes{
            concatenated.push(*byte);
        }
        for byte in self.signature.iter(){
            concatenated.push(*byte);
        }
        for byte in self.token_hash.iter(){
            concatenated.push(*byte);
        }

        let mut hasher = Sha256::new();
        hasher.update(concatenated);
        let result:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();
        return Box::new(result);
    }

    pub fn get_dump_size(&self) -> usize{
        let size:usize = 1 // header
                        +1 // is assigned
                        +33 // current owner
                        +64 // signature
                        +32 // token hash
                        +self.token_data.len()
                        +1
                        +self.smol_contract.len()
                        +1
                        +Tools::bigint_size(&self.coin_supply)
                        +1
                        +Tools::bigint_size(&self.transfer_fee)
                        +1;

        return size;
    }

    pub fn verify(&self) -> Result<bool,&'static str>{
        // converting BigUInts -> string -> bytes
        let coin_supply_as_string:String = self.coin_supply.to_str_radix(10);
        let coin_supply_as_bytes:&[u8] = coin_supply_as_string.as_bytes();

        let transfer_fee_as_string:String = self.transfer_fee.to_str_radix(10);
        let transfer_fee_as_bytes:&[u8] = transfer_fee_as_string.as_bytes();

        // getting overall size
        let mut concatenated_size:u64 = 1;

        concatenated_size += self.current_owner.len() as u64
                            + self.token_data.len() as u64
                            + self.smol_contract.len() as u64
                            + coin_supply_as_bytes.len() as u64
                            + transfer_fee_as_bytes.len() as u64
                            + self.token_hash.len() as u64;
        
        // concatenating data to be hashed
        let mut concatenated:Vec<u8> = Vec::with_capacity(concatenated_size as usize);
        
        concatenated.push(self.assigned as u8);

        for byte in self.current_owner.iter(){
            concatenated.push(*byte);
        }
        for byte in self.token_data.as_bytes().iter(){
            concatenated.push(*byte);
        }
        for byte in self.smol_contract.as_bytes().iter(){
            concatenated.push(*byte);
        }
        for byte in coin_supply_as_bytes{
            concatenated.push(*byte);
        }
        for byte in transfer_fee_as_bytes{
            concatenated.push(*byte);
        }
        for byte in self.token_hash.iter(){
            concatenated.push(*byte);
        }

        // getting hash of data
        let mut hasher = Sha256::new();
        hasher.update(concatenated);
        let signed_data:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();

        // creating verifier
        let verifier = Secp256k1::verification_only(); 
        
        // loading message
        let result = Message::from_slice(&signed_data);
        if result.is_err(){
            return Err("Error loading message");
        }
        let message = result.unwrap();

        // loading public key
        let result = PublicKey::from_slice(&self.current_owner);
        if result.is_err(){
            return Err("Error loading public key");
        }
        let public_key = result.unwrap();
        
        // load signature
        let result = Signature::from_compact(&self.signature);
        if result.is_err(){
            return Err("Error loading signature");
        }
        let signature = result.unwrap();


        // verifying hashed data with public key
        let result = verifier.verify_ecdsa(&message,
                                            &signature,
                                            &public_key);

        match result{
            Err(_) => {return Ok(false);}
            Ok(_) => {return Ok(true);}
        }
    }

    pub fn dump(&self) -> Result<Vec<u8>,&'static str>{
        let calculated_size:usize = self.get_dump_size();
        
        let mut dumped_token:Vec<u8> = Vec::with_capacity(calculated_size);

        dumped_token.push(1);//header

        // dump current owner
        for byte in self.current_owner.iter(){
            dumped_token.push(*byte);
        }

        // dump token hash
        for byte in self.token_hash.iter(){
            dumped_token.push(*byte);
        }

        // dump signature
        for byte in self.signature.iter(){
            dumped_token.push(*byte)
        }

        // dump assigned/token data, small contract
        if !self.assigned{
            dumped_token.push(0);
        }else{
            dumped_token.push(1);
            
            for byte in self.token_data.as_bytes().iter(){
                dumped_token.push(*byte);
            }
            dumped_token.push(0);

            for byte in self.smol_contract.as_bytes().iter(){
                dumped_token.push(*byte);
            }
            dumped_token.push(0);

            let res = Tools::dump_biguint(&self.transfer_fee, &mut dumped_token);
            match res{
                Err(e)=>{return Err(e)}
                Ok(_)=>{}
            }
            
            let res = Tools::dump_biguint(&self.coin_supply, &mut dumped_token);
            match res{
                Err(e)=>{return Err(e)}
                Ok(_)=>{}
            }
        }

        return Ok(dumped_token);
    }

    // pub fn parse_token(data:&[u8],token_size:u64) -> Result<Token,&'static str>{
    //     let mut index:usize = 0;

    //     if data.len() <= 131{
    //         return Err("Could not parse token");
    //     }

    //     //parsing current owner address
    //     let current_owner:[u8;33] = unsafe{transmute_copy(&data[index])};
    //     index += 33;

    //     //parsing token hash
    //     let token_hash:[u8;32] = unsafe{transmute_copy(&data[index])};
    //     index += 32;

    //     //parsing signature
    //     let signature:[u8;64] = unsafe{transmute_copy(&data[index])};
    //     index += 64;



    //     let mut assigned = false;
    //     if data[index] == 1{
    //         //assigned
    //         assigned = true;

    //         index += 1;
    //         new_index = index;

    //         //parsing token data
    //         while data[new_index] != 0{
    //             new_index += 1;
    //             if new_index >= token_size as usize{
    //                 return Err(&"Could not find end of data");
    //             }
    //         }

    //         if index == new_index{
    //             return Err(&"data not found");
    //         }

    //         let mut token_data:String = String::with_capacity(new_index-index);
    //         for i in index..new_index{
    //             token_data.push(data[i] as char);
    //         }
    //         new_index += 1;
    //         index = new_index;

    //         //parsing contract
    //         while data[new_index] != 0{
    //             new_index += 1;
    //             if new_index >= token_size as usize{
    //                 return Err(&"Could not find an end of contract");
    //             }
    //         }
    //         let mut contract:String = String::with_capacity(new_index-index);
    //         for i in index..new_index{
    //             contract.push(data[i] as char);
    //         }
    //         new_index += 1;
    //         index = new_index;

    //         //parsing transfer fee
    //         let res = Tools::load_biguint(&data[index..]);
    //         let transfer_fee:BigUint;
    //         match res{
    //             Err(e) =>{return Err(e)}
    //             Ok(a) => {
    //                 transfer_fee = a.0;
    //                 index += a.1;
    //             }
    //         }

    //         //parsing coin supply
    //         let res = Tools::load_biguint(&data[index..]);
    //         let coin_supply:BigUint;
    //         match res{
    //             Err(e) =>{return Err(e)}
    //             Ok(a) => {
    //                 coin_supply = a.0;
    //                 index += a.1;
    //             }
    //         }

    //         if index != token_size as usize{
    //             return Err(&"Wrong size of token");
    //         }

    //         let token_res = Token::new(current_owner,
    //                                     signature,
    //                                     token_hash,
    //                                     token_data,
    //                                     contract,
    //                                     coin_supply,
    //                                     transfer_fee,
    //                                     assigned);
    //         let token:Token;
    //         match token_res{
    //             Err(e) => {return Err(e)}
    //             Ok(a) => {token = a}
    //         }
            
    //         return Ok(token);
    //     }

    //     index += 1;
    //     if index != token_size as usize{
    //         return Err(&"Wrong size of token");
    //     }

    //     let token_res = Token::new(current_owner,
    //                                 signature,
    //                                 token_hash,
    //                                 String::with_capacity(0),
    //                                 String::with_capacity(0),
    //                                 BigUint::zero(),
    //                                 BigUint::zero(),
    //                                 assigned);
    //     let token:Token;
    //     match token_res{
    //         Err(e)=>{return Err(e)}
    //         Ok(a) => {token = a} 
    //     }

    //     return Ok(token);
    // }
}



#[derive(Debug)]
pub struct TokenAction{
    action:Action,
    current_owner:String,
    previous_owner:String,
    signature:String,
    token_hash:[u8;32],
    timestamp:u64
}

#[derive(Debug,Copy, Clone)]
pub enum Action{
    Send = 1,
    Assign,
    Burn
}

impl TokenAction{
    pub fn new(action:Action,
                current_owner:String,
                previous_owner:String,
                signature:String,
                token_hash:[u8;32],
                timestamp:u64) -> TokenAction{

        return TokenAction{
                        action:action,
                        current_owner:current_owner,
                        previous_owner:previous_owner,
                        signature:signature,
                        token_hash:token_hash,
                        timestamp:timestamp};
    }

    pub fn decode_current_owner(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.current_owner);
    }
    pub fn decode_previous_owner(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.previous_owner);
    }
    pub fn decode_signature(&self) -> Result<Vec<u8>, base64::DecodeError>{
        return base64::decode(&self.signature);
    }

    pub fn hash(&self,prev_hash:&[u8;32])->Box<[u8;32]>{
        let concatenated_size:usize = 32
                                +self.current_owner.len()
                                +self.previous_owner.len()
                                +32
                                +1;
        
        let mut concatenated:Vec<u8> = Vec::with_capacity(concatenated_size);

        for byte in prev_hash.iter(){
            concatenated.push(*byte);
        }
        for byte in self.current_owner.as_bytes().iter(){
            concatenated.push(*byte);
        }
        for byte in self.previous_owner.as_bytes().iter(){
            concatenated.push(*byte);
        }
        for byte in self.token_hash.iter(){
            concatenated.push(*byte);
        }
        concatenated.push(self.action as u8);

        let mut hasher = Sha256::new();
        hasher.update(concatenated);
        let result:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();
        return Box::new(result);
    }

    // pub fn verify(&self,prev_hash:&[u8;32])->Result<bool,&'static str>{
    //     let signed_data_hash = self.hash(prev_hash);
        
    //     let res = base64::decode(&self.signature);
    //     let decoded_signature:Vec<u8>;
    //     match res{
    //         Err(_) => {return Err(&"Signature decoding error")},
    //         Ok(r) => {decoded_signature = r;}
    //     }

    //     let res = base64::decode(&self.previous_owner);
    //     let decoded_sender:Vec<u8>;
    //     match res{
    //         Err(_) => {return Err(&"Sender decoding error")},
    //         Ok(r) => {decoded_sender = r;}
    //     }


    //     let sender_public = RsaPublicKey::from_pkcs1_der(&decoded_sender);
    //     let decoded_sender_public:RsaPublicKey;
    //     match sender_public{
    //         Err(_) => {return Err(&"Public key decoding error")},
    //         Ok(key) => {decoded_sender_public = key;}
    //     }
    //     let padding_scheme = PaddingScheme::new_pkcs1v15_sign(Some(SHA2_256));
    //     let res = decoded_sender_public.verify(padding_scheme, 
    //                                 signed_data_hash.as_ref(), 
    //                                 &decoded_signature);
    //     match res{
    //         Err(_) => {return Ok(false);}
    //         Ok(_) => {return Ok(true);}
    //     }
    // }
    pub fn get_dump_size(&self)->usize{
        return 0;
    }
    pub fn dump(&self)->Result<Vec<u8>,&'static str>{
        return Err("Not implemented yet");
    }
    pub fn parse(data:&[u8],token_size:u64)->Result<TokenAction,&'static str>{
        return Err("Not implemented yet");
    }
}