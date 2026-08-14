#!/usr/bin/env bash
# Wait for MariaDB TCP (Compose sets TFS_DB_WAIT_HOST=db), then exec the server.
set -euo pipefail

wait_host="${TFS_DB_WAIT_HOST:-}"
wait_port="${TFS_DB_WAIT_PORT:-3306}"
wait_tries="${TFS_DB_WAIT_TRIES:-60}"

if [[ -n "${wait_host}" ]]; then
  echo "waiting for ${wait_host}:${wait_port} (${wait_tries}s max)" >&2
  for _ in $(seq 1 "${wait_tries}"); do
    if bash -c "echo >/dev/tcp/${wait_host}/${wait_port}" 2>/dev/null; then
      exec /usr/local/bin/tfs-rust "$@"
    fi
    sleep 1
  done
  echo "timeout waiting for ${wait_host}:${wait_port}" >&2
  exit 1
fi

exec /usr/local/bin/tfs-rust "$@"
