#!/usr/bin/env bash
# Install MariaDB (Arch/CachyOS), create TFS user/db from config.lua, load schema.sql.
# C++ reference: classic TFS `schema.sql` + `config.lua` mysql* keys.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

DB_NAME="${TFS_DB_NAME:-TFS}"
DB_USER="${TFS_DB_USER:-tfs}"
DB_PASS="${TFS_DB_PASS:-}"
DB_HOST="${TFS_DB_HOST:-127.0.0.1}"

echo "==> TFS MariaDB setup"
echo "    database: ${DB_NAME}"
echo "    user:     ${DB_USER}@${DB_HOST}"
echo "    schema:   ${ROOT}/schema.sql"

if ! command -v mariadb >/dev/null 2>&1 && ! command -v mysql >/dev/null 2>&1; then
    echo "==> Installing mariadb (requires sudo)..."
    sudo pacman -S --needed --noconfirm mariadb
fi

MYSQL_CMD=""
if command -v mariadb >/dev/null 2>&1; then
    MYSQL_CMD=mariadb
elif command -v mysql >/dev/null 2>&1; then
    MYSQL_CMD=mysql
else
    echo "error: mariadb/mysql client not found after install" >&2
    exit 1
fi

if ! systemctl is-active --quiet mariadb 2>/dev/null; then
    echo "==> Initializing / starting mariadb service (requires sudo)..."
    if [[ ! -d /var/lib/mysql/mysql ]] && command -v mariadb-install-db >/dev/null 2>&1; then
        sudo mariadb-install-db --user=mysql --basedir=/usr --datadir=/var/lib/mysql
    fi
    sudo systemctl enable --now mariadb
fi

echo "==> Creating database and user (requires sudo)..."
sudo "${MYSQL_CMD}" -u root <<EOSQL
CREATE DATABASE IF NOT EXISTS \`${DB_NAME}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER IF NOT EXISTS '${DB_USER}'@'localhost' IDENTIFIED BY '${DB_PASS}';
CREATE USER IF NOT EXISTS '${DB_USER}'@'127.0.0.1' IDENTIFIED BY '${DB_PASS}';
GRANT ALL PRIVILEGES ON \`${DB_NAME}\`.* TO '${DB_USER}'@'localhost';
GRANT ALL PRIVILEGES ON \`${DB_NAME}\`.* TO '${DB_USER}'@'127.0.0.1';
FLUSH PRIVILEGES;
EOSQL

echo "==> Loading schema.sql..."
if [[ -n "${DB_PASS}" ]]; then
    "${MYSQL_CMD}" -h "${DB_HOST}" -u "${DB_USER}" -p"${DB_PASS}" "${DB_NAME}" < "${ROOT}/schema.sql"
else
    "${MYSQL_CMD}" -h "${DB_HOST}" -u "${DB_USER}" "${DB_NAME}" < "${ROOT}/schema.sql"
fi

echo "==> Verifying tables..."
TABLE_COUNT="$(
    if [[ -n "${DB_PASS}" ]]; then
        "${MYSQL_CMD}" -h "${DB_HOST}" -u "${DB_USER}" -p"${DB_PASS}" -N -e \
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='${DB_NAME}';"
    else
        "${MYSQL_CMD}" -h "${DB_HOST}" -u "${DB_USER}" -N -e \
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='${DB_NAME}';"
    fi
)"
echo "    ${TABLE_COUNT} tables in \`${DB_NAME}\`"

echo ""
echo "Done. Start the server from repo root:"
echo "  ./scripts/run_server.sh"
echo ""
echo "Optional: set DATABASE_URL to override config.lua:"
echo "  export DATABASE_URL='mysql://${DB_USER}${DB_PASS:+:${DB_PASS}}@${DB_HOST}:3306/${DB_NAME}'"
