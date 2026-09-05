#!/usr/bin/env bash
# Capture a `tfs_obs` baseline for docs/GAME_LOOP_OBS_BASELINES.md.
#
# Differs from run_server.sh in two ways that matter for timing numbers:
#   1. --release. run_server.sh runs a debug build; its beat/subsystem timings are
#      not a performance baseline.
#   2. RUST_LOG enables tfs_obs (default binary filter is tfs_obs=off).
#
# Usage from repo root:
#   ./scripts/capture_obs_baseline.sh dense_spawn
#
# Then play the scenario for at least 90s (30s warmup + 60s measured), Ctrl-C, and:
#   scripts/parse_obs_log.py /tmp/tfs_obs/dense_spawn.log --scenario "Dense spawn" --skip 3
#
# Env passthrough is the same as run_server.sh (DATABASE_URL, TFS_DATA_DIR, TFS_CONFIG, …).

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SCENARIO="${1:-baseline}"
OUT_DIR="${TFS_OBS_DIR:-/tmp/tfs_obs}"
mkdir -p "$OUT_DIR"
LOG="$OUT_DIR/${SCENARIO}.log"

if [[ -n "${DATABASE_URL:-}" ]]; then
  echo "capture_obs: using DATABASE_URL from environment" >&2
else
  echo "capture_obs: DATABASE_URL not set — using MySQL keys from \${TFS_CONFIG:-config.lua}" >&2
fi

if command -v fuser >/dev/null 2>&1; then
  fuser -k -n tcp 7171 2>/dev/null && echo "Stopped process(es) on TCP 7171." || true
  fuser -k -n tcp 7172 2>/dev/null && echo "Stopped process(es) on TCP 7172." || true
else
  echo "Tip: install psmisc for fuser, or free 7171/7172 manually." >&2
fi
sleep 0.25

echo "capture_obs: building release (first build is slow; later runs are cached)" >&2
cargo build --release --bin tfs-rust

{
  echo "# scenario: $SCENARIO"
  echo "# captured: $(date -Is)"
  echo "# build: release"
  echo "# git: $(git rev-parse --short HEAD)$(git diff --quiet || echo '-dirty')"
  echo "# host: $(uname -sr) / $(nproc) cpus"
  command -v lscpu >/dev/null 2>&1 && lscpu | sed -n 's/^Model name:[[:space:]]*/# cpu: /p'
} > "$LOG"

echo "capture_obs: logging to $LOG — play the scenario, then Ctrl-C" >&2
# `cargo run` rather than a hardcoded target/release path: CARGO_TARGET_DIR may be redirected.
RUST_LOG="${RUST_LOG:-info,tfs_obs=info}" cargo run --release --bin tfs-rust 2>&1 | tee -a "$LOG"
