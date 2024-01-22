use error_stack::{Report, Result, ResultExt};
use num_bigint::BigUint;
use num_traits::Zero;
use primitive_types::U256;
use sled::Db;

use crate::{
    errors::{BCTreeErrorKind, BlockChainTreeError, ChainErrorKind},
    tools,
};

pub struct SummaryDB {
    db: Db,
}

impl SummaryDB {
    pub fn new(db: Db) -> Self {
        SummaryDB { db }
    }
    /// Get funds
    ///
    /// Gets funds for specified address from summary db
    pub fn get_funds(&self, addr: &[u8; 33]) -> Result<BigUint, BlockChainTreeError> {
        match self.db.get(addr) {
            Ok(None) => Ok(Zero::zero()),
            Ok(Some(prev)) => {
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::GetFunds),
                )?;

                let previous = res.0;
                Ok(previous)
            }
            Err(_) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::GetDerivChain,
            ))
            .attach_printable(format!(
                "failed to get data from summary db at address: {}",
                std::str::from_utf8(addr).unwrap()
            ))),
        }
    }

    pub async fn flush(&self) -> Result<(), BlockChainTreeError> {
        self.db
            .flush_async()
            .await
            .change_context(BlockChainTreeError::Chain(ChainErrorKind::DumpConfig))
            .attach_printable("failed to flush db")?;

        Ok(())
    }

    /// Decrease funds
    ///
    /// Decreases funds for specified address in the summary db
    pub async fn decrease_funds(
        &self,
        addr: &[u8; 33],
        funds: &U256,
    ) -> Result<(), BlockChainTreeError> {
        let result = self.db.get(addr);
        match result {
            Ok(None) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::DecreaseFunds,
            ))
            .attach_printable(format!(
                "address: {} doesn't have any coins",
                std::str::from_utf8(addr).unwrap()
            ))),
            Ok(Some(prev)) => {
                let res = tools::load_u256(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DecreaseFunds),
                )?;

                let mut previous = res.0;
                if previous < *funds {
                    return Err(Report::new(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::DecreaseFunds,
                    ))
                    .attach_printable("insufficient balance"));
                }
                previous -= *funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::u256_size(&previous));
                tools::dump_u256(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DecreaseFunds),
                )?;

                self.db
                    .insert(addr, dump)
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::DecreaseFunds,
                    ))
                    .attach_printable(format!("failed to put funds at address: {addr:X?}"))?;

                self.db
                    .flush_async()
                    .await
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {addr:X?}"
                    ))?;

                Ok(())
            }
            Err(_) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::DecreaseFunds,
            ))
            .attach_printable(format!(
                "failed to get data from address: {}",
                std::str::from_utf8(addr).unwrap()
            ))),
        }
    }

    /// Add funds for address
    ///
    /// Adds funs for specified address in the summary db
    pub async fn add_funds(
        &self,
        addr: &[u8; 33],
        funds: &U256,
    ) -> Result<(), BlockChainTreeError> {
        if funds.is_zero() {
            return Ok(());
        }
        let result = self.db.get(addr);
        match result {
            Ok(None) => {
                let mut dump: Vec<u8> = Vec::with_capacity(tools::u256_size(funds));
                tools::dump_u256(funds, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                self.db
                    .insert(addr, dump)
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                self.db
                    .flush_async()
                    .await
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                Ok(())
            }
            Ok(Some(prev)) => {
                let res = tools::load_u256(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                let mut previous = res.0;
                previous += *funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::u256_size(&previous));
                tools::dump_u256(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                self.db
                    .insert(addr, dump)
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to put funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                self.db
                    .flush_async()
                    .await
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {}",
                        std::str::from_utf8(addr).unwrap()
                    ))?;

                Ok(())
            }
            Err(_) => Err(Report::new(BlockChainTreeError::BlockChainTree(
                BCTreeErrorKind::AddFunds,
            ))
            .attach_printable(format!(
                "failed to get data from address: {}",
                std::str::from_utf8(addr).unwrap()
            ))),
        }
    }
}
