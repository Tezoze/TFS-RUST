#!/usr/bin/env bash
# Start / stop the full CipSoft 7.72 reference stack (QM + login + game).
#
# Usage:
#   scripts/tibia_game_online.sh start
#   scripts/tibia_game_online.sh stop
#   scripts/tibia_game_online.sh status
#   scripts/tibia_game_online.sh restart
#   scripts/tibia_game_online.sh logs [qm|login|game]
#
# Optional:
#   TIBIA_IMPORT_CHARACTER=Crowoo   import before start
#   TIBIA_GAME_ONLINE_SKIP_BUILD=1  skip build when binaries exist

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=lib/reference_paths.sh
. "$ROOT/scripts/lib/reference_paths.sh"
reference_paths_init "$ROOT"
DEV="$ROOT/scripts/tibia_game_dev.sh"
STATE_DIR="$CIPSOFT_STATE"
PID_DIR="$STATE_DIR/pids"
LOG_DIR="$STATE_DIR/logs"

QM_PORT=7173
LOGIN_PORT=7171
GAME_PORT=7172

usage() {
    cat <<'EOF'
Usage: scripts/tibia_game_online.sh <command>

Commands:
  start     Build (if needed), setup, start query manager + login + game
  stop      Stop all three services
  restart   stop then start
  status    Show PIDs / ports / recent log errors
  logs      Tail logs: logs [qm|login|game|all]
  show-rsa  Print RSA modulus for client / IP changer config

Login (default test account):
  IP 127.0.0.1:7171  account 111111  password tibia

Optional env:
  TIBIA_IMPORT_CHARACTER=Crowoo     register character before start
  TIBIA_GAME_ONLINE_SKIP_BUILD=1    do not rebuild on start
  TIBIA_RSA_PEM=/path/to/tibia.pem     override RSA key path
EOF
}

die() { echo "error: $*" >&2; exit 1; }

ensure_dev() {
    [[ -x "$DEV" ]] || die "missing $DEV"
}

mkdir_state() {
    mkdir -p "$PID_DIR" "$LOG_DIR"
}

pid_file() { echo "$PID_DIR/$1.pid"; }
log_file() { echo "$LOG_DIR/$1.log"; }

read_pid() {
    local f
    f="$(pid_file "$1")"
    [[ -f "$f" ]] || return 1
    local pid
    pid="$(cat "$f")"
    [[ -n "$pid" ]] || return 1
    kill -0 "$pid" 2>/dev/null || return 1
    echo "$pid"
}

stop_pid() {
    local name="$1"
    local pid
    if pid="$(read_pid "$name" 2>/dev/null)"; then
        echo "stopping $name (pid $pid)..."
        kill "$pid" 2>/dev/null || true
        for _ in $(seq 1 20); do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.25
        done
        kill -9 "$pid" 2>/dev/null || true
    fi
    rm -f "$(pid_file "$name")"
}

free_port() {
    local port="$1"
    if command -v fuser >/dev/null 2>&1; then
        fuser -k -n tcp "$port" 2>/dev/null && echo "freed tcp/$port" || true
    fi
}

wait_for_port() {
    local port="$1" label="$2" timeout="${3:-45}"
    for ((i = 0; i < timeout; i++)); do
        if ss -ltn 2>/dev/null | grep -q ":${port} "; then
            echo "ok: $label listening on $port"
            return 0
        fi
        sleep 1
    done
    echo "error: $label did not bind tcp/$port within ${timeout}s" >&2
    tail -n 20 "$(log_file "$label")" 2>/dev/null >&2 || true
    return 1
}

game_data_dir() {
    local data
    if data="$(default_cipsoft_game_data "$ROOT")"; then
        echo "$data"
    else
        die "game data not found — extract reference/archives/tibia-game.tarball.tar.gz into reference/cipsoft-772/runtime/ or set TIBIA_GAME_DATA"
    fi
}

prepare() {
    ensure_dev
    mkdir_state

    if [[ "${TIBIA_GAME_ONLINE_SKIP_BUILD:-0}" == "1" ]] && [[ -x "$(game_data_dir)/bin/game" ]]; then
        "$DEV" setup-quick
    else
        "$DEV" setup
    fi

    if [[ -n "${TIBIA_IMPORT_CHARACTER:-}" ]]; then
        "$DEV" import-character "$TIBIA_IMPORT_CHARACTER"
    fi
}

start_qm() {
    local qm_pid log
    log="$(log_file qm)"
    echo "starting query manager -> $log"
    (
        cd "$ROOT"
        exec "$DEV" run-qm
    ) >>"$log" 2>&1 &
    qm_pid=$!
    echo "$qm_pid" >"$(pid_file qm)"
    wait_for_port "$QM_PORT" qm
}

start_login() {
    local login_pid log
    log="$(log_file login)"
    echo "starting login server -> $log"
    (
        cd "$ROOT"
        exec "$DEV" run-login
    ) >>"$log" 2>&1 &
    login_pid=$!
    echo "$login_pid" >"$(pid_file login)"
    wait_for_port "$LOGIN_PORT" login
}

start_game() {
    local game_pid log data
    data="$(game_data_dir)"
    log="$(log_file game)"
    rm -f "$data/save/game.pid" 2>/dev/null || true
    echo "starting game server -> $log"
    (
        cd "$ROOT"
        TIBIA_GAME_DATA="$data" exec "$DEV" run-game
    ) >>"$log" 2>&1 &
    game_pid=$!
    echo "$game_pid" >"$(pid_file game)"
    wait_for_port "$GAME_PORT" game 120
}

cmd_start() {
    prepare

    echo "== stopping any previous instance =="
    cmd_stop_quiet

    echo "== starting stack =="
    start_qm
    start_login
    start_game

    cat <<EOF

CipSoft stack is online (processes run in background — shell prompt is normal).

  query manager   127.0.0.1:$QM_PORT
  login server    127.0.0.1:$LOGIN_PORT
  game server     127.0.0.1:$GAME_PORT  (world Zanera)

Client login:
  account   111111
  password  tibia

IMPORTANT: client RSA must match server tibia.pem or login fails with
  "Failed to decrypt asymmetric data" in login.log
  Run: scripts/tibia_game_online.sh show-rsa

Logs:  $LOG_DIR
PIDs:  $PID_DIR

  scripts/tibia_game_online.sh status
  scripts/tibia_game_online.sh logs login
  scripts/tibia_game_online.sh stop
EOF
}

cmd_stop_quiet() {
    stop_pid game
    stop_pid login
    stop_pid qm
    free_port "$GAME_PORT"
    free_port "$LOGIN_PORT"
    free_port "$QM_PORT"
    rm -f "$(game_data_dir)/save/game.pid" 2>/dev/null || true
}

cmd_stop() {
    ensure_dev
    echo "== stopping stack =="
    cmd_stop_quiet
    echo "stopped"
}

cmd_status() {
    mkdir_state
    local data
    data="$(game_data_dir)"

    for svc in qm login game; do
        if pid="$(read_pid "$svc" 2>/dev/null)"; then
            printf "%-6s running  pid=%s  log=%s\n" "$svc" "$pid" "$(log_file "$svc")"
        else
            printf "%-6s stopped\n" "$svc"
        fi
    done

    for entry in "$QM_PORT:qm" "$LOGIN_PORT:login" "$GAME_PORT:game"; do
        port="${entry%%:*}"
        label="${entry##*:}"
        if ss -ltn 2>/dev/null | grep -qE ":${port}[[:space:]]"; then
            echo "port   $port ($label) listening"
        elif command -v ss >/dev/null 2>&1; then
            echo "port   $port ($label) closed"
        fi
    done

    if [[ -f "$(log_file login)" ]] && rg -q "Failed to decrypt asymmetric data" "$(log_file login)" 2>/dev/null; then
        echo "warn:  client RSA mismatch — run: scripts/tibia_game_online.sh show-rsa"
    fi

    if [[ -f "$data/save/game.pid" ]]; then
        echo "note: stale $data/save/game.pid present"
    fi
}

cmd_logs() {
    mkdir_state
    local target="${1:-all}"
    case "$target" in
        qm|login|game)
            tail -f "$(log_file "$target")"
            ;;
        all)
            tail -f "$(log_file qm)" "$(log_file login)" "$(log_file game)"
            ;;
        *)
            die "unknown log target: $target (qm|login|game|all)"
            ;;
    esac
}

cmd_show_rsa() {
    "$DEV" show-rsa
}

cmd_restart() {
    cmd_stop
    sleep 1
    cmd_start
}

main() {
    local cmd="${1:-start}"
    case "$cmd" in
        start) cmd_start ;;
        stop) cmd_stop ;;
        restart) cmd_restart ;;
        status) cmd_status ;;
        logs) shift || true; cmd_logs "${1:-all}" ;;
        show-rsa) cmd_show_rsa ;;
        -h|--help|help) usage ;;
        *) die "unknown command: $cmd"; usage >&2; exit 1 ;;
    esac
}

main "$@"
