use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Lifecycle status of a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "lowercase")]
pub enum BatchStatus {
    /// Batch is collecting orders.
    Collecting,
    /// Batch is being solved by the solver competition.
    Solving,
    /// Batch has been settled with a winning solution.
    Settled,
    /// Batch failed to settle (no valid solution found).
    Failed,
}

impl fmt::Display for BatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchStatus::Collecting => write!(f, "collecting"),
            BatchStatus::Solving => write!(f, "solving"),
            BatchStatus::Settled => write!(f, "settled"),
            BatchStatus::Failed => write!(f, "failed"),
        }
    }
}

/// A batch of orders to be solved together.
///
/// Batches are time-bounded: orders are collected during a window,
/// then the batch is closed and solvers compete to find the best settlement.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Batch {
    pub id: Uuid,
    pub status: BatchStatus,
    pub created_at: DateTime<Utc>,
    pub solved_at: Option<DateTime<Utc>>,
    pub settled_at: Option<DateTime<Utc>>,
    pub order_count: i64,
}

impl Batch {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            status: BatchStatus::Collecting,
            created_at: Utc::now(),
            solved_at: None,
            settled_at: None,
            order_count: 0,
        }
    }

    pub fn is_collecting(&self) -> bool {
        self.status == BatchStatus::Collecting
    }
}

impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_batch() {
        let batch = Batch::new();
        assert_eq!(batch.status, BatchStatus::Collecting);
        assert!(batch.is_collecting());
        assert!(batch.solved_at.is_none());
        assert!(batch.settled_at.is_none());
        assert_eq!(batch.order_count, 0);
    }

    #[test]
    fn test_batch_status_display() {
        assert_eq!(format!("{}", BatchStatus::Collecting), "collecting");
        assert_eq!(format!("{}", BatchStatus::Solving), "solving");
        assert_eq!(format!("{}", BatchStatus::Settled), "settled");
        assert_eq!(format!("{}", BatchStatus::Failed), "failed");
    }
}
