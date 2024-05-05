use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{
    block::{self, Block as _, BlockArc},
    chain,
    errors::{BCTreeErrorKind, BlockChainTreeError, ChainErrorKind},
    merkletree,
    static_values::{
        self, AMMOUNT_SUMMARY, BLOCKS_PER_EPOCH, BYTE_GAS_PRICE, COINS_PER_CYCLE, GAS_SUMMARY,
        MAIN_CHAIN_PAYMENT, OLD_AMMOUNT_SUMMARY, OLD_GAS_SUMMARY, ROOT_PUBLIC_ADDRESS,
    },
    tools,
    transaction::Transaction,
    transaction::Transactionable,
    txpool,
    types::Hash,
};
use error_stack::{Report, ResultExt};
use primitive_types::U256;
use sled::Db;
use std::fs;

pub struct BlockChainTree {
    main_chain: chain::MainChain,
    derivative_chains: HashMap<[u8; 33], chain::DerivativeChain>,
    summary_db: Db,
    old_summary_db: Db,
    gas_db: Db,
    old_gas_db: Db,
}

impl BlockChainTree {
    pub fn new() -> Result<Self, Report<BlockChainTreeError>> {
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
        let main_chain = chain::MainChain::new()?;

        if main_chain.get_height() == U256::one() {
            summary_db
                .transaction(
                    |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                        let mut buf: Vec<u8> =
                            Vec::with_capacity(tools::u256_size(&COINS_PER_CYCLE));
                        tools::dump_u256(&COINS_PER_CYCLE, &mut buf).unwrap();
                        db.insert(&(ROOT_PUBLIC_ADDRESS) as &[u8], buf)?;
                        Ok(())
                    },
                )
                .unwrap();
        }
        Ok(Self {
            main_chain,
            derivative_chains: HashMap::new(),
            summary_db,
            old_summary_db,
            gas_db,
            old_gas_db,
        })
    }

    pub fn get_derivative_chain(
        &mut self,
        owner: &[u8; 33],
    ) -> Result<chain::DerivativeChain, Report<BlockChainTreeError>> {
        if let Some(chain) = self.derivative_chains.get(owner) {
            return Ok(chain.clone());
        }
        let last_block = self.main_chain.get_last_block()?.unwrap(); // practically cannot fail
        let derivative_chain =
            chain::DerivativeChain::new(&hex::encode(owner), &last_block.hash().unwrap())?;
        self.derivative_chains
            .insert(*owner, derivative_chain.clone());
        Ok(derivative_chain)
    }

    pub fn get_main_chain(&self) -> chain::MainChain {
        self.main_chain.clone()
    }

    pub fn add_amount(
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

    pub fn set_amount(
        &self,
        owner: &[u8],
        amount: U256,
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.summary_db
            .transaction(
                |db| -> Result<(), sled::transaction::ConflictableTransactionError<()>> {
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&amount));
                    tools::dump_u256(&amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }

    pub fn sub_amount(
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
                    let new_amount = prev_amount - amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }
    pub fn get_amount(&self, owner: &[u8; 33]) -> Result<U256, Report<BlockChainTreeError>> {
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

    pub fn send_amount(
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

    pub fn add_gas(&self, owner: &[u8], amount: U256) -> Result<(), Report<BlockChainTreeError>> {
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
    pub fn sub_gas(&self, owner: &[u8], amount: U256) -> Result<(), Report<BlockChainTreeError>> {
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
                    let new_amount = prev_amount - amount;
                    let mut buf: Vec<u8> = Vec::with_capacity(tools::u256_size(&new_amount));
                    tools::dump_u256(&new_amount, &mut buf).unwrap();
                    db.insert(owner, buf)?;
                    Ok(())
                },
            )
            .unwrap();

        Ok(())
    }
    pub fn get_gas(&self, owner: &[u8; 33]) -> Result<U256, Report<BlockChainTreeError>> {
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

    pub fn send_gas(
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

    pub fn add_new_block(
        &self,
        block: BlockArc,
        transactions: &[Transaction],
    ) -> Result<(), Report<BlockChainTreeError>> {
        self.main_chain.add_block(block)?;

        self.main_chain.add_transactions(transactions)
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

    pub async fn emmit_new_derivative_block(
        &mut self,
        pow: &[u8; 32],
        founder: &[u8; 33],
        timestamp: u64,
    ) -> Result<block::BlockArc, Report<BlockChainTreeError>> {
        let derivative_chain = self.get_derivative_chain(founder)?;
        let (prev_hash, mut difficulty, prev_timestamp, height) =
            if let Some(block) = derivative_chain.get_last_block()? {
                (
                    block.hash().unwrap(),
                    block.get_info().difficulty,
                    block.get_info().timestamp,
                    block.get_info().height,
                )
            } else {
                let block = self
                    .main_chain
                    .find_by_hash(&derivative_chain.genesis_hash)?
                    .ok_or(BlockChainTreeError::Chain(ChainErrorKind::FindByHashE))?;
                (
                    block.hash().unwrap(),
                    static_values::BEGINNING_DIFFICULTY,
                    block.get_info().timestamp,
                    U256::zero(),
                )
            };

        if !tools::check_pow(&prev_hash, &difficulty, pow) {
            return Err(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::WrongPow).into());
        };
        tools::recalculate_difficulty(prev_timestamp, timestamp, &mut difficulty);
        let default_info = block::BasicInfo {
            timestamp,
            pow: *pow,
            previous_hash: prev_hash,
            height: height + 1,
            difficulty,
            founder: *founder,
        };

        let block = block::DerivativeBlock { default_info };
        derivative_chain.add_block(&block)?;
        Ok(Arc::new(block))
    }

    pub async fn emmit_new_main_block(
        &mut self,
        pow: &[u8; 32],
        founder: &[u8; 33],
        transactions: &[Hash],
        timestamp: u64,
    ) -> Result<block::BlockArc, Report<BlockChainTreeError>> {
        let last_block = self.main_chain.get_last_block()?.unwrap(); // practically cannot fail
        let prev_hash = last_block
            .hash()
            .change_context(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DumpDb))
            .attach_printable("failed to hash block")?;

        let mut difficulty = last_block.get_info().difficulty;
        if !tools::check_pow(&prev_hash, &difficulty, pow) {
            return Err(BlockChainTreeError::BlockChainTree(BCTreeErrorKind::WrongPow).into());
        };
        tools::recalculate_difficulty(last_block.get_info().timestamp, timestamp, &mut difficulty);
        let fee = tools::recalculate_fee(&difficulty);
        let default_info = block::BasicInfo {
            timestamp,
            pow: *pow,
            previous_hash: prev_hash,
            height: last_block.get_info().height + 1,
            difficulty,
            founder: *founder,
        };
        let new_block: block::BlockArc =
            if ((last_block.get_info().height + 1) % BLOCKS_PER_EPOCH).is_zero() {
                if !transactions.is_empty() {
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

                self.set_amount(&ROOT_PUBLIC_ADDRESS as &[u8], *COINS_PER_CYCLE)?;

                summarize_block
            } else {
                if transactions.is_empty() {
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

        self.main_chain.add_block(new_block.clone())?;
        Ok(new_block)
    }

    pub fn send_transaction(
        &self,
        transaction: &dyn Transactionable,
    ) -> Result<(), Report<BlockChainTreeError>> {
        let sender_gas_amount = self.get_gas(transaction.get_sender())?;
        let sender_amount = self.get_amount(transaction.get_sender())?;
        let amount_of_bytes = transaction.get_dump_size();
        let gas_required = *BYTE_GAS_PRICE * amount_of_bytes;
        if sender_gas_amount < gas_required {
            return Err(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))
            .attach_printable("not enough gas for the transaction");
        }
        let last_block = self.main_chain.get_last_block()?.unwrap(); // practically cannot fail
        let fee = tools::recalculate_fee(&last_block.get_info().difficulty);
        if sender_amount < fee + transaction.get_amount().unwrap_or(U256::zero()) {
            return Err(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::NewTransaction,
            ))
            .attach_printable("not enough coins to pay the fee");
        }
        self.main_chain.add_transaction(transaction)?;
        if let Some(amount) = transaction.get_amount() {
            // TODO: make into sled transaction
            self.send_amount(transaction.get_sender(), transaction.get_receiver(), amount)?;
            self.sub_amount(transaction.get_sender(), fee)?;
            self.sub_gas(transaction.get_sender(), gas_required)?;
        }
        Ok(())
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

    async fn rotate_dbs(&mut self) -> Result<(), Report<BlockChainTreeError>> {
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
