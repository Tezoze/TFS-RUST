#!/usr/bin/env python3
"""Compare harness `player_walk` tick/tile alignment — P1 real-map classification."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

DEFAULT_MAX_STEPS = 5


def load_steps(path: Path) -> dict[int, dict[str, Any]]:
    out: dict[int, dict[str, Any]] = {}
    if not path.is_file():
        return out
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line or line[0] != "{":
            continue
        try:
            evt = json.loads(line)
        except json.JSONDecodeError:
            continue
        if evt.get("evt") != "harness_player_step":
            continue
        step = int(evt.get("step", -1))
        pos = evt.get("pos", {})
        out[step] = {
            "tick": int(evt.get("tick", -1)),
            "x": int(pos.get("x", -1)),
            "y": int(pos.get("y", -1)),
            "z": int(pos.get("z", -1)),
        }
    return out


def fmt_pos(row: dict[str, Any] | None) -> str:
    if row is None:
        return "—"
    return f"({row['x']},{row['y']},{row['z']})"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", type=Path, required=True, help="C++ chase JSONL")
    parser.add_argument("--rust", type=Path, required=True, help="Rust chase JSONL")
    parser.add_argument(
        "--max-steps",
        type=int,
        default=DEFAULT_MAX_STEPS,
        help=f"Compare first N player_walk steps (default {DEFAULT_MAX_STEPS})",
    )
    args = parser.parse_args()

    ref = load_steps(args.ref)
    rust = load_steps(args.rust)

    if not ref:
        print(f"error: no harness_player_step events in {args.ref}", file=sys.stderr)
        return 2
    if not rust:
        print(f"error: no harness_player_step events in {args.rust}", file=sys.stderr)
        return 2

    print("step | ref_tick | rust_tick | ref_pos           | rust_pos          | match")
    print("-----+----------+-----------+-------------------+-------------------+------")

    mismatches = 0
    for step in range(args.max_steps):
        r = ref.get(step)
        u = rust.get(step)
        tick_match = r is not None and u is not None and r["tick"] == u["tick"]
        pos_match = (
            r is not None
            and u is not None
            and r["x"] == u["x"]
            and r["y"] == u["y"]
            and r["z"] == u["z"]
        )
        ok = tick_match and pos_match
        if not ok:
            mismatches += 1
        ref_tick = str(r["tick"]) if r else "—"
        rust_tick = str(u["tick"]) if u else "—"
        print(
            f"{step:4} | {ref_tick:>8} | {rust_tick:>9} | "
            f"{fmt_pos(r):17} | {fmt_pos(u):17} | {'yes' if ok else 'NO'}"
        )

    if mismatches:
        print(f"\nharness_player_step: {args.max_steps - mismatches}/{args.max_steps} match")
        return 1
    print(f"\nharness_player_step: {args.max_steps}/{args.max_steps} match")
    return 0


if __name__ == "__main__":
    sys.exit(main())
