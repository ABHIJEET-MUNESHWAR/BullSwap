use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::batch_repo::BatchRepo;
use crate::errors::{AppError, AppResult};

/// Configure batch routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/batches")
            .route("", web::get().to(list_batches))
            .route("/{id}", web::get().to(get_batch)),
    );
}

/// Query parameters for batch listing.
#[derive(Debug, serde::Deserialize)]
pub struct BatchQueryParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /v1/batches — List recent batches.
#[tracing::instrument(name = "list_batches", skip(pool))]
async fn list_batches(
    pool: web::Data<PgPool>,
    query: web::Query<BatchQueryParams>,
) -> AppResult<HttpResponse> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let batches = BatchRepo::list_recent(pool.get_ref(), limit, offset).await?;
    Ok(HttpResponse::Ok().json(batches))
}

/// GET /v1/batches/{id} — Get batch by ID.
#[tracing::instrument(name = "get_batch", skip(pool))]
async fn get_batch(pool: web::Data<PgPool>, id: web::Path<Uuid>) -> AppResult<HttpResponse> {
    let batch = BatchRepo::find_by_id(pool.get_ref(), *id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Batch {} not found", id)))?;

    Ok(HttpResponse::Ok().json(batch))
}
