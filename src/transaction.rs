use crate::errors::*;
use crate::tools;
use num_bigint::BigUint;
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::fmt::Debug;
use std::mem::transmute_copy;

use crate::dump_headers::Headers;
use secp256k1::ecdsa::Signature;
use secp256k1::hashes::sha256;
use secp256k1::PublicKey;
use secp256k1::{Message, Secp256k1, SecretKey};

use error_stack::{IntoReport, Report, Result, ResultExt};

pub trait Transactionable: Debug {
    fn hash(&self, prev_hash: &[u8; 32]) -> [u8; 32];
    fn hash_without_signature(&self, prev_hash: &[u8; 32]) -> Box<[u8; 32]>;

    fn verify(&self, prev_hash: &[u8; 32]) -> Result<bool, TransactionError>;

    fn dump(&self) -> Result<Vec<u8>, TransactionError>;
    fn get_dump_size(&self) -> usize;

    fn parse(data: &[u8], size: u64) -> Result<Self, TransactionError>
    where
        Self: Sized;

    fn get_sender(&self) -> &[u8; 33];
    fn get_receiver(&self) -> &[u8; 33];
    fn get_timestamp(&self) -> u64;
    fn get_signature(&self) -> &[u8; 64];
    fn sign(
        &mut self,
        prev_hash: &[u8; 32],
        private_key: &[u8; 32],
    ) -> Result<(), TransactionError>;
}

#[derive(Debug)]
pub struct Transaction {
    sender: [u8; 33],
    receiver: [u8; 33],
    timestamp: u64,
    signature: [u8; 64],
    amount: BigUint,
}

impl Transaction {
    pub fn new(
        sender: &[u8; 33],
        receiver: &[u8; 33],
        timestamp: u64,
        signature: &[u8; 64],
        amount: BigUint,
    ) -> Transaction {
        let transaction: Transaction = Transaction {
            sender: *sender,
            receiver: *receiver,
            timestamp,
            signature: *signature,
            amount,
        };
        return transaction;
    }

    pub fn get_amount(&self) -> &BigUint {
        return &self.amount;
    }
}

impl Transactionable for Transaction {
    fn hash(&self, prev_hash: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha256::new();

        let amount_as_bytes = self.amount.to_bytes_be();
        let calculated_size: usize = 32 + 33 + 33 + 8 + amount_as_bytes.len();

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in prev_hash.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.signature.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }
        for byte in amount_as_bytes.iter() {
            concatenated_input.push(*byte);
        }

        hasher.update(concatenated_input);
        hasher.finalize().as_slice().try_into().unwrap()
    }

    fn hash_without_signature(&self, prev_hash: &[u8; 32]) -> Box<[u8; 32]> {
        let mut hasher = Sha256::new();

        let amount_as_bytes = self.amount.to_bytes_be();
        let calculated_size: usize = 32 + 33 + 33 + 8 + amount_as_bytes.len();

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in prev_hash.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }
        for byte in amount_as_bytes.iter() {
            concatenated_input.push(*byte);
        }

        hasher.update(concatenated_input);
        let result: [u8; 32] = hasher.finalize().as_slice().try_into().unwrap();
        let to_return = Box::new(result);

        return to_return;
    }

    fn verify(&self, prev_hash: &[u8; 32]) -> Result<bool, TransactionError> {
        let signed_data_hash: Box<[u8; 32]> = self.hash_without_signature(prev_hash);

        // load sender
        let sender = PublicKey::from_slice(&self.sender)
            .report()
            .change_context(TransactionError::TxError(TxErrorKind::VerifyError))?;

        // creating verifier
        let verifier = Secp256k1::verification_only();

        // load message
        let message = Message::from_slice(Box::leak(signed_data_hash))
            .report()
            .change_context(TransactionError::TxError(TxErrorKind::VerifyError))?;

        // load signature
        let signature = Signature::from_compact(&self.signature)
            .report()
            .change_context(TransactionError::TxError(TxErrorKind::VerifyError))?;

        // verifying hashed data with public key
        let result = verifier.verify_ecdsa(&message, &signature, &sender);

        match result {
            Err(_) => {
                return Ok(false);
            }
            Ok(_) => {
                return Ok(true);
            }
        }
    }

    fn dump(&self) -> Result<Vec<u8>, TransactionError> {
        let timestamp_as_bytes: [u8; 8] = self.timestamp.to_be_bytes();

        let calculated_size: usize = self.get_dump_size();

        let mut transaction_dump: Vec<u8> = Vec::with_capacity(calculated_size);

        // header
        transaction_dump.push(Headers::Transaction as u8);

        // sender
        for byte in self.sender.iter() {
            transaction_dump.push(*byte);
        }

        // receiver
        for byte in self.receiver.iter() {
            transaction_dump.push(*byte);
        }

        // timestamp
        transaction_dump.extend(timestamp_as_bytes.iter());

        // signature
        for byte in self.signature.iter() {
            transaction_dump.push(*byte);
        }

        // amount
        tools::dump_biguint(&self.amount, &mut transaction_dump)
            .change_context(TransactionError::TxError(TxErrorKind::DumpError))?;

        return Ok(transaction_dump);
    }

    fn get_dump_size(&self) -> usize {
        let calculated_size: usize = 1 + 33 + 33 + 8 + 64 + tools::bigint_size(&self.amount);
        return calculated_size;
    }

    fn parse(data: &[u8], size: u64) -> Result<Transaction, TransactionError> {
        let mut index: usize = 0;

        if data.len() <= 138 {
            return Err(
                Report::new(TransactionError::TxError(TxErrorKind::ParseError))
                    .attach_printable("Data length <= 138"),
            );
        }

        // parsing sender address
        let sender: [u8; 33] = unsafe { transmute_copy(&data[index]) };
        index += 33;

        // parsing receiver address
        let receiver: [u8; 33] = unsafe { transmute_copy(&data[index]) };
        index += 33;

        // parsing timestamp
        let timestamp: u64 = u64::from_be_bytes(data[index..index + 8].try_into().unwrap());
        index += 8;

        // parsing signature
        let signature: [u8; 64] = unsafe { transmute_copy(&data[index]) };
        index += 64;

        // parsing amount
        let (amount, idx) = tools::load_biguint(&data[index..])
            .attach_printable("Couldn't parse amount")
            .change_context(TransactionError::TxError(TxErrorKind::ParseError))?;

        index += idx;
        if index != size as usize {
            return Err(
                Report::new(TransactionError::TxError(TxErrorKind::ParseError))
                    .attach_printable("Index != Tx size"),
            );
        }

        Ok(Transaction::new(
            &sender, &receiver, timestamp, &signature, amount,
        ))
    }

    fn get_sender(&self) -> &[u8; 33] {
        return &self.sender;
    }

    fn get_receiver(&self) -> &[u8; 33] {
        return &self.receiver;
    }

    fn get_timestamp(&self) -> u64 {
        return self.timestamp;
    }

    fn get_signature(&self) -> &[u8; 64] {
        return &self.signature;
    }

    fn sign(
        &mut self,
        prev_hash: &[u8; 32],
        private_key: &[u8; 32],
    ) -> Result<(), TransactionError> {
        let mut hasher = Sha256::new();

        let amount_as_bytes = self.amount.to_bytes_be();
        let calculated_size: usize = 32 + 33 + 33 + 8 + amount_as_bytes.len();

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in prev_hash.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }
        for byte in amount_as_bytes.iter() {
            concatenated_input.push(*byte);
        }

        hasher.update(concatenated_input);
        let result: [u8; 32] = hasher.finalize().as_slice().try_into().unwrap();
        let message = unsafe { Message::from_slice(&result).unwrap_unchecked() };

        let secret_key = unsafe { SecretKey::from_slice(private_key).unwrap_unchecked() };

        let signer = Secp256k1::new();

        let signature = signer.sign_ecdsa(&message, &secret_key);

        self.signature = signature.serialize_compact();

        Ok(())
    }
}
