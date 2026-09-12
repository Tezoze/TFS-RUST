#!/usr/bin/env python3
"""Bulk-seed 772 loadgen accounts/characters for Rust and TVP schemas.

Login is numeric `accounts.id` (7.72 account number), matching `tfs-loadgen`
`--account` / `--character`. Character names: `Test`, `Test1`, `Test2`, …

Password is SHA1 hex of the plaintext (same as `scripts/seed_test_account.sql`).

Default placement scatters characters across `data/world/spawns.xml`: one tile
south (then other neighbors) of unique NPC stands, then spawn-center tiles,
with a Chebyshev gap so login does not stack the Thais temple spectator set.
`--stack-temple` restores the old single-tile seed (32369,32241,7).

Login uses the saved `posx`/`posy`/`posz` first (Rust `place_player_on_login`;
TVP `SetOnMap`). Unplaceable tiles fall back to town_id 1 temple.

Usage:
  python3 scripts/seed_bench_accounts.py --count 600 --out-dir scripts/generated
  python3 scripts/seed_bench_accounts.py --count 600 --apply --target rust
  python3 scripts/seed_bench_accounts.py --count 10 --apply --target tvp
  python3 scripts/seed_bench_accounts.py --self-test
"""

from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHA1_ONE = "356a192b7913b04c54574d18c28d46e6395428ab"
THAIS_X, THAIS_Y, THAIS_Z = 32369, 32241, 7
DEFAULT_SPAWNS = ROOT / "data" / "world" / "spawns.xml"
# One step off the NPC tile (often blocked). Prefer south (in front of dir=2).
_NPC_NUDGE = ((0, 1), (1, 0), (0, -1), (-1, 0), (1, 1), (-1, 1), (1, -1), (-1, -1))


def sha1_hex(password: str) -> str:
    return hashlib.sha1(password.encode("utf-8")).hexdigest()


def character_name(base: str, index: int) -> str:
    """Match `tools/loadgen/src/main.rs` bot `i` naming."""
    return base if index == 0 else f"{base}{index}"


def _sql_str(s: str) -> str:
    return "'" + s.replace("\\", "\\\\").replace("'", "\\'") + "'"


def _chebyshev(a: tuple[int, int, int], b: tuple[int, int, int]) -> int:
    return max(abs(a[0] - b[0]), abs(a[1] - b[1]), abs(a[2] - b[2]))


def _parse_spawn_tiles(path: Path) -> tuple[list[tuple[int, int, int]], list[tuple[int, int, int]]]:
    """NPC world tiles (center + offset) and unique spawn-center tiles."""
    root = ET.parse(path).getroot()
    npcs: list[tuple[int, int, int]] = []
    seen_npc: set[tuple[int, int, int]] = set()
    centers: list[tuple[int, int, int]] = []
    seen_c: set[tuple[int, int, int]] = set()
    for sp in root.findall("spawn"):
        cx = int(sp.get("centerx", "0"))
        cy = int(sp.get("centery", "0"))
        cz = int(sp.get("centerz", "0"))
        c = (cx, cy, cz)
        if c not in seen_c:
            seen_c.add(c)
            centers.append(c)
        for npc in sp.findall("npc"):
            tile = (
                cx + int(npc.get("x") or 0),
                cy + int(npc.get("y") or 0),
                int(npc.get("z") or cz),
            )
            if tile not in seen_npc:
                seen_npc.add(tile)
                npcs.append(tile)
    return npcs, centers


def _nudge_off_npc(tile: tuple[int, int, int], blocked: set[tuple[int, int, int]]) -> tuple[int, int, int]:
    x, y, z = tile
    for dx, dy in _NPC_NUDGE:
        cand = (x + dx, y + dy, z)
        if cand not in blocked:
            return cand
    return (x, y + 1, z)


def _even_stride(seq: list[tuple[int, int, int]], n: int) -> list[tuple[int, int, int]]:
    """Spread `n` picks across `seq` (already unique, stable order)."""
    if n <= 0 or not seq:
        return []
    if n == 1:
        return [seq[0]]
    if n >= len(seq):
        return list(seq)
    last = len(seq) - 1
    out: list[tuple[int, int, int]] = []
    seen: set[tuple[int, int, int]] = set()
    for i in range(n):
        p = seq[i * last // (n - 1)]
        if p not in seen:
            seen.add(p)
            out.append(p)
    return out


def scatter_positions(
    count: int,
    *,
    spawns: Path = DEFAULT_SPAWNS,
    min_dist: int = 16,
    stack_temple: bool = False,
) -> list[tuple[int, int, int]]:
    """Deterministic spread: stride across NPC-adjacent tiles, then spawn centers."""
    if count < 1:
        return []
    if stack_temple:
        return [(THAIS_X, THAIS_Y, THAIS_Z)] * count
    if not spawns.is_file():
        print(f"seed_bench_accounts: missing {spawns}, stacking temple", file=sys.stderr)
        return [(THAIS_X, THAIS_Y, THAIS_Z)] * count

    npcs, centers = _parse_spawn_tiles(spawns)
    npc_set = set(npcs)
    nudged = [_nudge_off_npc(t, npc_set) for t in npcs]
    center_rest = [c for c in centers if c not in npc_set]

    def z_key(p: tuple[int, int, int]) -> tuple[int, int, int, int]:
        return (0 if p[2] == 7 else 1, p[0], p[1], p[2])

    surface_npc = sorted((p for p in dict.fromkeys(nudged) if p[2] == 7), key=z_key)
    other_npc = sorted((p for p in dict.fromkeys(nudged) if p[2] != 7), key=z_key)
    center_sorted = sorted(dict.fromkeys(center_rest), key=z_key)

    chosen: list[tuple[int, int, int]] = []
    chosen.extend(_even_stride(surface_npc, min(count, len(surface_npc))))
    if len(chosen) < count:
        chosen.extend(_even_stride(other_npc, count - len(chosen)))
    if len(chosen) < count:
        taken = set(chosen)
        dist = min_dist
        while dist >= 1 and len(chosen) < count:
            for p in center_sorted:
                if len(chosen) >= count:
                    break
                if p in taken:
                    continue
                if all(_chebyshev(p, q) >= dist for q in chosen):
                    chosen.append(p)
                    taken.add(p)
            dist //= 2
        extra = [p for p in center_sorted if p not in taken]
        chosen.extend(extra[: count - len(chosen)])

    if not chosen:
        chosen = [(THAIS_X, THAIS_Y, THAIS_Z)]
    while len(chosen) < count:
        base = chosen[len(chosen) % len(chosen)]
        k = len(chosen)
        chosen.append((base[0] + (k % 17), base[1] + (k % 13), base[2]))
    return chosen[:count]


def rust_sql(
    *,
    count: int,
    start_id: int,
    password_hex: str,
    char_base: str,
    level: int,
    vocation: int,
    health: int,
    mana: int,
    maglevel: int,
    positions: list[tuple[int, int, int]],
) -> str:
    if len(positions) != count:
        raise ValueError(f"positions length {len(positions)} != count {count}")
    lines = [
        "-- Generated by scripts/seed_bench_accounts.py for Rust schema.sql",
        "-- 772 login uses accounts.id (not accounts.name).",
        "SET FOREIGN_KEY_CHECKS=0;",
    ]
    names = [_sql_str(character_name(char_base, i)) for i in range(count)]
    ids = list(range(start_id, start_id + count))
    lines.append(
        "DELETE FROM `players` WHERE `account_id` IN ("
        + ",".join(str(i) for i in ids)
        + ") OR `name` IN ("
        + ",".join(names)
        + ");"
    )
    lines.append(
        "DELETE FROM `accounts` WHERE `id` IN (" + ",".join(str(i) for i in ids) + ");"
    )
    acc_rows = []
    for acc_id in ids:
        acc_rows.append(
            f"({acc_id}, {_sql_str(str(acc_id))}, {_sql_str(password_hex)}, 1, 0, '', UNIX_TIMESTAMP())"
        )
    lines.append(
        "INSERT INTO `accounts` (`id`, `name`, `password`, `type`, `premium_ends_at`, `email`, `creation`) VALUES"
    )
    lines.append(",\n".join(acc_rows) + ";")
    pl_rows = []
    for i, acc_id in enumerate(ids):
        name = character_name(char_base, i)
        x, y, z = positions[i]
        pl_rows.append(
            "("
            + ", ".join(
                [
                    _sql_str(name),
                    "1",
                    str(acc_id),
                    str(level),
                    str(vocation),
                    str(health),
                    str(health),
                    "0",
                    "0",
                    "0",
                    "0",
                    "0",
                    "136",
                    "0",
                    "2",
                    str(maglevel),
                    str(mana),
                    str(mana),
                    "100",
                    "1",
                    str(x),
                    str(y),
                    str(z),
                    "400",
                    "0",
                ]
            )
            + ")"
        )
    lines.append(
        "INSERT INTO `players` ("
        "`name`, `group_id`, `account_id`, `level`, `vocation`, "
        "`health`, `healthmax`, `experience`, "
        "`lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, `lookaddons`, "
        "`direction`, `maglevel`, `mana`, `manamax`, "
        "`soul`, `town_id`, `posx`, `posy`, `posz`, `cap`, `sex`"
        ") VALUES"
    )
    lines.append(",\n".join(pl_rows) + ";")
    lines.append("SET FOREIGN_KEY_CHECKS=1;")
    return "\n".join(lines) + "\n"


def tvp_sql(
    *,
    count: int,
    start_id: int,
    password_hex: str,
    char_base: str,
    level: int,
    vocation: int,
    health: int,
    mana: int,
    maglevel: int,
    positions: list[tuple[int, int, int]],
) -> str:
    if len(positions) != count:
        raise ValueError(f"positions length {len(positions)} != count {count}")
    lines = [
        "-- Generated by scripts/seed_bench_accounts.py for TVP gameserver/schema.sql",
        "-- TVP accounts have no `name`; login is accounts.id.",
        "SET FOREIGN_KEY_CHECKS=0;",
    ]
    names = [_sql_str(character_name(char_base, i)) for i in range(count)]
    ids = list(range(start_id, start_id + count))
    lines.append(
        "DELETE FROM `players` WHERE `account_id` IN ("
        + ",".join(str(i) for i in ids)
        + ") OR `name` IN ("
        + ",".join(names)
        + ");"
    )
    lines.append(
        "DELETE FROM `accounts` WHERE `id` IN (" + ",".join(str(i) for i in ids) + ");"
    )
    acc_rows = []
    for acc_id in ids:
        acc_rows.append(
            f"({acc_id}, {_sql_str(password_hex)}, 1, 0, '', UNIX_TIMESTAMP(), 0)"
        )
    lines.append(
        "INSERT INTO `accounts` (`id`, `password`, `type`, `premium_ends_at`, `email`, `creation`, `failed_bid_count`) VALUES"
    )
    lines.append(",\n".join(acc_rows) + ";")
    pl_rows = []
    for i, acc_id in enumerate(ids):
        name = character_name(char_base, i)
        x, y, z = positions[i]
        pl_rows.append(
            "("
            + ", ".join(
                [
                    _sql_str(name),
                    "1",
                    str(acc_id),
                    str(level),
                    str(vocation),
                    str(health),
                    str(health),
                    "0",
                    "0",
                    "0",
                    "0",
                    "0",
                    "136",
                    str(maglevel),
                    str(mana),
                    str(mana),
                    "0",
                    "100",
                    "1",
                    str(x),
                    str(y),
                    str(z),
                    "''",
                    "400",
                    "0",
                ]
            )
            + ")"
        )
    lines.append(
        "INSERT INTO `players` ("
        "`name`, `group_id`, `account_id`, `level`, `vocation`, "
        "`health`, `healthmax`, `experience`, "
        "`lookbody`, `lookfeet`, `lookhead`, `looklegs`, `looktype`, "
        "`maglevel`, `mana`, `manamax`, `manaspent`, "
        "`soul`, `town_id`, `posx`, `posy`, `posz`, `conditions`, `cap`, `sex`"
        ") VALUES"
    )
    lines.append(",\n".join(pl_rows) + ";")
    lines.append("SET FOREIGN_KEY_CHECKS=1;")
    return "\n".join(lines) + "\n"


def _lua_mysql(config: Path) -> dict[str, str]:
    out = {
        "mysqlHost": "127.0.0.1",
        "mysqlUser": "tfs",
        "mysqlPass": "",
        "mysqlDatabase": "TFS",
        "mysqlPort": "3306",
    }
    if not config.is_file():
        return out
    for line in config.read_text(encoding="utf-8", errors="replace").splitlines():
        s = line.strip()
        if s.startswith("--") or "=" not in s:
            continue
        key, _, rest = s.partition("=")
        key = key.strip()
        if key not in out:
            continue
        val = rest.strip().rstrip(",").strip()
        if val.startswith('"') and val.endswith('"'):
            val = val[1:-1]
        out[key] = val
    return out


def _mysql_cmd(host: str, user: str, password: str, database: str) -> list[str]:
    bin_ = shutil.which("mariadb") or shutil.which("mysql")
    if not bin_:
        raise SystemExit("mariadb/mysql client not found")
    cmd = [bin_, "--skip-ssl", "-h", host, "-u", user, database]
    if password:
        cmd.insert(-1, f"-p{password}")
    return cmd


def apply_sql(sql: str, *, host: str, user: str, password: str, database: str) -> None:
    cmd = _mysql_cmd(host, user, password, database)
    subprocess.run(cmd, input=sql, text=True, check=True)


def self_test() -> int:
    assert sha1_hex("1") == SHA1_ONE, sha1_hex("1")
    assert character_name("Test", 0) == "Test"
    assert character_name("Test", 1) == "Test1"
    stacked = scatter_positions(3, stack_temple=True)
    assert stacked == [(THAIS_X, THAIS_Y, THAIS_Z)] * 3
    pos = scatter_positions(20, min_dist=16)
    assert len(pos) == 20
    assert len(set(pos)) == 20
    xs = [p[0] for p in pos]
    assert max(xs) - min(xs) > 400, xs  # not stuck in one town
    rust = rust_sql(
        count=2,
        start_id=1,
        password_hex=SHA1_ONE,
        char_base="Test",
        level=50,
        vocation=0,
        health=1000,
        mana=1000,
        maglevel=20,
        positions=pos[:2],
    )
    assert "INSERT INTO `accounts`" in rust
    assert "'Test'" in rust and "'Test1'" in rust
    assert ", 1," in rust  # account id 1
    assert str(pos[0][0]) in rust and str(pos[1][0]) in rust
    tvp = tvp_sql(
        count=2,
        start_id=1,
        password_hex=SHA1_ONE,
        char_base="Test",
        level=50,
        vocation=0,
        health=1000,
        mana=1000,
        maglevel=20,
        positions=pos[:2],
    )
    assert "failed_bid_count" in tvp
    assert "`name`" not in tvp.split("INSERT INTO `accounts`")[1].split("VALUES")[0]
    print("seed_bench_accounts: self-test ok")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--count", type=int, default=600, help="accounts to seed (default 600)")
    parser.add_argument("--start-id", type=int, default=1, help="first accounts.id (default 1)")
    parser.add_argument("--password", default="1", help="plaintext password (SHA1 stored)")
    parser.add_argument("--character", default="Test", help="base character name")
    parser.add_argument("--level", type=int, default=50)
    parser.add_argument("--vocation", type=int, default=0)
    parser.add_argument("--health", type=int, default=1000)
    parser.add_argument("--mana", type=int, default=1000)
    parser.add_argument("--maglevel", type=int, default=20)
    parser.add_argument(
        "--spawns",
        type=Path,
        default=DEFAULT_SPAWNS,
        help="spawns.xml used to scatter login tiles",
    )
    parser.add_argument(
        "--min-dist",
        type=int,
        default=16,
        help="Chebyshev gap between login tiles (default 16)",
    )
    parser.add_argument(
        "--stack-temple",
        action="store_true",
        help="old behavior: every char on Thais temple 32369,32241,7",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="write seed_bench_rust.sql / seed_bench_tvp.sql",
    )
    parser.add_argument("--apply", action="store_true", help="run SQL against MariaDB")
    parser.add_argument(
        "--target",
        choices=("rust", "tvp", "both"),
        default="both",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.count < 1:
        print("count must be >= 1", file=sys.stderr)
        return 2

    positions = scatter_positions(
        args.count,
        spawns=args.spawns,
        min_dist=max(1, args.min_dist),
        stack_temple=args.stack_temple,
    )
    uniq = len(set(positions))
    print(
        f"seed_bench_accounts: {args.count} chars on {uniq} tiles "
        f"(min_dist={args.min_dist} stack_temple={args.stack_temple})",
        file=sys.stderr,
    )
    for i, (x, y, z) in enumerate(positions[:8]):
        print(f"  {character_name(args.character, i)} -> {x},{y},{z}", file=sys.stderr)

    pw = sha1_hex(args.password)
    kwargs = dict(
        count=args.count,
        start_id=args.start_id,
        password_hex=pw,
        char_base=args.character,
        level=args.level,
        vocation=args.vocation,
        health=args.health,
        mana=args.mana,
        maglevel=args.maglevel,
        positions=positions,
    )
    rust = rust_sql(**kwargs)
    tvp = tvp_sql(**kwargs)

    out_dir = args.out_dir
    if out_dir is None and not args.apply:
        out_dir = ROOT / "scripts" / "generated"
    if out_dir is not None:
        out_dir.mkdir(parents=True, exist_ok=True)
        (out_dir / "seed_bench_rust.sql").write_text(rust, encoding="utf-8")
        (out_dir / "seed_bench_tvp.sql").write_text(tvp, encoding="utf-8")
        print(f"wrote {out_dir / 'seed_bench_rust.sql'}")
        print(f"wrote {out_dir / 'seed_bench_tvp.sql'}")

    if args.apply:
        rust_db = _lua_mysql(ROOT / os.environ.get("TFS_CONFIG", "config.lua"))
        tvp_db = {
            "mysqlHost": os.environ.get("TVP_MYSQL_HOST", "127.0.0.1"),
            "mysqlUser": os.environ.get("TVP_MYSQL_USER", rust_db["mysqlUser"]),
            "mysqlPass": os.environ.get("TVP_MYSQL_PASS", rust_db["mysqlPass"]),
            "mysqlDatabase": os.environ.get("TVP_MYSQL_DATABASE", "test_tvp"),
        }
        if args.target in ("rust", "both"):
            print(
                f"apply rust → {rust_db['mysqlUser']}@{rust_db['mysqlHost']}/{rust_db['mysqlDatabase']}"
            )
            apply_sql(
                rust,
                host=rust_db["mysqlHost"],
                user=rust_db["mysqlUser"],
                password=os.environ.get("TFS_DB_PASS", rust_db["mysqlPass"]),
                database=os.environ.get("TFS_DB_NAME", rust_db["mysqlDatabase"]),
            )
        if args.target in ("tvp", "both"):
            print(
                f"apply tvp → {tvp_db['mysqlUser']}@{tvp_db['mysqlHost']}/{tvp_db['mysqlDatabase']}"
            )
            apply_sql(
                tvp,
                host=tvp_db["mysqlHost"],
                user=tvp_db["mysqlUser"],
                password=tvp_db["mysqlPass"],
                database=tvp_db["mysqlDatabase"],
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
