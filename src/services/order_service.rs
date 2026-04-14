use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::db::order_repo::OrderRepo;
use crate::db::token_repo::TokenRepo;
use crate::domain::order::{CreateOrderRequest, Order, OrderStatus, OrderUid};
use crate::errors::{AppError, AppResult};

/// Service for order business logic and validation.
pub struct OrderService;

impl OrderService {
    /// Create and persist a new order after validation.
    ///
    /// # Validation Rules
    /// - sell_amount > 0
    /// - buy_amount > 0
    /// - sell_token ≠ buy_token
    /// - valid_to is in the future
    /// - sell_token and buy_token must exist
    /// - signature must not be empty
    /// - owner must not be empty
    pub async fn create_order(
        pool: &PgPool,
        req: CreateOrderRequest,
    ) -> AppResult<Order> {
        // Validate
        Self::validate_order(&req)?;

        // Check tokens exist
        if !TokenRepo::exists(pool, req.sell_token).await? {
            return Err(AppError::Validation(format!(
                "Sell token {} does not exist",
                req.sell_token
            )));
        }
        if !TokenRepo::exists(pool, req.buy_token).await? {
            return Err(AppError::Validation(format!(
                "Buy token {} does not exist",
                req.buy_token
            )));
        }

        let uid = OrderUid::new();

        tracing::info!(
            uid = %uid,
            owner = %req.owner,
            sell_token = %req.sell_token,
            buy_token = %req.buy_token,
            sell_amount = %req.sell_amount,
            buy_amount = %req.buy_amount,
            "Creating new order"
        );

        let order = OrderRepo::insert(
            pool,
            uid,
            &req.owner,
            req.sell_token,
            req.buy_token,
            req.sell_amount,
            req.buy_amount,
            req.kind,
            &req.signature,
            req.valid_to,
        )
        .await?;

        Ok(order)
    }

    /// Get an order by UID.
    pub async fn get_order(pool: &PgPool, uid: OrderUid) -> AppResult<Order> {
        OrderRepo::find_by_uid(pool, uid)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Order {} not found", uid)))
    }

    /// Cancel an open order.
    pub async fn cancel_order(pool: &PgPool, uid: OrderUid) -> AppResult<()> {
        let order = Self::get_order(pool, uid).await?;

        if order.status != OrderStatus::Open {
            return Err(AppError::Validation(format!(
                "Cannot cancel order with status '{}'",
                order.status
            )));
        }

        let cancelled = OrderRepo::cancel(pool, uid).await?;
        if !cancelled {
            return Err(AppError::Internal("Failed to cancel order".to_string()));
        }

        tracing::info!(uid = %uid, "Order cancelled");
        Ok(())
    }

    /// List orders with optional filters.
    pub async fn list_orders(
        pool: &PgPool,
        owner: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Order>> {
        // Validate status filter
        if let Some(s) = status {
            match s {
                "open" | "matched" | "settled" | "cancelled" | "expired" => {}
                _ => {
                    return Err(AppError::Validation(format!(
                        "Invalid status filter: {}. Must be one of: open, matched, settled, cancelled, expired",
                        s
                    )));
                }
            }
        }

        let limit = limit.clamp(1, 100);
        let offset = offset.max(0);

        OrderRepo::list(pool, owner, status, limit, offset).await
    }

    /// Validate an order creation request.
    fn validate_order(req: &CreateOrderRequest) -> AppResult<()> {
        if req.owner.trim().is_empty() {
            return Err(AppError::Validation("Owner cannot be empty".to_string()));
        }

        if req.sell_amount <= Decimal::ZERO {
            return Err(AppError::Validation(
                "sell_amount must be greater than 0".to_string(),
            ));
        }

        if req.buy_amount <= Decimal::ZERO {
            return Err(AppError::Validation(
                "buy_amount must be greater than 0".to_string(),
            ));
        }

        if req.sell_token == req.buy_token {
            return Err(AppError::Validation(
                "sell_token and buy_token must be different".to_string(),
            ));
        }

        if req.valid_to <= Utc::now() {
            return Err(AppError::Validation(
                "valid_to must be in the future".to_string(),
            ));
        }

        if req.signature.trim().is_empty() {
            return Err(AppError::Validation(
                "Signature cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::order::OrderKind;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn valid_request() -> CreateOrderRequest {
        CreateOrderRequest {
            owner: "0xAlice".to_string(),
            sell_token: Uuid::new_v4(),
            buy_token: Uuid::new_v4(),
            sell_amount: dec!(100),
            buy_amount: dec!(50),
            kind: OrderKind::Sell,
            signature: "valid_sig".to_string(),
            valid_to: Utc::now() + chrono::Duration::hours(1),
        }
    }

    #[test]
    fn test_validate_valid_order() {
        let req = valid_request();
        assert!(OrderService::validate_order(&req).is_ok());
    }

    #[test]
    fn test_validate_empty_owner() {
        let mut req = valid_request();
        req.owner = "".to_string();
        assert!(OrderService::validate_order(&req).is_err());
    }

    #[test]
    fn test_validate_zero_sell_amount() {
        let mut req = valid_request();
        req.sell_amount = dec!(0);
        assert!(OrderService::validate_order(&req).is_err());
    }

    #[test]
    fn test_validate_negative_buy_amount() {
        let mut req = valid_request();
        req.buy_amount = dec!(-10);
        assert!(OrderService::validate_order(&req).is_err());
    }

    #[test]
    fn test_validate_same_tokens() {
        let mut req = valid_request();
        req.buy_token = req.sell_token;
        assert!(OrderService::validate_order(&req).is_err());
    }

    #[test]
    fn test_validate_expired_order() {
        let mut req = valid_request();
        req.valid_to = Utc::now() - chrono::Duration::hours(1);
        assert!(OrderService::validate_order(&req).is_err());
    }

    #[test]
    fn test_validate_empty_signature() {
        let mut req = valid_request();
        req.signature = "".to_string();
        assert!(OrderService::validate_order(&req).is_err());
    }
}

