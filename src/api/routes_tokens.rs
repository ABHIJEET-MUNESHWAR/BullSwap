use actix_web::{web, HttpResponse};
use sqlx::PgPool;

use crate::db::token_repo::TokenRepo;
use crate::domain::token::CreateTokenRequest;
use crate::errors::AppResult;

/// Configure token routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tokens")
            .route("", web::get().to(list_tokens))
            .route("", web::post().to(create_token)),
    );
}

/// GET /v1/tokens — List all registered tokens.
#[tracing::instrument(name = "list_tokens", skip(pool))]
async fn list_tokens(pool: web::Data<PgPool>) -> AppResult<HttpResponse> {
    let tokens = TokenRepo::find_all(pool.get_ref()).await?;
    Ok(HttpResponse::Ok().json(tokens))
}

/// POST /v1/tokens — Register a new token.
#[tracing::instrument(name = "create_token", skip(pool, body))]
async fn create_token(
    pool: web::Data<PgPool>,
    body: web::Json<CreateTokenRequest>,
) -> AppResult<HttpResponse> {
    let token = TokenRepo::insert(
        pool.get_ref(),
        &body.symbol,
        &body.name,
        body.decimals,
        &body.address,
    )
    .await?;
    Ok(HttpResponse::Created().json(token))
}

