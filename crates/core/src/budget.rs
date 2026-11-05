//! Work budgets: caps on how much computation an operation may do.

use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// A cap on the amount of work an operation may do, counted in basic
/// operations (such as solving one row of a puzzle), not in time, so that the
/// same budget gives the same result on every machine.
///
/// Operations that accept a budget either finish within it or return
/// [`BudgetExceeded`]; running out of budget never changes a result (G8 in
/// the guidelines).
///
/// ```
/// use pencil_puzzles_core::Budget;
///
/// let budget = Budget::new(2);
/// assert!(budget.spend(1));
/// assert!(budget.spend(1));
/// assert!(!budget.spend(1));
/// assert!(budget.check().is_err());
/// ```
#[derive(Debug)]
pub struct Budget {
    remaining: Cell<u64>,
    spent: Cell<u64>,
    exhausted: Cell<bool>,
    cancellation: Option<Arc<AtomicBool>>,
}

impl Budget {
    /// A budget of `operations` basic operations.
    pub fn new(operations: u64) -> Self {
        Self {
            remaining: Cell::new(operations),
            spent: Cell::new(0),
            exhausted: Cell::new(false),
            cancellation: None,
        }
    }

    /// A budget that never runs out.
    pub fn unlimited() -> Self {
        Self::new(u64::MAX)
    }

    /// A work budget with cooperative cancellation checked at work checkpoints.
    /// Cancellation stops an operation; it must never select another result.
    pub fn with_cancellation(operations: u64, cancellation: Arc<AtomicBool>) -> Self {
        Self {
            cancellation: Some(cancellation),
            ..Self::new(operations)
        }
    }

    /// Whether the external cancellation token has been set.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    /// Spends `operations`. Returns `false`, and stays exhausted from then
    /// on, if there wasn't enough left.
    pub fn spend(&self, operations: u64) -> bool {
        if self.is_exhausted() {
            return false;
        }
        match self.remaining.get().checked_sub(operations) {
            Some(left) => {
                self.remaining.set(left);
                self.spent.set(self.spent.get().saturating_add(operations));
                true
            }
            None => {
                self.remaining.set(0);
                self.exhausted.set(true);
                false
            }
        }
    }

    /// Whether the budget has run out.
    pub fn is_exhausted(&self) -> bool {
        self.exhausted.get() || self.is_cancelled()
    }

    /// How many operations have been spent. Benchmarks report this as a
    /// count that doesn't vary between machines (B3).
    pub fn spent(&self) -> u64 {
        self.spent.get()
    }

    /// `Err(BudgetExceeded)` if the budget has run out.
    pub fn check(&self) -> Result<(), BudgetExceeded> {
        if self.is_exhausted() {
            Err(BudgetExceeded)
        } else {
            Ok(())
        }
    }
}

/// An operation ran out of its work budget before finishing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BudgetExceeded;

impl fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the work budget ran out before the operation finished")
    }
}

impl Error for BudgetExceeded {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_stays_exhausted() {
        let budget = Budget::new(3);
        assert!(budget.spend(2));
        assert_eq!(budget.spent(), 2);
        assert!(!budget.spend(2));
        assert!(budget.is_exhausted());
        assert!(!budget.spend(0));
        assert_eq!(budget.check(), Err(BudgetExceeded));
    }

    #[test]
    fn unlimited_never_runs_out() {
        let budget = Budget::unlimited();
        assert!(budget.spend(u64::MAX / 2));
        assert!(budget.check().is_ok());
    }

    #[test]
    fn cancellation_is_observed_without_spending_work() {
        let flag = Arc::new(AtomicBool::new(false));
        let budget = Budget::with_cancellation(100, flag.clone());
        assert!(budget.spend(2));
        flag.store(true, Ordering::Relaxed);
        assert!(budget.is_cancelled());
        assert_eq!(budget.check(), Err(BudgetExceeded));
        assert!(!budget.spend(1));
        assert_eq!(budget.spent(), 2);
    }
}
