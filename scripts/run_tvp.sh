#!/usr/bin/env bash
# Run TVP 7.72 gameserver (CWD = gameserver/, loads ./config.lua).
#
# Usage from repo root:
#   ./scripts/build_tvp.sh && ./scripts/run_tvp.sh
#
# Defaults: dedicated MariaDB `test_tvp` (SHA1). Rust bcrypt lives on `TFS`.
# Override with TVP_MYSQL_*. Create/seed: ./scripts/setup_tvp_db.sh
#   TFS_BENCH_DISABLE_SAVES=1  → enablePlayerDataFiles=false, enableMapDataFiles=false
#   TFS_TVP_772_DIR            → override reference/tvp-772
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=lib/reference_paths.sh
. "$ROOT/scripts/lib/reference_paths.sh"
reference_paths_init "$ROOT"

lua_str() {
  local key="$1" file="$2"
  local line
  line="$(grep -E "^${key}[[:space:]]*=" "$file" 2>/dev/null | tail -n1 || true)"
  line="${line#*=}"
  line="${line%%--*}"
  line="$(echo "$line" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' -e 's/,$//')"
  if [[ "$line" == \"*\" ]]; then
    line="${line:1:-1}"
  fi
  printf '%s' "$line"
}

GS="${TVP772}/gameserver"
BIN="${GS}/build/tfs"
if [[ ! -x "$BIN" ]]; then
  echo "run_tvp: missing $BIN — run ./scripts/build_tvp.sh" >&2
  exit 1
fi

if [[ ! -f "$GS/config.lua" ]]; then
  cp "$GS/config.lua.dist" "$GS/config.lua"
  echo "run_tvp: copied config.lua.dist → config.lua" >&2
fi

set_lua() {
  local key="$1" val="$2"
  if grep -qE "^${key}[[:space:]]*=" "$GS/config.lua"; then
    sed -i -E "s|^${key}[[:space:]]*=.*|${key} = ${val}|" "$GS/config.lua"
  else
    printf '\n%s = %s\n' "$key" "$val" >> "$GS/config.lua"
  fi
}

RUST_CFG="${TFS_CONFIG:-$ROOT/config.lua}"
MYSQL_HOST="${TVP_MYSQL_HOST:-$(lua_str mysqlHost "$RUST_CFG")}"
MYSQL_USER="${TVP_MYSQL_USER:-$(lua_str mysqlUser "$RUST_CFG")}"
MYSQL_PASS="${TVP_MYSQL_PASS:-$(lua_str mysqlPass "$RUST_CFG")}"
# Not the Rust `TFS` schema: TVP compares SHA1; Rust upgrades logins to bcrypt.
MYSQL_DB="${TVP_MYSQL_DATABASE:-test_tvp}"
MYSQL_PORT="${TVP_MYSQL_PORT:-$(lua_str mysqlPort "$RUST_CFG")}"
MYSQL_SOCK="${TVP_MYSQL_SOCK:-/run/mysqld/mysqld.sock}"

set_lua mysqlHost "\"${MYSQL_HOST:-127.0.0.1}\""
set_lua mysqlUser "\"${MYSQL_USER:-tfs}\""
set_lua mysqlPass "\"${MYSQL_PASS:-}\""
set_lua mysqlDatabase "\"${MYSQL_DB}\""
set_lua mysqlPort "${MYSQL_PORT:-3306}"
set_lua mysqlSock "\"${MYSQL_SOCK}\""
echo "run_tvp: mysql ${MYSQL_USER}@${MYSQL_HOST}/${MYSQL_DB} sock=${MYSQL_SOCK}" >&2

"$ROOT/scripts/sync_tvp_world.sh"

if [[ "${TFS_BENCH_DISABLE_SAVES:-0}" == "1" ]]; then
  set_lua enablePlayerDataFiles false
  set_lua enableMapDataFiles false
  echo "run_tvp: TFS_BENCH_DISABLE_SAVES=1 (player/map data files off)" >&2
fi

echo "run_tvp: ensuring TVP schema on ${MYSQL_DB}" >&2
TVP_MYSQL_HOST="$MYSQL_HOST" TVP_MYSQL_USER="$MYSQL_USER" TVP_MYSQL_PASS="$MYSQL_PASS" \
  TVP_MYSQL_DATABASE="$MYSQL_DB" "$ROOT/scripts/setup_tvp_db.sh"

if command -v fuser >/dev/null 2>&1; then
  fuser -k -n tcp 7171 2>/dev/null && echo "run_tvp: freed tcp/7171" || true
  fuser -k -n tcp 7172 2>/dev/null && echo "run_tvp: freed tcp/7172" || true
  sleep 0.25
fi

echo "run_tvp: cwd=$GS exec $BIN" >&2
echo "run_tvp: note daily ServerSave at 04:30 shuts the process down (globalevents.xml)" >&2
cd "$GS"
exec "$BIN"
