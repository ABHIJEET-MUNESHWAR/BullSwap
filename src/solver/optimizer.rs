use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::order::Order;

/// Compute uniform clearing prices for token pairs.
///
/// In a batch auction, all orders for the same token pair execute
/// at the same uniform clearing price. This prevents front-running
/// and ensures fairness.
///
/// # Algorithm
/// 1. Group orders by token pair
/// 2. For each pair, compute the clearing price that maximizes total surplus
/// 3. Use a simple mid-price heuristic between best bid and best ask
///
/// # Time Complexity
/// - O(n log n) per token pair for sorting
/// - O(n) for price computation
/// - Overall: O(n log n) where n is total number of orders
pub fn compute_clearing_prices(orders: &[Order]) -> HashMap<Uuid, Decimal> {
    let mut prices: HashMap<Uuid, Decimal> = HashMap::new();

    // Group by (sell_token, buy_token)
    let mut pair_orders: HashMap<(Uuid, Uuid), Vec<&Order>> = HashMap::new();
    for order in orders {
        if order.is_matchable() {
            pair_orders
                .entry((order.sell_token, order.buy_token))
                .or_default()
                .push(order);
        }
    }

    // For each token pair, find the clearing price
    let keys: Vec<(Uuid, Uuid)> = pair_orders.keys().cloned().collect();
    for (sell_tok, buy_tok) in &keys {
        let counter_key = (*buy_tok, *sell_tok);

        // Only process each pair once
        if sell_tok >= buy_tok {
            continue;
        }

        let forward = pair_orders.get(&(*sell_tok, *buy_tok));
        let backward = pair_orders.get(&counter_key);

        if let (Some(sellers), Some(buyers)) = (forward, backward) {
            // Sellers: offering A for B → limit price = sell_amount / buy_amount (price of B in A)
            let mut seller_prices: Vec<Decimal> = sellers
                .iter()
                .filter(|o| !o.buy_amount.is_zero())
                .map(|o| o.sell_amount / o.buy_amount)
                .collect();

            // Buyers: offering B for A → effective price of B in A = buy_amount / sell_amount
            let mut buyer_prices: Vec<Decimal> = buyers
                .iter()
                .filter(|o| !o.buy_amount.is_zero())
                .map(|o| o.buy_amount / o.sell_amount)
                .collect();

            seller_prices.sort();
            buyer_prices.sort_by(|a, b| b.cmp(a));

            if let (Some(&best_ask), Some(&best_bid)) =
                (seller_prices.first(), buyer_prices.first())
            {
                if best_ask <= best_bid {
                    // Clearing price is midpoint
                    let clearing = (best_ask + best_bid) / dec!(2);

                    // Set price for sell_token relative to buy_token
                    // price[A] = clearing means 1 B costs `clearing` units of A
                    prices.insert(*sell_tok, clearing);
                    if !clearing.is_zero() {
                        prices.insert(*buy_tok, dec!(1) / clearing);
                    }
                }
            }
        }
    }

    prices
}

/// Optimize the execution amounts for a set of orders given clearing prices.
///
/// Returns (order_index, executed_sell, executed_buy) for each fillable order.
/// Only fills orders where both tokens have established clearing prices from
/// actual matched pairs — prevents phantom fills without counterparty.
///
/// # Time Complexity
/// O(n) where n is the number of orders.
pub fn optimize_execution(
    orders: &[Order],
    clearing_prices: &HashMap<Uuid, Decimal>,
) -> Vec<(usize, Decimal, Decimal)> {
    let mut executions = Vec::new();

    for (idx, order) in orders.iter().enumerate() {
        if !order.is_matchable() {
            continue;
        }

        let sell_price = match clearing_prices.get(&order.sell_token) {
            Some(p) => *p,
            None => continue, // No clearing price = no matched pair = skip
        };
        let buy_price = match clearing_prices.get(&order.buy_token) {
            Some(p) if !p.is_zero() => *p,
            _ => continue,
        };

        // At the clearing price, how much buy_token does the seller get?
        let effective_buy = order.sell_amount * sell_price / buy_price;

        // Only execute if the trader gets at least their limit amount
        if effective_buy >= order.buy_amount {
            executions.push((idx, order.sell_amount, effective_buy));
        }
    }

    executions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
    use chrono::Utc;

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
    fn test_clearing_prices_empty() {
        let prices = compute_clearing_prices(&[]);
        assert!(prices.is_empty());
    }

    #[test]
    fn test_clearing_prices_with_matching_pair() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(50)),
            make_order(token_b, token_a, dec!(50), dec!(100)),
        ];

        let prices = compute_clearing_prices(&orders);
        assert!(!prices.is_empty());
        assert!(prices.contains_key(&token_a));
        assert!(prices.contains_key(&token_b));
    }

    #[test]
    fn test_optimize_execution() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        let orders = vec![make_order(token_a, token_b, dec!(100), dec!(50))];

        let mut prices = HashMap::new();
        prices.insert(token_a, dec!(2));
        prices.insert(token_b, dec!(1));

        let executions = optimize_execution(&orders, &prices);
        assert_eq!(executions.len(), 1);
        let (idx, exec_sell, exec_buy) = &executions[0];
        assert_eq!(*idx, 0);
        assert_eq!(*exec_sell, dec!(100));
        // 100 * 2 / 1 = 200 >= 50 ✓
        assert_eq!(*exec_buy, dec!(200));
    }
}
