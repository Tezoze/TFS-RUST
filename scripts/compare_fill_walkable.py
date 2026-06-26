#!/usr/bin/env python3
"""Compare Rust vs C++ TShortway FillMap dumps — P2.5c / P1 real-map cyclops bowl probe."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

PRIORITY_TILES_NW = [
    (32359, 32290, 7),
    (32358, 32289, 7),
    (32360, 32289, 7),
]

PRIORITY_TILES_CYCLOPS_BOWL = [
    (32451, 32065, 7),  # player_start
    (32453, 32065, 7),  # monster (scenario)
    (32454, 32065, 7),  # monster after SetOnMap / harness login
    (32450, 32065, 7),
    (32450, 32066, 7),
    (32451, 32066, 7),
    (32452, 32066, 7),
    (32458, 32065, 7),  # east 1099 cliff bank
]

CYCLOPS_BOWL_BBOX = ((32448, 32462), (32060, 32075), 7)

PRESETS: dict[str, list[tuple[int, int, int]]] = {
    "nw": PRIORITY_TILES_NW,
    "cyclops-bowl": PRIORITY_TILES_CYCLOPS_BOWL,
}


def in_cyclops_bowl_bbox(pos: tuple[int, int, int]) -> bool:
    (x_lo, x_hi), (y_lo, y_hi), z = CYCLOPS_BOWL_BBOX
    x, y, pz = pos
    return pz == z and x_lo <= x <= x_hi and y_lo <= y <= y_hi


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


def load_cpp_jsonl(
    path: Path,
    *,
    want_start: tuple[int, int, int] | None,
    monster_nw: bool,
) -> dict[str, Any] | None:
    """Pick `fill_map` event from C++ JSONL, optionally filtered by monster origin."""
    if monster_nw and want_start is None:
        want_start = (32359, 32289, 7)
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
    parser.add_argument(
        "--preset",
        choices=sorted(PRESETS),
        help="Priority tile preset (nw | cyclops-bowl)",
    )
    parser.add_argument(
        "--start",
        nargs=3,
        type=int,
        metavar=("X", "Y", "Z"),
        help="Select C++ fill_map by monster start position",
    )
    args = parser.parse_args()

    want_start: tuple[int, int, int] | None = None
    if args.start is not None:
        want_start = (args.start[0], args.start[1], args.start[2])

    if args.preset:
        priority_tiles = PRESETS[args.preset]
    elif args.monster_nw:
        priority_tiles = PRIORITY_TILES_NW
    elif want_start is not None and in_cyclops_bowl_bbox(want_start):
        priority_tiles = PRIORITY_TILES_CYCLOPS_BOWL
    else:
        priority_tiles = PRIORITY_TILES_NW

    if not args.rust.is_file():
        print(f"rust dump missing: {args.rust}", file=sys.stderr)
        return 2
    if not args.ref.is_file():
        print(f"ref dump missing: {args.ref}", file=sys.stderr)
        return 2

    rust = load_rust_dump(args.rust)
    cpp = load_cpp_jsonl(args.ref, want_start=want_start, monster_nw=args.monster_nw)
    if cpp is None:
        print("no C++ fill_map event found in ref log", file=sys.stderr)
        return 2

    print(f"rust tick={rust['tick']} state={rust['monster_state']}")
    print(f"cpp  tick={cpp['tick']} state={cpp['monster_state']}")
    if cpp.get("start"):
        s = cpp["start"]
        print(f"cpp  start=({s.get('x')},{s.get('y')},{s.get('z')})")

    mismatches: list[str] = []
    for key in priority_tiles:
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

    all_keys = set(rust["tiles"]) | set(cpp["tiles"])
    viewport_mismatches = 0
    mismatch_samples: list[str] = []
    for key in sorted(all_keys):
        rw = tile_walkable(rust["tiles"].get(key))
        cw = tile_walkable(cpp["tiles"].get(key))
        if rw != cw:
            viewport_mismatches += 1
            if len(mismatch_samples) < 5:
                rt = rust["tiles"].get(key, {})
                ct = cpp["tiles"].get(key, {})
                mismatch_samples.append(
                    f"{key}: rust walkable={rw} wp={rt.get('wp')} "
                    f"| cpp walkable={cw} wp={ct.get('wp')}"
                )

    print(f"viewport mismatches: {viewport_mismatches}/{len(all_keys)}")
    for line in mismatch_samples:
        print(f"  {line}")

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
