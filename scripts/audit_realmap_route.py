#!/usr/bin/env python3
"""P2 real-map route tile audit — OTBM vs `.sec` per scenario coordinate.

For each `player_start`, `monster`, and `player_walk` tile:
  - `.sec` first `Content` id + FillMap walkability (`objects.srv`)
  - OTBM ground id + walkability (via `chase_kite_sim --audit-route`)

C++ reference: `map.cc` `Content`; `cract.cc` `TShortway::FillMap`.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REF_DIR_NAMES = ("classic-772", "cipsoft-772")

CONTENT_LINE = re.compile(r"^(\d{2}-\d{2}):\s*Content=\{([^}]+)\}")
LEADING_ID = re.compile(r"^(\d+)")


@dataclass(frozen=True)
class RouteCoord:
    x: int
    y: int
    z: int
    roles: tuple[str, ...]


@dataclass
class SecTile:
    exists: bool
    first_id: int | None = None
    bank: bool = False
    unpass: bool = False
    waypoints: int = -1

    @property
    def fill_walkable(self) -> bool:
        if self.first_id is None:
            return False
        return self.bank and not self.unpass and self.waypoints > 0


def resolve_sec_map_dir() -> Path | None:
    env = os.environ.get("TFS_SEC_MAP_DIR")
    if env:
        p = Path(env)
        if p.is_dir():
            return p
    ref_root = os.environ.get("TFS_REFERENCE_DIR")
    bases = [Path(ref_root)] if ref_root else [ROOT / "reference"]
    for base in bases:
        for name in REF_DIR_NAMES:
            candidate = base / name / "runtime" / "map"
            if candidate.is_dir():
                return candidate
    return None


def resolve_objects_srv_path() -> Path | None:
    for key in ("TFS_OBJECTS_SRV", "TFS_CIPSOFT_OBJECTS_SRV"):
        env = os.environ.get(key)
        if env:
            p = Path(env)
            if p.is_file():
                return p
    ref_root = os.environ.get("TFS_REFERENCE_DIR")
    bases = [Path(ref_root)] if ref_root else [ROOT / "reference"]
    for base in bases:
        for name in REF_DIR_NAMES:
            candidate = base / name / "runtime" / "dat" / "objects.srv"
            if candidate.is_file():
                return candidate
    return None


def parse_scenario_route(scenario_path: Path) -> tuple[int, list[RouteCoord]]:
    z = 7
    by_key: dict[tuple[int, int, int], list[str]] = {}

    def add(role: str, x: int, y: int) -> None:
        key = (x, y, z)
        by_key.setdefault(key, [])
        if role not in by_key[key]:
            by_key[key].append(role)

    for raw in scenario_path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        key = parts[0]
        if key == "z" and len(parts) >= 2:
            z = int(parts[1])
        elif key == "player_start" and len(parts) >= 3:
            add("player_start", int(parts[1]), int(parts[2]))
        elif key == "monster" and len(parts) >= 4:
            add(f"monster:{parts[1]}", int(parts[2]), int(parts[3]))
        elif key == "player_walk" and len(parts) >= 3:
            add("player_walk", int(parts[1]), int(parts[2]))

    coords = [
        RouteCoord(x=k[0], y=k[1], z=k[2], roles=tuple(roles))
        for k, roles in sorted(by_key.items())
    ]
    return z, coords


def parse_content_ids(raw: str) -> list[int]:
    ids: list[int] = []
    for part in raw.split(","):
        part = part.strip()
        if not part:
            continue
        m = LEADING_ID.match(part)
        if m:
            ids.append(int(m.group(1)))
    return ids


def load_objects_srv(path: Path) -> dict[int, dict[str, Any]]:
    text = path.read_text(encoding="utf-8", errors="replace")
    out: dict[int, dict[str, Any]] = {}
    for block in text.split("\nTypeID"):
        if not block.strip():
            continue
        if not block.startswith("TypeID"):
            block = "TypeID" + block
        tid_m = re.search(r"TypeID\s*=\s*(\d+)", block)
        if not tid_m:
            continue
        tid = int(tid_m.group(1))
        flags_m = re.search(r"Flags\s*=\s*\{([^}]+)\}", block)
        flags: set[str] = set()
        if flags_m:
            flags = {f.strip() for f in flags_m.group(1).split(",") if f.strip()}
        wp_m = re.search(r"Waypoints=(\d+)", block)
        waypoints = int(wp_m.group(1)) if wp_m else 0
        out[tid] = {
            "bank": "Bank" in flags,
            "unpass": "Unpass" in flags,
            "waypoints": waypoints,
        }
    return out


class SecCache:
    def __init__(self, map_dir: Path) -> None:
        self.map_dir = map_dir
        self._sectors: dict[tuple[int, int, int], dict[str, str]] = {}

    def _sector_key(self, x: int, y: int, z: int) -> tuple[int, int, int]:
        return (x // 32, y // 32, z)

    def _load_sector(self, sx: int, sy: int, z: int) -> dict[str, str]:
        key = (sx, sy, z)
        if key in self._sectors:
            return self._sectors[key]
        path = self.map_dir / f"{sx:04d}-{sy:04d}-{z:02d}.sec"
        tiles: dict[str, str] = {}
        if path.is_file():
            for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
                m = CONTENT_LINE.match(line.strip())
                if m:
                    tiles[m.group(1)] = m.group(2)
        self._sectors[key] = tiles
        return tiles

    def tile(self, x: int, y: int, z: int, srv: dict[int, dict[str, Any]]) -> SecTile:
        sx, sy = x // 32, y // 32
        lx, ly = x % 32, y % 32
        tiles = self._load_sector(sx, sy, z)
        raw = tiles.get(f"{lx:02d}-{ly:02d}")
        if raw is None:
            return SecTile(exists=False)
        ids = parse_content_ids(raw)
        if not ids:
            return SecTile(exists=True)
        first = ids[0]
        meta = srv.get(first, {})
        return SecTile(
            exists=True,
            first_id=first,
            bank=bool(meta.get("bank")),
            unpass=bool(meta.get("unpass")),
            waypoints=int(meta.get("waypoints", -1)),
        )


def run_rust_audit(
    scenario_path: Path,
    *,
    data_dir: Path | None,
    map_rel: str | None,
) -> dict[tuple[int, int, int], dict[str, Any]]:
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "tfs-rust-core",
        "--bin",
        "chase_kite_sim",
        "--",
        "--audit-route",
        str(scenario_path),
    ]
    if data_dir is not None:
        cmd.extend(["--data-dir", str(data_dir)])
    if map_rel is not None:
        cmd.extend(["--map", map_rel])
    proc = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=os.environ.copy(),
    )
    if proc.returncode != 0:
        print(proc.stderr, file=sys.stderr)
        raise RuntimeError(f"rust audit failed (exit {proc.returncode})")
    data = json.loads(proc.stdout)
    out: dict[tuple[int, int, int], dict[str, Any]] = {}
    for t in data.get("tiles", []):
        key = (int(t["x"]), int(t["y"]), int(t["z"]))
        out[key] = t
    return out


def wp_compatible(sec_wp: int, otbm_wp: int, walkable: bool) -> bool:
    if walkable:
        return sec_wp == otbm_wp
    # blocked tiles: 0 vs -1 both mean non-walkable terrain
    return sec_wp <= 0 and otbm_wp <= 0


def classify_row(
    sec: SecTile,
    otbm: dict[str, Any] | None,
) -> str:
    if otbm is None or not otbm.get("exists"):
        return "FAIL"
    if not sec.exists:
        return "FAIL"

    sec_walk = sec.fill_walkable
    otbm_walk = bool(otbm.get("walkable"))
    sec_wp = sec.waypoints
    otbm_wp = int(otbm.get("wp", -1))

    if sec_walk != otbm_walk:
        return "FAIL"
    if not wp_compatible(sec_wp, otbm_wp, sec_walk):
        return "FAIL"

    sec_id = sec.first_id
    otbm_id = otbm.get("ground_id")
    if sec_id is not None and otbm_id is not None and int(sec_id) != int(otbm_id):
        return "WARN"
    return "PASS"


def audit_scenario(
    scenario_path: Path,
    *,
    data_dir: Path | None,
    map_rel: str | None,
) -> list[dict[str, Any]]:
    sec_dir = resolve_sec_map_dir()
    if sec_dir is None:
        raise RuntimeError(
            "772 .sec map not found — set TFS_SEC_MAP_DIR or install reference/…/runtime/map"
        )
    srv_path = resolve_objects_srv_path()
    if srv_path is None:
        raise RuntimeError(
            "objects.srv not found — set TFS_OBJECTS_SRV or install reference/…/runtime/dat/objects.srv"
        )

    _, coords = parse_scenario_route(scenario_path)
    srv = load_objects_srv(srv_path)
    sec_cache = SecCache(sec_dir)
    otbm_by_pos = run_rust_audit(scenario_path, data_dir=data_dir, map_rel=map_rel)

    rows: list[dict[str, Any]] = []
    for coord in coords:
        key = (coord.x, coord.y, coord.z)
        sec = sec_cache.tile(coord.x, coord.y, coord.z, srv)
        otbm = otbm_by_pos.get(key)
        status = classify_row(sec, otbm)
        rows.append(
            {
                "roles": list(coord.roles),
                "x": coord.x,
                "y": coord.y,
                "z": coord.z,
                "sec_first_id": sec.first_id,
                "otbm_ground_id": otbm.get("ground_id") if otbm else None,
                "sec_wp": sec.waypoints,
                "otbm_wp": int(otbm["wp"]) if otbm else None,
                "sec_walk": sec.fill_walkable,
                "otbm_walk": bool(otbm.get("walkable")) if otbm else None,
                "status": status,
            }
        )
    return rows


def print_table(rows: list[dict[str, Any]]) -> None:
    print(
        f"{'roles':<24} {'coord':<18} {'sec_id':>7} {'otbm_id':>7} "
        f"{'sec_wp':>6} {'otbm_wp':>7} {'sec_w':>5} {'otbm_w':>6} {'status':>6}"
    )
    print("-" * 96)
    for r in rows:
        roles = "+".join(r["roles"])
        coord = f"({r['x']},{r['y']},{r['z']})"
        sec_id = str(r["sec_first_id"]) if r["sec_first_id"] is not None else "—"
        otbm_id = str(r["otbm_ground_id"]) if r["otbm_ground_id"] is not None else "—"
        sec_w = "yes" if r["sec_walk"] else "no"
        otbm_w = "yes" if r["otbm_walk"] else "no" if r["otbm_walk"] is not None else "—"
        print(
            f"{roles:<24} {coord:<18} {sec_id:>7} {otbm_id:>7} "
            f"{r['sec_wp']:>6} {str(r['otbm_wp']):>7} {sec_w:>5} {otbm_w:>6} {r['status']:>6}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scenario", type=Path, help="Kite .scenario file")
    parser.add_argument("--json", action="store_true", help="Emit JSON report")
    parser.add_argument("--data-dir", type=Path, help="TFS data dir (OTBM/items)")
    parser.add_argument("--map", dest="map_rel", help="OTBM path relative to data dir")
    args = parser.parse_args()

    if not args.scenario.is_file():
        print(f"error: scenario not found: {args.scenario}", file=sys.stderr)
        return 2

    try:
        rows = audit_scenario(
            args.scenario,
            data_dir=args.data_dir,
            map_rel=args.map_rel,
        )
    except RuntimeError as e:
        print(f"error: {e}", file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps({"scenario": str(args.scenario), "tiles": rows}, indent=2))
    else:
        print_table(rows)

    fails = sum(1 for r in rows if r["status"] == "FAIL")
    warns = sum(1 for r in rows if r["status"] == "WARN")
    passes = sum(1 for r in rows if r["status"] == "PASS")
    print(f"\naudit: {passes} PASS, {warns} WARN, {fails} FAIL", file=sys.stderr)
    return 1 if fails else 0


if __name__ == "__main__":
    raise SystemExit(main())
