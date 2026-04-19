use chrono::Utc;
use rust_decimal_macros::dec;
use std::time::Instant;
use uuid::Uuid;

use crate::domain::order::Order;
use crate::domain::settlement::{ClearingPrice, Settlement, SettlementDetails, Trade};
use crate::solver::cow_finder;
use crate::solver::engine::{BatchSolver, SolveResult, SolverError};
use crate::solver::optimizer;
use crate::solver::surplus;

/// A naive solver that combines CoW matching with clearing price optimization.
///
/// # Strategy
/// 1. First, attempt to find Coincidence of Wants (direct peer-to-peer matches)
/// 2. For remaining unmatched orders, compute uniform clearing prices
/// 3. Execute remaining orders at clearing prices
/// 4. Calculate and distribute surplus
///
/// # Performance Characteristics
/// - Time complexity: O(n log n) dominated by sorting in CoW finder and optimizer
/// - Space complexity: O(n) for storing matches and executions
pub struct NaiveSolver {
    pub id: Uuid,
    pub name: String,
}

impl NaiveSolver {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            name: "naive_solver".to_string(),
        }
    }
}

impl BatchSolver for NaiveSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn solve(&self, orders: &[Order], batch_id: Uuid) -> Result<SolveResult, SolverError> {
        let start = Instant::now();

        let matchable: Vec<&Order> = orders.iter().filter(|o| o.is_matchable()).collect();
        if matchable.is_empty() {
            return Err(SolverError::NoMatchableOrders);
        }

        let mut all_trades = Vec::new();
        let mut all_clearing_prices = Vec::new();
        let mut total_surplus = dec!(0);
        let solver_id = self.id;

        // Phase 1: Find CoW matches
        let cow_result = cow_finder::find_cows(orders);
        for cow_match in &cow_result.matches {
            all_trades.push(Trade {
                id: Uuid::new_v4(),
                settlement_id: Uuid::nil(), // Will be set later
                order_uid: cow_match.sell_order_uid.into(),
                executed_sell: cow_match.amount_a,
                executed_buy: cow_match.amount_b,
                surplus: cow_match.surplus_sell,
            });
            all_trades.push(Trade {
                id: Uuid::new_v4(),
                settlement_id: Uuid::nil(),
                order_uid: cow_match.buy_order_uid.into(),
                executed_sell: cow_match.amount_b,
                executed_buy: cow_match.amount_a,
                surplus: cow_match.surplus_buy,
            });
        }
        total_surplus += cow_result.total_surplus;

        // Phase 2: Optimize remaining orders
        let remaining_orders: Vec<Order> = cow_result
            .unmatched_order_indices
            .iter()
            .map(|&i| orders[i].clone())
            .collect();

        if !remaining_orders.is_empty() {
            let clearing_prices = optimizer::compute_clearing_prices(&remaining_orders);
            let executions = optimizer::optimize_execution(&remaining_orders, &clearing_prices);

            // Build clearing price entries
            for (token_id, price) in &clearing_prices {
                all_clearing_prices.push(ClearingPrice {
                    id: Uuid::new_v4(),
                    settlement_id: Uuid::nil(),
                    token_id: *token_id,
                    price: *price,
                });
            }

            // Build trade entries for optimizer results
            let surplus_dist = surplus::distribute_surplus(&remaining_orders, &executions);
            for (idx, exec_sell, exec_buy) in &executions {
                let order = &remaining_orders[*idx];
                let trade_surplus = surplus_dist
                    .iter()
                    .find(|(i, _)| *i == *idx)
                    .map(|(_, s)| *s)
                    .unwrap_or(dec!(0));

                all_trades.push(Trade {
                    id: Uuid::new_v4(),
                    settlement_id: Uuid::nil(),
                    order_uid: order.uid,
                    executed_sell: *exec_sell,
                    executed_buy: *exec_buy,
                    surplus: trade_surplus,
                });

                total_surplus += trade_surplus;
            }
        }

        if all_trades.is_empty() {
            return Err(SolverError::NoSolution(
                "No trades could be executed".to_string(),
            ));
        }

        let settlement_id = Uuid::new_v4();

        // Update settlement IDs
        for trade in &mut all_trades {
            trade.settlement_id = settlement_id;
        }
        for cp in &mut all_clearing_prices {
            cp.settlement_id = settlement_id;
        }

        let settlement = Settlement {
            id: settlement_id,
            batch_id,
            solver_id,
            objective_value: total_surplus,
            surplus_total: total_surplus,
            created_at: Utc::now(),
        };

        let duration = start.elapsed();

        Ok(SolveResult {
            solver_name: self.name.clone(),
            solver_id,
            settlement: SettlementDetails {
                settlement,
                trades: all_trades,
                clearing_prices: all_clearing_prices,
            },
            score: total_surplus,
            duration,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn make_order(
        sell_token: Uuid,
        buy_token: Uuid,
        sell_amount: Decimal,
        buy_amount: Decimal,
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
    fn test_naive_solver_no_orders() {
        let solver = NaiveSolver::new(Uuid::new_v4());
        let result = solver.solve(&[], Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn test_naive_solver_cow_match() {
        let solver = NaiveSolver::new(Uuid::new_v4());
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(50)),
            make_order(token_b, token_a, dec!(50), dec!(100)),
        ];

        let result = solver.solve(&orders, Uuid::new_v4());
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.settlement.trades.is_empty());
    }

    #[test]
    fn test_naive_solver_produces_surplus() {
        let solver = NaiveSolver::new(Uuid::new_v4());
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        // Overlapping prices → surplus
        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(40)),
            make_order(token_b, token_a, dec!(60), dec!(100)),
        ];

        let result = solver.solve(&orders, Uuid::new_v4());
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.score >= dec!(0));
    }
}
