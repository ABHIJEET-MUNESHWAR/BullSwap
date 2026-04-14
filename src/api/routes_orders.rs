use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::order::{CreateOrderRequest, OrderQueryParams, OrderUid};
use crate::errors::AppResult;
use crate::services::order_service::OrderService;

/// Configure order routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/orders")
            .route("", web::post().to(create_order))
            .route("", web::get().to(list_orders))
            .route("/{uid}", web::get().to(get_order))
            .route("/{uid}", web::delete().to(cancel_order)),
    );
}

/// POST /v1/orders — Submit a new order.
///
/// # Request Body
/// ```json
/// {
///   "owner": "0xAlice",
///   "sell_token": "uuid",
///   "buy_token": "uuid",
///   "sell_amount": "100.0",
///   "buy_amount": "50.0",
///   "kind": "sell",
///   "signature": "sig_hex",
///   "valid_to": "2025-12-31T23:59:59Z"
/// }
/// ```
///
/// # Response
/// 201 Created with the order details.
#[tracing::instrument(
    name = "create_order",
    skip(pool, body),
    fields(owner = %body.owner)
)]
async fn create_order(
    pool: web::Data<PgPool>,
    body: web::Json<CreateOrderRequest>,
) -> AppResult<HttpResponse> {
    let order = OrderService::create_order(pool.get_ref(), body.into_inner()).await?;
    Ok(HttpResponse::Created().json(order))
}

/// GET /v1/orders/{uid} — Get order by UID.
#[tracing::instrument(name = "get_order", skip(pool))]
async fn get_order(
    pool: web::Data<PgPool>,
    uid: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    let order = OrderService::get_order(pool.get_ref(), OrderUid(*uid)).await?;
    Ok(HttpResponse::Ok().json(order))
}

/// GET /v1/orders — List orders with optional filters.
///
/// # Query Parameters
/// - `owner` — Filter by owner address
/// - `status` — Filter by status (open, matched, settled, cancelled, expired)
/// - `limit` — Max results (default: 20, max: 100)
/// - `offset` — Pagination offset (default: 0)
#[tracing::instrument(name = "list_orders", skip(pool))]
async fn list_orders(
    pool: web::Data<PgPool>,
    query: web::Query<OrderQueryParams>,
) -> AppResult<HttpResponse> {
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let orders = OrderService::list_orders(
        pool.get_ref(),
        query.owner.as_deref(),
        query.status.as_deref(),
        limit,
        offset,
    )
    .await?;

    Ok(HttpResponse::Ok().json(orders))
}

/// DELETE /v1/orders/{uid} — Cancel an open order.
#[tracing::instrument(name = "cancel_order", skip(pool))]
async fn cancel_order(
    pool: web::Data<PgPool>,
    uid: web::Path<Uuid>,
) -> AppResult<HttpResponse> {
    OrderService::cancel_order(pool.get_ref(), OrderUid(*uid)).await?;
    Ok(HttpResponse::NoContent().finish())
}

