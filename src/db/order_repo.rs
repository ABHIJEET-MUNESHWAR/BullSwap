use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::order::{Order, OrderKind, OrderStatus, OrderUid};
use crate::errors::AppError;

/// Repository for order operations.
pub struct OrderRepo;

impl OrderRepo {
    /// Insert a new order.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        pool: &PgPool,
        uid: OrderUid,
        owner: &str,
        sell_token: Uuid,
        buy_token: Uuid,
        sell_amount: Decimal,
        buy_amount: Decimal,
        kind: OrderKind,
        signature: &str,
        valid_to: DateTime<Utc>,
    ) -> Result<Order, AppError> {
        let order = sqlx::query_as::<_, Order>(
            r#"
            INSERT INTO orders (uid, owner, sell_token, buy_token, sell_amount, buy_amount, kind, status, signature, valid_to, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'open', $8, $9, NOW())
            RETURNING uid, owner, sell_token, buy_token, sell_amount, buy_amount,
                      kind AS "kind: OrderKind", status AS "status: OrderStatus",
                      signature, batch_id, valid_to, created_at
            "#,
        )
        .bind(uid.0)
        .bind(owner)
        .bind(sell_token)
        .bind(buy_token)
        .bind(sell_amount)
        .bind(buy_amount)
        .bind(kind.to_string())
        .bind(signature)
        .bind(valid_to)
        .fetch_one(pool)
        .await?;
        Ok(order)
    }

    /// Find an order by its UID.
    pub async fn find_by_uid(pool: &PgPool, uid: OrderUid) -> Result<Option<Order>, AppError> {
        let order = sqlx::query_as::<_, Order>(
            r#"
            SELECT uid, owner, sell_token, buy_token, sell_amount, buy_amount,
                   kind AS "kind: OrderKind", status AS "status: OrderStatus",
                   signature, batch_id, valid_to, created_at
            FROM orders WHERE uid = $1
            "#,
        )
        .bind(uid.0)
        .fetch_optional(pool)
        .await?;
        Ok(order)
    }

    /// List orders with optional filters.
    pub async fn list(
        pool: &PgPool,
        owner: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Order>, AppError> {
        let orders = sqlx::query_as::<_, Order>(
            r#"
            SELECT uid, owner, sell_token, buy_token, sell_amount, buy_amount,
                   kind AS "kind: OrderKind", status AS "status: OrderStatus",
                   signature, batch_id, valid_to, created_at
            FROM orders
            WHERE ($1::TEXT IS NULL OR owner = $1)
              AND ($2::TEXT IS NULL OR status = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(owner)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(orders)
    }

    /// Find all open orders not yet assigned to a batch.
    pub async fn find_open_unassigned(pool: &PgPool, limit: i64) -> Result<Vec<Order>, AppError> {
        let orders = sqlx::query_as::<_, Order>(
            r#"
            SELECT uid, owner, sell_token, buy_token, sell_amount, buy_amount,
                   kind AS "kind: OrderKind", status AS "status: OrderStatus",
                   signature, batch_id, valid_to, created_at
            FROM orders
            WHERE status = 'open' AND batch_id IS NULL AND valid_to > NOW()
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(orders)
    }

    /// Assign orders to a batch.
    pub async fn assign_to_batch(
        pool: &PgPool,
        order_uids: &[Uuid],
        batch_id: Uuid,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE orders
            SET batch_id = $1, status = 'matched'
            WHERE uid = ANY($2) AND status = 'open'
            "#,
        )
        .bind(batch_id)
        .bind(order_uids)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Update the status of an order.
    pub async fn update_status(
        pool: &PgPool,
        uid: OrderUid,
        status: OrderStatus,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET status = $1 WHERE uid = $2",
        )
        .bind(status.to_string())
        .bind(uid.0)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Cancel an order (only if it's still open).
    pub async fn cancel(pool: &PgPool, uid: OrderUid) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET status = 'cancelled' WHERE uid = $1 AND status = 'open'",
        )
        .bind(uid.0)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update status of all orders in a batch.
    pub async fn update_batch_orders_status(
        pool: &PgPool,
        batch_id: Uuid,
        status: OrderStatus,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET status = $1 WHERE batch_id = $2",
        )
        .bind(status.to_string())
        .bind(batch_id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Expire orders past their valid_to timestamp.
    pub async fn expire_orders(pool: &PgPool) -> Result<u64, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET status = 'expired' WHERE status = 'open' AND valid_to <= NOW()",
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}

