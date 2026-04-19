use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppResult;
use crate::services::settlement_service::SettlementService;

/// Configure settlement routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/settlements").route("/{batch_id}", web::get().to(get_settlement)));
}

/// GET /v1/settlements/{batch_id} — Get settlement details by batch ID.
///
/// Returns the winning settlement including all trades and clearing prices.
#[tracing::instrument(name = "get_settlement", skip(pool))]
async fn get_settlement(
    pool: web::Data<PgPool>,
    batch_id: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let details = SettlementService::get_by_batch_id(pool.get_ref(), *batch_id).await?;
    Ok(HttpResponse::Ok().json(details))
}
