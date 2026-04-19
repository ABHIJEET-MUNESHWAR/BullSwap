use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::batch::{Batch, BatchStatus};
use crate::errors::AppError;

/// Repository for batch operations.
pub struct BatchRepo;

impl BatchRepo {
    /// Create a new batch in collecting state.
    pub async fn create(pool: &PgPool) -> Result<Batch, AppError> {
        let batch = sqlx::query_as::<_, Batch>(
            r#"
            INSERT INTO batches (id, status, created_at, order_count)
            VALUES ($1, 'collecting', NOW(), 0)
            RETURNING id, status AS "status: BatchStatus", created_at, solved_at, settled_at, order_count
            "#,
        )
        .bind(Uuid::new_v4())
        .fetch_one(pool)
        .await?;
        Ok(batch)
    }

    /// Find a batch by ID.
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Batch>, AppError> {
        let batch = sqlx::query_as::<_, Batch>(
            r#"
            SELECT id, status AS "status: BatchStatus", created_at, solved_at, settled_at, order_count
            FROM batches WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(batch)
    }

    /// Get the current collecting batch (most recent with 'collecting' status).
    pub async fn get_current_collecting(pool: &PgPool) -> Result<Option<Batch>, AppError> {
        let batch = sqlx::query_as::<_, Batch>(
            r#"
            SELECT id, status AS "status: BatchStatus", created_at, solved_at, settled_at, order_count
            FROM batches
            WHERE status = 'collecting'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(pool)
        .await?;
        Ok(batch)
    }

    /// Update batch status.
    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: BatchStatus,
    ) -> Result<bool, AppError> {
        let result = if status == BatchStatus::Settled {
            let now = Utc::now();
            sqlx::query("UPDATE batches SET status = $1, settled_at = $2 WHERE id = $3")
                .bind(status.to_string())
                .bind(now)
                .bind(id)
                .execute(pool)
                .await?
        } else {
            sqlx::query("UPDATE batches SET status = $1 WHERE id = $2")
                .bind(status.to_string())
                .bind(id)
                .execute(pool)
                .await?
        };
        Ok(result.rows_affected() > 0)
    }

    /// Set the solved_at timestamp and update order count.
    pub async fn mark_solved(pool: &PgPool, id: Uuid, order_count: i64) -> Result<bool, AppError> {
        let result =
            sqlx::query("UPDATE batches SET solved_at = NOW(), order_count = $1 WHERE id = $2")
                .bind(order_count)
                .bind(id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// List recent batches.
    pub async fn list_recent(
        pool: &PgPool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Batch>, AppError> {
        let batches = sqlx::query_as::<_, Batch>(
            r#"
            SELECT id, status AS "status: BatchStatus", created_at, solved_at, settled_at, order_count
            FROM batches
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(batches)
    }
}
