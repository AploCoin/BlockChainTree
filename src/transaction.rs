use crate::errors::*;
use crate::tools;
use num_bigint::BigUint;
use primitive_types::U256;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt::Debug;

use crate::dump_headers::Headers;
use secp256k1::ecdsa::Signature;
use secp256k1::PublicKey;
use secp256k1::{Message, Secp256k1, SecretKey};
use std::mem::transmute;

use error_stack::{IntoReport, Report, Result, ResultExt};

pub type TransactionableItem = Box<dyn Transactionable + Send + Sync>;

impl Eq for TransactionableItem {}

impl Ord for TransactionableItem {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.get_timestamp().cmp(&other.get_timestamp()) {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => {
                let tr_hash: [u64; 4] = unsafe { transmute(self.hash()) };
                let other_hash: [u64; 4] = unsafe { transmute(other.hash()) };

                for (left, right) in tr_hash.iter().zip(other_hash.iter()) {
                    match left.cmp(right) {
                        Ordering::Less => return Ordering::Greater,
                        Ordering::Equal => {}
                        Ordering::Greater => return Ordering::Less,
                    }
                }
                Ordering::Equal
            }
            Ordering::Greater => Ordering::Less,
        }
    }
}

impl PartialOrd for TransactionableItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match self.get_timestamp().cmp(&other.get_timestamp()) {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => {
                let tr_hash: [u64; 4] = unsafe { transmute(self.hash()) };
                let other_hash: [u64; 4] = unsafe { transmute(other.hash()) };

                for (left, right) in tr_hash.iter().zip(other_hash.iter()) {
                    match left.cmp(right) {
                        Ordering::Less => return Some(Ordering::Greater),
                        Ordering::Equal => {}
                        Ordering::Greater => return Some(Ordering::Less),
                    }
                }
                Ordering::Equal
            }
            Ordering::Greater => Ordering::Less,
        })
    }
}

impl PartialEq for TransactionableItem {
    fn eq(&self, other: &Self) -> bool {
        self.get_timestamp() == other.get_timestamp()
    }
}

pub trait Transactionable: Send + Sync {
    fn hash(&self) -> [u8; 32];
    fn hash_without_signature(&self) -> [u8; 32];

    fn verify(&self) -> Result<bool, TransactionError>;

    fn dump(&self) -> Result<Vec<u8>, TransactionError>;
    fn get_dump_size(&self) -> usize;

    fn parse(data: &[u8], size: u64) -> Result<Self, TransactionError>
    where
        Self: Sized;

    fn get_sender(&self) -> &[u8; 33];
    fn get_receiver(&self) -> &[u8; 33];
    fn get_timestamp(&self) -> u64;
    fn get_signature(&self) -> &[u8; 64];
    fn get_amount(&self) -> Option<U256>;
}

#[derive(Debug, Clone)]
pub struct Transaction {
    hash: [u8; 32],
    sender: [u8; 33],
    receiver: [u8; 33],
    timestamp: u64,
    signature: [u8; 64],
    amount: U256,
    //data:
}

impl Transaction {
    pub fn generate_signature(
        sender: &[u8; 33],
        receiver: &[u8; 33],
        timestamp: u64,
        amount: &U256,
        private_key: &[u8; 32],
    ) -> [u8; 64] {
        let mut hasher = Sha256::new();

        let calculated_size: usize = 33 + 33 + 8 + tools::u256_size(amount);

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }

        tools::dump_u256(amount, &mut concatenated_input);

        hasher.update(concatenated_input);
        let result: [u8; 32] = hasher.finalize().as_slice().try_into().unwrap();
        let message = unsafe { Message::from_slice(&result).unwrap_unchecked() };

        let secret_key = unsafe { SecretKey::from_slice(private_key).unwrap_unchecked() };

        let signer = Secp256k1::new();

        let signature = signer.sign_ecdsa(&message, &secret_key);

        signature.serialize_compact()
    }

    pub fn generate_hash(
        sender: &[u8; 33],
        receiver: &[u8; 33],
        timestamp: u64,
        signature: &[u8; 64],
        amount: &U256,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();

        let calculated_size: usize = 33 + 33 + 8 + tools::u256_size(amount);

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in signature.iter() {
            concatenated_input.push(*byte);
        }
        for byte in timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }
        // for byte in amount_as_bytes.iter() {
        //     concatenated_input.push(*byte);
        // }
        tools::dump_u256(amount, &mut concatenated_input);

        hasher.update(concatenated_input);
        hasher.finalize().as_slice().try_into().unwrap()
    }

    pub fn new(
        sender: [u8; 33],
        receiver: [u8; 33],
        timestamp: u64,
        amount: U256,
        private_key: [u8; 32],
    ) -> Transaction {
        let signature =
            Transaction::generate_signature(&sender, &receiver, timestamp, &amount, &private_key);
        Transaction {
            hash: Transaction::generate_hash(&sender, &receiver, timestamp, &signature, &amount),
            sender,
            receiver,
            timestamp,
            signature,
            amount,
        }
    }

    pub fn new_signed(
        hash: [u8; 32],
        sender: [u8; 33],
        receiver: [u8; 33],
        timestamp: u64,
        amount: U256,
        signature: [u8; 64],
    ) -> Transaction {
        Transaction {
            hash,
            sender,
            receiver,
            timestamp,
            signature,
            amount,
        }
    }

    pub fn get_amount(&self) -> &U256 {
        &self.amount
    }
}

impl Transactionable for Transaction {
    fn hash(&self) -> [u8; 32] {
        self.hash
    }
    fn hash_without_signature(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        //let amount_as_bytes = tools;
        let calculated_size: usize = 33 + 33 + 8 + tools::u256_size(&self.amount);

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        for byte in self.sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }
        tools::dump_u256(&self.amount, &mut concatenated_input).unwrap();

        hasher.update(concatenated_input);
        let result: [u8; 32] = hasher.finalize().as_slice().try_into().unwrap();

        result
    }

    fn verify(&self) -> Result<bool, TransactionError> {
        let signed_data_hash = self.hash_without_signature();

        // load sender
        let sender = PublicKey::from_slice(&self.sender)
            .into_report()
            .change_context(TransactionError::Tx(TxErrorKind::Verify))?;

        // creating verifier
        let verifier = Secp256k1::verification_only();

        // load message
        let message = Message::from_slice(&signed_data_hash)
            .into_report()
            .change_context(TransactionError::Tx(TxErrorKind::Verify))?;

        // load signature
        let signature = Signature::from_compact(&self.signature)
            .into_report()
            .change_context(TransactionError::Tx(TxErrorKind::Verify))?;

        // verifying hashed data with public key
        let result = verifier.verify_ecdsa(&message, &signature, &sender);

        match result {
            Err(_) => Ok(false),
            Ok(_) => Ok(true),
        }
    }

    fn dump(&self) -> Result<Vec<u8>, TransactionError> {
        let timestamp_as_bytes: [u8; 8] = self.timestamp.to_be_bytes();

        let calculated_size: usize = self.get_dump_size();

        let mut transaction_dump: Vec<u8> = Vec::with_capacity(calculated_size);

        // header
        transaction_dump.push(Headers::Transaction as u8);

        // hash
        for byte in self.hash.iter() {
            transaction_dump.push(*byte);
        }

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
        tools::dump_u256(&self.amount, &mut transaction_dump)
            .change_context(TransactionError::Tx(TxErrorKind::Dump))?;

        Ok(transaction_dump)
    }

    fn get_dump_size(&self) -> usize {
        1 + 32 + 33 + 33 + 8 + 64 + tools::u256_size(&self.amount)
    }

    fn parse(data: &[u8], size: u64) -> Result<Transaction, TransactionError> {
        let mut index: usize = 0;

        if data.len() <= 170 {
            return Err(Report::new(TransactionError::Tx(TxErrorKind::Parse))
                .attach_printable("Data length <= 170"));
        }

        // parsing hash
        let hash: [u8; 32] = unsafe { data[index..index + 32].try_into().unwrap_unchecked() };
        index += 32;

        // parsing sender address
        let sender: [u8; 33] = unsafe { data[index..index + 33].try_into().unwrap_unchecked() };
        index += 33;

        // parsing receiver address
        let receiver: [u8; 33] = unsafe { data[index..index + 33].try_into().unwrap_unchecked() };
        index += 33;

        // parsing timestamp
        let timestamp: u64 = u64::from_be_bytes(data[index..index + 8].try_into().unwrap());
        index += 8;

        // parsing signature
        let signature: [u8; 64] = unsafe { data[index..index + 64].try_into().unwrap_unchecked() };
        index += 64;

        // parsing amount
        let (amount, idx) = tools::load_u256(&data[index..])
            .attach_printable("Couldn't parse amount")
            .change_context(TransactionError::Tx(TxErrorKind::Parse))?;

        index += idx;
        if index != size as usize {
            return Err(Report::new(TransactionError::Tx(TxErrorKind::Parse))
                .attach_printable("Index != Tx size"));
        }

        Ok(Transaction::new_signed(
            hash, sender, receiver, timestamp, amount, signature,
        ))
    }

    fn get_sender(&self) -> &[u8; 33] {
        &self.sender
    }

    fn get_receiver(&self) -> &[u8; 33] {
        &self.receiver
    }

    fn get_timestamp(&self) -> u64 {
        self.timestamp
    }

    fn get_signature(&self) -> &[u8; 64] {
        &self.signature
    }
    fn get_amount(&self) -> Option<U256> {
        Some(self.amount.clone())
    }
}
