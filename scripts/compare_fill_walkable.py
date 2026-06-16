#!/usr/bin/env python3
"""Compare Rust vs C++ TShortway FillMap dumps — P2.5c parity probe."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

PRIORITY_TILES = [
    (32359, 32290, 7),
    (32358, 32289, 7),
    (32360, 32289, 7),
]


def load_rust_dump(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text())
    tiles = {
        (t["x"], t["y"], t["z"]): t for t in data.get("tiles", [])
    }
    return {
        "tick": data.get("tick"),
        "monster_state": data.get("monster_state"),
        "start": data.get("start"),
        "tiles": tiles,
    }


def load_cpp_jsonl(path: Path, monster_nw: bool) -> dict[str, Any] | None:
    """Pick NW cyclops `fill_map` @ start (32359,32289) from C++ JSONL."""
    want_start = (32359, 32289, 7) if monster_nw else None
    last: dict[str, Any] | None = None
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            evt = json.loads(line)
        except json.JSONDecodeError:
            continue
        if evt.get("evt") != "fill_map":
            continue
        start = evt.get("start", {})
        sx, sy, sz = start.get("x"), start.get("y"), start.get("z")
        if want_start and (sx, sy, sz) != want_start:
            continue
        tiles = {
            (t["x"], t["y"], t["z"]): t for t in evt.get("tiles", [])
        }
        last = {
            "tick": evt.get("tick"),
            "monster_state": evt.get("monster_state"),
            "start": start,
            "tiles": tiles,
        }
    return last


def tile_walkable(tile: dict[str, Any] | None) -> bool | None:
    if tile is None:
        return None
    if "walkable" in tile:
        return bool(tile["walkable"])
    wp = tile.get("wp", -1)
    return wp > 0 if wp is not None else None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path, required=True, help="Rust fill_walkable JSON")
    parser.add_argument(
        "--ref",
        type=Path,
        required=True,
        help="C++ chase JSONL with fill_map events",
    )
    parser.add_argument(
        "--monster-nw",
        action="store_true",
        help="Select C++ fill_map for NW cyclops @ (32359,32289)",
    )
    args = parser.parse_args()

    if not args.rust.is_file():
        print(f"rust dump missing: {args.rust}", file=sys.stderr)
        return 2
    if not args.ref.is_file():
        print(f"ref dump missing: {args.ref}", file=sys.stderr)
        return 2

    rust = load_rust_dump(args.rust)
    cpp = load_cpp_jsonl(args.ref, args.monster_nw)
    if cpp is None:
        print("no C++ fill_map event found in ref log", file=sys.stderr)
        return 2

    print(f"rust tick={rust['tick']} state={rust['monster_state']}")
    print(f"cpp  tick={cpp['tick']} state={cpp['monster_state']}")

    mismatches: list[str] = []
    for key in PRIORITY_TILES:
        rw = tile_walkable(rust["tiles"].get(key))
        cw = tile_walkable(cpp["tiles"].get(key))
        rt = rust["tiles"].get(key, {})
        ct = cpp["tiles"].get(key, {})
        print(
            f"  {key}: rust walkable={rw} wp={rt.get('wp')} "
            f"| cpp walkable={cw} wp={ct.get('wp')}"
        )
        if rw != cw:
            mismatches.append(f"{key}: rust={rw} cpp={cw}")

  # full viewport diff count
    all_keys = set(rust["tiles"]) | set(cpp["tiles"])
    viewport_mismatches = 0
    for key in sorted(all_keys):
        rw = tile_walkable(rust["tiles"].get(key))
        cw = tile_walkable(cpp["tiles"].get(key))
        if rw != cw:
            viewport_mismatches += 1

    print(f"viewport mismatches: {viewport_mismatches}/{len(all_keys)}")

    if mismatches:
        print("PRIORITY MISMATCH (first):", mismatches[0])
        return 1
    if viewport_mismatches:
        print("priority tiles match; non-priority viewport diffs remain")
        return 0
    print("fill_map PASS (priority + viewport)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
