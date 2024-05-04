use std::collections::binary_heap::Iter;
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::transaction::TransactionableItem;

pub type SharedTxPool = Arc<RwLock<TxPool>>;

#[derive(Default)]
pub struct TxPool {
    pool: BinaryHeap<TransactionableItem>,
    hashes: HashSet<[u8; 32]>,
}

impl TxPool {
    pub fn new() -> TxPool {
        TxPool::default()
    }
    pub fn with_capacity(capacity: usize) -> TxPool {
        TxPool {
            pool: BinaryHeap::with_capacity(capacity),
            hashes: HashSet::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, transaction: TransactionableItem) -> bool {
        if !self.hashes.insert(transaction.hash()) {
            return false;
        }
        self.pool.push(transaction);
        true
    }

    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn transactions_iter(&self) -> Iter<'_, TransactionableItem> {
        self.pool.iter()
    }

    pub fn pop(&mut self) -> Option<([u8; 32], TransactionableItem)> {
        let tr = self.pool.pop()?;
        let hash = tr.hash();
        self.hashes.remove(&hash);
        Some((hash, tr))
    }

    pub fn transaction_exists(&self, hash: &[u8; 32]) -> bool {
        self.hashes.contains(hash)
    }
}
