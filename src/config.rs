use std::env;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// PostgreSQL connection string.
    pub database_url: String,
    /// Server host (e.g., "0.0.0.0").
    pub host: String,
    /// Server port (e.g., 8080).
    pub port: u16,
    /// Interval in seconds between batch settlements.
    pub batch_interval_secs: u64,
    /// Log level filter (e.g., "info", "debug").
    pub log_level: String,
    /// Optional API key for authenticated endpoints.
    pub api_key: Option<String>,
    /// Maximum number of orders per batch.
    pub max_orders_per_batch: usize,
    /// Number of worker threads for solver parallelism.
    pub solver_threads: usize,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// Falls back to sensible defaults for optional values.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Attempt to load .env file, ignore if missing
        let _ = dotenvy::dotenv();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::Missing("DATABASE_URL".to_string()))?;

        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|_| ConfigError::Invalid("PORT".to_string(), "must be a valid u16".to_string()))?;

        let batch_interval_secs = env::var("BATCH_INTERVAL_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|_| ConfigError::Invalid(
                "BATCH_INTERVAL_SECS".to_string(),
                "must be a valid u64".to_string(),
            ))?;

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let api_key = env::var("API_KEY").ok();

        let max_orders_per_batch = env::var("MAX_ORDERS_PER_BATCH")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<usize>()
            .map_err(|_| ConfigError::Invalid(
                "MAX_ORDERS_PER_BATCH".to_string(),
                "must be a valid usize".to_string(),
            ))?;

        let solver_threads = env::var("SOLVER_THREADS")
            .unwrap_or_else(|_| num_cpus().to_string())
            .parse::<usize>()
            .map_err(|_| ConfigError::Invalid(
                "SOLVER_THREADS".to_string(),
                "must be a valid usize".to_string(),
            ))?;

        Ok(AppConfig {
            database_url,
            host,
            port,
            batch_interval_secs,
            log_level,
            api_key,
            max_orders_per_batch,
            solver_threads,
        })
    }

    /// Returns the full server bind address.
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Get the number of available CPU cores.
pub(crate) fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    Missing(String),
    #[error("Invalid value for {0}: {1}")]
    Invalid(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_addr() {
        let config = AppConfig {
            database_url: "postgres://localhost/test".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3000,
            batch_interval_secs: 30,
            log_level: "info".to_string(),
            api_key: None,
            max_orders_per_batch: 1000,
            solver_threads: 4,
        };
        assert_eq!(config.server_addr(), "127.0.0.1:3000");
    }

    #[test]
    fn test_num_cpus() {
        let cpus = num_cpus();
        assert!(cpus >= 1);
    }
}

