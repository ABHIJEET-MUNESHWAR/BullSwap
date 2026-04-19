use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::order::OrderUid;

/// A clearing price for a token within a settlement.
///
/// In a batch auction, all trades of the same token pair receive
/// the same uniform clearing price, ensuring fairness.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClearingPrice {
    pub id: Uuid,
    pub settlement_id: Uuid,
    pub token_id: Uuid,
    /// Price denominated in the reference unit.
    pub price: Decimal,
}

/// A single trade executed as part of a settlement.
///
/// Each trade corresponds to one order and records the actual
/// amounts exchanged and the surplus returned to the user.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Trade {
    pub id: Uuid,
    pub settlement_id: Uuid,
    pub order_uid: OrderUid,
    /// Actual amount of sell token transferred.
    pub executed_sell: Decimal,
    /// Actual amount of buy token received.
    pub executed_buy: Decimal,
    /// Surplus returned to the trader (better price than limit).
    pub surplus: Decimal,
}

/// The winning settlement for a batch.
///
/// Contains the solver's solution including all trades,
/// clearing prices, and aggregate metrics.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Settlement {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub solver_id: Uuid,
    /// Objective value used to rank solutions (total surplus).
    pub objective_value: Decimal,
    /// Total surplus distributed across all trades.
    pub surplus_total: Decimal,
    pub created_at: DateTime<Utc>,
}

/// Full settlement details including trades and clearing prices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDetails {
    pub settlement: Settlement,
    pub trades: Vec<Trade>,
    pub clearing_prices: Vec<ClearingPrice>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_trade_surplus() {
        let trade = Trade {
            id: Uuid::new_v4(),
            settlement_id: Uuid::new_v4(),
            order_uid: OrderUid::new(),
            executed_sell: dec!(100),
            executed_buy: dec!(55),
            surplus: dec!(5),
        };
        assert_eq!(trade.surplus, dec!(5));
    }

    #[test]
    fn test_settlement_details() {
        let settlement = Settlement {
            id: Uuid::new_v4(),
            batch_id: Uuid::new_v4(),
            solver_id: Uuid::new_v4(),
            objective_value: dec!(10),
            surplus_total: dec!(10),
            created_at: Utc::now(),
        };
        let details = SettlementDetails {
            settlement,
            trades: vec![],
            clearing_prices: vec![],
        };
        assert!(details.trades.is_empty());
    }
}
