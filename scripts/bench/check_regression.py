#!/usr/bin/env python3
"""Compare Criterion medians against a cached `main` baseline.

Walks `target/criterion/**/new/estimates.json` (grouped layout) and compares
`median.point_estimate` to the sibling `main/estimates.json`. Fails when any
bench is more than +25% slower. Missing `main/` baselines are a skip (exit 0)
so the first PR before a main push has cached a baseline does not fail closed.

Stdlib only. Invoked from CI `bench_gate` after `cargo bench`.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

THRESHOLD = 1.25
ROOT = Path(__file__).resolve().parents[2]


def criterion_root(explicit: Path | None) -> Path:
    if explicit is not None:
        return explicit
    target = os.environ.get("CARGO_TARGET_DIR")
    if target:
        return Path(target) / "criterion"
    return ROOT / "target" / "criterion"


def point_estimate(estimates: dict) -> float | None:
    for key in ("median", "mean"):
        node = estimates.get(key)
        if isinstance(node, dict) and "point_estimate" in node:
            return float(node["point_estimate"])
    return None


def bench_id(new_estimates: Path, root: Path) -> str:
    # .../<group>/<func>/<param>/new/estimates.json → group/func/param
    rel = new_estimates.relative_to(root)
    parts = rel.parts[:-2]  # drop new/estimates.json
    return "/".join(parts) if parts else str(rel)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--dir",
        type=Path,
        default=None,
        help="Criterion output directory (default: target/criterion)",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=THRESHOLD,
        help="Fail when new/baseline median ratio exceeds this (default: 1.25)",
    )
    args = parser.parse_args()
    root = criterion_root(args.dir)
    if not root.is_dir():
        print(f"skip: no criterion output at {root}", file=sys.stderr)
        return 0

    rows: list[tuple[str, float, float, float]] = []
    missing_baseline = 0
    for new_path in sorted(root.glob("**/new/estimates.json")):
        main_path = new_path.parent.parent / "main" / "estimates.json"
        if not main_path.is_file():
            missing_baseline += 1
            continue
        try:
            new_est = json.loads(new_path.read_text(encoding="utf-8"))
            base_est = json.loads(main_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            print(f"error reading {new_path}: {exc}", file=sys.stderr)
            return 2
        new_ns = point_estimate(new_est)
        base_ns = point_estimate(base_est)
        if new_ns is None or base_ns is None or base_ns <= 0:
            print(f"skip (malformed estimates): {new_path}", file=sys.stderr)
            continue
        rows.append((bench_id(new_path, root), base_ns, new_ns, new_ns / base_ns))

    if not rows:
        print(
            f"skip: no `main` baselines to compare "
            f"({missing_baseline} new result(s) under {root})",
            file=sys.stderr,
        )
        return 0

    name_w = max(4, max(len(name) for name, *_ in rows))
    header = f"{'bench':<{name_w}}  {'baseline_ns':>14}  {'new_ns':>14}  {'ratio':>8}  status"
    print(header)
    print("-" * len(header))
    failed = 0
    for name, base_ns, new_ns, ratio in rows:
        status = "ok" if ratio <= args.threshold else "REGRESS"
        if status != "ok":
            failed += 1
        print(f"{name:<{name_w}}  {base_ns:14.1f}  {new_ns:14.1f}  {ratio:8.3f}  {status}")

    if failed:
        print(
            f"\n{failed} bench(es) exceeded +{(args.threshold - 1) * 100:.0f}% vs main",
            file=sys.stderr,
        )
        return 1
    print(f"\n{len(rows)} bench(es) within +{(args.threshold - 1) * 100:.0f}% of main")
    return 0


if __name__ == "__main__":
    sys.exit(main())
