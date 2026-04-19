use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::order::Order;

/// A matched pair of orders forming a Coincidence of Wants.
///
/// When two traders want to swap opposite tokens, they can be matched
/// directly without needing external liquidity. This is the most
/// efficient form of trade execution.
#[derive(Debug, Clone)]
pub struct CowMatch {
    /// The sell-side order.
    pub sell_order_uid: Uuid,
    /// The buy-side order.
    pub buy_order_uid: Uuid,
    /// Amount of token A transferred from seller to buyer.
    pub amount_a: Decimal,
    /// Amount of token B transferred from buyer to seller.
    pub amount_b: Decimal,
    /// Surplus generated for the sell-side order.
    pub surplus_sell: Decimal,
    /// Surplus generated for the buy-side order.
    pub surplus_buy: Decimal,
}

/// Result of the CoW finding algorithm.
#[derive(Debug, Clone)]
pub struct CowFinderResult {
    /// Successfully matched order pairs.
    pub matches: Vec<CowMatch>,
    /// Orders that could not be matched (remain for external liquidity).
    pub unmatched_order_indices: Vec<usize>,
    /// Total surplus generated from CoW matches.
    pub total_surplus: Decimal,
}

/// Find Coincidence of Wants among a set of orders.
///
/// Groups orders by opposing token pairs (A→B and B→A) and attempts
/// to match them directly at overlapping limit prices.
///
/// # Algorithm
/// 1. Group orders by (sell_token, buy_token) pair
/// 2. For each pair (A→B), look for counter-orders (B→A)
/// 3. Sort by limit price: sellers ascending, buyers descending
/// 4. Greedily match overlapping orders
///
/// # Time Complexity
/// - O(n log n) for sorting orders per pair
/// - O(n) for matching within each pair
/// - Overall: O(n log n) where n is the total number of orders
pub fn find_cows(orders: &[Order]) -> CowFinderResult {
    if orders.is_empty() {
        return CowFinderResult {
            matches: vec![],
            unmatched_order_indices: vec![],
            total_surplus: dec!(0),
        };
    }

    // Group orders by (sell_token, buy_token) pair
    // Key: (sell_token, buy_token) → Vec<(index, &Order)>
    let mut pair_groups: HashMap<(Uuid, Uuid), Vec<(usize, &Order)>> = HashMap::new();

    for (idx, order) in orders.iter().enumerate() {
        if !order.is_matchable() {
            continue;
        }
        pair_groups
            .entry((order.sell_token, order.buy_token))
            .or_default()
            .push((idx, order));
    }

    let mut all_matches = Vec::new();
    let mut total_surplus = dec!(0);

    // Track which indices have been matched
    let mut is_matched = vec![false; orders.len()];

    // For each pair (A→B), find counter-pair (B→A)
    let keys: Vec<(Uuid, Uuid)> = pair_groups.keys().cloned().collect();
    for (sell_tok, buy_tok) in &keys {
        let counter_key = (*buy_tok, *sell_tok);

        // Only process each pair once (A→B and B→A together)
        if sell_tok >= buy_tok {
            continue;
        }

        let forward = match pair_groups.get(&(*sell_tok, *buy_tok)) {
            Some(v) => v.clone(),
            None => continue,
        };
        let backward = match pair_groups.get(&counter_key) {
            Some(v) => v.clone(),
            None => continue,
        };

        // Forward orders (A→B): sell A, want B
        // seller_price = sell_amount / buy_amount = max price of B in A the seller will pay
        // Higher = more generous seller (willing to pay more A per B)
        // Sort DESCENDING so most generous sellers come first
        let mut sellers: Vec<_> = forward
            .iter()
            .filter(|(idx, _)| !is_matched[*idx])
            .filter(|(_, order)| !order.buy_amount.is_zero())
            .map(|(idx, order)| {
                let price = order.sell_amount / order.buy_amount;
                (*idx, *order, price)
            })
            .collect();
        sellers.sort_by(|a, b| b.2.cmp(&a.2));

        // Backward orders (B→A): sell B, want A
        // buyer_price = buy_amount / sell_amount = min price of B in A the buyer demands
        // Lower = less demanding buyer (accepts less A per B)
        // Sort ASCENDING so least demanding buyers come first
        let mut buyers: Vec<_> = backward
            .iter()
            .filter(|(idx, _)| !is_matched[*idx])
            .filter(|(_, order)| !order.sell_amount.is_zero())
            .map(|(idx, order)| {
                let price = order.buy_amount / order.sell_amount;
                (*idx, *order, price)
            })
            .collect();
        buyers.sort_by(|a, b| a.2.cmp(&b.2));

        // Greedily match overlapping orders
        let mut si = 0;
        let mut bi = 0;

        while si < sellers.len() && bi < buyers.len() {
            let (s_idx, seller, seller_price) = &sellers[si];
            let (b_idx, buyer, buyer_price) = &buyers[bi];

            // Match when seller's max price >= buyer's min price
            if seller_price < buyer_price {
                break; // No more overlapping since both are sorted optimally
            }

            // Determine clearing price (midpoint of overlapping range)
            let clearing_price = (*seller_price + *buyer_price) / dec!(2);

            // Determine matched amounts
            let seller_available_a = seller.sell_amount;
            let buyer_available_a = buyer.buy_amount; // buyer wants this much A
            let matched_a = seller_available_a.min(buyer_available_a);

            // Amount of B at clearing price
            let matched_b = matched_a / clearing_price;

            // Calculate surplus
            // Seller surplus: they offered sell_amount A for buy_amount B,
            // but got matched_b for matched_a where matched_b may be > (matched_a * buy_amount/sell_amount)
            let seller_expected_b = matched_a * seller.buy_amount / seller.sell_amount;
            let surplus_sell = (matched_b - seller_expected_b).max(dec!(0));

            // Buyer surplus: they offered sell_amount B for buy_amount A,
            // but the effective price is better
            let buyer_expected_b = matched_a * buyer.sell_amount / buyer.buy_amount;
            let surplus_buy = (buyer_expected_b - matched_b).max(dec!(0));

            all_matches.push(CowMatch {
                sell_order_uid: seller.uid.0,
                buy_order_uid: buyer.uid.0,
                amount_a: matched_a,
                amount_b: matched_b,
                surplus_sell,
                surplus_buy,
            });

            total_surplus += surplus_sell + surplus_buy;

            is_matched[*s_idx] = true;
            is_matched[*b_idx] = true;

            si += 1;
            bi += 1;
        }
    }

    // Collect unmatched indices
    let unmatched: Vec<usize> = (0..orders.len())
        .filter(|i| !is_matched[*i] && orders[*i].is_matchable())
        .collect();

    CowFinderResult {
        matches: all_matches,
        unmatched_order_indices: unmatched,
        total_surplus,
    }
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
    fn test_empty_orders() {
        let result = find_cows(&[]);
        assert!(result.matches.is_empty());
        assert_eq!(result.total_surplus, dec!(0));
    }

    #[test]
    fn test_single_order_no_match() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();
        let orders = vec![make_order(token_a, token_b, dec!(100), dec!(50))];
        let result = find_cows(&orders);
        assert!(result.matches.is_empty());
        assert_eq!(result.unmatched_order_indices.len(), 1);
    }

    #[test]
    fn test_perfect_cow_match() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        // Alice: sell 100 A, want at least 50 B (price: 2 A per B)
        // Bob: sell 50 B, want at least 100 A (price: 2 A per B)
        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(50)),
            make_order(token_b, token_a, dec!(50), dec!(100)),
        ];

        let result = find_cows(&orders);
        assert_eq!(result.matches.len(), 1);
        assert!(result.unmatched_order_indices.is_empty());
    }

    #[test]
    fn test_overlapping_prices_generate_surplus() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        // Alice: sell 100 A, want at least 40 B (willing to pay 2.5 A per B)
        // Bob: sell 60 B, want at least 100 A (willing to accept ~1.67 A per B)
        // Prices overlap → surplus generated
        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(40)),
            make_order(token_b, token_a, dec!(60), dec!(100)),
        ];

        let result = find_cows(&orders);
        assert_eq!(result.matches.len(), 1);
        assert!(result.total_surplus > dec!(0));
    }

    #[test]
    fn test_no_match_when_prices_dont_overlap() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        // Alice: sell 100 A, want at least 200 B (price: 0.5 A per B) — very expensive
        // Bob: sell 50 B, want at least 200 A (price: 4 A per B) — wants too much
        let orders = vec![
            make_order(token_a, token_b, dec!(100), dec!(200)),
            make_order(token_b, token_a, dec!(50), dec!(200)),
        ];

        let result = find_cows(&orders);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn test_expired_orders_not_matched() {
        let token_a = Uuid::new_v4();
        let token_b = Uuid::new_v4();

        let mut order = make_order(token_a, token_b, dec!(100), dec!(50));
        order.valid_to = Utc::now() - chrono::Duration::hours(1); // expired

        let orders = vec![order, make_order(token_b, token_a, dec!(50), dec!(100))];

        let result = find_cows(&orders);
        assert!(result.matches.is_empty());
    }
}
