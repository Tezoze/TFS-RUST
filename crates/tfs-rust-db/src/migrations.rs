//! Sqlx migrator. Default migration set ships in `crates/tfs-rust-db/migrations/` (TFS 1.4.2 baseline).
// C++ reference: schema parity with `schema.sql` at repository root.

use crate::pool::DbPool;
use std::path::{Path, PathBuf};
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

/// Built-in migrations directory (sqlx versioned `*.sql` files).
pub fn default_migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

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
