use rust_decimal::Decimal;
use std::time::Duration;
use uuid::Uuid;

use crate::domain::order::Order;
use crate::domain::settlement::SettlementDetails;

/// Result of a solver execution.
#[derive(Debug, Clone)]
pub struct SolveResult {
    pub solver_name: String,
    pub solver_id: Uuid,
    pub settlement: SettlementDetails,
    pub score: Decimal,
    pub duration: Duration,
}

/// Trait defining the interface for batch auction solvers.
///
/// Each solver implementation competes to find the best settlement
/// for a given set of orders. The solver with the highest objective
/// score wins and its solution is executed.
///
/// # Performance Characteristics
/// - `solve()` is expected to complete within a reasonable time bound.
/// - Implementations should be CPU-bound and parallelizable.
pub trait BatchSolver: Send + Sync {
    /// Unique name of this solver.
    fn name(&self) -> &str;

    /// Unique ID of this solver.
    fn id(&self) -> Uuid;

    /// Solve a batch of orders and produce a settlement.
    ///
    /// # Arguments
    /// * `orders` - The orders to settle in this batch
    /// * `batch_id` - The ID of the batch being solved
    ///
    /// # Returns
    /// A `SolveResult` containing the settlement, or an error if no valid solution exists.
    fn solve(&self, orders: &[Order], batch_id: Uuid) -> Result<SolveResult, SolverError>;
}

/// Errors that can occur during solving.
#[derive(Debug, thiserror::Error)]
pub enum SolverError {
    #[error("No matchable orders in batch")]
    NoMatchableOrders,

    #[error("No valid solution found: {0}")]
    NoSolution(String),

    #[error("Solver timeout after {0:?}")]
    Timeout(Duration),

    #[error("Internal solver error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_error_display() {
        let err = SolverError::NoMatchableOrders;
        assert!(err.to_string().contains("No matchable orders"));

        let err = SolverError::Timeout(Duration::from_secs(30));
        assert!(err.to_string().contains("30"));
    }
}

