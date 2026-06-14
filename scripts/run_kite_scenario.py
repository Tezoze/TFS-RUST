#!/usr/bin/env python3
"""
Run a shared kite scenario on TFS-RUST and tibia-game-master, then diff chase JSONL logs.

Usage:
  python3 scripts/run_kite_scenario.py scripts/scenarios/kite_rat_melee.scenario
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import socket
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
LOG_DIR = ROOT / "log"
RUST_LOG = LOG_DIR / "chase_path_rust.log"
CIP_LOG = LOG_DIR / "chase_path_cip.log"
COMPARE = ROOT / "scripts" / "compare_chase_live_logs.py"


def scenario_wall_ms(scenario_path: Path) -> int:
    total = 0
    for line in scenario_path.read_text(encoding="utf-8", errors="replace").splitlines():
        parts = line.strip().split()
        if len(parts) >= 2 and parts[0] == "advance_ms":
            total += int(parts[1])
    return total


def count_diag_go_exec(path: Path, monster: str | None) -> tuple[int, int]:
    total = 0
    diag = 0
    if not path.is_file():
        return total, diag
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line or line[0] != "{":
            continue
        try:
            evt = json.loads(line)
        except json.JSONDecodeError:
            continue
        if evt.get("evt") != "go_exec":
            continue
        if monster and not _monster_name_matches(str(evt.get("name", "")), monster):
            continue
        total += 1
        if int(evt.get("diag", 0)) != 0:
            diag += 1
    return total, diag


def _monster_name_matches(event_name: str, filter_name: str) -> bool:
    en = event_name.lower().strip()
    fn = filter_name.lower().strip()
    if en == fn or en.endswith(f" {fn}") or en.endswith(fn):
        return True
    return fn in en


def run(cmd: list[str], *, env: dict[str, str] | None = None, cwd: Path | None = None) -> None:
    print("+", " ".join(cmd), file=sys.stderr)
    subprocess.run(cmd, check=True, env=env, cwd=cwd or ROOT)


def scenario_monster_label(scenario_path: Path) -> str | None:
    for line in scenario_path.read_text(encoding="utf-8", errors="replace").splitlines():
        parts = line.strip().split()
        if len(parts) >= 2 and parts[0] == "monster":
            return parts[1]
    return None


def query_manager_listening(host: str = "127.0.0.1", port: int = 7173) -> bool:
    try:
        with socket.create_connection((host, port), timeout=0.5):
            return True
    except OSError:
        return False


def print_cpp_prerequisites(runtime: Path) -> None:
    print(
        "\nC++ chase-scenario needs the query manager (even headless — InitAll loads world config).\n"
        "Start it in another terminal, then re-run:\n"
        "  scripts/tibia_game_dev.sh run-qm\n"
        "Or start the full stack:\n"
        "  scripts/tibia_game_online.sh start\n"
        f"Runtime cwd for game: {runtime}\n"
        "Rust-only iteration (no compare):\n"
        "  python3 scripts/run_kite_scenario.py --skip-cpp --synthetic <scenario>\n",
        file=sys.stderr,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Run kite scenario on Rust + C++ and compare logs")
    parser.add_argument("scenario", type=Path, help="Path to .scenario file")
    parser.add_argument(
        "--monster",
        default=None,
        help="Monster name filter for compare/diag stats (default: first monster line in scenario)",
    )
    parser.add_argument("--skip-cpp", action="store_true", help="Only run Rust executor")
    parser.add_argument(
        "--synthetic",
        action="store_true",
        help="Rust: lay flat synthetic arena tiles instead of OTBM",
    )
    parser.add_argument("--data-dir", type=Path, help="Rust: TFS_DATA_DIR (default data/)")
    parser.add_argument("--map", help="Rust: OTBM path relative to data dir")
    args = parser.parse_args()

    scenario = args.scenario.resolve()
    if not scenario.is_file():
        print(f"error: scenario not found: {scenario}", file=sys.stderr)
        return 1

    monster = args.monster or scenario_monster_label(scenario) or "rat"
    print(f"monster filter: {monster}", file=sys.stderr)

    LOG_DIR.mkdir(parents=True, exist_ok=True)
    for path in (RUST_LOG, CIP_LOG):
        if path.exists():
            path.unlink()

    rust_cmd = [
        "cargo",
        "run",
        "-p",
        "tfs-rust-core",
        "--bin",
        "chase_kite_sim",
        "--",
        str(scenario),
        "--log",
        str(RUST_LOG),
    ]
    if args.synthetic:
        rust_cmd.append("--synthetic")
    if args.data_dir:
        rust_cmd.extend(["--data-dir", str(args.data_dir.resolve())])
    if args.map:
        rust_cmd.extend(["--map", args.map])

    run(
        rust_cmd,
        env={
            **os.environ,
            "TFS_CHASE_PATH_DEBUG": "1",
            "TFS_CHASE_PATH_LOG": str(RUST_LOG),
            "TFS_SIM_SEED": os.environ.get("TFS_SIM_SEED", "772"),
            **({"TFS_KITE_SYNTHETIC_ARENA": "1"} if args.synthetic else {}),
        },
    )

    if not args.skip_cpp:
        tgm = os.environ.get(
            "TIBIA_GAME_MASTER_DIR",
            str(ROOT / "reference" / "cipsoft-772" / "tibia-game-master"),
        )
        runtime = os.environ.get(
            "TIBIA_GAME_DATA",
            str(ROOT / "reference" / "cipsoft-772" / "runtime"),
        )
        game_bin = Path(tgm) / "build" / "game"
        if not game_bin.is_file():
            print("error: C++ game binary missing — run scripts/tibia_game_dev.sh build", file=sys.stderr)
            return 1

        runtime_path = Path(runtime)
        if not (runtime_path / ".tibia").is_file():
            print(f"error: missing {runtime_path}/.tibia — run scripts/tibia_game_dev.sh setup", file=sys.stderr)
            return 1

        if not query_manager_listening():
            print("error: query manager not listening on 127.0.0.1:7173", file=sys.stderr)
            print_cpp_prerequisites(runtime_path)
            return 1

        cip_tmp = LOG_DIR / "chase_path.log"
        if cip_tmp.exists():
            cip_tmp.unlink()

        runtime_log = Path(runtime) / "log" / "chase_ai.jsonl"
        if runtime_log.exists():
            runtime_log.unlink()

        run(
            [str(game_bin), "chase-scenario", str(scenario)],
            env={
                **os.environ,
                "TIBIA_CHASE_PATH_DEBUG": "1",
                "TFS_SIM_SEED": os.environ.get("TFS_SIM_SEED", "772"),
                **({"TFS_KITE_SYNTHETIC_ARENA": "1"} if args.synthetic else {}),
            },
            cwd=Path(runtime),
        )

        runtime_log = Path(runtime) / "log" / "chase_ai.jsonl"
        if not runtime_log.is_file():
            runtime_log = Path(runtime) / "log" / "chase_path.log"
        if runtime_log.is_file():
            shutil.copy2(runtime_log, CIP_LOG)
        elif cip_tmp.is_file():
            shutil.move(str(cip_tmp), str(CIP_LOG))
        elif not CIP_LOG.is_file():
            print(f"warn: C++ did not write {cip_tmp}", file=sys.stderr)

    if not CIP_LOG.is_file() and not args.skip_cpp:
        return 1

    if args.skip_cpp:
        rust_total, rust_diag = count_diag_go_exec(RUST_LOG, monster)
        print(f"Rust go_exec: {rust_total}  diagonal: {rust_diag}")
        return 0

    compare_rc = 0
    wall_ms = scenario_wall_ms(scenario)

    summarize_cmd = [
        sys.executable,
        str(ROOT / "scripts" / "summarize_chase_gaps.py"),
        "--ref",
        str(CIP_LOG),
        "--rust",
        str(RUST_LOG),
        "--monster",
        monster,
        "--lockstep",
    ]
    if wall_ms > 0:
        summarize_cmd.extend(["--max-tick", str(wall_ms)])

    compare_cmd = [
        sys.executable,
        str(COMPARE),
        "--ref",
        str(CIP_LOG),
        "--rust",
        str(RUST_LOG),
        "--monster",
        monster,
    ]
    if wall_ms > 0:
        compare_cmd.extend(["--max-tick", str(wall_ms)])
    try:
        run(compare_cmd)
    except subprocess.CalledProcessError:
        compare_rc = 1

    try:
        run(summarize_cmd)
    except subprocess.CalledProcessError as exc:
        compare_rc = max(compare_rc, exc.returncode or 1)

    ref_total, ref_diag = count_diag_go_exec(CIP_LOG, monster)
    rust_total, rust_diag = count_diag_go_exec(RUST_LOG, monster)
    print(f"\nScenario wall_ms={wall_ms}")
    print(f"Diagonal go_exec — ref: {ref_diag}/{ref_total}  rust: {rust_diag}/{rust_total}")
    return compare_rc


if __name__ == "__main__":
    raise SystemExit(main())
