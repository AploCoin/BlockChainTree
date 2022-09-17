use crate::Errors::*;
use error_stack::{IntoReport, Report, Result, ResultExt};
use num_bigint::BigUint;
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::mem::transmute_copy;
use std::path::Path;

pub fn dump_biguint(number: &BigUint, buffer: &mut Vec<u8>) -> Result<(), ToolsError> {
    let number_bytes: Vec<u8> = number.to_bytes_le();

    let amount_of_bunches: usize = number_bytes.len();
    if amount_of_bunches > 255 {
        return Err(Report::new(ToolsError::BiguintError(
            BiguintErrorKind::DumpError,
        )));
    }

    buffer.push(amount_of_bunches as u8);

    for byte in number_bytes.iter().rev() {
        buffer.push(*byte);
    }

    return Ok(());
}

pub fn load_biguint(data: &[u8]) -> Result<(BigUint, usize), ToolsError> {
    let amount_of_bunches: u8 = data[0];
    let amount_of_bytes: usize = amount_of_bunches as usize; //*4;
    if data.len() < amount_of_bytes {
        return Err(
            Report::new(ToolsError::BiguintError(BiguintErrorKind::LoadError)).attach_printable(
                format!("data = {} // bytes = {}", data.len(), amount_of_bytes),
            ),
        );
    }

    let amount: BigUint = BigUint::from_bytes_be(&data[1..1 + amount_of_bytes]);
    return Ok((amount, amount_of_bytes + 1));
}

pub fn bigint_size(number: &BigUint) -> usize {
    let bits_size: usize = number.bits() as usize;
    let mut amount_byte_size: usize = bits_size / 8;
    if number.bits() % 8 != 0 {
        amount_byte_size += 1;
    }

    return amount_byte_size + 1;
}

pub fn hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result: [u8; 32] = hasher.finalize().as_slice().try_into().unwrap();
    return result;
}

pub fn compress_to_file(output_file: String, data: &[u8]) -> Result<(), ToolsError> {
    let path = Path::new(&output_file);
    let target = File::create(path)
        .report()
        .change_context(ToolsError::ZstdError(ZstdErrorKind::CompressingFileError))?;

    let encoder = zstd::Encoder::new(target, 1)
        .report()
        .change_context(ToolsError::ZstdError(ZstdErrorKind::CompressingFileError))?;

    encoder
        .write_all(data)
        .report()
        .change_context(ToolsError::ZstdError(ZstdErrorKind::CompressingFileError))?;

    encoder
        .finish()
        .report()
        .change_context(ToolsError::ZstdError(ZstdErrorKind::CompressingFileError))?;

    return Ok(());
}

pub fn decompress_from_file(filename: String) -> Result<Vec<u8>, ToolsError> {
    let path = Path::new(&filename);
    let mut decoded_data: Vec<u8> = Vec::new();

    let file = File::open(path)
        .report()
        .attach_printable("Error opening file")
        .change_context(ToolsError::ZstdError(ZstdErrorKind::DecompressingFileError))?;

    let mut decoder = zstd::Decoder::new(file)
        .report()
        .attach_printable("Error creating decoder")
        .change_context(ToolsError::ZstdError(ZstdErrorKind::DecompressingFileError))?;

    let result = decoder
        .read_to_end(&mut decoded_data)
        .report()
        .attach_printable("Error reading file")
        .change_context(ToolsError::ZstdError(ZstdErrorKind::DecompressingFileError));

    return Ok(decoded_data);
}
