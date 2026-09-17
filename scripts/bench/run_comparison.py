#!/usr/bin/env python3
"""Orchestrate Tier 4 A/B runs: start one server, sample /proc, drive tfs-loadgen.

Never runs both servers at once. Alternates rust/tvp per cell. Writes
`results/<timestamp>/<server>/<bots>/repN/{loadgen.json,proc.csv,threads.csv}`.

Modes (pinned-host, not CI):
  load-curve   bots 25/50/100/200/300/400/600; default both mixed_300 + clustered_hunt
  steady       300 bots, duration from scenario
  overload     one point past the requested knee (default 600)
  soak         200 bots, 3600 s
  equivalence  5 bots × each isolation role, then check_equivalence.py

`--scenario` pins a single RON (flat `results/<ts>/<server>/…` tree).
Default load-curve (no `--scenario`) writes `results/<ts>/<scenario_stem>/…`.

Usage:
  python3 scripts/bench/run_comparison.py --dry-run --mode load-curve
  python3 scripts/bench/run_comparison.py --mode equivalence --reps 1
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ROLES = ("walker", "melee", "caster", "rune", "aoe_rune", "noise")
DEFAULT_POINTS = (25, 50, 100, 200, 300, 400, 600)
SCENARIO_MIXED = ROOT / "bench" / "scenarios" / "mixed_300.ron"
SCENARIO_HUNT = ROOT / "bench" / "scenarios" / "clustered_hunt.ron"
HEADLINE_SCENARIOS = (SCENARIO_MIXED, SCENARIO_HUNT)


def monotonic_s() -> float:
    return time.clock_gettime(time.CLOCK_MONOTONIC)


def wait_for_port(port: int, timeout: float = 90.0) -> None:
    deadline = monotonic_s() + timeout
    while monotonic_s() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(0.5)
            try:
                s.connect(("127.0.0.1", port))
                return
            except OSError:
                time.sleep(0.25)
    raise RuntimeError(f"nothing listening on 127.0.0.1:{port} within {timeout}s")


def free_ports() -> None:
    if shutil.which("fuser"):
        subprocess.run(
            ["fuser", "-k", "-n", "tcp", "7171"],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        subprocess.run(
            ["fuser", "-k", "-n", "tcp", "7172"],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(0.35)


def host_meta() -> dict:
    cpu = ""
    try:
        for line in Path("/proc/cpuinfo").read_text(encoding="utf-8", errors="replace").splitlines():
            if line.lower().startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass
    mem_kb = 0
    try:
        for line in Path("/proc/meminfo").read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("MemTotal:"):
                mem_kb = int(line.split()[1])
                break
    except OSError:
        pass
    gov = ""
    gpath = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    if gpath.is_file():
        gov = gpath.read_text(encoding="utf-8", errors="replace").strip()
    git = ""
    try:
        git = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    uname = os.uname()
    return {
        "cpu_model": cpu,
        "mem_kb": mem_kb,
        "governor": gov,
        "kernel": f"{uname.sysname} {uname.release}",
        "git": git,
        "clk": "CLOCK_MONOTONIC",
    }


def cargo_target_dir() -> Path:
    """Where `cargo build` writes. Honors `CARGO_TARGET_DIR` (sandboxed shells set it) so
    the binary we run is the binary we built — a stale `./target` copy silently invalidates
    an A/B cell."""
    return Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))


def rust_bin() -> Path:
    return cargo_target_dir() / "release" / "tfs-rust"


def tvp_bin() -> Path:
    tvp = os.environ.get("TFS_TVP_772_DIR", str(ROOT / "reference" / "tvp-772"))
    return Path(tvp) / "gameserver" / "build" / "tfs"


def tvp_cwd() -> Path:
    return tvp_bin().parent.parent


def taskset_prefix(cpuset: str | None) -> list[str]:
    if not cpuset:
        return []
    if not shutil.which("taskset"):
        raise RuntimeError("taskset not found (util-linux)")
    return ["taskset", "-c", cpuset]


def build_rust(dry: bool) -> list[str]:
    cmd = ["cargo", "build", "--release", "--bin", "tfs-rust"]
    if dry:
        return cmd
    subprocess.run(cmd, cwd=ROOT, check=True)
    return cmd


def build_loadgen(dry: bool) -> list[str]:
    cmd = ["cargo", "build", "--release", "-p", "tfs-loadgen", "--bin", "tfs-loadgen"]
    if dry:
        return cmd
    subprocess.run(cmd, cwd=ROOT, check=True)
    return cmd


def loadgen_bin() -> Path:
    return cargo_target_dir() / "release" / "tfs-loadgen"


def set_lua_key(path: Path, key: str, value: str) -> None:
    if not path.is_file():
        return
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    out: list[str] = []
    found = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith(f"{key} ") or stripped.startswith(f"{key}="):
            out.append(f"{key} = {value}")
            found = True
        else:
            out.append(line)
    if not found:
        out.append(f"{key} = {value}")
    path.write_text("\n".join(out) + "\n", encoding="utf-8")


def apply_tvp_overlay(*, disable_saves: bool, dry: bool = False) -> None:
    cfg = tvp_cwd() / "config.lua"
    dist = tvp_cwd() / "config.lua.dist"
    if not cfg.is_file() and dist.is_file():
        if dry:
            print(f"would copy {dist} → {cfg}", file=sys.stderr)
        else:
            shutil.copy2(dist, cfg)
    sync = ROOT / "scripts" / "sync_tvp_world.sh"
    print("sync_tvp:", sync, file=sys.stderr)
    if not dry:
        subprocess.run(["bash", str(sync)], cwd=ROOT, check=True)
    if disable_saves and cfg.is_file() and not dry:
        set_lua_key(cfg, "enablePlayerDataFiles", "false")
        set_lua_key(cfg, "enableMapDataFiles", "false")


def ensure_tfs_obs(env: dict[str, str]) -> None:
    """Force `tfs_obs=info` so `game_obs_summary` lands in rust `server.log`.

    `setdefault` is not enough: a parent `RUST_LOG` (often `tfs_obs=off` from
    `rust-src/main.rs` default) wins and the 10s windows never emit. Later
    EnvFilter directives override, so appending always enables the target.
    """
    cur = (env.get("RUST_LOG") or "").strip()
    if not cur:
        env["RUST_LOG"] = "tfs_obs=info,info"
        return
    env["RUST_LOG"] = f"{cur},tfs_obs=info"


def start_server(server: str, *, cpuset: str | None, log_path: Path, dry: bool) -> subprocess.Popen | None:
    env = os.environ.copy()
    if server == "rust":
        ensure_tfs_obs(env)
        # TVP verifies SHA1 and stops. Rust otherwise bcrypt-upgrades every
        # seeder SHA1 on login (cost 12 ≈ 200ms × N, Tokio blocking pool).
        env.setdefault("TFS_UPGRADE_SHA1_ON_LOGIN", "0")
        cmd = taskset_prefix(cpuset) + [str(rust_bin())]
        cwd = ROOT
    else:
        cmd = taskset_prefix(cpuset) + [str(tvp_bin())]
        cwd = tvp_cwd()
    print("start:", " ".join(cmd), f"(cwd={cwd})", file=sys.stderr)
    if dry:
        return None
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_fh = log_path.open("w", encoding="utf-8")
    proc = subprocess.Popen(
        cmd,
        cwd=str(cwd),
        env=env,
        stdout=log_fh,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    proc._bench_log = log_fh  # type: ignore[attr-defined]
    return proc


def stop_proc(proc: subprocess.Popen | None) -> None:
    if proc is None:
        return
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except (ProcessLookupError, PermissionError, OSError):
        proc.terminate()
    try:
        proc.wait(timeout=15)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError, OSError):
            proc.kill()
        proc.wait(timeout=5)
    log_fh = getattr(proc, "_bench_log", None)
    if log_fh is not None:
        log_fh.close()


def start_sampler(pid: int, out_dir: Path, duration: float, dry: bool) -> subprocess.Popen | None:
    cmd = [
        sys.executable,
        str(ROOT / "scripts" / "bench" / "sample_proc.py"),
        "--pid",
        str(pid),
        "--interval",
        "1",
        "--duration",
        str(int(duration) + 30),
        "--out",
        str(out_dir / "proc.csv"),
        "--threads-out",
        str(out_dir / "threads.csv"),
    ]
    print("sample:", " ".join(cmd), file=sys.stderr)
    if dry:
        return None
    return subprocess.Popen(cmd)


def loadgen_cmd(
    *,
    scenario: Path,
    bots: int,
    duration_s: int | None,
    out: Path,
    cpuset: str | None,
) -> list[str]:
    cmd = taskset_prefix(cpuset) + [
        str(loadgen_bin()),
        "--scenario",
        str(scenario),
        "--bots",
        str(bots),
        "--out",
        str(out),
        "--progress",
    ]
    if duration_s is not None:
        cmd.extend(["--duration-s", str(duration_s)])
    return cmd


def items_otb_for(server: str) -> Path:
    if server == "tvp":
        tvp = Path(os.environ.get("TFS_TVP_772_DIR", str(ROOT / "reference" / "tvp-772")))
        return tvp / "gameserver" / "data" / "items" / "items.otb"
    return ROOT / "data" / "items" / "items.otb"


def run_loadgen(cmd: list[str], *, timeout: float, dry: bool, server: str) -> int:
    print("loadgen:", " ".join(cmd), file=sys.stderr)
    if dry:
        return 0
    env = os.environ.copy()
    otb = items_otb_for(server)
    if otb.is_file():
        env["TFS_ITEMS_OTB"] = str(otb)
    return subprocess.run(cmd, cwd=ROOT, timeout=timeout, check=False, env=env).returncode


def cell_dir(root: Path, server: str, bots: int, rep: int) -> Path:
    return root / server / str(bots) / f"rep{rep}"


def run_cell(
    *,
    server: str,
    bots: int,
    rep: int,
    scenario: Path,
    duration_s: int | None,
    out_root: Path,
    cpuset_server: str | None,
    cpuset_loadgen: str | None,
    dry: bool,
    warmup_s: int,
) -> None:
    dest = cell_dir(out_root, server, bots, rep)
    dest.mkdir(parents=True, exist_ok=True)
    measure = duration_s if duration_s is not None else 120
    wall = warmup_s + measure + max(30, bots // 8 + 15)
    if dry:
        start_server(server, cpuset=cpuset_server, log_path=dest / "server.log", dry=True)
        start_sampler(0, dest, wall, True)
        run_loadgen(
            loadgen_cmd(
                scenario=scenario,
                bots=bots,
                duration_s=duration_s,
                out=dest / "loadgen.json",
                cpuset=cpuset_loadgen,
            ),
            timeout=wall + 60,
            dry=True,
            server=server,
        )
        return

    free_ports()
    srv = start_server(server, cpuset=cpuset_server, log_path=dest / "server.log", dry=False)
    assert srv is not None
    sampler = None
    try:
        wait_for_port(7171)
        wait_for_port(7172)
        sampler = start_sampler(srv.pid, dest, wall, False)
        rc = run_loadgen(
            loadgen_cmd(
                scenario=scenario,
                bots=bots,
                duration_s=duration_s,
                out=dest / "loadgen.json",
                cpuset=cpuset_loadgen,
            ),
            timeout=wall + 120,
            dry=False,
            server=server,
        )
        (dest / "loadgen_exit.txt").write_text(f"{rc}\n", encoding="utf-8")
        if rc != 0:
            print(f"loadgen exit {rc} for {server} bots={bots} rep={rep}", file=sys.stderr)
    finally:
        if sampler is not None:
            sampler.send_signal(signal.SIGINT)
            try:
                sampler.wait(timeout=5)
            except subprocess.TimeoutExpired:
                sampler.kill()
        stop_proc(srv)
        free_ports()


def alternate_servers(servers: list[str], rep: int) -> list[str]:
    if rep % 2 == 1:
        return list(reversed(servers))
    return list(servers)


def write_meta(out_root: Path, args: argparse.Namespace) -> None:
    meta = host_meta()
    meta["args"] = {
        "mode": args.mode,
        "servers": args.servers,
        "reps": args.reps,
        "points": args.points,
        "scenario": str(args.scenario) if args.scenario else "headlines",
        "cpuset_server": args.cpuset_server,
        "cpuset_loadgen": args.cpuset_loadgen,
        "disable_saves": args.disable_saves,
    }
    meta["rust_bin"] = str(rust_bin())
    meta["tvp_bin"] = str(tvp_bin())
    # Binary age next to `git` makes a stale-build cell detectable after the fact.
    for key, path in (("rust_bin", rust_bin()), ("loadgen_bin", loadgen_bin())):
        if path.is_file():
            meta[f"{key}_mtime"] = time.strftime(
                "%Y-%m-%dT%H:%M:%S", time.localtime(path.stat().st_mtime)
            )
    (out_root / "meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")


def parse_points(text: str) -> list[int]:
    return [int(x.strip()) for x in text.split(",") if x.strip()]


def seed_cluster_accounts(*, dry: bool, count: int = 10) -> None:
    """Reset Test* login tiles to the Cyclops disk. Logout after a cell can
    persist temple (death) or walked-off coords; each isolation role must start
    on the hunt layout or unique_creatures compares the wrong map. Health 5000
    (both servers identically) so Cyclops focus does not kill a 1000-HP bot
    mid-cell — a death drops inventory and respawns at temple, poisoning the
    combat rows."""
    cmd = [
        sys.executable,
        str(ROOT / "scripts" / "seed_bench_accounts.py"),
        "--count",
        str(count),
        "--apply",
        "--target",
        "both",
        "--layout",
        "cluster",
        "--health",
        "5000",
    ]
    print("seed:", " ".join(cmd), file=sys.stderr)
    if dry:
        return
    subprocess.run(cmd, cwd=ROOT, check=True)


def run_equivalence(args: argparse.Namespace, out_root: Path) -> int:
    duration = args.duration_s or 30
    servers = [s.strip() for s in args.servers.split(",") if s.strip()]
    for server in servers:
        for role in ROLES:
            seed_cluster_accounts(dry=args.dry_run)
            scenario = ROOT / "bench" / "scenarios" / f"{role}.ron"
            dest = out_root / "equivalence" / server
            dest.mkdir(parents=True, exist_ok=True)
            run_cell(
                server=server,
                bots=5,
                rep=0,
                scenario=scenario,
                duration_s=duration,
                out_root=out_root / "equivalence",
                cpuset_server=args.cpuset_server,
                cpuset_loadgen=args.cpuset_loadgen,
                dry=args.dry_run,
                warmup_s=5,
            )
            src = cell_dir(out_root / "equivalence", server, 5, 0) / "loadgen.json"
            role_json = dest / f"{role}.json"
            if args.dry_run:
                print(f"would copy {src} → {role_json}", file=sys.stderr)
            elif src.is_file():
                shutil.copy2(src, role_json)
    gate = [
        sys.executable,
        str(ROOT / "scripts" / "bench" / "check_equivalence.py"),
        "--dir",
        str(out_root / "equivalence"),
    ]
    print("gate:", " ".join(gate), file=sys.stderr)
    if args.dry_run:
        return 0
    return subprocess.run(gate, check=False).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--mode",
        choices=("load-curve", "steady", "overload", "soak", "equivalence"),
        default="load-curve",
    )
    parser.add_argument("--servers", default="rust,tvp", help="comma list, run sequentially")
    parser.add_argument("--reps", type=int, default=3)
    parser.add_argument(
        "--points",
        default=",".join(str(p) for p in DEFAULT_POINTS),
        help="comma-separated bot counts for load-curve",
    )
    parser.add_argument("--bots", type=int, default=None, help="override bots for steady/overload/soak")
    parser.add_argument("--duration-s", type=int, default=None)
    parser.add_argument(
        "--scenario",
        type=Path,
        default=None,
        help="single RON (default load-curve: mixed_300 + clustered_hunt; other modes: mixed_300)",
    )
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--cpuset-server", default=None)
    parser.add_argument("--cpuset-loadgen", default=None)
    parser.add_argument("--disable-saves", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()

    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out_root = args.out or (ROOT / "results" / ts)
    out_root.mkdir(parents=True, exist_ok=True)
    servers = [s.strip() for s in args.servers.split(",") if s.strip()]
    for s in servers:
        if s not in ("rust", "tvp"):
            print(f"unknown server {s}", file=sys.stderr)
            return 2

    if args.disable_saves:
        os.environ["TFS_BENCH_DISABLE_SAVES"] = "1"
    if "tvp" in servers:
        apply_tvp_overlay(disable_saves=args.disable_saves, dry=args.dry_run)

    if not args.skip_build:
        if "rust" in servers:
            build_rust(args.dry_run)
        build_loadgen(args.dry_run)
        if "tvp" in servers and not args.dry_run and not tvp_bin().is_file():
            print("tvp binary missing — run ./scripts/build_tvp.sh", file=sys.stderr)
            return 1

    write_meta(out_root, args)
    print(f"results → {out_root}", file=sys.stderr)

    if args.mode == "equivalence":
        return run_equivalence(args, out_root)

    if args.scenario is not None:
        scenarios = [args.scenario]
        nested = False
    elif args.mode == "load-curve":
        scenarios = list(HEADLINE_SCENARIOS)
        nested = True
    else:
        scenarios = [SCENARIO_MIXED]
        nested = False

    if args.mode == "load-curve":
        points = parse_points(args.points)
        duration = args.duration_s
        warmup = 30
    elif args.mode == "steady":
        points = [args.bots or 300]
        duration = args.duration_s
        warmup = 30
    elif args.mode == "overload":
        points = [args.bots or 600]
        duration = args.duration_s
        warmup = 30
    else:  # soak
        points = [args.bots or 200]
        duration = args.duration_s or 3600
        warmup = 30

    for scenario in scenarios:
        cell_root = out_root / scenario.stem if nested else out_root
        print(f"scenario {scenario} → {cell_root}", file=sys.stderr)
        for bots in points:
            for rep in range(args.reps):
                for server in alternate_servers(servers, rep):
                    run_cell(
                        server=server,
                        bots=bots,
                        rep=rep,
                        scenario=scenario,
                        duration_s=duration,
                        out_root=cell_root,
                        cpuset_server=args.cpuset_server,
                        cpuset_loadgen=args.cpuset_loadgen,
                        dry=args.dry_run,
                        warmup_s=warmup,
                    )

    plot = [
        sys.executable,
        str(ROOT / "scripts" / "bench" / "plot_results.py"),
        str(out_root),
    ]
    print("plot:", " ".join(plot), file=sys.stderr)
    if not args.dry_run:
        subprocess.run(plot, check=False)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
