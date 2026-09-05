#!/usr/bin/env python3
"""Turn a `tfs_obs` capture into the markdown rows used by docs/GAME_LOOP_OBS_BASELINES.md.

Reads `game_obs_summary` lines (one per 10s window, emitted only under
`RUST_LOG=...,tfs_obs=info`) and prints the three table rows for one scenario.

Percentiles come from a power-of-two bucketed histogram (`obs.rs` FixedHistogram),
so every value is a bucket upper edge: 0, 1, 2, 4, 8, 16, 32, ... A reported 16
means "in (8, 16]". Percentiles from separate windows cannot be averaged, so where
windows disagree this prints the observed range, matching the existing tables.

Usage:
    scripts/parse_obs_log.py /tmp/obs/active_chase.log --scenario "Active chase"
    scripts/parse_obs_log.py run.log --scenario "Dense spawn" --skip 3 --last 6
"""

from __future__ import annotations

import argparse
import re
import sys

FIELD_RE = re.compile(r"(\w+)=(-?[\d.]+)")

# Percentile fields: range across windows. Counters: per-window range. Maxes: max.
PCT = "pct"
COUNTER = "counter"
MAXIMUM = "max"


def parse(path: str) -> list[dict[str, float]]:
    windows: list[dict[str, float]] = []
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if "game_obs_summary" not in line:
                continue
            fields = {k: float(v) for k, v in FIELD_RE.findall(line)}
            if "beats" in fields:
                windows.append(fields)
    return windows


def fmt(value: float) -> str:
    return str(int(value)) if value == int(value) else f"{value:g}"


def agg(windows: list[dict[str, float]], key: str, kind: str) -> str:
    vals = [w[key] for w in windows if key in w]
    if not vals:
        return "—"
    if kind == MAXIMUM:
        return fmt(max(vals))
    lo, hi = min(vals), max(vals)
    return fmt(lo) if lo == hi else f"{fmt(lo)}–{fmt(hi)}"


def triple(windows: list[dict[str, float]], prefix: str) -> list[str]:
    return [agg(windows, f"{prefix}_p{p}", PCT) for p in (50, 95, 99)]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("log")
    ap.add_argument("--scenario", required=True, help="Row label, e.g. 'Dense spawn'")
    ap.add_argument("--skip", type=int, default=0, help="Drop N leading warmup windows")
    ap.add_argument("--last", type=int, default=0, help="Keep only the last N windows")
    args = ap.parse_args()

    windows = parse(args.log)
    if not windows:
        print(
            f"no game_obs_summary lines in {args.log}\n"
            "Was the server run with RUST_LOG=info,tfs_obs=info? Default filter is tfs_obs=off.",
            file=sys.stderr,
        )
        return 1

    windows = windows[args.skip :]
    if args.last:
        windows = windows[-args.last :]
    if not windows:
        print("no windows left after --skip/--last", file=sys.stderr)
        return 1

    name = args.scenario
    beats = sum(w.get("beats", 0) for w in windows)
    print(f"# {name}: {len(windows)} windows (~{len(windows) * 10}s), {int(beats)} beats\n")

    print("## Beat lateness + wall (ms)")
    lateness = triple(windows, "beat_lateness_ms")
    wall = triple(windows, "beat_wall_ms")
    print(f"| {name} | " + " | ".join(lateness + wall) + " |\n")

    print("## Subsystem wall (µs)")
    cells = [" / ".join(triple(windows, p)) for p in ("creatures_us", "cron_us", "skills_us", "other_us", "todo_us")]
    print(f"| {name} | " + " | ".join(cells) + " |\n")

    print("## ToDo / decay / path")
    todo = [
        agg(windows, "todo_heap_max", MAXIMUM),
        agg(windows, "todo_popped", COUNTER),
        agg(windows, "todo_executed", COUNTER),
        agg(windows, "todo_stale", COUNTER),
        agg(windows, "todo_lateness_ms_p95", PCT),
        agg(windows, "decay_due", COUNTER),
        agg(windows, "path_searches", COUNTER),
        agg(windows, "path_failures", COUNTER),
        agg(windows, "path_us_p95", PCT),
    ]
    print(f"| {name} | " + " | ".join(todo) + " |\n")

    # Saturation signals worth seeing even though they have no table column.
    print("## Signals")
    for key, kind in (
        ("cmd_queue_depth_max", MAXIMUM),
        ("cmd_age_ms_p99", PCT),
        ("coalesced_beats", COUNTER),
        ("output_queued_bytes_max", MAXIMUM),
        ("output_full", COUNTER),
        ("output_slow_shed", COUNTER),
        ("decay_live_max", MAXIMUM),
    ):
        print(f"- {key}: {agg(windows, key, kind)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
