#!/usr/bin/env bash
# Integrated server: OTBM + DB + game loop + login/game ports (7171 / 7172).
# Usage from repo root:
#   export DATABASE_URL='mysql://tfs@127.0.0.1:3306/TFS'
#   ./scripts/run_server.sh
#
# Optional: TFS_DATA_DIR (default `data` at repo root), TFS_MAP_OTBM (default world/forgotten.otbm),
# TFS_CONFIG (default config.lua), TFS_RSA_PEM, TFS_LOGIN_ADDR, TFS_GAME_ADDR

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "Set DATABASE_URL (MariaDB), e.g. export DATABASE_URL='mysql://tfs@127.0.0.1:3306/TFS'" >&2
  exit 1
fi

if command -v fuser >/dev/null 2>&1; then
  fuser -k -n tcp 7171 2>/dev/null && echo "Stopped process(es) on TCP 7171." || true
  fuser -k -n tcp 7172 2>/dev/null && echo "Stopped process(es) on TCP 7172." || true
else
  echo "Tip: install psmisc for fuser, or free 7171/7172 manually."
fi
sleep 0.25

exec cargo run --bin tfs-rust
