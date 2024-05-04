use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    block::{self, BlockArc, TransactionBlock},
    chain,
    errors::{BCTreeErrorKind, BlockChainTreeError},
    merkletree,
    static_values::{
        AMMOUNT_SUMMARY, BLOCKS_PER_EPOCH, GAS_SUMMARY, OLD_AMMOUNT_SUMMARY, OLD_GAS_SUMMARY,
    },
    tools,
    transaction::Transaction,
    txpool,
    types::Hash,
};
use error_stack::{Report, ResultExt};
use primitive_types::U256;
use sled::Db;
use std::fs;

pub struct BlockChainTree {
    pub main_chain: chain::MainChain,
    pub derivative_chains: HashMap<[u8; 32], chain::DerivativeChain>,
    summary_db: Db,
    old_summary_db: Db,
    gas_db: Db,
    old_gas_db: Db,
}

impl BlockChainTree {
    pub async fn new() -> Result<Self, Report<BlockChainTreeError>> {
        let path_summary = Path::new(AMMOUNT_SUMMARY);
        let path_summary_old = Path::new(OLD_AMMOUNT_SUMMARY);
        let path_gas = Path::new(GAS_SUMMARY);
        let path_gas_old = Path::new(OLD_GAS_SUMMARY);

        // open summary DB
        let summary_db = sled::open(path_summary)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open summary db")?;

        // open old summary DB
        let old_summary_db = sled::open(path_summary_old)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open old summary db")?;

        // open gas DB
        let gas_db = sled::open(path_gas)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open gas db")?;

        let old_gas_db = sled::open(path_gas_old)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open old gas db")?;

        Ok(Self {
            main_chain: chain::MainChain::new().await?,
            derivative_chains: HashMap::new(),
            summary_db,
            old_summary_db,
            gas_db,
            old_gas_db,
        })
    }

    pub async fn add_amount(
        &self,
        owner: &[u8],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.summary_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let prev_amount = match db.get(owner)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    let new_amount = prev_amount + amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }

    pub async fn sub_amount(
        &self,
        owner: &[u8],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.summary_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let prev_amount = match db.get(owner)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    if prev_amount < amount {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(()));
                    }
                    let new_amount = prev_amount + amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }
    pub async fn get_amount(&self, owner: &[u8; 33]) -> Result<U256, Report<BlockChainTreeError>> {
        match self
            .summary_db
            .get(owner)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::GetFunds,
            ))
            .attach_printable("failed to get funds")?
        {
            Some(v) => Ok(tools::load_u256(&v).unwrap().0),
            None => Ok(U256::zero()),
        }
    }

    pub async fn send_amount(
        &self,
        from: &[u8],
        to: &[u8],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.summary_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let mut from_amount = match db.get(from)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    let mut to_amount = match db.get(to)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    if from_amount < amount {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(()));
                    }

                    from_amount -= amount;
                    to_amount += amount;

                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&from_amount));
                    tools::dump_u256(&from_amount, &mut buf).unwrap();
                    db.insert(from, buf)?;

                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&to_amount));
                    tools::dump_u256(&to_amount, &mut buf).unwrap();
                    db.insert(to, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }

    pub async fn add_gas_amount(
        &self,
        owner: &[u8],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.gas_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let prev_amount = match db.get(owner)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    let new_amount = prev_amount + amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }
    pub async fn sub_gas_amount(
        &self,
        owner: &[u8],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.gas_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let prev_amount = match db.get(owner)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    if prev_amount < amount {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(()));
                    }
                    let new_amount = prev_amount + amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }
    pub async fn get_gas_amount(
        &self,
        owner: &[u8; 33],
    ) -> Result<U256, Report<BlockChainTreeError>> {
        match self
            .gas_db
            .get(owner)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::GetFunds,
            ))
            .attach_printable("failed to get gas amount")?
        {
            Some(v) => Ok(tools::load_u256(&v).unwrap().0),
            None => Ok(U256::zero()),
        }
    }

    pub async fn send_gas(
        &self,
        from: &[u8],
        to: &[u8],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.gas_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let mut from_amount = match db.get(from)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    let mut to_amount = match db.get(to)? {
                        Some(v) => tools::load_u256(&v).unwrap().0,
                        None => U256::zero(),
                    };
                    if from_amount < amount {
                        return Err(sled::transaction::ConflictableTransactionError::Abort(()));
                    }

                    from_amount -= amount;
                    to_amount += amount;

                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&from_amount));
                    tools::dump_u256(&from_amount, &mut buf).unwrap();
                    db.insert(from, buf)?;

                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&to_amount));
                    tools::dump_u256(&to_amount, &mut buf).unwrap();
                    db.insert(to, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }

    pub async fn add_new_block(
        &self,
        block: BlockArc,
        transactions: &[Transaction],
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.main_chain.add_block(block).await?;

        self.main_chain.add_transactions(transactions).await
    }

    fn summarize(&self) -> Result<[u8; 32], Report<BlockChainTreeError>> {
        let mut hashes: Vec<[u8; 32]> = Vec::with_capacity(self.summary_db.len());
        for res in self.summary_db.iter() {
            let (address, amount) = res
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::GetFunds,
                ))
                .attach_printable("failed to get funds from summary_db")?;
            let gas_amount = self
                .gas_db
                .get(&address)
                .change_context(BlockChainTreeError::BlockChainTree(
                    BCTreeErrorKind::GetFunds,
                ))
                .attach_printable("failed to get funds from summary_db")?
                .map(|val| val.to_vec())
                .unwrap_or(Vec::with_capacity(0));
            let mut data_to_hash: Vec<u8> =
                Vec::with_capacity(address.len() + amount.len() + gas_amount.len() + 2);
            data_to_hash.extend(address.iter());
            data_to_hash.push(b'|');
            data_to_hash.extend(amount.iter());
            data_to_hash.push(b'|');
            data_to_hash.extend(gas_amount.iter());

            hashes.push(tools::hash(&data_to_hash));
        }

        let merkle_tree = merkletree::MerkleTree::build_tree(&hashes);

        Ok(*merkle_tree.get_root())
    }

    pub async fn emmit_new_main_block(
        &mut self,
        pow: [u8; 32],
        founder: [u8; 33],
        transactions: &[Hash],
        timestamp: u64,
    ) -> Result<block::BlockArc, Report<BlockChainTreeError>> {
        let last_block = self.main_chain.get_last_block().await?.unwrap(); // practically cannot fail
        let prev_hash = last_block
            .hash()
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DumpDb))
            .attach_printable("failed to hash block")?;

        if !tools::check_pow(&prev_hash, &last_block.get_info().difficulty, &pow) {
            return Err(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::WrongPow).into());
        };
        let mut difficulty = last_block.get_info().difficulty;
        tools::recalculate_difficulty(last_block.get_info().timestamp, timestamp, &mut difficulty);
        let fee = tools::recalculate_fee(&difficulty);
        let default_info = block::BasicInfo {
            timestamp,
            pow,
            previous_hash: prev_hash,
            height: last_block.get_info().height,
            difficulty,
            founder,
        };
        let new_block: block::BlockArc =
            if ((last_block.get_info().height + 1) % BLOCKS_PER_EPOCH).is_zero() {
                if transactions.len() != 0 {
                    return Err(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::SummarizeBlockWrongTransactionsAmount,
                    )
                    .into());
                }

                let merkle_tree_root = self.summarize()?;

                let summarize_block = Arc::new(block::SummarizeBlock {
                    default_info,
                    merkle_tree_root,
                });
                self.rotate_dbs().await?;

                summarize_block
            } else {
                if transactions.len() == 0 {
                    return Err(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::CreateMainChainBlock,
                    )
                    .into());
                }

                let merkle_tree = merkletree::MerkleTree::build_tree(transactions);
                let transaction_block = Arc::new(block::TransactionBlock::new(
                    fee,
                    default_info,
                    *merkle_tree.get_root(),
                    Vec::from_iter(transactions.iter().cloned()),
                ));
                transaction_block
            };

        self.main_chain.add_block(new_block.clone()).await?;
        Ok(new_block)
    }

    pub async fn flush(&self) -> Result<(), Report<BlockChainTreeError>> {
        self.main_chain.flush().await?;
        self.summary_db
            .flush_async()
            .await
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DumpDb))
            .attach_printable("failed to flush summary db")?;

        self.old_summary_db
            .flush_async()
            .await
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DumpDb))
            .attach_printable("failed to flush old summary db")?;

        self.gas_db
            .flush_async()
            .await
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DumpDb))
            .attach_printable("failed to flush old summary db")?;

        self.old_gas_db
            .flush_async()
            .await
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DumpDb))
            .attach_printable("failed to flush old summary db")?;

        Ok(())
    }

    pub async fn rotate_dbs(&mut self) -> Result<(), Report<BlockChainTreeError>> {
        self.flush().await?;

        let path_summary = Path::new(AMMOUNT_SUMMARY);
        let path_summary_old = Path::new(OLD_AMMOUNT_SUMMARY);
        let path_gas = Path::new(GAS_SUMMARY);
        let path_gas_old = Path::new(OLD_GAS_SUMMARY);

        fs::remove_dir_all(path_summary_old)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to remove previous summary database")?;

        fs::create_dir(path_summary_old)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to create previous summary database folder")?;

        fs::remove_dir_all(path_gas_old)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to remove previous gas database")?;

        fs::create_dir(path_gas_old)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to remove previous gas database folder")?;

        tools::copy_dir_all(path_summary, path_summary_old)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to copy summary database")?;

        tools::copy_dir_all(path_gas, path_gas_old)
            .change_context(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::MoveSummaryDB,
            ))
            .attach_printable("failed to copy gas database")?;

        self.old_summary_db = sled::open(path_summary_old)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open old summary db")?;

        self.old_gas_db = sled::open(path_gas_old)
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::Init))
            .attach_printable("failed to open old gas db")?;

        Ok(())
    }
}
