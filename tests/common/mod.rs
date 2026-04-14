use sqlx::PgPool;
use std::net::TcpListener;

/// Spawn the application on a random port for integration testing.
///
/// Returns (base_url, pool) so tests can make HTTP requests and verify DB state.
#[allow(dead_code)]
pub async fn spawn_app() -> (String, PgPool) {
    // Find a random available port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // Free the port

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://bullswap:bullswap@localhost:5432/bullswap_test".to_string());

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let base_url = format!("http://127.0.0.1:{}", port);
    (base_url, pool)
}

/// Clean up test data after tests.
#[allow(dead_code)]
pub async fn cleanup(pool: &PgPool) {
    // Clean in reverse FK order
    let _ = sqlx::query("DELETE FROM clearing_prices").execute(pool).await;
    let _ = sqlx::query("DELETE FROM trades").execute(pool).await;
    let _ = sqlx::query("DELETE FROM settlements").execute(pool).await;
    let _ = sqlx::query("DELETE FROM orders").execute(pool).await;
    let _ = sqlx::query("DELETE FROM batches").execute(pool).await;
}

