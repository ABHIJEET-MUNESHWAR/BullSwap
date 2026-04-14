pub mod middleware;
pub mod routes_batches;
pub mod routes_health;
pub mod routes_orders;
pub mod routes_settlements;
pub mod routes_tokens;

use actix_web::web;

/// Configure all API routes.
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .configure(routes_orders::configure)
            .configure(routes_batches::configure)
            .configure(routes_settlements::configure)
            .configure(routes_tokens::configure),
    )
    .configure(routes_health::configure);
}

