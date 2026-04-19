use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::batch_repo::BatchRepo;
use crate::db::order_repo::OrderRepo;
use crate::domain::batch::BatchStatus;
use crate::domain::order::OrderStatus;
use crate::errors::AppResult;
use crate::services::settlement_service::SettlementService;
use crate::solver::competition::SolverCompetition;
use crate::solver::engine::BatchSolver;
use crate::solver::naive_solver::NaiveSolver;

/// Service managing the batch auction lifecycle.
///
/// The batch lifecycle is:
/// 1. **Collecting**: Orders are accepted into the current batch
/// 2. **Solving**: Batch is closed, solvers compete to find best settlement
/// 3. **Settled**: Winning solution is persisted, order statuses updated
///
/// If no valid solution is found, the batch transitions to Failed and
/// orders return to Open status for the next batch.
pub struct BatchService;

impl BatchService {
    /// Ensure there is a collecting batch; create one if not.
    pub async fn ensure_collecting_batch(pool: &PgPool) -> AppResult<Uuid> {
        if let Some(batch) = BatchRepo::get_current_collecting(pool).await? {
            return Ok(batch.id);
        }

        let batch = BatchRepo::create(pool).await?;
        tracing::info!(batch_id = %batch.id, "Created new collecting batch");
        Ok(batch.id)
    }

    /// Close the current batch and run the solver competition.
    ///
    /// # Steps
    /// 1. Get the current collecting batch
    /// 2. Fetch all open unassigned orders
    /// 3. Assign orders to the batch
    /// 4. Transition batch to Solving
    /// 5. Run solver competition (parallel)
    /// 6. Persist winning settlement
    /// 7. Transition batch to Settled
    /// 8. Create new collecting batch
    pub async fn close_and_solve(
        pool: &PgPool,
        max_orders: i64,
        solver_ids: &[(Uuid, String)],
    ) -> AppResult<Option<Uuid>> {
        // Step 1: Get current collecting batch
        let batch = match BatchRepo::get_current_collecting(pool).await? {
            Some(b) => b,
            None => {
                tracing::debug!("No collecting batch found, creating one");
                BatchRepo::create(pool).await?
            }
        };

        let batch_id = batch.id;
        tracing::info!(batch_id = %batch_id, "Closing batch for solving");

        // Step 2: Expire stale orders
        let expired = OrderRepo::expire_orders(pool).await?;
        if expired > 0 {
            tracing::info!(count = expired, "Expired stale orders");
        }

        // Step 3: Fetch open unassigned orders
        let orders = OrderRepo::find_open_unassigned(pool, max_orders).await?;
        if orders.is_empty() {
            tracing::info!(batch_id = %batch_id, "No orders to solve, skipping");
            // Create a new batch for the next window
            BatchRepo::create(pool).await?;
            return Ok(None);
        }

        let order_count = orders.len() as i64;
        tracing::info!(
            batch_id = %batch_id,
            order_count = order_count,
            "Solving batch"
        );

        // Step 4: Assign orders to batch
        let order_uids: Vec<Uuid> = orders.iter().map(|o| o.uid.0).collect();
        OrderRepo::assign_to_batch(pool, &order_uids, batch_id).await?;

        // Step 5: Transition to Solving
        BatchRepo::update_status(pool, batch_id, BatchStatus::Solving).await?;

        // Step 6: Build solver competition
        let solvers: Vec<Arc<dyn BatchSolver>> = solver_ids
            .iter()
            .map(|(id, _name)| {
                // In a real system, different solver types would be registered
                Arc::new(NaiveSolver::new(*id)) as Arc<dyn BatchSolver>
            })
            .collect();

        let competition = SolverCompetition::new(solvers);
        let result = competition.run(&orders, batch_id);

        match result {
            Some(solve_result) => {
                // Step 7: Persist the winning settlement
                SettlementService::persist_settlement(pool, &solve_result, batch_id).await?;

                // Step 8: Mark batch as settled
                BatchRepo::mark_solved(pool, batch_id, order_count).await?;
                BatchRepo::update_status(pool, batch_id, BatchStatus::Settled).await?;

                // Update order statuses to settled
                OrderRepo::update_batch_orders_status(pool, batch_id, OrderStatus::Settled).await?;

                tracing::info!(
                    batch_id = %batch_id,
                    solver = %solve_result.solver_name,
                    score = %solve_result.score,
                    trades = solve_result.settlement.trades.len(),
                    "Batch settled successfully"
                );
            }
            None => {
                // No valid solution found
                BatchRepo::update_status(pool, batch_id, BatchStatus::Failed).await?;

                // Return orders to open status
                OrderRepo::update_batch_orders_status(pool, batch_id, OrderStatus::Open).await?;

                tracing::warn!(batch_id = %batch_id, "No valid solution found, batch failed");
            }
        }

        // Step 9: Create new collecting batch
        BatchRepo::create(pool).await?;

        Ok(Some(batch_id))
    }
}
