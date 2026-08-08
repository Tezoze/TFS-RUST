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

/// Decide whether the baseline row in `_sqlx_migrations` needs its checksum/description
/// re-synced to the current migration file.
///
/// The baseline is an "adopt existing schema" marker (DDL never re-run), so its stored
/// checksum is bookkeeping only. When the file changes after adoption, SQLx's strict
/// checksum guard would reject `migrator.run`; this returns `true` so the caller can
/// UPDATE the row first. Pure (no I/O) so it can be unit-tested without a live DB.
fn baseline_needs_resync(
    stored_checksum: &[u8],
    stored_description: &str,
    baseline: &sqlx::migrate::Migration,
) -> bool {
    stored_checksum != baseline.checksum.as_ref()
        || stored_description != baseline.description.as_ref()
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

    let baseline = migrator
        .iter()
        .find(|m| m.version == BASELINE_VERSION)
        .ok_or_else(|| {
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
        // The baseline is an "adopt existing schema" marker — its DDL is never re-run
        // (the schema already exists from legacy C++ TFS), so the checksum is bookkeeping
        // only. Re-sync it to the current file so a baseline edit/revert doesn't trip
        // SQLx's strict checksum guard in `migrator.run`. Normal migrations stay strict.
        let row: Option<(Vec<u8>, String)> = sqlx::query_as(
            "SELECT checksum, description FROM _sqlx_migrations WHERE version = ?",
        )
        .bind(BASELINE_VERSION)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| TfsRustError::Database(e.to_string()))?;

        let need_resync = row
            .as_ref()
            .map(|(stored, desc)| baseline_needs_resync(stored, desc, baseline))
            .unwrap_or(false);

        if need_resync {
            sqlx::query(
                "UPDATE _sqlx_migrations SET checksum = ?, description = ? WHERE version = ?",
            )
            .bind(baseline.checksum.as_ref())
            .bind(baseline.description.as_ref())
            .bind(BASELINE_VERSION)
            .execute(pool.inner())
            .await
            .map_err(|e| TfsRustError::Database(e.to_string()))?;
            info!(
                version = BASELINE_VERSION,
                "existing TFS schema detected; re-synced baseline checksum to current file"
            );
        } else {
            info!("existing TFS schema detected; baseline migration already recorded");
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::MigrationType;
    use std::borrow::Cow;

    /// Build a `Migration` with the given checksum and description for testing.
    fn fake_baseline(checksum: &[u8], description: &str) -> sqlx::migrate::Migration {
        sqlx::migrate::Migration {
            version: BASELINE_VERSION,
            description: Cow::Owned(description.to_owned()),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed("-- baseline"),
            checksum: Cow::Owned(checksum.to_vec()),
        }
    }

    #[test]
    fn resync_required_when_checksum_differs() {
        let baseline = fake_baseline(&[1, 2, 3], "tfs_142_baseline");
        // Stored checksum differs from the file → must resync.
        assert!(baseline_needs_resync(&[9, 9, 9], "tfs_142_baseline", &baseline));
    }

    #[test]
    fn resync_required_when_description_differs() {
        let baseline = fake_baseline(&[1, 2, 3], "tfs_142_baseline");
        // Same checksum, different description → must resync.
        assert!(baseline_needs_resync(&[1, 2, 3], "renamed_baseline", &baseline));
    }

    #[test]
    fn no_resync_when_checksum_and_description_match() {
        let baseline = fake_baseline(&[1, 2, 3], "tfs_142_baseline");
        // Exact match → no resync needed.
        assert!(!baseline_needs_resync(&[1, 2, 3], "tfs_142_baseline", &baseline));
    }

    #[test]
    fn no_resync_for_empty_checksum_does_not_panic() {
        // Edge case: empty stored checksum (shouldn't happen, but must not panic).
        let baseline = fake_baseline(&[1, 2, 3], "tfs_142_baseline");
        assert!(baseline_needs_resync(&[], "tfs_142_baseline", &baseline));
    }
}
