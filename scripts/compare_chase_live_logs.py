#!/usr/bin/env python3
"""
Diff live monster chase JSONL logs from tibia-game-master vs TFS-RUST.

Both servers emit one JSON object per line when chase-path debug is enabled:
  Reference: log/chase_path.log   (ChasePathDebug=1 or TIBIA_CHASE_PATH_DEBUG=1)
  TFS-RUST:  log/chase_path.log   (TFS_CHASE_PATH_DEBUG=1)

Usage:
  python scripts/compare_chase_live_logs.py \\
    --ref /mnt/storage2/TFS_RUST/log/chase_path.log \\
    --rust ./log/chase_path.log

  python scripts/compare_chase_live_logs.py --ref log/chase_path.log --rust log/chase_path.log --monster Rat
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple


def load_events(path: Path) -> List[Dict[str, Any]]:
    events: List[Dict[str, Any]] = []
    if not path.is_file():
        return events
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = line.strip()
        if not line or line[0] != "{":
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return events


def pos_key(obj: Dict[str, Any], field: str) -> Tuple[int, int, int]:
    p = obj.get(field) or {}
    return (int(p.get("x", 0)), int(p.get("y", 0)), int(p.get("z", 0)))


def steps_key(evt: Dict[str, Any]) -> Tuple[Tuple[int, int, int], ...]:
    out: List[Tuple[int, int, int]] = []
    for step in evt.get("steps") or []:
        out.append((int(step["x"]), int(step["y"]), int(step["z"])))
    return tuple(out)


def filter_monster(events: Iterable[Dict[str, Any]], name: Optional[str]) -> List[Dict[str, Any]]:
    if not name:
        return list(events)
    name_l = name.lower()
    return [e for e in events if str(e.get("name", "")).lower() == name_l]


def summarize(events: List[Dict[str, Any]]) -> Dict[str, int]:
    counts: Dict[str, int] = defaultdict(int)
    for e in events:
        counts[str(e.get("evt", "?"))] += 1
    return dict(counts)


def compare_shortway(
    ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]
) -> List[str]:
    diffs: List[str] = []
    ref_sw = [e for e in ref if e.get("evt") == "shortway"]
    rust_sw = [e for e in rust if e.get("evt") == "shortway"]
    n = min(len(ref_sw), len(rust_sw))
    for i in range(n):
        a, b = ref_sw[i], rust_sw[i]
        if pos_key(a, "dest") != pos_key(b, "dest"):
            diffs.append(f"shortway[{i}] dest mismatch ref={pos_key(a,'dest')} rust={pos_key(b,'dest')}")
        if steps_key(a) != steps_key(b):
            diffs.append(
                f"shortway[{i}] steps mismatch\n"
                f"  ref:  {steps_key(a)}\n"
                f"  rust: {steps_key(b)}"
            )
        if bool(a.get("ok")) != bool(b.get("ok")):
            diffs.append(f"shortway[{i}] ok mismatch ref={a.get('ok')} rust={b.get('ok')}")
    if len(ref_sw) != len(rust_sw):
        diffs.append(f"shortway count: ref={len(ref_sw)} rust={len(rust_sw)}")
    return diffs


def compare_go_exec(ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]) -> List[str]:
    diffs: List[str] = []
    ref_go = [e for e in ref if e.get("evt") == "go_exec"]
    rust_go = [e for e in rust if e.get("evt") == "go_exec"]
    n = min(len(ref_go), len(rust_go))
    for i in range(n):
        a, b = ref_go[i], rust_go[i]
        if pos_key(a, "from") != pos_key(b, "from") or pos_key(a, "to") != pos_key(b, "to"):
            diffs.append(
                f"go_exec[{i}] ref {pos_key(a,'from')}->{pos_key(a,'to')} "
                f"rust {pos_key(b,'from')}->{pos_key(b,'to')}"
            )
        if int(a.get("diag", 0)) != int(b.get("diag", 0)):
            diffs.append(f"go_exec[{i}] diag ref={a.get('diag')} rust={b.get('diag')}")
    if len(ref_go) != len(rust_go):
        diffs.append(f"go_exec count: ref={len(ref_go)} rust={len(rust_go)}")
    return diffs


def main() -> int:
    parser = argparse.ArgumentParser(description="Compare reference vs Rust chase_path JSONL logs")
    parser.add_argument("--ref", type=Path, help="tibia-game-master chase_path.log")
    parser.add_argument("--cip", type=Path, help="deprecated alias for --ref")
    parser.add_argument("--rust", type=Path, required=True, help="TFS-RUST chase_path.log")
    parser.add_argument("--monster", help="Filter to one monster name (case-insensitive)")
    parser.add_argument("--json", action="store_true", help="Print JSON report")
    args = parser.parse_args()

    ref_path = args.ref or args.cip
    if ref_path is None:
        parser.error("one of --ref or --cip is required")

    ref_events = filter_monster(load_events(ref_path), args.monster)
    rust_events = filter_monster(load_events(args.rust), args.monster)

    report = {
        "ref_file": str(ref_path),
        "rust_file": str(args.rust),
        "monster": args.monster,
        "ref_summary": summarize(ref_events),
        "rust_summary": summarize(rust_events),
        "shortway_diffs": compare_shortway(ref_events, rust_events),
        "go_exec_diffs": compare_go_exec(ref_events, rust_events),
    }

    if args.json:
        print(json.dumps(report, indent=2))
        return 0 if not report["shortway_diffs"] and not report["go_exec_diffs"] else 1

    print(f"Reference events: {len(ref_events)}  {report['ref_summary']}")
    print(f"Rust events:      {len(rust_events)}  {report['rust_summary']}")
    if not ref_events:
        print(f"warn: no reference events in {ref_path}", file=sys.stderr)
    if not rust_events:
        print(f"warn: no Rust events in {args.rust}", file=sys.stderr)

    all_diffs = report["shortway_diffs"] + report["go_exec_diffs"]
    if not all_diffs:
        print("ok: shortway steps and go_exec sequences match (pairwise order)")
        return 0

    print(f"\n{len(all_diffs)} mismatch(es):")
    for line in all_diffs:
        print(f"  - {line}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
