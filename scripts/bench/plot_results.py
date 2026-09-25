#!/usr/bin/env python3
"""Plot Tier 4 A/B CSVs: load-curve (bots vs CPU%/RSS/p99) and steady time series.

matplotlib is optional — missing import exits 0 after a skip message.

Usage:
  python3 scripts/bench/plot_results.py results/<timestamp>
  python3 scripts/bench/plot_results.py results/<timestamp> -o results/<timestamp>/plots
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path


def _read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _p99_us(loadgen: dict) -> float:
    walk = loadgen.get("walk") or {}
    return float(walk.get("p99_us") or 0)


def collect_load_curve_from(server_root: Path) -> dict[str, list[tuple[int, float, float, float, float]]]:
    """One results tree whose children are rust/ and/or tvp/."""
    out: dict[str, list[tuple[int, float, float, float, float]]] = {}
    for server_dir in sorted(p for p in server_root.iterdir() if p.is_dir() and p.name in ("rust", "tvp")):
        points: list[tuple[int, float, float, float, float]] = []
        for bots_dir in sorted(server_dir.iterdir(), key=lambda p: _int_or_none(p.name) or 0):
            n = _int_or_none(bots_dir.name)
            if n is None or not bots_dir.is_dir():
                continue
            reps = [p for p in bots_dir.iterdir() if p.is_dir()]
            if not reps:
                continue
            p99s: list[float] = []
            cpus: list[float] = []
            rss: list[float] = []
            work: list[float] = []
            for rep in reps:
                lg = rep / "loadgen.json"
                proc = rep / "proc.csv"
                if not lg.is_file():
                    continue
                report = _read_json(lg)
                p99s.append(_p99_us(report))
                cpu_s, cpu_pct, rss_kb = summarize_proc(proc)
                cpus.append(cpu_pct)
                rss.append(rss_kb)
                actions = float((report.get("walk") or {}).get("samples") or 0) + float(
                    (report.get("spell_rune") or {}).get("samples") or 0
                )
                if actions > 0 and cpu_s > 0:
                    work.append(cpu_s / actions)
            if not p99s:
                continue
            points.append(
                (
                    n,
                    sorted(p99s)[len(p99s) // 2],
                    sum(cpus) / len(cpus) if cpus else 0.0,
                    rss[-1] if rss else 0.0,
                    sorted(work)[len(work) // 2] if work else 0.0,
                )
            )
        out[server_dir.name] = points
    return out


def result_trees(root: Path) -> list[tuple[str, Path]]:
    """(label, path-with-rust/tvp children). Nested headline dirs or a flat tree."""
    if any(p.is_dir() and p.name in ("rust", "tvp") for p in root.iterdir()):
        return [("", root)]
    trees: list[tuple[str, Path]] = []
    for p in sorted(root.iterdir()):
        if p.is_dir() and any((p / s).is_dir() for s in ("rust", "tvp")):
            trees.append((p.name, p))
    return trees


def collect_load_curve(root: Path) -> dict[str, list[tuple[int, float, float, float, float]]]:
    """label -> points. Labels are `rust`/`tvp` or `rust/mixed_300` when nested."""
    out: dict[str, list[tuple[int, float, float, float, float]]] = {}
    for stem, tree in result_trees(root):
        part = collect_load_curve_from(tree)
        for server, points in part.items():
            key = f"{server}/{stem}" if stem else server
            out[key] = points
    return out


def _int_or_none(name: str) -> int | None:
    try:
        return int(name)
    except ValueError:
        return None


def summarize_proc(path: Path) -> tuple[float, float, float]:
    """Return (cpu_seconds last, mean cpu_pct, last rss_kb)."""
    if not path.is_file():
        return 0.0, 0.0, 0.0
    cpu_s = 0.0
    rss = 0.0
    pcts: list[float] = []
    with path.open(encoding="utf-8", newline="") as fh:
        for row in csv.DictReader(fh):
            cpu_s = float(row.get("cpu_seconds") or 0)
            rss = float(row.get("rss_kb") or 0)
            try:
                pcts.append(float(row.get("cpu_pct") or 0))
            except ValueError:
                pass
    mean_pct = sum(pcts) / len(pcts) if pcts else 0.0
    return cpu_s, mean_pct, rss


def collect_csv_xy(path: Path, x: str, y: str) -> list[tuple[float, float]]:
    if not path.is_file():
        return []
    rows: list[tuple[float, float]] = []
    with path.open(encoding="utf-8", newline="") as fh:
        for row in csv.DictReader(fh):
            try:
                rows.append((float(row.get(x) or 0), float(row.get(y) or 0)))
            except ValueError:
                continue
    return rows


def bot_counts_in(tree: Path) -> list[int]:
    counts: set[int] = set()
    for server in ("rust", "tvp"):
        d = tree / server
        if not d.is_dir():
            continue
        for p in d.iterdir():
            n = _int_or_none(p.name)
            if n is not None and p.is_dir():
                counts.add(n)
    return sorted(counts)


def collect_series(root: Path, server: str, bots: int) -> list[tuple[float, float, float]]:
    """elapsed_s, cpu_pct, rss_kb from the first rep."""
    d = root / server / str(bots)
    if not d.is_dir():
        return []
    reps = sorted(p for p in d.iterdir() if p.is_dir())
    if not reps:
        return []
    proc = reps[0] / "proc.csv"
    if not proc.is_file():
        return []
    rows: list[tuple[float, float, float]] = []
    with proc.open(encoding="utf-8", newline="") as fh:
        for row in csv.DictReader(fh):
            rows.append(
                (
                    float(row.get("elapsed_s") or 0),
                    float(row.get("cpu_pct") or 0),
                    float(row.get("rss_kb") or 0),
                )
            )
    return rows


def game_thread_cpu_series(threads_csv: Path, comm: str = "game") -> list[tuple[float, float]]:
    """Diagnosis-only: `comm` tid CPU% from consecutive `cpu_seconds` samples."""
    if not threads_csv.is_file():
        return []
    by_tid: dict[str, list[tuple[float, float]]] = {}
    with threads_csv.open(encoding="utf-8", newline="") as fh:
        for row in csv.DictReader(fh):
            if (row.get("comm") or "") != comm:
                continue
            tid = str(row.get("tid") or "")
            try:
                elapsed = float(row.get("elapsed_s") or 0)
                cpu_s = float(row.get("cpu_seconds") or 0)
            except ValueError:
                continue
            by_tid.setdefault(tid, []).append((elapsed, cpu_s))
    if not by_tid:
        return []
    series = max(by_tid.values(), key=len)
    out: list[tuple[float, float]] = []
    prev: tuple[float, float] | None = None
    for t, c in series:
        if prev is not None and t > prev[0]:
            out.append((t, 100.0 * (c - prev[1]) / (t - prev[0])))
        prev = (t, c)
    return out


def _first_rep_dir(root: Path, server: str, bots: int) -> Path | None:
    d = root / server / str(bots)
    if not d.is_dir():
        return None
    reps = sorted(p for p in d.iterdir() if p.is_dir())
    return reps[0] if reps else None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("results_dir", type=Path)
    parser.add_argument("-o", "--out", type=Path, default=None)
    args = parser.parse_args()
    try:
        import matplotlib.pyplot as plt
    except ImportError:
        print("plot_results: matplotlib not installed; skip", file=sys.stderr)
        return 0

    root = args.results_dir
    if not root.is_dir():
        print(f"plot_results: missing {root}", file=sys.stderr)
        return 1
    out = args.out or (root / "plots")
    out.mkdir(parents=True, exist_ok=True)

    curve = collect_load_curve(root)
    if any(curve.values()):
        fig, axes = plt.subplots(3, 1, figsize=(8, 10), sharex=True)
        for label, points in curve.items():
            if not points:
                continue
            ns = [p[0] for p in points]
            axes[0].plot(ns, [p[1] / 1000.0 for p in points], marker="o", label=label)
            axes[1].plot(ns, [p[2] for p in points], marker="o", label=label)
            axes[2].plot(ns, [p[3] / 1024.0 for p in points], marker="o", label=label)
        axes[0].set_ylabel("walk-ack p99 (ms)")
        axes[1].set_ylabel("CPU % (mean)")
        axes[2].set_ylabel("RSS (MiB)")
        axes[2].set_xlabel("concurrent bots")
        for ax in axes:
            ax.grid(True, alpha=0.3)
            ax.legend()
        fig.suptitle("load curve (median across reps)")
        fig.tight_layout()
        dest = out / "load_curve.png"
        fig.savefig(dest, dpi=120)
        plt.close(fig)
        print(f"wrote {dest}")

        fig, ax = plt.subplots(figsize=(8, 4))
        for label, points in curve.items():
            if not points:
                continue
            ax.plot(
                [p[0] for p in points],
                [p[4] * 1e6 for p in points],
                marker="o",
                label=label,
            )
        ax.set_xlabel("concurrent bots")
        ax.set_ylabel("CPU-µs per delivered action")
        ax.set_title("work-normalized CPU")
        ax.grid(True, alpha=0.3)
        ax.legend()
        fig.tight_layout()
        dest = out / "cpu_per_action.png"
        fig.savefig(dest, dpi=120)
        plt.close(fig)
        print(f"wrote {dest}")

    trees = result_trees(root)
    bots_all: set[int] = set()
    for _, tree in trees:
        bots_all.update(bot_counts_in(tree))
    for bots in sorted(bots_all) or (200, 300):
        fig, axes = plt.subplots(2, 1, figsize=(8, 6), sharex=True)
        drawn = False
        host_drawn = False
        host_fig, host_axes = plt.subplots(3, 1, figsize=(8, 8), sharex=True)
        mem_ax = None
        for stem, tree in trees:
            suffix = f"/{stem}" if stem else ""
            for server in ("rust", "tvp"):
                series = collect_series(tree, server, bots)
                if not series:
                    continue
                drawn = True
                t = [r[0] for r in series]
                axes[0].plot(t, [r[1] for r in series], label=f"{server}{suffix}")
                axes[1].plot(t, [r[2] / 1024.0 for r in series], label=f"{server}{suffix}")
                rep = _first_rep_dir(tree, server, bots)
                if rep is None:
                    continue
                if server == "rust":
                    game = game_thread_cpu_series(rep / "threads.csv")
                    if game:
                        axes[0].plot(
                            [g[0] for g in game],
                            [g[1] for g in game],
                            linestyle="--",
                            label=f"rust game thread{suffix}",
                        )
                lg = collect_csv_xy(rep / "loadgen_proc.csv", "elapsed_s", "cpu_pct")
                if lg:
                    axes[0].plot(
                        [p[0] for p in lg],
                        [p[1] for p in lg],
                        linestyle=":",
                        label=f"{server} loadgen{suffix}",
                    )
                mysql = collect_csv_xy(rep / "mysql_proc.csv", "elapsed_s", "cpu_pct")
                if mysql:
                    axes[0].plot(
                        [p[0] for p in mysql],
                        [p[1] for p in mysql],
                        linestyle="-.",
                        label=f"{server} mysql{suffix}",
                    )
                host = rep / "host.csv"
                idle = collect_csv_xy(host, "elapsed_s", "cpu_idle_pct")
                if idle:
                    host_drawn = True
                    tag = f"{server}{suffix}"
                    host_axes[0].plot([p[0] for p in idle], [p[1] for p in idle], label=f"{tag} idle")
                    iow = collect_csv_xy(host, "elapsed_s", "cpu_iowait_pct")
                    if iow:
                        host_axes[0].plot(
                            [p[0] for p in iow],
                            [p[1] for p in iow],
                            linestyle="--",
                            label=f"{tag} iowait",
                        )
                    load = collect_csv_xy(host, "elapsed_s", "loadavg_1")
                    mem = collect_csv_xy(host, "elapsed_s", "mem_available_kb")
                    if load:
                        host_axes[1].plot([p[0] for p in load], [p[1] for p in load], label=f"{tag} loadavg1")
                    if mem:
                        if mem_ax is None:
                            mem_ax = host_axes[1].twinx()
                            mem_ax.set_ylabel("GiB")
                        mem_ax.plot(
                            [p[0] for p in mem],
                            [p[1] / (1024.0 * 1024.0) for p in mem],
                            linestyle="--",
                            color="tab:gray",
                            label=f"{tag} MemAvail GiB",
                        )
                    freq = collect_csv_xy(host, "elapsed_s", "freq_mhz_avg")
                    power = collect_csv_xy(host, "elapsed_s", "power_w")
                    if freq:
                        host_axes[2].plot([p[0] for p in freq], [p[1] for p in freq], label=f"{tag} MHz")
                    if power and any(p[1] > 0 for p in power):
                        host_axes[2].plot(
                            [p[0] for p in power],
                            [p[1] for p in power],
                            linestyle="--",
                            label=f"{tag} RAPL W",
                        )
        if not drawn:
            plt.close(fig)
            plt.close(host_fig)
            continue
        axes[0].set_ylabel("CPU %")
        axes[1].set_ylabel("RSS (MiB)")
        axes[1].set_xlabel("elapsed s")
        axes[0].set_title(f"steady / soak time series ({bots} bots)")
        for ax in axes:
            ax.grid(True, alpha=0.3)
            ax.legend()
        fig.tight_layout()
        dest = out / f"series_{bots}.png"
        fig.savefig(dest, dpi=120)
        plt.close(fig)
        print(f"wrote {dest}")
        if host_drawn:
            host_axes[0].set_ylabel("%")
            host_axes[1].set_ylabel("loadavg")
            host_axes[2].set_ylabel("MHz / W")
            host_axes[2].set_xlabel("elapsed s")
            host_axes[0].set_title(f"host load ({bots} bots)")
            for ax in host_axes:
                ax.grid(True, alpha=0.3)
                ax.legend()
            host_fig.tight_layout()
            hdest = out / f"host_{bots}.png"
            host_fig.savefig(hdest, dpi=120)
            print(f"wrote {hdest}")
        plt.close(host_fig)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
