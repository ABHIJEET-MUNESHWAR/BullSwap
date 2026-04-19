use actix_web::{web, App, HttpServer};
use tracing_actix_web::TracingLogger;

use crate::api;
use crate::config::{num_cpus, AppConfig};
use crate::db::pool;
use crate::db::solver_repo::SolverRepo;
use crate::tasks::batch_timer;

/// Build and run the BullSwap server.
///
/// This function:
/// 1. Creates the database connection pool
/// 2. Runs migrations
/// 3. Spawns the batch timer background task
/// 4. Starts the Actix Web HTTP server
pub async fn run_server(config: AppConfig) -> std::io::Result<()> {
    // Create database pool
    let db_pool = pool::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    pool::run_migrations(&db_pool)
        .await
        .expect("Failed to run database migrations");

    // Get solver IDs for the batch timer
    let solvers = SolverRepo::find_active(&db_pool).await.unwrap_or_default();
    let solver_ids: Vec<(uuid::Uuid, String)> =
        solvers.iter().map(|s| (s.id, s.name.clone())).collect();

    // Initialize Rayon thread pool for parallel solver execution
    rayon::ThreadPoolBuilder::new()
        .num_threads(config.solver_threads)
        .build_global()
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to configure Rayon thread pool, using defaults");
        });

    tracing::info!(
        solver_threads = config.solver_threads,
        "Rayon thread pool configured"
    );

    // Spawn batch timer
    let timer_pool = db_pool.clone();
    let batch_interval = config.batch_interval_secs;
    let max_orders = config.max_orders_per_batch as i64;
    let timer_solver_ids = solver_ids.clone();

    tokio::spawn(async move {
        batch_timer::run_batch_timer(timer_pool, batch_interval, max_orders, timer_solver_ids)
            .await;
    });

    let addr = config.server_addr();
    tracing::info!(addr = %addr, "Starting BullSwap server");

    // Build and start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::JsonConfig::default().limit(1024 * 64)) // 64KB limit
            .configure(api::configure_routes)
    })
    .bind(&addr)?
    .workers(num_cpus())
    .run()
    .await
}
