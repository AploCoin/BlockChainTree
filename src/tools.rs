use crate::errors::*;
use error_stack::{IntoReport, Report, Result, ResultExt};
use num_bigint::BigUint;
use primitive_types::U256;
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::mem::transmute;
use std::path::Path;
use std::{fs, io};

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn dump_u256(number: &U256, buffer: &mut Vec<u8>) -> Result<(), ToolsError> {
    buffer.push(0);
    let ind = buffer.len() - 1;

    let mut found_non_null = false;
    let mut counter: u8 = 0;

    for num in number.0.iter().rev() {
        let bytes = unsafe { transmute::<u64, [u8; 8]>(num.to_be()) };
        for byte in bytes {
            if found_non_null {
                buffer.push(byte);
                counter += 1;
            } else {
                if byte != 0 {
                    buffer.push(byte);
                    counter += 1;
                    found_non_null = true;
                }
            }
        }
    }

    unsafe { *buffer.get_unchecked_mut(ind) = counter };

    Ok(())
}

pub fn load_u256(data: &[u8]) -> Result<(U256, usize), ToolsError> {
    let amount_of_bytes: usize = data[0] as usize;

    if amount_of_bytes > 32 {
        return Err(Report::new(ToolsError::Biguint(BiguintErrorKind::Dump)));
    }

    if data.len() < amount_of_bytes {
        return Err(
            Report::new(ToolsError::Biguint(BiguintErrorKind::Load)).attach_printable(format!(
                "data = {} // bytes = {}",
                data.len(),
                amount_of_bytes
            )),
        );
    }

    Ok((
        U256::from_big_endian(&data[1..1 + amount_of_bytes]),
        amount_of_bytes,
    ))
}

pub fn u256_size(number: &U256) -> usize {
    let bits_size: usize = number.bits();
    if bits_size == 0 {
        return 2;
    }
    let mut amount_byte_size: usize = bits_size / 8;
    if number.bits() % 8 != 0 {
        amount_byte_size += 1;
    }

    amount_byte_size + 1
}

pub fn dump_biguint(number: &BigUint, buffer: &mut Vec<u8>) -> Result<(), ToolsError> {
    let number_bytes: Vec<u8> = number.to_bytes_le();

    let amount_of_bunches: usize = number_bytes.len();
    if amount_of_bunches > 255 {
        return Err(Report::new(ToolsError::Biguint(BiguintErrorKind::Dump)));
    }

    buffer.push(amount_of_bunches as u8);

    for byte in number_bytes.iter().rev() {
        buffer.push(*byte);
    }

    Ok(())
}

pub fn load_biguint(data: &[u8]) -> Result<(BigUint, usize), ToolsError> {
    let amount_of_bunches: u8 = data[0];
    let amount_of_bytes: usize = amount_of_bunches as usize; //*4;
    if data.len() < amount_of_bytes {
        return Err(
            Report::new(ToolsError::Biguint(BiguintErrorKind::Load)).attach_printable(format!(
                "data = {} // bytes = {}",
                data.len(),
                amount_of_bytes
            )),
        );
    }

    let amount: BigUint = BigUint::from_bytes_be(&data[1..1 + amount_of_bytes]);

    Ok((amount, amount_of_bytes + 1))
}

pub fn bigint_size(number: &BigUint) -> usize {
    let bits_size: usize = number.bits() as usize;
    if bits_size == 0 {
        return 2;
    }
    let mut amount_byte_size: usize = bits_size / 8;
    if number.bits() % 8 != 0 {
        amount_byte_size += 1;
    }

    amount_byte_size + 1
}

pub fn hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().as_slice().try_into().unwrap()
}

pub fn compress_to_file(output_file: String, data: &[u8]) -> Result<(), ToolsError> {
    let path = Path::new(&output_file);
    let target = File::create(path)
        .into_report()
        .change_context(ToolsError::Zstd(ZstdErrorKind::CompressingFile))?;

    let mut encoder = zstd::Encoder::new(target, 1)
        .into_report()
        .change_context(ToolsError::Zstd(ZstdErrorKind::CompressingFile))?;

    encoder
        .write_all(data)
        .into_report()
        .change_context(ToolsError::Zstd(ZstdErrorKind::CompressingFile))?;

    encoder
        .finish()
        .into_report()
        .change_context(ToolsError::Zstd(ZstdErrorKind::CompressingFile))?;

    Ok(())
}

pub fn decompress_from_file(filename: String) -> Result<Vec<u8>, ToolsError> {
    let path = Path::new(&filename);
    let mut decoded_data: Vec<u8> = Vec::new();

    let file = File::open(path)
        .into_report()
        .attach_printable("Error opening file")
        .change_context(ToolsError::Zstd(ZstdErrorKind::DecompressingFile))?;

    let mut decoder = zstd::Decoder::new(file)
        .into_report()
        .attach_printable("Error creating decoder")
        .change_context(ToolsError::Zstd(ZstdErrorKind::DecompressingFile))?;

    decoder
        .read_to_end(&mut decoded_data)
        .into_report()
        .attach_printable("Error reading file")
        .change_context(ToolsError::Zstd(ZstdErrorKind::DecompressingFile))?;

    Ok(decoded_data)
}

pub fn check_pow(prev_hash: &[u8; 32], difficulty: &[u8; 32], pow: &[u8]) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash);
    hasher.update(pow);
    let result: [u8; 32] = unsafe { hasher.finalize().as_slice().try_into().unwrap_unchecked() };
    let result: [u64; 4] = unsafe { transmute(result) };

    let difficulty: &[u64; 4] = unsafe { transmute(difficulty) };

    //println!("difficulty: {:?}", difficulty);

    for (r, d) in result.iter().zip(difficulty) {
        match r.cmp(d) {
            std::cmp::Ordering::Less => {
                return true;
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use primitive_types::U256;

    use super::{dump_u256, load_u256};

    #[test]
    fn dump_load_u256() {
        let mut dump: Vec<u8> = Vec::new();

        println!(
            "{:?}",
            U256::from_dec_str("10000000000000000000001000000001")
        );

        dump_u256(
            &U256::from_dec_str("10000000000000000000001000000001").unwrap(),
            &mut dump,
        )
        .unwrap();

        let num = load_u256(&dump).unwrap();

        assert_eq!(
            U256::from_dec_str("10000000000000000000001000000001").unwrap(),
            num.0
        );
    }
}
