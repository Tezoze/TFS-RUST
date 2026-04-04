use crate::pool::DbPool;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

pub async fn run_migrations(pool: &DbPool, path: &Path) -> Result<()> {
    info!("Running database migrations from {:?}", path);

    let migrator = sqlx::migrate::Migrator::new(path)
        .await
        .map_err(|e| TfsRustError::Database(format!("Failed to load migrations: {}", e)))?;

    migrator
        .run(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(format!("Failed to apply migrations: {}", e)))?;

    info!("Database migrations applied successfully.");
    Ok(())
}
