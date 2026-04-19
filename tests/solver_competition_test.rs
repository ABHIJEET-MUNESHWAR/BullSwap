mod common;

use bullswap::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
use bullswap::solver::competition::SolverCompetition;
use bullswap::solver::naive_solver::NaiveSolver;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::sync::Arc;
use uuid::Uuid;

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
fn test_competition_selects_best_solver() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();

    let solver1 = Arc::new(NaiveSolver::new(Uuid::new_v4()));
    let solver2 = Arc::new(NaiveSolver::new(Uuid::new_v4()));

    let competition = SolverCompetition::new(vec![solver1, solver2]);

    let orders = vec![
        make_order(token_a, token_b, dec!(100), dec!(40)),
        make_order(token_b, token_a, dec!(60), dec!(100)),
    ];

    let result = competition.run(&orders, Uuid::new_v4());
    assert!(result.is_some());

    let result = result.unwrap();
    assert!(!result.settlement.trades.is_empty());
    assert!(result.score >= dec!(0));
}

#[test]
fn test_competition_no_solution_with_incompatible_orders() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();

    let solver = Arc::new(NaiveSolver::new(Uuid::new_v4()));
    let competition = SolverCompetition::new(vec![solver]);

    // Only sell orders in one direction, no matching buys
    // The naive solver needs at least counter-orders to match
    let orders = vec![make_order(token_a, token_b, dec!(100), dec!(50))];

    let result = competition.run(&orders, Uuid::new_v4());
    // Single order with no counter-party: CoW finder can't match,
    // optimizer may still compute clearing prices but won't find a match
    // without counter-orders in a different direction.
    // The result may be None (no solution) since we need both sides.
    // With only one direction, the optimizer has no overlapping pairs.
    assert!(
        result.is_none(),
        "Single-direction order should not produce a solution"
    );
}

#[test]
fn test_competition_handles_empty_orders() {
    let solver = Arc::new(NaiveSolver::new(Uuid::new_v4()));
    let competition = SolverCompetition::new(vec![solver]);

    let result = competition.run(&[], Uuid::new_v4());
    assert!(result.is_none());
}

#[test]
fn test_solver_count() {
    let solver1 = Arc::new(NaiveSolver::new(Uuid::new_v4()));
    let solver2 = Arc::new(NaiveSolver::new(Uuid::new_v4()));
    let solver3 = Arc::new(NaiveSolver::new(Uuid::new_v4()));

    let competition = SolverCompetition::new(vec![solver1, solver2, solver3]);
    assert_eq!(competition.solver_count(), 3);
}

#[test]
fn test_large_order_batch() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();

    let solver = Arc::new(NaiveSolver::new(Uuid::new_v4()));
    let competition = SolverCompetition::new(vec![solver]);

    let mut orders = Vec::new();
    for i in 0..100 {
        let sell_amount = Decimal::from(100 + i);
        let buy_amount = Decimal::from(40 + i % 30);
        orders.push(make_order(token_a, token_b, sell_amount, buy_amount));
        orders.push(make_order(token_b, token_a, buy_amount, sell_amount));
    }

    let result = competition.run(&orders, Uuid::new_v4());
    assert!(result.is_some());

    let result = result.unwrap();
    assert!(!result.settlement.trades.is_empty());
}
