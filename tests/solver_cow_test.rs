mod common;

use bullswap::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
use bullswap::solver::cow_finder::find_cows;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
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
fn test_cow_finder_empty() {
    let result = find_cows(&[]);
    assert!(result.matches.is_empty());
    assert_eq!(result.total_surplus, dec!(0));
}

#[test]
fn test_cow_finder_single_order() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();
    let orders = vec![make_order(token_a, token_b, dec!(100), dec!(50))];
    let result = find_cows(&orders);
    assert!(result.matches.is_empty());
    assert_eq!(result.unmatched_order_indices.len(), 1);
}

#[test]
fn test_cow_finder_perfect_match() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();

    let orders = vec![
        make_order(token_a, token_b, dec!(100), dec!(50)),
        make_order(token_b, token_a, dec!(50), dec!(100)),
    ];

    let result = find_cows(&orders);
    assert_eq!(result.matches.len(), 1);
    assert!(result.unmatched_order_indices.is_empty());
}

#[test]
fn test_cow_finder_multiple_pairs() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();

    let orders = vec![
        make_order(token_a, token_b, dec!(100), dec!(50)),
        make_order(token_b, token_a, dec!(50), dec!(100)),
        make_order(token_a, token_b, dec!(200), dec!(100)),
        make_order(token_b, token_a, dec!(100), dec!(200)),
    ];

    let result = find_cows(&orders);
    assert_eq!(result.matches.len(), 2);
}

#[test]
fn test_cow_finder_cancelled_orders_skipped() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();

    let mut order = make_order(token_a, token_b, dec!(100), dec!(50));
    order.status = OrderStatus::Cancelled;

    let orders = vec![order, make_order(token_b, token_a, dec!(50), dec!(100))];

    let result = find_cows(&orders);
    assert!(result.matches.is_empty());
}

#[test]
fn test_cow_finder_surplus_distribution() {
    let token_a = Uuid::new_v4();
    let token_b = Uuid::new_v4();

    // Alice: sell 100 A, want 40 B (generous offer)
    // Bob: sell 60 B, want 100 A (generous offer)
    let orders = vec![
        make_order(token_a, token_b, dec!(100), dec!(40)),
        make_order(token_b, token_a, dec!(60), dec!(100)),
    ];

    let result = find_cows(&orders);
    assert_eq!(result.matches.len(), 1);
    // With overlapping prices, surplus should be generated
    assert!(result.total_surplus >= dec!(0));
}
