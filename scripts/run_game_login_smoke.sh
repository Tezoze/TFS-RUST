#!/usr/bin/env bash
# Free the game port (7171) from earlier smoke runs, then start game_login_smoke.
# Usage: from repo root: ./scripts/run_game_login_smoke.sh

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if command -v fuser >/dev/null 2>&1; then
  fuser -k -n tcp 7171 2>/dev/null && echo "Stopped process(es) on TCP 7171." || true
  fuser -k -n tcp 7172 2>/dev/null && echo "Stopped process(es) on TCP 7172." || true
else
  echo "Tip: install psmisc for fuser, or run: ss -tlnp | grep -E '7171|7172'  and kill PIDs manually."
fi
sleep 0.25

if ss -tlnp 2>/dev/null | grep -qE ':7171\s'; then
  echo "Warning: something is still listening on 7171 — check: ss -tlnp | grep 7171"
fi
if ss -tlnp 2>/dev/null | grep -qE ':7172\s'; then
  echo "Warning: something is still listening on 7172 — check: ss -tlnp | grep 7172"
fi

exec cargo run -p tfs-rust-net --example game_login_smoke
