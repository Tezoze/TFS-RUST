#!/usr/bin/env python3
"""Plot a Tier 2 scale_sweep JSON: N vs beat-wall p99 and stacked subsystem µs.

matplotlib is optional — missing import exits 0 after a skip message so CI
without plotting deps does not fail.

Usage:
  python3 scripts/bench/plot_sweep.py results/sweep_monsters.json
  python3 scripts/bench/plot_sweep.py results/sweep_monsters.json -o results/sweep_monsters.png
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("json_path", type=Path, help="scale_sweep JSON output")
    parser.add_argument(
        "-o",
        "--out",
        type=Path,
        default=None,
        help="output PNG (default: alongside JSON as .png)",
    )
    args = parser.parse_args()

    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print(
            "plot_sweep: matplotlib not installed; skip. pip install matplotlib",
            file=sys.stderr,
        )
        return 0

    data = json.loads(args.json_path.read_text())
    points = data.get("points") or []
    if not points:
        print("plot_sweep: no points in JSON", file=sys.stderr)
        return 1

    ns = [p["n"] for p in points]
    p99 = [p["beat_wall_us_p99"] for p in points]
    creatures = [p["creatures_us_p50"] for p in points]
    skills = [p["skills_us_p50"] for p in points]
    todo = [p["todo_us_p50"] for p in points]
    path = [p["path_us_p50"] for p in points]

    fig, axes = plt.subplots(2, 1, figsize=(8, 8), sharex=True)
    axes[0].plot(ns, p99, marker="o")
    axes[0].set_ylabel("beat wall p99 (µs)")
    axes[0].set_title(
        f"scale_sweep axis={data.get('axis')} seed={data.get('seed')} "
        f"beats={data.get('beats')}"
    )
    axes[0].grid(True, alpha=0.3)

    axes[1].stackplot(
        ns,
        creatures,
        skills,
        todo,
        path,
        labels=("creatures_us p50", "skills_us p50", "todo_us p50", "path_us p50"),
    )
    axes[1].set_xlabel("N")
    axes[1].set_ylabel("subsystem µs (p50)")
    axes[1].legend(loc="upper left")
    axes[1].grid(True, alpha=0.3)

    out = args.out
    if out is None:
        out = args.json_path.with_suffix(".png")
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.tight_layout()
    fig.savefig(out, dpi=120)
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
