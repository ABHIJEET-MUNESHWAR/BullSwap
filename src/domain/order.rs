use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Strongly-typed order identifier — newtype over UUID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct OrderUid(pub Uuid);

impl OrderUid {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OrderUid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for OrderUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for OrderUid {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

/// Whether the trader wants to buy or sell a fixed amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum OrderKind {
    /// The trader specifies the exact amount they want to sell.
    Sell,
    /// The trader specifies the exact amount they want to buy.
    Buy,
}

impl fmt::Display for OrderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderKind::Sell => write!(f, "sell"),
            OrderKind::Buy => write!(f, "buy"),
        }
    }
}

/// Lifecycle status of an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum OrderStatus {
    /// Order is open and eligible for the next batch.
    Open,
    /// Order has been assigned to a batch and is being solved.
    Matched,
    /// Order has been settled as part of a winning solution.
    Settled,
    /// Order was cancelled by the user.
    Cancelled,
    /// Order expired (valid_to passed).
    Expired,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderStatus::Open => write!(f, "open"),
            OrderStatus::Matched => write!(f, "matched"),
            OrderStatus::Settled => write!(f, "settled"),
            OrderStatus::Cancelled => write!(f, "cancelled"),
            OrderStatus::Expired => write!(f, "expired"),
        }
    }
}

/// A signed order submitted by a trader.
///
/// Orders specify a token pair, amounts, and a limit price.
/// They are collected into batches and solved for optimal execution.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Order {
    /// Unique order identifier.
    pub uid: OrderUid,
    /// Address / identifier of the order owner.
    pub owner: String,
    /// Token to sell (references token.id).
    pub sell_token: Uuid,
    /// Token to buy (references token.id).
    pub buy_token: Uuid,
    /// Amount of sell_token offered.
    pub sell_amount: Decimal,
    /// Minimum amount of buy_token expected (limit price).
    pub buy_amount: Decimal,
    /// Whether the order is a buy or sell order.
    pub kind: OrderKind,
    /// Current lifecycle status.
    pub status: OrderStatus,
    /// Cryptographic signature proving ownership.
    pub signature: String,
    /// The batch this order was assigned to, if any.
    pub batch_id: Option<Uuid>,
    /// Expiration timestamp.
    pub valid_to: DateTime<Utc>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Request payload for creating a new order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOrderRequest {
    pub owner: String,
    pub sell_token: Uuid,
    pub buy_token: Uuid,
    pub sell_amount: Decimal,
    pub buy_amount: Decimal,
    pub kind: OrderKind,
    pub signature: String,
    pub valid_to: DateTime<Utc>,
}

/// Query parameters for listing orders.
#[derive(Debug, Clone, Deserialize)]
pub struct OrderQueryParams {
    pub owner: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Order {
    /// Compute the limit price for this order (sell_amount / buy_amount).
    /// For a sell order, this is the minimum price the trader accepts.
    pub fn limit_price(&self) -> Option<Decimal> {
        if self.buy_amount.is_zero() {
            return None;
        }
        Some(self.sell_amount / self.buy_amount)
    }

    /// Check if this order has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.valid_to
    }

    /// Check if this order can be matched (open and not expired).
    pub fn is_matchable(&self) -> bool {
        self.status == OrderStatus::Open && !self.is_expired()
    }
}

/// Test helpers for creating sample orders (available to all test modules).
#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use rust_decimal::Decimal;

    /// Create a sample order with specified token pair and amounts.
    pub fn make_test_order(
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_order() -> Order {
        Order {
            uid: OrderUid::new(),
            owner: "0xAlice".to_string(),
            sell_token: Uuid::new_v4(),
            buy_token: Uuid::new_v4(),
            sell_amount: dec!(100),
            buy_amount: dec!(50),
            kind: OrderKind::Sell,
            status: OrderStatus::Open,
            signature: "sig_abc".to_string(),
            batch_id: None,
            valid_to: Utc::now() + chrono::Duration::hours(1),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_limit_price() {
        let order = sample_order();
        assert_eq!(order.limit_price(), Some(dec!(2)));
    }

    #[test]
    fn test_limit_price_zero_buy() {
        let mut order = sample_order();
        order.buy_amount = dec!(0);
        assert_eq!(order.limit_price(), None);
    }

    #[test]
    fn test_is_matchable() {
        let order = sample_order();
        assert!(order.is_matchable());
    }

    #[test]
    fn test_is_matchable_cancelled() {
        let mut order = sample_order();
        order.status = OrderStatus::Cancelled;
        assert!(!order.is_matchable());
    }

    #[test]
    fn test_is_expired() {
        let mut order = sample_order();
        order.valid_to = Utc::now() - chrono::Duration::hours(1);
        assert!(order.is_expired());
        assert!(!order.is_matchable());
    }

    #[test]
    fn test_order_uid_display() {
        let uid = OrderUid::new();
        let display = format!("{}", uid);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_order_kind_display() {
        assert_eq!(format!("{}", OrderKind::Sell), "sell");
        assert_eq!(format!("{}", OrderKind::Buy), "buy");
    }
}
