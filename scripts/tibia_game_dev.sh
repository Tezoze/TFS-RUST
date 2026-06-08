#!/usr/bin/env bash
# Local dev helper for fusion32 tibia-game-master (772 mechanics reference).
# Source-only tree is gitignored; this script lives in the tracked repo.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=lib/reference_paths.sh
. "$ROOT/scripts/lib/reference_paths.sh"
reference_paths_init "$ROOT"
GAME_DATA=""

TIBIA772="${TIBIA772:-1}"
DEBUG="${DEBUG:-1}"
JOBS="${JOBS:-$(nproc)}"

usage() {
    cat <<'EOF'
Usage: scripts/tibia_game_dev.sh <command>

Commands:
  check       Verify toolchain and source trees
  build         Build tibia-game-master (+ query manager + login if present)
  build-qm      Build tibia-querymanager (SQLite, debug)
  build-login   Build tibia-login (772-aware when TIBIA772=1)
  setup         Prepare a game data directory from TIBIA_GAME_DATA tarball extract
  run-qm        Start query manager in foreground (SQLite, port 7173)
  run-login     Start login server in foreground (port 7171)
  run-game      Start game server (nofork) — requires setup + run-qm in another terminal
  gdb           Launch game under gdb (nofork) — same prerequisites as run-game
  import-character [name|id]
                Register a leaked .usr character in the query manager (login DB)
  list-characters [--level N] [--near x,y,z]
                Search leaked .usr files (default: level 50)

Environment:
  TFS_REFERENCE_DIR       Reference root (default: ./reference)
  TIBIA_GAME_MASTER_DIR   Path to tibia-game-master (default: reference/classic-772/tibia-game-master)
  TIBIA_QUERYMANAGER_DIR  Path to tibia-querymanager (default: reference/classic-772/tibia-querymanager)
  TIBIA_LOGIN_DIR         Path to tibia-login (default: reference/classic-772/tibia-login)
  TIBIA_GAME_DATA         Path to 772 reference runtime data (default: reference/classic-772/runtime)
  TIBIA_RSA_PEM           RSA private key (default: reference/classic-772/client/tibia.pem)
  TIBIA_IMPORT_ACCOUNT    Account number for import-character (default: 111111)
  TIBIA772=1              Build with -DTIBIA772=1 (default: 1 — matches 772 parity work)
  DEBUG=1                 Debug build with assertions (default: 1)

Quick start (after extracting reference/archives/tibia-game.tarball.tar.gz into runtime/):
  scripts/tibia_game_dev.sh check
  scripts/tibia_game_dev.sh build
  scripts/tibia_game_dev.sh setup
  # terminal 1:
  scripts/tibia_game_dev.sh run-qm
  # terminal 2:
  scripts/tibia_game_dev.sh run-login
  # terminal 3:
  scripts/tibia_game_dev.sh run-game

Pathfinding-only parity (no full server):
  python scripts/compare_chase_pathfinding.py --build-rust

Import leaked character for client login:
  scripts/tibia_game_dev.sh import-character Crowoo
  # login: account 111111 / password tibia
EOF
}

die() { echo "error: $*" >&2; exit 1; }

require_tgm() {
    [[ -d "$TGM/src" ]] || die "tibia-game-master not found at $TGM (clone or set TIBIA_GAME_MASTER_DIR)"
}

require_qm() {
    [[ -d "$QM/src" ]] || die "tibia-querymanager not found at $QM — clone: git clone https://github.com/fusion32/tibia-querymanager.git $QM"
}

require_login() {
    [[ -d "$LOGIN/src" ]] || die "tibia-login not found at $LOGIN — clone: git clone https://github.com/fusion32/tibia-login.git $LOGIN"
}

require_game_data() {
    if [[ -z "$GAME_DATA" ]]; then
        if ! GAME_DATA="$(default_ref_772_game_data "$ROOT")"; then
            die "set TIBIA_GAME_DATA to the 772 reference runtime directory (default: reference/classic-772/runtime)"
        fi
    fi
    [[ -d "$GAME_DATA" ]] || die "TIBIA_GAME_DATA is not a directory: $GAME_DATA"
}

patch_tibia_paths() {
    local cfg="$GAME_DATA/.tibia"
    [[ -f "$cfg" ]] || die "missing $cfg"
    local base="$GAME_DATA"
    sed -i \
        -e "s|BINPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|BINPATH     = \"$base/bin\"|" \
        -e "s|MAPPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|MAPPATH     = \"$base/map\"|" \
        -e "s|ORIGMAPPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|ORIGMAPPATH = \"$base/origmap\"|" \
        -e "s|DATAPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|DATAPATH    = \"$base/dat\"|" \
        -e "s|USERPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|USERPATH    = \"$base/usr\"|" \
        -e "s|LOGPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|LOGPATH     = \"$base/log\"|" \
        -e "s|SAVEPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|SAVEPATH    = \"$base/save\"|" \
        -e "s|MONSTERPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|MONSTERPATH = \"$base/mon\"|" \
        -e "s|NPCPATH[[:space:]]*=[[:space:]]*\"[^\"]*\"|NPCPATH     = \"$base/npc\"|" \
        "$cfg"
}

ensure_qm_sample_accounts() {
    require_qm
    local patch="$QM/sqlite/patches/z-999-initial-data.sql"
    if [[ ! -f "$patch" ]]; then
        cp "$QM/sqlite/z-999-initial-data.sql" "$patch"
        rm -f "$QM/tibia.db"
        echo "note: installed QM sample accounts patch (111111/tibia) — DB will init on next run-qm"
    fi
}

resolve_rsa_pem() {
    resolve_rsa_pem_path "$ROOT"
}

install_rsa_pem() {
    local src
    src="$(resolve_rsa_pem)"
    [[ -f "$src" ]] || die "RSA key not found: $src (place tibia.pem in reference/classic-772/client/ or set TIBIA_RSA_PEM)"
    require_game_data
    if [[ ! "$src" -ef "$GAME_DATA/tibia.pem" ]]; then
        cp -f "$src" "$GAME_DATA/tibia.pem"
    fi
    if [[ -d "$LOGIN" ]] && [[ ! "$src" -ef "$LOGIN/tibia.pem" ]]; then
        cp -f "$src" "$LOGIN/tibia.pem"
    fi
    echo "rsa:    $src (login + game use tibia.pem)"
}

require_qm_db() {
    require_qm
    ensure_qm_sample_accounts
    [[ -f "$QM/build/querymanager" ]] || cmd_build_qm
    [[ -f "$QM/config.cfg" ]] || cp "$QM/config.cfg.dist" "$QM/config.cfg"
    if [[ ! -f "$QM/tibia.db" ]]; then
        echo "note: initializing query manager database (first boot)..."
        (cd "$QM" && timeout 5 ./build/querymanager) >/dev/null 2>&1 || true
    fi
    [[ -f "$QM/tibia.db" ]] || die "query manager database missing — run: scripts/tibia_game_dev.sh run-qm (once), then retry"
    command -v sqlite3 >/dev/null || die "sqlite3 required for import-character (e.g. pacman -S sqlite)"
}

usr_parse_py() {
    cat <<'PY'
import re
import sys
from pathlib import Path

PROF = {
    0: "None", 1: "Knight", 2: "Paladin", 3: "Sorcerer", 4: "Druid",
    10: "Promotion", 11: "Elite Knight", 12: "Royal Paladin",
    13: "Master Sorcerer", 14: "Elder Druid",
}

def parse_usr(text: str) -> dict:
    def one(pattern: str, cast=str, default=None):
        m = re.search(pattern, text, re.I)
        if not m:
            return default
        return cast(m.group(1))

    skill0 = re.search(r"Skill\s*=\s*\(\s*0\s*,\s*(\d+)\s*,", text, re.I)
    level = int(skill0.group(1)) if skill0 else 0
    pos_m = re.search(r"CurrentPosition\s*=\s*\[(\d+),(\d+),(\d+)\]", text, re.I)
    pos = tuple(map(int, pos_m.groups())) if pos_m else None
    prof = one(r"Profession\s*=\s*(\d+)", int, 0)
    return {
        "id": one(r"ID\s*=\s*(\d+)", int),
        "name": one(r'Name\s*=\s*"([^"]+)"'),
        "race": one(r"Race\s*=\s*(\d+)", int, 1),
        "level": level,
        "prof": prof,
        "prof_name": PROF.get(prof, str(prof)),
        "pos": pos,
    }

def find_usr(usr_root: Path, query: str) -> Path | None:
    if query.isdigit():
        cid = int(query)
        path = usr_root / f"{cid % 100:02d}" / f"{cid}.usr"
        return path if path.is_file() else None
    q = query.casefold()
    for path in usr_root.rglob("*.usr"):
        text = path.read_text(errors="replace")
        m = re.search(r'Name\s*=\s*"([^"]+)"', text, re.I)
        if m and m.group(1).casefold() == q:
            return path
    return None

def iter_usrs(usr_root: Path):
    for path in usr_root.rglob("*.usr"):
        try:
            data = parse_usr(path.read_text(errors="replace"))
        except Exception:
            continue
        if data.get("id") and data.get("name"):
            data["path"] = str(path)
            yield data

mode = sys.argv[1]
usr_root = Path(sys.argv[2])
if mode == "find":
    path = find_usr(usr_root, sys.argv[3])
    if not path:
        sys.exit(1)
    data = parse_usr(path.read_text(errors="replace"))
    data["path"] = str(path)
    print("|".join([
        str(data["id"]), data["name"], str(data["level"]),
        str(data["prof"]), str(data["race"]),
        str(data["pos"][0]) if data["pos"] else "",
        str(data["pos"][1]) if data["pos"] else "",
        str(data["pos"][2]) if data["pos"] else "",
        data["prof_name"], data["path"],
    ]))
elif mode == "list":
    level = int(sys.argv[3]) if len(sys.argv) > 3 else 50
    tol = int(sys.argv[4]) if len(sys.argv) > 4 else 0
    nx = ny = nz = radius = 0
    if len(sys.argv) > 5 and sys.argv[5]:
        nx, ny, nz = (int(x) for x in sys.argv[5].split(","))
        radius = int(sys.argv[6]) if len(sys.argv) > 6 and sys.argv[6] else 30
    limit = int(sys.argv[7]) if len(sys.argv) > 7 else 20
    rows = []
    for data in iter_usrs(usr_root):
        if abs(data["level"] - level) > tol:
            continue
        if radius > 0 and data["pos"]:
            x, y, z = data["pos"]
            if z != nz or abs(x - nx) > radius or abs(y - ny) > radius:
                continue
        rows.append(data)
    rows.sort(key=lambda d: (abs(d["level"] - level), d["name"].casefold()))
    for data in rows[:limit]:
        x, y, z = data["pos"] or (0, 0, 0)
        print(f"{data['level']}\t{data['id']}\t{data['name']}\t{data['prof_name']}\t{x},{y},{z}\t{data['path']}")
elif mode == "import":
    import sqlite3
    db_path, account, query = sys.argv[2], int(sys.argv[3]), sys.argv[4]
    path = find_usr(Path(sys.argv[5]), query)
    if not path:
        sys.exit(1)
    data = parse_usr(path.read_text(errors="replace"))
    sex = 1 if data["race"] else 0
    con = sqlite3.connect(db_path)
    row = con.execute(
        "SELECT AccountID FROM Accounts WHERE AccountID=? AND Deleted=0", (account,)
    ).fetchone()
    if not row:
        print(f"account {account} not in query manager", file=sys.stderr)
        sys.exit(2)
    con.execute(
        """INSERT INTO Characters
           (WorldID, CharacterID, AccountID, Name, Sex, Level, Profession, Residence, Deleted)
           VALUES (1, ?, ?, ?, ?, ?, ?, '', 0)
           ON CONFLICT(CharacterID) DO UPDATE SET
             AccountID=excluded.AccountID, Name=excluded.Name, Sex=excluded.Sex,
             Level=excluded.Level, Profession=excluded.Profession, Deleted=0""",
        (data["id"], account, data["name"], sex, data["level"], data["prof_name"]),
    )
    con.commit()
    con.close()
    x, y, z = data["pos"] or (0, 0, 0)
    print(f"imported: {data['name']} (CharacterID={data['id']}, level={data['level']}, {data['prof_name']})")
    print(f"  position: ({x}, {y}, {z})")
    print(f"  usr:      {path}")
    print(f"  login:    account {account} / password tibia")
    print("  note:     restart run-login if it was already running")
PY
}

cmd_list_characters() {
    require_game_data
    local level=50 tol=0 limit=20
    local near="" radius=30
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --level) level="$2"; shift 2 ;;
            --tolerance) tol="$2"; shift 2 ;;
            --near) near="$2"; shift 2 ;;
            --radius) radius="$2"; shift 2 ;;
            --limit) limit="$2"; shift 2 ;;
            *) die "unknown list-characters option: $1" ;;
        esac
    done
    echo -e "level\tid\tname\tvocation\tposition\tusr"
    if [[ -n "$near" ]]; then
        python3 -c "$(usr_parse_py)" list "$GAME_DATA/usr" "$level" "$tol" "$near" "$radius" "$limit"
    else
        python3 -c "$(usr_parse_py)" list "$GAME_DATA/usr" "$level" "$tol" "" "" "$limit"
    fi
}

cmd_import_character() {
    require_game_data
    require_qm_db

    local account="${TIBIA_IMPORT_ACCOUNT:-111111}"
    local query=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --account) account="$2"; shift 2 ;;
            -*) die "unknown import-character option: $1" ;;
            *)
                [[ -z "$query" ]] || die "pass one character name or id"
                query="$1"
                shift
                ;;
        esac
    done
    [[ -n "$query" ]] || query="Crowoo"

    python3 -c "$(usr_parse_py)" import "$QM/tibia.db" "$account" "$query" "$GAME_DATA/usr" \
        || die "import failed for: $query"
}

tgm_cflags() {
    local base="-m64 -fno-strict-aliasing -pedantic -Wall -Wextra"
    base+=" -Wno-deprecated-declarations -Wno-unused-parameter -Wno-format-truncation"
    base+=" -std=c++11 -pthread -DOS_LINUX=1 -DARCH_X64=1"
    if [[ "$TIBIA772" == "1" ]]; then
        base+=" -DTIBIA772=1"
    fi
    if [[ "$DEBUG" == "1" ]]; then
        base+=" -g -Og -DENABLE_ASSERTIONS=1"
    else
        base+=" -O2"
    fi
    printf '%s' "$base"
}

cmd_check() {
    require_tgm
    command -v g++ >/dev/null || die "g++ not found"
    pkg-config --exists libcrypto 2>/dev/null || command -v openssl >/dev/null || die "OpenSSL libcrypto required (e.g. pacman -S openssl)"
    echo "ok: tibia-game-master at $TGM"
    if [[ -d "$QM/src" ]]; then
        echo "ok: tibia-querymanager at $QM"
    else
        echo "note: tibia-querymanager not present (optional for build; required to run game)"
    fi
    if [[ -d "$LOGIN/src" ]]; then
        echo "ok: tibia-login at $LOGIN"
    else
        echo "note: tibia-login not present (optional; required for client login)"
    fi
    if [[ -n "$GAME_DATA" ]]; then
        [[ -d "$GAME_DATA" ]] && echo "ok: TIBIA_GAME_DATA=$GAME_DATA" || echo "warn: TIBIA_GAME_DATA set but missing: $GAME_DATA"
    elif data="$(default_ref_772_game_data "$ROOT" 2>/dev/null || true)" && [[ -n "$data" ]]; then
        echo "ok: game data at $data (default)"
    else
        echo "note: game data not found (extract tarball to reference/classic-772/runtime/)"
    fi
}

cmd_build_tgm() {
    require_tgm
    echo "building tibia-game-master (TIBIA772=$TIBIA772 DEBUG=$DEBUG)..."
    make -C "$TGM" -B -j"$JOBS" CFLAGS="$(tgm_cflags)"
    echo "built: $TGM/build/game"
}

cmd_build_qm() {
    require_qm
    echo "building tibia-querymanager (SQLite DEBUG=$DEBUG)..."
    make -C "$QM" -B DEBUG="$DEBUG" DATABASE=sqlite -j"$JOBS"
    echo "built: $QM/build/querymanager"
}

login_cxxflags() {
    local base="-m64 -fno-strict-aliasing -Wno-deprecated-declarations -pedantic -Wall -Wextra -pthread --std=c++11"
    if [[ "$TIBIA772" == "1" ]]; then
        base+=" -DTIBIA772=1"
    fi
    if [[ "$DEBUG" == "1" ]]; then
        base+=" -g -Og -DENABLE_ASSERTIONS=1"
    else
        base+=" -O2"
    fi
    printf '%s' "$base"
}

cmd_build_login() {
    require_login
    echo "building tibia-login (TIBIA772=$TIBIA772 DEBUG=$DEBUG)..."
    make -C "$LOGIN" -B -j"$JOBS" CXXFLAGS="$(login_cxxflags)"
    echo "built: $LOGIN/build/login"
}

cmd_build() {
    cmd_build_tgm
    if [[ -d "$QM/src" ]]; then
        cmd_build_qm
    fi
    if [[ -d "$LOGIN/src" ]]; then
        cmd_build_login
    fi
}

cmd_setup() {
    require_tgm
    require_game_data
    cmd_build_tgm
    if [[ -d "$QM/src" ]]; then
        cmd_build_qm
    fi
    if [[ -d "$LOGIN/src" ]]; then
        cmd_build_login
    fi
    ensure_qm_sample_accounts

    local bin_dir="$GAME_DATA/bin"
    mkdir -p "$bin_dir"
    cp -f "$TGM/build/game" "$bin_dir/game"
    install_rsa_pem

    patch_tibia_paths

    # README: stale pid blocks startup; usr/XX dirs must exist for saves.
    rm -f "$GAME_DATA/save/game.pid" 2>/dev/null || true
    mkdir -p "$GAME_DATA/save" "$GAME_DATA/log"
    mkdir -p "$GAME_DATA/usr"/{00..99}

    if [[ -d "$GAME_DATA/origmap" && ! -f "$GAME_DATA/map/00000000.seg" ]]; then
        echo "note: copying fresh map from origmap (first-time setup)"
        rm -rf "$GAME_DATA/map"
        cp -a "$GAME_DATA/origmap" "$GAME_DATA/map"
    fi

    echo "setup ok: $GAME_DATA"
    echo "  binary: $bin_dir/game"
    echo "  config: $GAME_DATA/.tibia (paths patched to this directory)"
    echo "  login:  account 111111 / password tibia (after run-qm creates DB)"
    echo "  next: run-qm → run-login → run-game"
}

cmd_setup_quick() {
    require_tgm
    require_game_data
    ensure_qm_sample_accounts
    [[ -x "$GAME_DATA/bin/game" ]] || die "missing $GAME_DATA/bin/game — run: scripts/tibia_game_dev.sh setup"

    install_rsa_pem
    patch_tibia_paths
    rm -f "$GAME_DATA/save/game.pid" 2>/dev/null || true
    mkdir -p "$GAME_DATA/save" "$GAME_DATA/log"
    mkdir -p "$GAME_DATA/usr"/{00..99}
}

cmd_run_qm() {
    require_qm
    [[ -f "$QM/build/querymanager" ]] || cmd_build_qm
    [[ -f "$QM/config.cfg" ]] || cp "$QM/config.cfg.dist" "$QM/config.cfg"
    cd "$QM"
    exec ./build/querymanager
}

cmd_run_login() {
    require_login
    cmd_build_login
    [[ -f "$LOGIN/config.cfg" ]] || cp "$LOGIN/config.cfg.dist" "$LOGIN/config.cfg"
    install_rsa_pem
    cd "$LOGIN"
    exec ./build/login
}

cmd_run_game() {
    require_tgm
    require_game_data
    [[ -x "$GAME_DATA/bin/game" ]] || die "run setup first (missing $GAME_DATA/bin/game)"
    [[ -f "$GAME_DATA/.tibia" ]] || die "missing $GAME_DATA/.tibia"
    install_rsa_pem
    rm -f "$GAME_DATA/save/game.pid" 2>/dev/null || true
    cd "$GAME_DATA"
    exec "$GAME_DATA/bin/game" nofork
}

cmd_show_rsa() {
    local src
    src="$(resolve_rsa_pem)"
    [[ -f "$src" ]] || die "RSA key not found: $src"
    echo "Server RSA key: $src"
    echo "Priority: TIBIA_RSA_PEM -> reference/classic-772/client/tibia.pem -> tibia-game-master/tibia.pem"
    echo "Use these values in OTClient init.lua or fusion32 tibia-ipchanger:"
    echo
    openssl rsa -in "$src" -modulus -noout
    openssl rsa -in "$src" -text -noout 2>/dev/null | grep publicExponent
}

cmd_gdb() {
    require_tgm
    require_game_data
    [[ -x "$GAME_DATA/bin/game" ]] || die "run setup first"
    rm -f "$GAME_DATA/save/game.pid" 2>/dev/null || true
    cd "$GAME_DATA"
    exec gdb --args "$GAME_DATA/bin/game" nofork
}

main() {
    local cmd="${1:-}"
    case "$cmd" in
        check)    cmd_check ;;
        build)       cmd_build ;;
        build-qm)    cmd_build_qm ;;
        build-login) cmd_build_login ;;
        setup)       cmd_setup ;;
        setup-quick) cmd_setup_quick ;;
        run-qm)      cmd_run_qm ;;
        run-login)   cmd_run_login ;;
        run-game)    cmd_run_game ;;
        gdb)         cmd_gdb ;;
        import-character) shift; cmd_import_character "$@" ;;
        list-characters) shift; cmd_list_characters "$@" ;;
        show-rsa) cmd_show_rsa ;;
        -h|--help|help|"") usage ;;
        *) die "unknown command: $cmd (try --help)" ;;
    esac
}

main "$@"
