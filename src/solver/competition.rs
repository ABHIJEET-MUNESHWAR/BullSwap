use rayon::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::order::Order;
use crate::solver::engine::{BatchSolver, SolveResult};

/// Runs a competition among multiple solvers in parallel.
///
/// Each solver receives the same set of orders and produces a settlement.
/// The solver with the highest objective score (total surplus) wins.
///
/// # Parallel Processing
/// Uses Rayon to run solvers on all available CPU cores simultaneously.
/// Each solver gets its own thread, maximizing throughput.
///
/// # Performance Characteristics
/// - Time complexity: O(max(solver_time)) — parallel, bounded by slowest solver
/// - Space complexity: O(S * n) where S is number of solvers, n is orders
pub struct SolverCompetition {
    solvers: Vec<Arc<dyn BatchSolver>>,
}

impl SolverCompetition {
    pub fn new(solvers: Vec<Arc<dyn BatchSolver>>) -> Self {
        Self { solvers }
    }

    /// Run all solvers in parallel and return the best result.
    ///
    /// # Arguments
    /// * `orders` - The orders to solve
    /// * `batch_id` - The batch being solved
    ///
    /// # Returns
    /// The `SolveResult` from the winning solver, or None if no solver produced a valid solution.
    pub fn run(&self, orders: &[Order], batch_id: Uuid) -> Option<SolveResult> {
        if self.solvers.is_empty() || orders.is_empty() {
            return None;
        }

        tracing::info!(
            num_solvers = self.solvers.len(),
            num_orders = orders.len(),
            batch_id = %batch_id,
            "Starting solver competition"
        );

        // Run all solvers in parallel using Rayon
        let results: Vec<Option<SolveResult>> = self
            .solvers
            .par_iter()
            .map(|solver| {
                let solver_name = solver.name().to_string();
                tracing::info!(solver = %solver_name, "Solver starting");

                match solver.solve(orders, batch_id) {
                    Ok(result) => {
                        tracing::info!(
                            solver = %solver_name,
                            score = %result.score,
                            duration_ms = result.duration.as_millis(),
                            num_trades = result.settlement.trades.len(),
                            "Solver completed"
                        );
                        Some(result)
                    }
                    Err(e) => {
                        tracing::warn!(
                            solver = %solver_name,
                            error = %e,
                            "Solver failed"
                        );
                        None
                    }
                }
            })
            .collect();

        // Find the best result (highest score)
        let winner = results
            .into_iter()
            .flatten()
            .max_by(|a, b| a.score.cmp(&b.score));

        if let Some(ref w) = winner {
            tracing::info!(
                winner = %w.solver_name,
                score = %w.score,
                num_trades = w.settlement.trades.len(),
                "Solver competition winner"
            );
        } else {
            tracing::warn!("No solver produced a valid solution");
        }

        winner
    }

    /// Number of registered solvers.
    pub fn solver_count(&self) -> usize {
        self.solvers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
    use crate::solver::naive_solver::NaiveSolver;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn make_order(
        sell_token: Uuid,
        buy_token: Uuid,
        sell_amount: rust_decimal::Decimal,
        buy_amount: rust_decimal::Decimal,
    ) -> Order {
        Order {
            uid: OrderUid::new(),
            owner: "0xTest".to_string(),
            sell_token,
            buy_token,
            sell_amount,
            buy_amount,
            kind: OrderKind::Sell,
            status: OrderStatus::Open,
            signature: "sig".to_string(),
            batch_id: None,
            valid_to: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_competition_with_single_solver() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        let solver = Arc::new(NaiveSolver::new(Uuid::new_v4()));
        let competition = SolverCompetition::new(vec![solver]);

        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(50)),
            make_order(token_b, token_a, dec!(50), dec!(100)),
        ];

        let result = competition.run(&orders, Uuid::new_v4());
        assert!(result.is_some());
    }

    #[test]
    fn test_competition_with_multiple_solvers() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        let solver1 = Arc::new(NaiveSolver::new(Uuid::new_v4()));
        let solver2 = Arc::new(NaiveSolver::new(Uuid::new_v4()));
        let competition = SolverCompetition::new(vec![solver1, solver2]);

        assert_eq!(competition.solver_count(), 2);

        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(50)),
            make_order(token_b, token_a, dec!(50), dec!(100)),
        ];

        let result = competition.run(&orders, Uuid::new_v4());
        assert!(result.is_some());
    }

    #[test]
    fn test_competition_empty_orders() {
        let solver = Arc::new(NaiveSolver::new(Uuid::new_v4()));
        let competition = SolverCompetition::new(vec![solver]);
        let result = competition.run(&[], Uuid::new_v4());
        assert!(result.is_none());
    }

    #[test]
    fn test_competition_no_solvers() {
        let competition = SolverCompetition::new(vec![]);
        let result = competition.run(&[], Uuid::new_v4());
        assert!(result.is_none());
    }
}
