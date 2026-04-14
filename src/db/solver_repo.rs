use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::solver::Solver;
use crate::errors::AppError;

/// Repository for solver operations.
pub struct SolverRepo;

impl SolverRepo {
    /// Find all active solvers.
    pub async fn find_active(pool: &PgPool) -> Result<Vec<Solver>, AppError> {
        let solvers = sqlx::query_as::<_, Solver>(
            "SELECT id, name, active FROM solvers WHERE active = TRUE ORDER BY name",
        )
        .fetch_all(pool)
        .await?;
        Ok(solvers)
    }

    /// Find a solver by ID.
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Solver>, AppError> {
        let solver = sqlx::query_as::<_, Solver>(
            "SELECT id, name, active FROM solvers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(solver)
    }

    /// Find a solver by name.
    pub async fn find_by_name(pool: &PgPool, name: &str) -> Result<Option<Solver>, AppError> {
        let solver = sqlx::query_as::<_, Solver>(
            "SELECT id, name, active FROM solvers WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        Ok(solver)
    }

    /// Insert a new solver.
    pub async fn insert(pool: &PgPool, name: &str) -> Result<Solver, AppError> {
        let solver = sqlx::query_as::<_, Solver>(
            r#"
            INSERT INTO solvers (id, name, active)
            VALUES ($1, $2, TRUE)
            RETURNING id, name, active
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(name)
        .fetch_one(pool)
        .await?;
        Ok(solver)
    }
}

