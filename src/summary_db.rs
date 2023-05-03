use error_stack::{IntoReport, Report, Result, ResultExt};
use num_bigint::BigUint;
use num_traits::Zero;
use sled::Db;

use crate::{
    errors::{BCTreeErrorKind, BlockChainTreeError},
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

    /// Decrease funds
    ///
    /// Decreases funds for specified address in the summary db
    pub async fn decrease_funds(
        &self,
        addr: &[u8; 33],
        funds: &BigUint,
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
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DecreaseFunds),
                )?;

                let mut previous = res.0;
                if previous < *funds {
                    return Err(Report::new(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::DecreaseFunds,
                    ))
                    .attach_printable("insufficient balance"));
                }
                previous -= funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&previous));
                tools::dump_biguint(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::DecreaseFunds),
                )?;

                self.db
                    .insert(addr, dump)
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::DecreaseFunds,
                    ))
                    .attach_printable(format!("failed to put funds at address: {:X?}", addr))?;

                self.db
                    .flush_async()
                    .await
                    .into_report()
                    .change_context(BlockChainTreeError::BlockChainTree(
                        BCTreeErrorKind::AddFunds,
                    ))
                    .attach_printable(format!(
                        "failed to create and add funds at address: {:X?}",
                        addr
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
        funds: &BigUint,
    ) -> Result<(), BlockChainTreeError> {
        if funds.is_zero() {
            return Ok(());
        }
        let result = self.db.get(addr);
        match result {
            Ok(None) => {
                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(funds));
                tools::dump_biguint(funds, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                self.db
                    .insert(addr, dump)
                    .into_report()
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
                    .into_report()
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
                let res = tools::load_biguint(&prev).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                let mut previous = res.0;
                previous += funds;

                let mut dump: Vec<u8> = Vec::with_capacity(tools::bigint_size(&previous));
                tools::dump_biguint(&previous, &mut dump).change_context(
                    BlockChainTreeError::BlockChainTree(BCTreeErrorKind::AddFunds),
                )?;

                self.db
                    .insert(addr, dump)
                    .into_report()
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
                    .into_report()
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
