use actix_web::{HttpResponse, ResponseError};
use std::fmt;

/// Central error type for the BullSwap application.
///
/// Maps to appropriate HTTP status codes via `ResponseError`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Validation error (400 Bad Request).
    #[error("Validation error: {0}")]
    Validation(String),

    /// Resource not found (404 Not Found).
    #[error("Not found: {0}")]
    NotFound(String),

    /// Duplicate / conflict (409 Conflict).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Database error (500 Internal Server Error).
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Internal server error (500).
    #[error("Internal error: {0}")]
    Internal(String),

    /// Unauthorized (401).
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Rate limited (429).
    #[error("Rate limited: {0}")]
    RateLimited(String),
}

/// JSON error response body.
#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let (status, error_type) = match self {
            AppError::Validation(_) => {
                (actix_web::http::StatusCode::BAD_REQUEST, "validation_error")
            }
            AppError::NotFound(_) => (actix_web::http::StatusCode::NOT_FOUND, "not_found"),
            AppError::Conflict(_) => (actix_web::http::StatusCode::CONFLICT, "conflict"),
            AppError::Database(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
            ),
            AppError::Internal(_) => (
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
            ),
            AppError::Unauthorized(_) => {
                (actix_web::http::StatusCode::UNAUTHORIZED, "unauthorized")
            }
            AppError::RateLimited(_) => (
                actix_web::http::StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
            ),
        };

        tracing::error!(
            error_type = error_type,
            message = %self,
            "Request failed"
        );

        HttpResponse::build(status).json(ErrorResponse {
            error: error_type.to_string(),
            message: self.to_string(),
        })
    }
}

/// Convenience type alias for Results using AppError.
pub type AppResult<T> = Result<T, AppError>;

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display() {
        let err = AppError::Validation("sell_amount must be positive".to_string());
        assert!(err.to_string().contains("sell_amount must be positive"));
    }

    #[test]
    fn test_not_found_error_display() {
        let err = AppError::NotFound("Order abc-123".to_string());
        assert!(err.to_string().contains("Order abc-123"));
    }

    #[test]
    fn test_error_response_type() {
        let err = AppError::Internal("something broke".to_string());
        let resp = err.error_response();
        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
