#!/usr/bin/env python3
"""P1 real-map FillMap probe — cyclops gravel bowl Rust vs C++."""

from __future__ import annotations

import argparse
import os
import socket
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOG_DIR = ROOT / "log"
SCENARIO = ROOT / "scripts" / "scenarios" / "kite_cyclops_one_real.scenario"
COMPARE = ROOT / "scripts" / "compare_fill_walkable.py"
RUN = ROOT / "scripts" / "run_kite_scenario.py"

RUST_DUMP = LOG_DIR / "fill_walkable_rust_cyclops_bowl.json"
CIP_LOG = LOG_DIR / "chase_path_cip_realmap_cyclops_one_real.log"


def query_manager_listening(host: str = "127.0.0.1", port: int = 7173) -> bool:
    try:
        with socket.create_connection((host, port), timeout=0.5):
            return True
    except OSError:
        return False


def run_rust_dump() -> int:
    env = {
        **os.environ,
        "TFS_FILLMAP_DUMP": "1",
        "TFS_FILLMAP_DUMP_PATH": str(RUST_DUMP),
    }
    cmd = [
        "cargo",
        "test",
        "-p",
        "tfs-rust-core",
        "cyclops_bowl_real_fill_walkable_dump_at_tick_2000",
        "--",
        "--nocapture",
    ]
    print("+", " ".join(cmd), file=sys.stderr)
    proc = subprocess.run(cmd, cwd=ROOT, env=env)
    if proc.returncode != 0:
        return proc.returncode
    if not RUST_DUMP.is_file():
        print(f"error: Rust dump not written: {RUST_DUMP}", file=sys.stderr)
        return 1
    return 0


def run_cpp_scenario(*, skip_cpp: bool) -> int:
    if skip_cpp:
        print("skip-cpp: using existing C++ log if present", file=sys.stderr)
        return 0 if CIP_LOG.is_file() else 2

    if not query_manager_listening():
        print(
            "error: query manager not listening on 127.0.0.1:7173 — start scripts/tibia_game_dev.sh run-qm",
            file=sys.stderr,
        )
        return 1

    env = {
        **os.environ,
        "TIBIA_CHASE_FILLMAP_DUMP": "1",
        "TFS_KITE_NO_WILD": "1",
        "TFS_SIM_SEED": os.environ.get("TFS_SIM_SEED", "772"),
    }
    cmd = [sys.executable, str(RUN), "--real-map", str(SCENARIO)]
    print("+", " ".join(cmd), file=sys.stderr)
    proc = subprocess.run(cmd, cwd=ROOT, env=env)
    if proc.returncode not in (0, 1, 2):
        return proc.returncode

    default_cip = LOG_DIR / "chase_path_cip_realmap.log"
    if default_cip.is_file():
        default_cip.replace(CIP_LOG)
    if not CIP_LOG.is_file():
        print(f"error: C++ log missing: {CIP_LOG}", file=sys.stderr)
        return 1
    return 0


def compare() -> int:
    if not RUST_DUMP.is_file():
        print(f"error: missing rust dump {RUST_DUMP}", file=sys.stderr)
        return 2
    if not CIP_LOG.is_file():
        print(f"error: missing ref log {CIP_LOG}", file=sys.stderr)
        return 2

    cmd = [
        sys.executable,
        str(COMPARE),
        "--rust",
        str(RUST_DUMP),
        "--ref",
        str(CIP_LOG),
        "--preset",
        "cyclops-bowl",
        "--start",
        "32454",
        "32065",
        "7",
    ]
    print("+", " ".join(cmd), file=sys.stderr)
    return subprocess.run(cmd, cwd=ROOT).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-cpp", action="store_true", help="Rust dump + compare only")
    parser.add_argument("--rust-only", action="store_true", help="Only write Rust JSON dump")
    args = parser.parse_args()

    LOG_DIR.mkdir(parents=True, exist_ok=True)

    rc = run_rust_dump()
    if rc != 0:
        return rc
    if args.rust_only:
        print(f"rust dump: {RUST_DUMP}", file=sys.stderr)
        return 0

    rc = run_cpp_scenario(skip_cpp=args.skip_cpp)
    if rc != 0:
        return rc

    return compare()


if __name__ == "__main__":
    raise SystemExit(main())
