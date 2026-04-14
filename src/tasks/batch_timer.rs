use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::services::batch_service::BatchService;

/// Background task that periodically closes the current batch and triggers solving.
///
/// Runs on a configurable interval (default: 30 seconds).
/// Each tick:
/// 1. Closes the current collecting batch
/// 2. Runs the solver competition
/// 3. Persists the winning settlement
/// 4. Opens a new collecting batch
pub async fn run_batch_timer(
    pool: PgPool,
    interval_secs: u64,
    max_orders: i64,
    solver_ids: Vec<(Uuid, String)>,
) {
    let interval = Duration::from_secs(interval_secs);

    tracing::info!(
        interval_secs = interval_secs,
        max_orders = max_orders,
        "Batch timer started"
    );

    // Ensure there's a collecting batch on startup
    if let Err(e) = BatchService::ensure_collecting_batch(&pool).await {
        tracing::error!(error = %e, "Failed to create initial collecting batch");
    }

    loop {
        tokio::time::sleep(interval).await;

        tracing::debug!("Batch timer tick");

        match BatchService::close_and_solve(&pool, max_orders, &solver_ids).await {
            Ok(Some(batch_id)) => {
                tracing::info!(batch_id = %batch_id, "Batch cycle completed");
            }
            Ok(None) => {
                tracing::debug!("No orders to process in this batch cycle");
            }
            Err(e) => {
                tracing::error!(error = %e, "Batch cycle failed");
                // Recovery: ensure a collecting batch exists
                if let Err(e) = BatchService::ensure_collecting_batch(&pool).await {
                    tracing::error!(error = %e, "Failed to recover collecting batch");
                }
            }
        }
    }
}

