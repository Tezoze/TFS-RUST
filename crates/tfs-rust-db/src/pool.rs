use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use std::time::Duration;
use tfs_rust_common::error::{Result, TfsRustError};
use tokio::time::sleep;
use tracing::{info, warn};

#[derive(Clone)]
pub struct DbPool {
    pool: MySqlPool,
}

impl DbPool {
    pub async fn connect(url: &str, min_connections: u32, max_connections: u32) -> Result<Self> {
        let min_connections = min_connections.min(max_connections);
        let pool = MySqlPoolOptions::new()
            .min_connections(min_connections)
            .max_connections(max_connections)
            .connect(url)
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;

        info!("Connected to MariaDB successfully.");
        Ok(Self { pool })
    }

    pub fn inner(&self) -> &MySqlPool {
        &self.pool
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
