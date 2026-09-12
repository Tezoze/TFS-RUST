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


def collect_load_curve(root: Path) -> dict[str, list[tuple[int, float, float, float, float]]]:
    """server -> [(bots, p99_us, cpu_pct_mean, rss_kb_last, cpu_s_per_action)]."""
    out: dict[str, list[tuple[int, float, float, float, float]]] = {}
    for server_dir in sorted(p for p in root.iterdir() if p.is_dir() and p.name in ("rust", "tvp")):
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
        for server, points in curve.items():
            if not points:
                continue
            ns = [p[0] for p in points]
            axes[0].plot(ns, [p[1] / 1000.0 for p in points], marker="o", label=server)
            axes[1].plot(ns, [p[2] for p in points], marker="o", label=server)
            axes[2].plot(ns, [p[3] / 1024.0 for p in points], marker="o", label=server)
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
        for server, points in curve.items():
            if not points:
                continue
            ax.plot(
                [p[0] for p in points],
                [p[4] * 1e6 for p in points],
                marker="o",
                label=server,
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

    for bots in (200, 300):
        fig, axes = plt.subplots(2, 1, figsize=(8, 6), sharex=True)
        drawn = False
        for server in ("rust", "tvp"):
            series = collect_series(root, server, bots)
            if not series:
                continue
            drawn = True
            t = [r[0] for r in series]
            axes[0].plot(t, [r[1] for r in series], label=server)
            axes[1].plot(t, [r[2] / 1024.0 for r in series], label=server)
        if not drawn:
            plt.close(fig)
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

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
