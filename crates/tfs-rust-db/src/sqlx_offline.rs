//! Compile-time `query!` hooks for `cargo sqlx prepare` and `SQLX_OFFLINE=true` builds.
// C++ N/A — tooling only. Task: `.kiro/specs/ausera-rust-engine/tasks.md` Phase 2.2b.
//
// Runtime queries elsewhere use `sqlx::query` / `query_as` and do not emit offline metadata.
// This module keeps at least one `query!` so the `.sqlx/` cache stays valid for CI without a DB.
//
// Regenerate `.sqlx/` after changing any `query!` / `query_as!`:
// `DATABASE_URL='mysql://USER:PASS@HOST:PORT/DB' cargo sqlx prepare --workspace -- --workspace`
// (The second `--workspace` is forwarded to `cargo check` so every workspace crate is compiled.)

/// Never called; exists so the `query!` macro is compiled and recorded by `cargo sqlx prepare`.
#[allow(dead_code)]
pub(crate) fn _sqlx_offline_metadata_anchor() {
    let _ = sqlx::query!("SELECT 1 as `ok`");
}
