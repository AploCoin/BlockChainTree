use num_bigint::BigUint;
use sha2::{Sha256, Digest};
use std::convert::TryInto;


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