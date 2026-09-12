#!/usr/bin/env python3
"""1 Hz /proc sampler for a target pid (language-agnostic).

Writes CSV of utime/stime, RSS, PSS (smaps_rollup), thread count, ctx
switches, /proc/pid/io, and netns /proc/pid/net/dev byte/packet counters.
Optional per-thread CPU CSV.

Usage:
  python3 scripts/bench/sample_proc.py --pid 123 --out proc.csv
  python3 scripts/bench/sample_proc.py --pid 123 --duration 60 --threads-out threads.csv
"""

from __future__ import annotations

import argparse
import csv
import os
import sys
import time
from pathlib import Path

CLK_TCK = os.sysconf("SC_CLK_TCK")
PAGE_KB = os.sysconf("SC_PAGE_SIZE") // 1024


def monotonic_s() -> float:
    return time.clock_gettime(time.CLOCK_MONOTONIC)


def _parse_stat(text: str) -> tuple[int, int, int, int] | None:
    """utime, stime, num_threads, rss_pages — `man 5 proc` after comm."""
    rparen = text.rfind(")")
    if rparen < 0:
        return None
    fields = text[rparen + 2 :].split()
    # fields[0] is state; utime is 14th 1-based field of full stat → index 11 here
    # full: 1 pid, 2 comm, 3 state, ... 14 utime → after comm+state offset 11
    if len(fields) < 22:
        return None
    utime = int(fields[11])
    stime = int(fields[12])
    num_threads = int(fields[17])
    rss_pages = int(fields[21])
    return utime, stime, num_threads, rss_pages


def read_status(pid: int) -> tuple[int, int, int]:
    """voluntary_ctxt, nonvoluntary_ctxt, threads (fallback)."""
    vol = nonvol = threads = 0
    path = Path(f"/proc/{pid}/status")
    try:
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("voluntary_ctxt_switches:"):
                vol = int(line.split()[1])
            elif line.startswith("nonvoluntary_ctxt_switches:"):
                nonvol = int(line.split()[1])
            elif line.startswith("Threads:"):
                threads = int(line.split()[1])
    except OSError:
        return 0, 0, 0
    return vol, nonvol, threads


def read_pss_kb(pid: int) -> int:
    path = Path(f"/proc/{pid}/smaps_rollup")
    try:
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("Pss:"):
                return int(line.split()[1])
    except OSError:
        return 0
    return 0


def read_io(pid: int) -> dict[str, int]:
    out = {
        "rchar": 0,
        "wchar": 0,
        "syscr": 0,
        "syscw": 0,
        "read_bytes": 0,
        "write_bytes": 0,
    }
    path = Path(f"/proc/{pid}/io")
    try:
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            key, _, rest = line.partition(":")
            key = key.strip()
            if key in out:
                out[key] = int(rest.strip())
    except OSError:
        pass
    return out


def read_net_dev(pid: int) -> tuple[int, int, int, int]:
    """Sum rx/tx bytes and packets across interfaces in the pid's netns."""
    path = Path(f"/proc/{pid}/net/dev")
    rx_b = tx_b = rx_p = tx_p = 0
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return 0, 0, 0, 0
    for line in lines[2:]:
        if ":" not in line:
            continue
        _iface, rest = line.split(":", 1)
        cols = rest.split()
        if len(cols) < 10:
            continue
        rx_b += int(cols[0])
        rx_p += int(cols[1])
        tx_b += int(cols[8])
        tx_p += int(cols[9])
    return rx_b, rx_p, tx_b, tx_p


def sample_once(pid: int) -> dict[str, object] | None:
    try:
        stat = Path(f"/proc/{pid}/stat").read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    parsed = _parse_stat(stat)
    if parsed is None:
        return None
    utime, stime, nthreads, rss_pages = parsed
    vol, nonvol, status_threads = read_status(pid)
    io = read_io(pid)
    rx_b, rx_p, tx_b, tx_p = read_net_dev(pid)
    return {
        "mono_s": f"{monotonic_s():.6f}",
        "utime_ticks": utime,
        "stime_ticks": stime,
        "cpu_seconds": (utime + stime) / CLK_TCK,
        "rss_kb": rss_pages * PAGE_KB,
        "pss_kb": read_pss_kb(pid),
        "threads": nthreads or status_threads,
        "ctx_vol": vol,
        "ctx_nonvol": nonvol,
        "io_rchar": io["rchar"],
        "io_wchar": io["wchar"],
        "io_read_bytes": io["read_bytes"],
        "io_write_bytes": io["write_bytes"],
        "net_rx_bytes": rx_b,
        "net_rx_packets": rx_p,
        "net_tx_bytes": tx_b,
        "net_tx_packets": tx_p,
        "clk_tck": CLK_TCK,
    }


def sample_threads(pid: int) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    task = Path(f"/proc/{pid}/task")
    try:
        tids = [p.name for p in task.iterdir() if p.name.isdigit()]
    except OSError:
        return rows
    now = f"{monotonic_s():.6f}"
    for tid in tids:
        try:
            text = Path(f"/proc/{pid}/task/{tid}/stat").read_text(
                encoding="utf-8", errors="replace"
            )
        except OSError:
            continue
        parsed = _parse_stat(text)
        if parsed is None:
            continue
        utime, stime, _, _ = parsed
        comm = ""
        l = text.find("(")
        r = text.rfind(")")
        if l >= 0 and r > l:
            comm = text[l + 1 : r]
        rows.append(
            {
                "mono_s": now,
                "tid": int(tid),
                "comm": comm,
                "utime_ticks": utime,
                "stime_ticks": stime,
                "cpu_seconds": (utime + stime) / CLK_TCK,
            }
        )
    return rows


PROC_FIELDS = [
    "mono_s",
    "elapsed_s",
    "utime_ticks",
    "stime_ticks",
    "cpu_seconds",
    "cpu_pct",
    "rss_kb",
    "pss_kb",
    "threads",
    "ctx_vol",
    "ctx_nonvol",
    "io_rchar",
    "io_wchar",
    "io_read_bytes",
    "io_write_bytes",
    "net_rx_bytes",
    "net_rx_packets",
    "net_tx_bytes",
    "net_tx_packets",
    "clk_tck",
]

THREAD_FIELDS = ["mono_s", "elapsed_s", "tid", "comm", "utime_ticks", "stime_ticks", "cpu_seconds"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--interval", type=float, default=1.0, help="seconds (default 1)")
    parser.add_argument("--duration", type=float, default=None, help="stop after N seconds")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--threads-out", type=Path, default=None)
    args = parser.parse_args()

    args.out.parent.mkdir(parents=True, exist_ok=True)
    t_out = None
    t_writer = None
    if args.threads_out is not None:
        args.threads_out.parent.mkdir(parents=True, exist_ok=True)
        t_out = args.threads_out.open("w", newline="", encoding="utf-8")
        t_writer = csv.DictWriter(t_out, fieldnames=THREAD_FIELDS)
        t_writer.writeheader()

    start = monotonic_s()
    prev: dict[str, object] | None = None
    with args.out.open("w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=PROC_FIELDS)
        writer.writeheader()
        fh.flush()
        try:
            while True:
                now = monotonic_s()
                if args.duration is not None and (now - start) >= args.duration:
                    break
                row = sample_once(args.pid)
                if row is None:
                    if prev is None:
                        print(f"sample_proc: pid {args.pid} not in /proc", file=sys.stderr)
                        return 1
                    break
                elapsed = now - start
                cpu_pct = 0.0
                if prev is not None:
                    dt = float(row["mono_s"]) - float(prev["mono_s"])
                    d_ticks = (int(row["utime_ticks"]) + int(row["stime_ticks"])) - (
                        int(prev["utime_ticks"]) + int(prev["stime_ticks"])
                    )
                    if dt > 0:
                        cpu_pct = 100.0 * (d_ticks / CLK_TCK) / dt
                out_row = dict(row)
                out_row["elapsed_s"] = f"{elapsed:.3f}"
                out_row["cpu_pct"] = f"{cpu_pct:.2f}"
                writer.writerow({k: out_row[k] for k in PROC_FIELDS})
                fh.flush()
                if t_writer is not None:
                    for trow in sample_threads(args.pid):
                        trow["elapsed_s"] = f"{elapsed:.3f}"
                        t_writer.writerow({k: trow[k] for k in THREAD_FIELDS})
                    if t_out is not None:
                        t_out.flush()
                prev = row
                time.sleep(max(0.05, args.interval))
        except KeyboardInterrupt:
            pass
        finally:
            if t_out is not None:
                t_out.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
