//! Sqlx migrator. Default migration set ships in `crates/tfs-rust-db/migrations/` (TFS 1.4.2 baseline).
// C++ reference: schema parity with `schema.sql` at repository root.

use crate::pool::DbPool;
use sqlx::migrate::Migrator;
use std::path::{Path, PathBuf};
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

/// Baseline schema migration version (`20240101000000_tfs_142_baseline.sql`).
const BASELINE_VERSION: i64 = 20240101000000;

/// Built-in migrations directory (sqlx versioned `*.sql` files).
pub fn default_migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

/// Resolve SQLx migrations at runtime — avoids stale `CARGO_MANIFEST_DIR` when the repo is
/// moved or mounted at a different path than at last compile of `tfs-rust-db`.
///
/// Search order: `TFS_MIGRATIONS_DIR` → compile-time crate path (if present) →
/// `crates/tfs-rust-db/migrations` under cwd → workspace-root sibling of cwd.
pub fn resolve_migrations_dir() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("TFS_MIGRATIONS_DIR") {
        let path = PathBuf::from(p);
        if path.is_dir() {
            return Ok(path);
        }
        return Err(TfsRustError::Database(format!(
            "TFS_MIGRATIONS_DIR={} is not a directory",
            path.display()
        )));
    }

    let baked = default_migrations_dir();
    if baked.is_dir() {
        return Ok(baked);
    }

    let from_cwd = PathBuf::from("crates/tfs-rust-db/migrations");
    if from_cwd.is_dir() {
        return Ok(from_cwd);
    }

    if let Ok(cwd) = std::env::current_dir() {
        let from_parent = cwd.join("../crates/tfs-rust-db/migrations");
        if let Ok(canonical) = from_parent.canonicalize() {
            if canonical.is_dir() {
                return Ok(canonical);
            }
        }
    }

    Err(TfsRustError::Database(format!(
        "SQLx migrations directory not found.\n\
         Expected at {} (compile-time path — rebuild with `cargo clean -p tfs-rust-db` if the repo moved),\n\
         or crates/tfs-rust-db/migrations relative to cwd,\n\
         or set TFS_MIGRATIONS_DIR to the directory containing *.sql migration files.",
        baked.display()
    )))
}

async fn table_exists(pool: &DbPool, name: &str) -> Result<bool> {
    let found: Option<i32> = sqlx::query_scalar(
        r#"
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = DATABASE() AND table_name = ?
        LIMIT 1
        "#,
    )
    .bind(name)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;
    Ok(found.is_some())
}

/// Drop failed SQLx rows and, for legacy C++ TFS databases, mark baseline applied without re-running DDL.
async fn heal_and_adopt_existing_schema(pool: &DbPool, migrator: &Migrator) -> Result<()> {
    if table_exists(pool, "_sqlx_migrations").await? {
        sqlx::query("DELETE FROM _sqlx_migrations WHERE success = 0")
            .execute(pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
    }

    if !table_exists(pool, "accounts").await? {
        return Ok(());
    }

    let baseline = migrator.iter().find(|m| m.version == BASELINE_VERSION).ok_or_else(|| {
        TfsRustError::Database(format!(
            "baseline migration {BASELINE_VERSION} missing from migrator"
        ))
    })?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS `_sqlx_migrations` (
            `version` bigint NOT NULL PRIMARY KEY,
            `description` text NOT NULL,
            `installed_on` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
            `success` boolean NOT NULL,
            `checksum` blob NOT NULL,
            `execution_time` bigint NOT NULL
        )
        "#,
    )
    .execute(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;

    let baseline_applied: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM _sqlx_migrations WHERE version = ? AND success = 1 LIMIT 1",
    )
    .bind(BASELINE_VERSION)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;

    if baseline_applied.is_some() {
        info!("existing TFS schema detected; baseline migration already recorded");
        return Ok(());
    }

    sqlx::query(
        r#"
        INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
        VALUES (?, ?, TRUE, ?, 0)
        "#,
    )
    .bind(baseline.version)
    .bind(baseline.description.as_ref())
    .bind(baseline.checksum.as_ref())
    .execute(pool.inner())
    .await
    .map_err(|e| TfsRustError::Database(e.to_string()))?;

    info!(
        version = BASELINE_VERSION,
        "existing TFS/C++ schema detected; adopted baseline migration without re-running DDL"
    );
    Ok(())
}

pub async fn run_migrations(pool: &DbPool, path: &Path) -> Result<()> {
    info!("Running database migrations from {:?}", path);

    let migrator = Migrator::new(path)
        .await
        .map_err(|e| TfsRustError::Database(format!("Failed to load migrations: {e}")))?;

    heal_and_adopt_existing_schema(pool, &migrator).await?;

    migrator
        .run(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(format!("Failed to apply migrations: {e}")))?;

    info!("Database migrations applied successfully.");
    Ok(())
}
