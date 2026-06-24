#!/usr/bin/env python3
"""Run the full E0–E6 chase sim battery and write per-scenario summaries."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOG_DIR = ROOT / "log"
SCENARIOS = ROOT / "scripts" / "scenarios"
RUN = ROOT / "scripts" / "run_kite_scenario.py"
SUMMARIZE = ROOT / "scripts" / "summarize_chase_gaps.py"

BATTERY = [
    ("stand", "kite_rat_stand_melee.scenario", "rat", 6000),
    ("kite", "kite_rat_melee.scenario", "rat", 6000),
    ("cyclops", "kite_cyclops_quad_chase.scenario", "cyclops", 4000),
    ("cobra", "kite_cobra_poison.scenario", "cobra", 12000),
    ("panic", "kite_rat_panic.scenario", "rat", 6000),
    ("kill", "kite_rat_kill.scenario", "rat", 2000),
]

EXTENDED_BATTERY = [
    ("hunter_chase", "kite_hunter_dist_chase.scenario", "hunter", 6000),
    ("hunter_dist_flee", "kite_hunter_dist_flee.scenario", "hunter", 6000),
    ("dragon_lowhp_flee", "kite_dragon_lowhp_flee.scenario", "dragon", 4000),
]


def run_battery(*, synthetic: bool, skip_cpp: bool, extended: bool) -> int:
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "TFS_SIM_SEED": os.environ.get("TFS_SIM_SEED", "772")}
    results: list[tuple[str, int, int, int]] = []

    battery = list(BATTERY)
    if extended:
        battery.extend(EXTENDED_BATTERY)

    for slug, scenario_file, monster, max_tick in battery:
        scenario = SCENARIOS / scenario_file
        if not scenario.is_file():
            print(f"error: missing scenario {scenario}", file=sys.stderr)
            return 1

        cmd = [sys.executable, str(RUN), str(scenario)]
        if synthetic:
            cmd.append("--synthetic")
        if skip_cpp:
            cmd.append("--skip-cpp")
        cmd.extend(["--monster", monster])

        print(f"\n=== battery: {slug} ({scenario_file}) ===", file=sys.stderr)
        proc = subprocess.run(cmd, cwd=ROOT, env=env)
        if proc.returncode not in (0, 1, 2):
            print(f"error: run_kite_scenario failed for {slug} (exit {proc.returncode})", file=sys.stderr)
            return proc.returncode

        cip_log = LOG_DIR / f"chase_path_cip_{slug}.log"
        rust_log = LOG_DIR / f"chase_path_rust_{slug}.log"
        if (LOG_DIR / "chase_path_cip.log").is_file():
            (LOG_DIR / "chase_path_cip.log").replace(cip_log)
        if (LOG_DIR / "chase_path_rust.log").is_file():
            (LOG_DIR / "chase_path_rust.log").replace(rust_log)

        summary_path = LOG_DIR / f"summary_{slug}.txt"
        sum_cmd = [
            sys.executable,
            str(SUMMARIZE),
            "--ref",
            str(cip_log),
            "--rust",
            str(rust_log),
            "--monster",
            monster,
            "--max-tick",
            str(max_tick),
            "--lockstep",
        ]
        with summary_path.open("w", encoding="utf-8") as out:
            sum_proc = subprocess.run(sum_cmd, cwd=ROOT, stdout=out, stderr=subprocess.PIPE, text=True)
        if sum_proc.stderr:
            print(sum_proc.stderr, file=sys.stderr, end="")
        lockstep = sum_proc.returncode
        results.append((slug, proc.returncode, lockstep, max_tick))
        print(f"  logs: {cip_log.name}, {rust_log.name} → {summary_path.name} lockstep={lockstep}", file=sys.stderr)

    print("\n=== battery summary ===", file=sys.stderr)
    print(f"{'scenario':<10} {'run':>4} {'lockstep':>10} {'max_tick':>8}", file=sys.stderr)
    for slug, run_code, lockstep, max_tick in results:
        ls = "PASS" if lockstep == 0 else "FAIL"
        print(f"{slug:<10} {run_code:>4} {ls:>10} {max_tick:>8}", file=sys.stderr)

    if any(ls != 0 for _, _, ls, _ in results):
        return 2
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Run full chase sim battery")
    parser.add_argument("--synthetic", action="store_true")
    parser.add_argument("--skip-cpp", action="store_true")
    parser.add_argument(
        "--extended",
        action="store_true",
        help="Also run hunter/dist-flee and dragon low-HP flee scenarios (non-gating)",
    )
    args = parser.parse_args()
    return run_battery(
        synthetic=args.synthetic,
        skip_cpp=args.skip_cpp,
        extended=args.extended,
    )


if __name__ == "__main__":
    raise SystemExit(main())
