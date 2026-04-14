use sqlx::PgPool;
use uuid::Uuid;

use crate::db::settlement_repo::SettlementRepo;
use crate::domain::order::OrderUid;
use crate::domain::settlement::SettlementDetails;
use crate::errors::{AppError, AppResult};
use crate::solver::engine::SolveResult;

/// Service for settlement persistence and retrieval.
pub struct SettlementService;

impl SettlementService {
    /// Persist the winning settlement from a solver result.
    pub async fn persist_settlement(
        pool: &PgPool,
        result: &SolveResult,
        batch_id: Uuid,
    ) -> AppResult<()> {
        let trades: Vec<(OrderUid, rust_decimal::Decimal, rust_decimal::Decimal, rust_decimal::Decimal)> =
            result
                .settlement
                .trades
                .iter()
                .map(|t| (t.order_uid, t.executed_sell, t.executed_buy, t.surplus))
                .collect();

        let clearing_prices: Vec<(Uuid, rust_decimal::Decimal)> = result
            .settlement
            .clearing_prices
            .iter()
            .map(|cp| (cp.token_id, cp.price))
            .collect();

        SettlementRepo::insert_full(
            pool,
            batch_id,
            result.solver_id,
            result.settlement.settlement.objective_value,
            result.settlement.settlement.surplus_total,
            &trades,
            &clearing_prices,
        )
        .await?;

        tracing::info!(
            batch_id = %batch_id,
            solver = %result.solver_name,
            trades = trades.len(),
            "Settlement persisted"
        );

        Ok(())
    }

    /// Get settlement details by batch ID.
    pub async fn get_by_batch_id(
        pool: &PgPool,
        batch_id: Uuid,
    ) -> AppResult<SettlementDetails> {
        SettlementRepo::find_by_batch_id(pool, batch_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Settlement for batch {} not found", batch_id))
            })
    }
}

