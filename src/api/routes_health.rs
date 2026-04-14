use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::errors::AppResult;

/// Configure health check routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health_check));
}

/// Health check endpoint.
///
/// Returns 200 if the server and database are healthy.
/// Returns 503 if the database is unreachable.
///
/// # Response
/// ```json
/// {
///   "status": "ok",
///   "version": "0.1.0",
///   "database": "connected"
/// }
/// ```
async fn health_check(pool: web::Data<PgPool>) -> AppResult<HttpResponse> {
    let db_status = match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool.get_ref())
        .await
    {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    let is_healthy = db_status == "connected";

    let body = serde_json::json!({
        "status": if is_healthy { "ok" } else { "degraded" },
        "version": env!("CARGO_PKG_VERSION"),
        "database": db_status,
    });

    if is_healthy {
        Ok(HttpResponse::Ok().json(body))
    } else {
        Ok(HttpResponse::ServiceUnavailable().json(body))
    }
}

