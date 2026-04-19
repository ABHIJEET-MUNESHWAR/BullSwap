use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::domain::order::Order;

/// Calculate the surplus for a single trade.
///
/// Surplus = (actual_buy - expected_buy) where expected_buy is
/// the minimum the trader would accept (their limit price).
///
/// # Arguments
/// * `order` - The original order
/// * `executed_sell` - Actual amount sold
/// * `executed_buy` - Actual amount bought
///
/// # Returns
/// The surplus amount (always >= 0 for valid trades)
pub fn calculate_trade_surplus(
    order: &Order,
    executed_sell: Decimal,
    executed_buy: Decimal,
) -> Decimal {
    if order.sell_amount.is_zero() {
        return dec!(0);
    }

    // Expected buy amount at the limit price, proportional to executed sell
    let expected_buy = order.buy_amount * executed_sell / order.sell_amount;

    // Surplus is the excess over expectation
    (executed_buy - expected_buy).max(dec!(0))
}

/// Calculate total surplus for a set of trades.
///
/// # Time Complexity
/// O(n) where n is the number of trades.
pub fn calculate_total_surplus(
    orders: &[Order],
    executions: &[(usize, Decimal, Decimal)],
) -> Decimal {
    executions
        .iter()
        .map(|(idx, exec_sell, exec_buy)| {
            calculate_trade_surplus(&orders[*idx], *exec_sell, *exec_buy)
        })
        .sum()
}

/// Distribute surplus pro-rata among trades.
///
/// Each trade receives surplus proportional to its contribution
/// to the total traded volume.
///
/// # Returns
/// Vec of (order_index, surplus_share) pairs.
///
/// # Time Complexity
/// O(n) where n is the number of executions.
pub fn distribute_surplus(
    orders: &[Order],
    executions: &[(usize, Decimal, Decimal)],
) -> Vec<(usize, Decimal)> {
    executions
        .iter()
        .map(|(idx, exec_sell, exec_buy)| {
            let surplus = calculate_trade_surplus(&orders[*idx], *exec_sell, *exec_buy);
            (*idx, surplus)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
    use chrono::Utc;

    fn make_order(sell_amount: Decimal, buy_amount: Decimal) -> Order {
        Order {
            uid: OrderUid::new(),
            owner: "0xTest".to_string(),
            sell_token: uuid::Uuid::new_v4(),
            buy_token: uuid::Uuid::new_v4(),
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
    fn test_no_surplus_at_limit_price() {
        let order = make_order(dec!(100), dec!(50));
        let surplus = calculate_trade_surplus(&order, dec!(100), dec!(50));
        assert_eq!(surplus, dec!(0));
    }

    #[test]
    fn test_surplus_better_than_limit() {
        let order = make_order(dec!(100), dec!(50));
        // Got 60 B instead of expected 50 → surplus = 10
        let surplus = calculate_trade_surplus(&order, dec!(100), dec!(60));
        assert_eq!(surplus, dec!(10));
    }

    #[test]
    fn test_surplus_partial_fill() {
        let order = make_order(dec!(100), dec!(50));
        // Partially filled: sold 50 A, got 30 B
        // Expected: 50 * 50/100 = 25 B. Got 30 → surplus = 5
        let surplus = calculate_trade_surplus(&order, dec!(50), dec!(30));
        assert_eq!(surplus, dec!(5));
    }

    #[test]
    fn test_total_surplus() {
        let orders = vec![
            make_order(dec!(100), dec!(50)),
            make_order(dec!(200), dec!(80)),
        ];
        let executions = vec![
            (0, dec!(100), dec!(60)),  // surplus = 10
            (1, dec!(200), dec!(100)), // surplus = 20
        ];
        let total = calculate_total_surplus(&orders, &executions);
        assert_eq!(total, dec!(30));
    }

    #[test]
    fn test_distribute_surplus() {
        let orders = vec![
            make_order(dec!(100), dec!(50)),
            make_order(dec!(200), dec!(80)),
        ];
        let executions = vec![(0, dec!(100), dec!(60)), (1, dec!(200), dec!(100))];
        let distribution = distribute_surplus(&orders, &executions);
        assert_eq!(distribution.len(), 2);
        assert_eq!(distribution[0], (0, dec!(10)));
        assert_eq!(distribution[1], (1, dec!(20)));
    }
}
