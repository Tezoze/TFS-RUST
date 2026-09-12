#!/usr/bin/env bash
# Create a dedicated MariaDB for TVP (SHA1 passwords). Rust keeps bcrypt on `TFS`.
#
# `tfs` cannot CREATE DATABASE `TVP` (ALL is only on `TFS`.*). MariaDB PUBLIC
# grants allow `test_%`, so the default name is `test_tvp`. Override with
# TVP_MYSQL_DATABASE if you grant a better name.
#
# Usage from repo root:
#   ./scripts/setup_tvp_db.sh
#   TVP_SEED_COUNT=600 ./scripts/setup_tvp_db.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=lib/reference_paths.sh
. "$ROOT/scripts/lib/reference_paths.sh"
reference_paths_init "$ROOT"

DB="${TVP_MYSQL_DATABASE:-test_tvp}"
USER="${TVP_MYSQL_USER:-tfs}"
HOST="${TVP_MYSQL_HOST:-127.0.0.1}"
PASS="${TVP_MYSQL_PASS:-}"
SEED_COUNT="${TVP_SEED_COUNT:-10}"
SCHEMA="${TVP772}/gameserver/schema.sql"

if [[ ! -f "$SCHEMA" ]]; then
  echo "setup_tvp_db: missing $SCHEMA" >&2
  exit 1
fi

mysql() {
  local extra=()
  if [[ -n "$PASS" ]]; then
    extra+=(-p"$PASS")
  fi
  mariadb --skip-ssl -h "$HOST" -u "$USER" "${extra[@]}" "$@"
}

echo "setup_tvp_db: CREATE DATABASE \`${DB}\`" >&2
mysql -e "CREATE DATABASE IF NOT EXISTS \`${DB}\` CHARACTER SET utf8;"

tables="$(mysql "$DB" -N -e "SHOW TABLES LIKE 'accounts';" || true)"
if [[ -z "$tables" ]]; then
  echo "setup_tvp_db: importing $SCHEMA → ${DB}" >&2
  # Dist dump targets `tibia`; rewrite so we never touch that name or `TFS`.
  sed -e '/CREATE DATABASE IF NOT EXISTS `tibia`/d' -e "s/USE \`tibia\`;/USE \`${DB}\`;/" "$SCHEMA" \
    | mysql
else
  echo "setup_tvp_db: schema already present on ${DB}" >&2
fi

echo "setup_tvp_db: seeding ${SEED_COUNT} SHA1 accounts on ${DB}" >&2
TVP_MYSQL_HOST="$HOST" TVP_MYSQL_USER="$USER" TVP_MYSQL_PASS="$PASS" TVP_MYSQL_DATABASE="$DB" \
  python3 "$ROOT/scripts/seed_bench_accounts.py" --count "$SEED_COUNT" --apply --target tvp

echo "setup_tvp_db: ok ${USER}@${HOST}/${DB}" >&2
