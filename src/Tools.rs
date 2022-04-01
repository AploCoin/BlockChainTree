use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use std::convert::TryInto;
use std::mem::transmute_copy;
use zstd;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::io::Read;


pub fn dump_biguint(number:&BigUint,buffer:&mut Vec<u8>)->Result<(),&'static str>{
    let number_bytes:Vec<u8> = number.to_bytes_le();

    let amount_of_bunches:usize = number_bytes.len();
    if amount_of_bunches>255{
        return Err(&"Too big number");
    }

    buffer.push(amount_of_bunches as u8);


    for byte in number_bytes.iter().rev(){
        buffer.push(*byte);
    }


    return Ok(());
}

pub fn load_biguint(data:&[u8]) -> Result<(BigUint,usize),&'static str>{
    let amount_of_bunches:u8 = data[0];
    let amount_of_bytes:usize = amount_of_bunches as usize;//*4;
    if data.len()<amount_of_bytes{
        return Err(&"Wrong amount of bunches");
    }

    let amount:BigUint = BigUint::from_bytes_be(&data[1..1+amount_of_bytes]);
    return Ok((amount,amount_of_bytes+1));
}

pub fn bigint_size(number:&BigUint) -> usize{
    let bits_size:usize = number.bits() as usize;
    let mut amount_byte_size:usize = bits_size/8;
    if number.bits()%8 !=0{
        amount_byte_size += 1;
    }

    return amount_byte_size;
}


pub fn hash(data:&[u8]) -> [u8;32]{
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result:[u8;32] = hasher.finalize().as_slice().try_into().unwrap();
    return result;
}


pub fn compress_to_file(output_file:String,data:&[u8])->Result<(),&'static str>{
    let path = Path::new(&output_file);
    let result = File::create(path);
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
    let path = Path::new(&filename);
    let mut decoded_data:Vec<u8> = Vec::new();

    let result = File::open(path);
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