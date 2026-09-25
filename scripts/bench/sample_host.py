#!/usr/bin/env python3
"""1 Hz host sampler (machine, not a single pid).

Writes loadavg, /proc/stat CPU breakdown (idle/iowait/user/system),
MemAvailable/swap, scaling_cur_freq, and RAPL energy when the kernel
exposes it. Clock is CLOCK_MONOTONIC, same as sample_proc.py.

Usage:
  python3 scripts/bench/sample_host.py --out host.csv --duration 60
  python3 scripts/bench/sample_host.py --self-test
"""

from __future__ import annotations

import argparse
import csv
import os
import sys
import time
from pathlib import Path

HOST_FIELDS = [
    "mono_s",
    "elapsed_s",
    "loadavg_1",
    "loadavg_5",
    "loadavg_15",
    "procs_running",
    "nproc",
    "cpu_user_pct",
    "cpu_nice_pct",
    "cpu_system_pct",
    "cpu_idle_pct",
    "cpu_iowait_pct",
    "cpu_irq_pct",
    "cpu_softirq_pct",
    "cpu_steal_pct",
    "mem_available_kb",
    "mem_free_kb",
    "mem_total_kb",
    "swap_used_kb",
    "freq_mhz_avg",
    "freq_mhz_min",
    "freq_mhz_max",
    "temp_c",
    "energy_uj",
    "power_w",
]

# /proc/stat "cpu" columns after the name: user nice system idle iowait irq softirq steal …
_STAT_KEYS = ("user", "nice", "system", "idle", "iowait", "irq", "softirq", "steal")


def monotonic_s() -> float:
    return time.clock_gettime(time.CLOCK_MONOTONIC)


def parse_stat_cpu(line: str) -> dict[str, int] | None:
    """Parse the aggregate `cpu ` line from /proc/stat."""
    parts = line.split()
    if not parts or parts[0] != "cpu":
        return None
    nums = [int(x) for x in parts[1:]]
    if len(nums) < 5:
        return None
    out = {k: nums[i] if i < len(nums) else 0 for i, k in enumerate(_STAT_KEYS)}
    return out


def cpu_pcts(prev: dict[str, int], cur: dict[str, int]) -> dict[str, float]:
    dt = sum(cur[k] - prev[k] for k in _STAT_KEYS)
    if dt <= 0:
        return {f"cpu_{k}_pct": 0.0 for k in _STAT_KEYS}
    return {f"cpu_{k}_pct": 100.0 * (cur[k] - prev[k]) / dt for k in _STAT_KEYS}


def parse_loadavg(text: str) -> tuple[float, float, float, int]:
    """`0.12 0.34 0.56 2/1234 56789` → 1/5/15, procs_running."""
    parts = text.split()
    load1 = float(parts[0])
    load5 = float(parts[1])
    load15 = float(parts[2])
    running = 0
    if len(parts) >= 4 and "/" in parts[3]:
        running = int(parts[3].split("/", 1)[0])
    return load1, load5, load15, running


def parse_meminfo(text: str) -> dict[str, int]:
    want = ("MemTotal", "MemFree", "MemAvailable", "SwapTotal", "SwapFree")
    got: dict[str, int] = {k: 0 for k in want}
    for line in text.splitlines():
        key = line.split(":", 1)[0]
        if key in got:
            got[key] = int(line.split()[1])
    return got


def read_stat_cpu() -> dict[str, int] | None:
    try:
        for line in Path("/proc/stat").read_text(encoding="utf-8", errors="replace").splitlines():
            parsed = parse_stat_cpu(line)
            if parsed is not None:
                return parsed
    except OSError:
        return None
    return None


def read_loadavg() -> tuple[float, float, float, int]:
    try:
        return parse_loadavg(Path("/proc/loadavg").read_text(encoding="utf-8", errors="replace"))
    except (OSError, ValueError, IndexError):
        return 0.0, 0.0, 0.0, 0


def read_meminfo() -> dict[str, int]:
    try:
        return parse_meminfo(Path("/proc/meminfo").read_text(encoding="utf-8", errors="replace"))
    except OSError:
        return parse_meminfo("")


def read_freq_mhz() -> tuple[float, float, float]:
    mhz: list[float] = []
    root = Path("/sys/devices/system/cpu")
    if not root.is_dir():
        return 0.0, 0.0, 0.0
    for p in sorted(root.glob("cpu[0-9]*/cpufreq/scaling_cur_freq")):
        try:
            khz = int(p.read_text(encoding="utf-8", errors="replace").strip())
        except (OSError, ValueError):
            continue
        mhz.append(khz / 1000.0)
    if not mhz:
        return 0.0, 0.0, 0.0
    return sum(mhz) / len(mhz), min(mhz), max(mhz)


def read_cpu_temp_c() -> float:
    """k10temp Tctl (AMD) or the first hwmon CPU temp, else 0."""
    hwmon = Path("/sys/class/hwmon")
    if not hwmon.is_dir():
        return 0.0
    preferred = 0.0
    fallback = 0.0
    for d in sorted(hwmon.glob("hwmon*")):
        try:
            name = (d / "name").read_text(encoding="utf-8", errors="replace").strip()
        except OSError:
            continue
        if name not in {"k10temp", "coretemp", "zenpower", "cpu_thermal"}:
            continue
        labels: dict[str, str] = {}
        for lp in d.glob("temp*_label"):
            try:
                labels[lp.name.split("_", 1)[0]] = lp.read_text(
                    encoding="utf-8", errors="replace"
                ).strip()
            except OSError:
                continue
        for ip in sorted(d.glob("temp*_input")):
            key = ip.name.split("_", 1)[0]
            try:
                milli = int(ip.read_text(encoding="utf-8", errors="replace").strip())
            except (OSError, ValueError):
                continue
            c = milli / 1000.0
            if fallback == 0.0:
                fallback = c
            if labels.get(key, "").lower() in {"tctl", "tccd1", "package id 0"}:
                preferred = c
                break
        if preferred > 0:
            return preferred
        if fallback > 0 and name == "k10temp":
            return fallback
    return preferred or fallback


def read_energy_uj() -> int:
    """Package energy in microjoules, or 0 if the kernel has no RAPL node."""
    candidates = [
        Path("/sys/class/powercap/intel-rapl:0/energy_uj"),
        Path("/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj"),
    ]
    cap = Path("/sys/class/powercap")
    if cap.is_dir():
        candidates.extend(sorted(cap.glob("intel-rapl:0/energy_uj")))
        candidates.extend(sorted(cap.glob("*/intel-rapl:0/energy_uj")))
    seen: set[Path] = set()
    for p in candidates:
        rp = p.resolve() if p.exists() else p
        if rp in seen:
            continue
        seen.add(rp)
        try:
            return int(p.read_text(encoding="utf-8", errors="replace").strip())
        except (OSError, ValueError):
            continue
    return 0


def sample_once() -> dict[str, object]:
    load1, load5, load15, running = read_loadavg()
    mem = read_meminfo()
    freq_avg, freq_min, freq_max = read_freq_mhz()
    nproc = os.cpu_count() or 0
    return {
        "mono_s": f"{monotonic_s():.6f}",
        "loadavg_1": f"{load1:.2f}",
        "loadavg_5": f"{load5:.2f}",
        "loadavg_15": f"{load15:.2f}",
        "procs_running": running,
        "nproc": nproc,
        "mem_available_kb": mem.get("MemAvailable", 0),
        "mem_free_kb": mem.get("MemFree", 0),
        "mem_total_kb": mem.get("MemTotal", 0),
        "swap_used_kb": max(0, mem.get("SwapTotal", 0) - mem.get("SwapFree", 0)),
        "freq_mhz_avg": f"{freq_avg:.1f}",
        "freq_mhz_min": f"{freq_min:.1f}",
        "freq_mhz_max": f"{freq_max:.1f}",
        "temp_c": f"{read_cpu_temp_c():.1f}",
        "energy_uj": read_energy_uj(),
        "_stat": read_stat_cpu(),
    }


def self_test() -> int:
    stat = parse_stat_cpu("cpu  100 10 50 800 40 2 8 0 0 0")
    assert stat is not None
    assert stat["user"] == 100 and stat["idle"] == 800 and stat["iowait"] == 40
    prev = parse_stat_cpu("cpu  100 10 50 800 40 2 8 0")
    cur = parse_stat_cpu("cpu  110 10 55 890 45 2 8 0")
    assert prev is not None and cur is not None
    pct = cpu_pcts(prev, cur)
    # deltas: user 10, system 5, idle 90, iowait 5 → total 110
    assert abs(pct["cpu_user_pct"] - 100.0 * 10 / 110) < 1e-9
    assert abs(pct["cpu_idle_pct"] - 100.0 * 90 / 110) < 1e-9
    assert abs(pct["cpu_iowait_pct"] - 100.0 * 5 / 110) < 1e-9
    load1, load5, load15, running = parse_loadavg("1.23 4.56 7.89 3/2100 9999")
    assert (load1, load5, load15, running) == (1.23, 4.56, 7.89, 3)
    mem = parse_meminfo(
        "MemTotal:        16000000 kB\nMemFree:          1000000 kB\n"
        "MemAvailable:     8000000 kB\nSwapTotal:         2000000 kB\n"
        "SwapFree:          1500000 kB\n"
    )
    assert mem["MemAvailable"] == 8000000
    assert mem["SwapTotal"] - mem["SwapFree"] == 500000
    print("sample_host: self-test ok", file=sys.stderr)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--interval", type=float, default=1.0, help="seconds (default 1)")
    parser.add_argument("--duration", type=float, default=None, help="stop after N seconds")
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.out is None:
        parser.error("--out is required unless --self-test")

    args.out.parent.mkdir(parents=True, exist_ok=True)
    start = monotonic_s()
    prev_stat: dict[str, int] | None = None
    prev_energy: int | None = None
    prev_mono: float | None = None
    zero_pct = {f"cpu_{k}_pct": 0.0 for k in _STAT_KEYS}
    with args.out.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=HOST_FIELDS)
        writer.writeheader()
        fh.flush()
        try:
            while True:
                now = monotonic_s()
                if args.duration is not None and (now - start) >= args.duration:
                    break
                raw = sample_once()
                elapsed = now - start
                stat = raw.pop("_stat")
                pcts = zero_pct
                if isinstance(stat, dict) and prev_stat is not None:
                    pcts = cpu_pcts(prev_stat, stat)
                if isinstance(stat, dict):
                    prev_stat = stat
                energy = int(raw["energy_uj"])
                power_w = 0.0
                if prev_energy is not None and prev_mono is not None:
                    dt = float(raw["mono_s"]) - prev_mono
                    if dt > 0 and energy >= prev_energy:
                        power_w = (energy - prev_energy) / dt / 1_000_000.0
                prev_energy = energy
                prev_mono = float(raw["mono_s"])
                out_row = dict(raw)
                out_row["elapsed_s"] = f"{elapsed:.3f}"
                out_row["power_w"] = f"{power_w:.2f}"
                for k, v in pcts.items():
                    out_row[k] = f"{v:.2f}"
                writer.writerow({k: out_row.get(k, 0) for k in HOST_FIELDS})
                fh.flush()
                time.sleep(max(0.05, args.interval))
        except KeyboardInterrupt:
            pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
