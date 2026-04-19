use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use super::settlement::SettlementDetails;

/// A registered solver that competes to settle batches.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Solver {
    pub id: Uuid,
    pub name: String,
    pub active: bool,
}

/// The result produced by a solver for a given batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverResult {
    pub solver_id: Uuid,
    pub solver_name: String,
    /// The settlement produced by this solver.
    pub settlement: SettlementDetails,
    /// Objective score: higher is better (total surplus).
    pub score: Decimal,
    /// Time taken by the solver.
    pub duration: Duration,
}

impl SolverResult {
    /// Check if this result is better than another based on score.
    pub fn is_better_than(&self, other: &SolverResult) -> bool {
        self.score > other.score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::settlement::Settlement;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn make_result(score: Decimal) -> SolverResult {
        SolverResult {
            solver_id: Uuid::new_v4(),
            solver_name: "test_solver".to_string(),
            settlement: SettlementDetails {
                settlement: Settlement {
                    id: Uuid::new_v4(),
                    batch_id: Uuid::new_v4(),
                    solver_id: Uuid::new_v4(),
                    objective_value: score,
                    surplus_total: score,
                    created_at: Utc::now(),
                },
                trades: vec![],
                clearing_prices: vec![],
            },
            score,
            duration: Duration::from_millis(100),
        }
    }

    #[test]
    fn test_solver_result_comparison() {
        let a = make_result(dec!(10));
        let b = make_result(dec!(5));
        assert!(a.is_better_than(&b));
        assert!(!b.is_better_than(&a));
    }
}
