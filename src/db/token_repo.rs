use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::token::Token;
use crate::errors::AppError;

/// Repository for token operations.
pub struct TokenRepo;

impl TokenRepo {
    /// Find all tokens.
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Token>, AppError> {
        let tokens = sqlx::query_as::<_, Token>(
            "SELECT id, symbol, name, decimals, address FROM tokens ORDER BY symbol"
        )
        .fetch_all(pool)
        .await?;
        Ok(tokens)
    }

    /// Find a token by its ID.
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Token>, AppError> {
        let token = sqlx::query_as::<_, Token>(
            "SELECT id, symbol, name, decimals, address FROM tokens WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(token)
    }

    /// Find a token by its address.
    pub async fn find_by_address(pool: &PgPool, address: &str) -> Result<Option<Token>, AppError> {
        let token = sqlx::query_as::<_, Token>(
            "SELECT id, symbol, name, decimals, address FROM tokens WHERE address = $1"
        )
        .bind(address)
        .fetch_optional(pool)
        .await?;
        Ok(token)
    }

    /// Insert a new token.
    pub async fn insert(
        pool: &PgPool,
        symbol: &str,
        name: &str,
        decimals: i16,
        address: &str,
    ) -> Result<Token, AppError> {
        let token = sqlx::query_as::<_, Token>(
            r#"
            INSERT INTO tokens (id, symbol, name, decimals, address)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, symbol, name, decimals, address
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(symbol)
        .bind(name)
        .bind(decimals)
        .bind(address)
        .fetch_one(pool)
        .await?;
        Ok(token)
    }

    /// Check if a token exists by ID.
    pub async fn exists(pool: &PgPool, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tokens WHERE id = $1)"
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(result)
    }
}

