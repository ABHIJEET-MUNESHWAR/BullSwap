use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::order::OrderUid;
use crate::domain::settlement::{ClearingPrice, Settlement, SettlementDetails, Trade};
use crate::errors::AppError;

/// Repository for settlement operations.
pub struct SettlementRepo;

impl SettlementRepo {
    /// Insert a full settlement with trades and clearing prices (transactional).
    pub async fn insert_full(
        pool: &PgPool,
        batch_id: Uuid,
        solver_id: Uuid,
        objective_value: Decimal,
        surplus_total: Decimal,
        trades: &[(OrderUid, Decimal, Decimal, Decimal)], // (order_uid, exec_sell, exec_buy, surplus)
        clearing_prices: &[(Uuid, Decimal)],               // (token_id, price)
    ) -> Result<Settlement, AppError> {
        let mut tx = pool.begin().await?;

        let settlement_id = Uuid::new_v4();

        // Insert settlement
        let settlement = sqlx::query_as::<_, Settlement>(
            r#"
            INSERT INTO settlements (id, batch_id, solver_id, objective_value, surplus_total, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING id, batch_id, solver_id, objective_value, surplus_total, created_at
            "#,
        )
        .bind(settlement_id)
        .bind(batch_id)
        .bind(solver_id)
        .bind(objective_value)
        .bind(surplus_total)
        .fetch_one(&mut *tx)
        .await?;

        // Insert trades
        for (order_uid, exec_sell, exec_buy, surplus) in trades {
            sqlx::query(
                r#"
                INSERT INTO trades (id, settlement_id, order_uid, executed_sell, executed_buy, surplus)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(settlement_id)
            .bind(order_uid.0)
            .bind(exec_sell)
            .bind(exec_buy)
            .bind(surplus)
            .execute(&mut *tx)
            .await?;
        }

        // Insert clearing prices
        for (token_id, price) in clearing_prices {
            sqlx::query(
                r#"
                INSERT INTO clearing_prices (id, settlement_id, token_id, price)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (settlement_id, token_id) DO UPDATE SET price = $4
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(settlement_id)
            .bind(token_id)
            .bind(price)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(settlement)
    }

    /// Find settlement by batch ID, including trades and clearing prices.
    pub async fn find_by_batch_id(
        pool: &PgPool,
        batch_id: Uuid,
    ) -> Result<Option<SettlementDetails>, AppError> {
        let settlement = sqlx::query_as::<_, Settlement>(
            r#"
            SELECT id, batch_id, solver_id, objective_value, surplus_total, created_at
            FROM settlements WHERE batch_id = $1
            "#,
        )
        .bind(batch_id)
        .fetch_optional(pool)
        .await?;

        let settlement = match settlement {
            Some(s) => s,
            None => return Ok(None),
        };

        let trades = sqlx::query_as::<_, Trade>(
            r#"
            SELECT id, settlement_id, order_uid, executed_sell, executed_buy, surplus
            FROM trades WHERE settlement_id = $1
            "#,
        )
        .bind(settlement.id)
        .fetch_all(pool)
        .await?;

        let clearing_prices = sqlx::query_as::<_, ClearingPrice>(
            r#"
            SELECT id, settlement_id, token_id, price
            FROM clearing_prices WHERE settlement_id = $1
            "#,
        )
        .bind(settlement.id)
        .fetch_all(pool)
        .await?;

        Ok(Some(SettlementDetails {
            settlement,
            trades,
            clearing_prices,
        }))
    }
}

