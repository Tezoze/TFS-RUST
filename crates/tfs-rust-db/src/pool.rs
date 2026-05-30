use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::time::Duration;
use tfs_rust_common::error::{Result, TfsRustError};
use tokio::time::sleep;
use tracing::{info, warn};

/// Options for [`MySqlPoolOptions`] (shared with TFS `config.lua` `mysqlConnection*` pool keys).
#[derive(Debug, Clone)]
pub struct DbPoolConnectOptions {
    pub min_connections: u32,
    pub max_connections: u32,
    /// How long a connection may sit idle in the pool before it is closed (`None` = no idle limit).
    pub idle_timeout: Option<Duration>,
    /// Max time to wait for a free connection from the pool.
    pub acquire_timeout: Duration,
}

impl Default for DbPoolConnectOptions {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 5,
            idle_timeout: Some(Duration::from_secs(300)),
            acquire_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Clone)]
pub struct DbPool {
    pool: MySqlPool,
}

impl DbPool {
    /// Connect using explicit pool tuning (e.g. from `config.lua` `mysqlConnectionPoolSize`, …).
    pub async fn connect(url: &str, options: &DbPoolConnectOptions) -> Result<Self> {
        let min_connections = options.min_connections.min(options.max_connections);
        let mut builder = MySqlPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(options.max_connections)
            .acquire_timeout(options.acquire_timeout);
        builder = builder.idle_timeout(options.idle_timeout);
        let pool = builder
            .connect(url)
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        let idle_label = options
            .idle_timeout
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|| "none".to_string());
        info!(
            min = min_connections,
            max = options.max_connections,
            idle_timeout_secs = %idle_label,
            acquire_timeout_secs = options.acquire_timeout.as_secs(),
            "Connected to MariaDB (pooled)"
        );
        Ok(Self { pool })
    }

    pub fn inner(&self) -> &MySqlPool {
        &self.pool
    }

    /// Lazy pool for unit tests that never execute SQL.
    #[doc(hidden)]
    pub fn lazy_for_tests() -> Result<Self> {
        let pool = MySqlPoolOptions::new()
            .connect_lazy("mysql://127.0.0.1:3306/tfs_rust_unit_test_unused")
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
        Ok(Self { pool })
    }

    /// Execute a closure that returns a Future with exponential backoff on transient errors.
    pub async fn execute_with_retry<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, sqlx::Error>>,
    {
        let backoffs = [100, 200, 400];
        let mut attempts = 0;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempts >= backoffs.len() {
                        return Err(TfsRustError::Database(e.to_string()));
                    }

                    let is_transient = if let sqlx::Error::Database(ref db_err) = e {
                        let code = db_err.code();
                        let code_str = code.as_deref().unwrap_or("0");
                        matches!(code_str, "1213" | "1205") // 1213: Deadlock, 1205: Lock wait timeout
                    } else {
                        matches!(
                            e,
                            sqlx::Error::Io(_)
                                | sqlx::Error::PoolTimedOut
                                | sqlx::Error::PoolClosed
                                | sqlx::Error::Tls(_)
                        )
                    };

                    if !is_transient {
                        return Err(TfsRustError::Database(e.to_string()));
                    }

                    let delay = backoffs[attempts];
                    warn!(
                        "Transient database error ({}), retrying in {}ms... (Attempt {}/{})",
                        e,
                        delay,
                        attempts + 1,
                        backoffs.len() + 1
                    );
                    sleep(Duration::from_millis(delay)).await;
                    attempts += 1;
                }
            }
        }
    }
}
