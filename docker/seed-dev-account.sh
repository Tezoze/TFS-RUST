#!/bin/sh
# Wait for SQLx migrations (`accounts` table), then insert the dev account if missing.
set -eu
host="${TFS_DB_WAIT_HOST:-db}"
user="${MARIADB_USER:-forgottenserver}"
pass="${MARIADB_PASSWORD:-forgottenserver}"
database="${MARIADB_DATABASE:-forgottenserver}"

i=0
while [ "$i" -lt 60 ]; do
  if mariadb -h "$host" -u "$user" -p"$pass" "$database" -N -e "SHOW TABLES LIKE 'accounts'" 2>/dev/null | grep -q accounts; then
    mariadb -h "$host" -u "$user" -p"$pass" "$database" < /seed.sql
    echo "seed: account 1 / password 1 / God, Master Sorcerer, Elder Druid, Royal Paladin, Elite Knight"
    exit 0
  fi
  i=$((i + 1))
  sleep 2
done
echo "seed: timeout waiting for accounts table (tfs migrations not finished?)" >&2
exit 1
