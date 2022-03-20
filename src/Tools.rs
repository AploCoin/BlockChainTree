use num_bigint::BigUint;

pub fn count_u64_digits(input:u64)->u64{
    if input < 10000000000 {
        // [10,1]
        if input < 100000 {
            // [5,1]
            if input < 1000 {
                // [3,1]
                if input < 100 {
                    // [2,1]
                    if input < 10 {
                        return 1;
                    }
                    else {
                        return 2;
                    }
                }
                else {
                    return 3;
                }
            }
            else {
                if input < 10000 {
                    return 4;
                }
                else {
                    return 5;
                }

            }
        }
        else {
            // [10,6]
            if input < 100000000 {
                // [8,6]
                if input < 10000000 {
                    // [7,6]
                    if input < 1000000 {
                        return 6;
                    }
                    else {
                        return 7;
                    }
                }
                else {
                    return 8;
                }
            }
            else {
                if input < 1000000000 {
                    return 9;
                }
                else {
                    return 10;
                }
            }
        }
    }
    else {
        // [20,11]
        if input < 1000000000000000 {
            // [15,11]
            if input < 10000000000000 {
                // [13,11]
                if input < 1000000000000 {
                    if input < 100000000000 {
                        return 11;
                    }
                    else {
                        return 12;
                    }
                }
                else {
                    return 13;
                }
            }
            else {
                //[15,14]
                if input < 100000000000000 {
                    return 14;
                }
                else {
                    return 15;
                }
            }
        }
        else {
            // [20,16]
            if input < 1000000000000000000 {
                // [18,16]
                if input < 100000000000000000 {
                    // [17,16]
                    if input < 10000000000000000 {
                        return 16;
                    }
                    else {
                        return 17;
                    }
                }
                else {
                    return 18;
                }
            }
            else {
                // [20,19]
                if input < 10000000000000000000 {
                    return 19;
                }
                else {
                    return 20;
                }
            }
        }
    }
}


pub fn dump_biguint(number:&BigUint,buffer:&mut Vec<u8>)->Result<(),&'static str>{
    let mut number_bytes:Vec<u8> = number.to_bytes_le();

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