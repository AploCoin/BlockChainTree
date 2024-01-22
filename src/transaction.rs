use crate::errors::*;
use crate::tools;
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

    fn parse(data: &[u8]) -> Result<Self, TransactionError>
    where
        Self: Sized;

    fn get_sender(&self) -> &[u8; 33];
    fn get_receiver(&self) -> &[u8; 33];
    fn get_timestamp(&self) -> u64;
    fn get_signature(&self) -> &[u8; 64];
    fn get_amount(&self) -> Option<U256>;
    fn get_data(&self) -> Option<&[u8]>;
}

#[derive(Debug, Clone)]
pub struct Transaction {
    sender: [u8; 33],
    receiver: [u8; 33],
    timestamp: u64,
    signature: [u8; 64],
    amount: U256,
    data: Option<Vec<u8>>,
}

impl Transaction {
    pub fn generate_signature(
        sender: &[u8; 33],
        receiver: &[u8; 33],
        timestamp: u64,
        amount: &U256,
        data: Option<&[u8]>,
        private_key: &[u8; 32],
    ) -> [u8; 64] {
        let mut hasher = Sha256::new();

        let calculated_size: usize =
            1 + 33 + 33 + 8 + tools::u256_size(amount) + data.map_or(0, |data| data.len());

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        concatenated_input.push(Headers::Transaction as u8);
        for byte in sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }
        tools::dump_u256(amount, &mut concatenated_input)
            .attach_printable("Error to dump amount")
            .change_context(TransactionError::Tx(TxErrorKind::Dump))
            .unwrap();
        if let Some(data) = data {
            concatenated_input.extend(data.iter());
        }

        hasher.update(concatenated_input);
        let result: [u8; 32] = hasher.finalize().as_slice().try_into().unwrap();
        let message = unsafe { Message::from_digest_slice(&result).unwrap_unchecked() };

        let secret_key = unsafe { SecretKey::from_slice(private_key).unwrap_unchecked() };

        let signer = Secp256k1::new();

        let signature = signer.sign_ecdsa(&message, &secret_key);

        signature.serialize_compact()
    }

    pub fn generate_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        let calculated_size: usize = 1
            + 33
            + 33
            + 8
            + 64
            + tools::u256_size(&self.amount)
            + self.data.as_ref().map_or(0, |data| data.len());

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        concatenated_input.push(Headers::Transaction as u8);
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
        tools::dump_u256(&self.amount, &mut concatenated_input)
            .attach_printable("Error to dump amount")
            .change_context(TransactionError::Tx(TxErrorKind::Dump))
            .unwrap();
        if let Some(data) = self.data.as_ref() {
            concatenated_input.extend(data.iter());
        }

        hasher.update(concatenated_input);
        unsafe { hasher.finalize().as_slice().try_into().unwrap_unchecked() }
    }

    pub fn new(
        sender: [u8; 33],
        receiver: [u8; 33],
        timestamp: u64,
        amount: U256,
        private_key: [u8; 32],
        data: Option<Vec<u8>>,
    ) -> Transaction {
        let signature = Transaction::generate_signature(
            &sender,
            &receiver,
            timestamp,
            &amount,
            data.as_ref().map(|data| data.as_slice()),
            &private_key,
        );
        Transaction {
            sender,
            receiver,
            timestamp,
            signature,
            amount,
            data,
        }
    }

    pub fn new_signed(
        //hash: [u8; 32],
        sender: [u8; 33],
        receiver: [u8; 33],
        timestamp: u64,
        amount: U256,
        data: Option<Vec<u8>>,
        signature: [u8; 64],
    ) -> Transaction {
        Transaction {
            //hash,
            sender,
            receiver,
            timestamp,
            signature,
            amount,
            data,
        }
    }

    pub fn get_amount(&self) -> &U256 {
        &self.amount
    }
}

impl Transactionable for Transaction {
    fn hash(&self) -> [u8; 32] {
        self.generate_hash()
    }
    fn hash_without_signature(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        let calculated_size: usize = 1
            + 33
            + 33
            + 8
            + tools::u256_size(&self.amount)
            + self.data.as_ref().map_or(0, |data| data.len());

        let mut concatenated_input: Vec<u8> = Vec::with_capacity(calculated_size);
        concatenated_input.push(Headers::Transaction as u8);
        for byte in self.sender.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.receiver.iter() {
            concatenated_input.push(*byte);
        }
        for byte in self.timestamp.to_be_bytes().iter() {
            concatenated_input.push(*byte);
        }
        tools::dump_u256(&self.amount, &mut concatenated_input)
            .attach_printable("Error to dump amount")
            .change_context(TransactionError::Tx(TxErrorKind::Dump))
            .unwrap();
        if let Some(data) = self.data.as_ref() {
            concatenated_input.extend(data.iter());
        }

        hasher.update(concatenated_input);
        unsafe { hasher.finalize().as_slice().try_into().unwrap_unchecked() }
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
        let message = Message::from_digest_slice(&signed_data_hash)
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
        transaction_dump.extend(self.timestamp.to_be_bytes().iter());

        // signature
        for byte in self.signature.iter() {
            transaction_dump.push(*byte);
        }

        // amount
        tools::dump_u256(&self.amount, &mut transaction_dump)
            .change_context(TransactionError::Tx(TxErrorKind::Dump))?;

        // data
        if let Some(data) = self.data.as_ref() {
            transaction_dump.extend(data.iter());
        }

        Ok(transaction_dump)
    }

    fn get_dump_size(&self) -> usize {
        1 + 33
            + 33
            + 8
            + 64
            + tools::u256_size(&self.amount)
            + self.data.as_ref().map_or(0, |data| data.len())
    }

    fn parse(data: &[u8]) -> Result<Transaction, TransactionError> {
        let mut index: usize = 0;

        if data.len() < 139 {
            return Err(Report::new(TransactionError::Tx(TxErrorKind::Parse))
                .attach_printable("Data length < 139"));
        }

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

        index += idx + 1;

        let tx_data = if index == data.len() {
            None
        } else {
            let mut new_data = Vec::<u8>::with_capacity(data.len() - index);
            new_data.extend(data[index..].iter());
            index += new_data.len();
            Some(new_data)
        };

        if index != data.len() {
            return Err(Report::new(TransactionError::Tx(TxErrorKind::Parse))
                .attach_printable("Index != Tx size"));
        }

        Ok(Transaction::new_signed(
            sender, receiver, timestamp, amount, tx_data, signature,
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

    fn get_data(&self) -> Option<&[u8]> {
        self.data.as_ref().map(|data| data.as_slice())
    }
}
