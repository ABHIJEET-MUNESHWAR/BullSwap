use bullswap::config::AppConfig;
use bullswap::startup::run_server;
use bullswap::telemetry::init_telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let config = AppConfig::from_env().expect("Failed to load configuration");

    // Initialize telemetry
    init_telemetry(&config.log_level);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        host = %config.host,
        port = config.port,
        batch_interval = config.batch_interval_secs,
        "🐂 BullSwap starting up"
    );

    // Run the server
    run_server(config).await
}
