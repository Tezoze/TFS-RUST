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


def monster_name_matches(event_name: str, filter_name: str) -> bool:
    """Match 'rat' against C++ article names like 'a rat' and Rust 'Rat'."""
    en = event_name.lower().strip()
    fn = filter_name.lower().strip()
    if not fn:
        return True
    if en == fn:
        return True
    # C++ race names often include an article prefix.
    if en.endswith(f" {fn}") or en.endswith(fn):
        return True
    return fn in en


def filter_monster(events: Iterable[Dict[str, Any]], name: Optional[str]) -> List[Dict[str, Any]]:
    if not name:
        return list(events)
    return [e for e in events if monster_name_matches(str(e.get("name", "")), name)]


def filter_max_tick(events: Iterable[Dict[str, Any]], max_tick: Optional[int]) -> List[Dict[str, Any]]:
    if max_tick is None:
        return list(events)
    return [e for e in events if int(e.get("tick", 0)) <= max_tick]


def summarize(events: List[Dict[str, Any]]) -> Dict[str, int]:
    counts: Dict[str, int] = defaultdict(int)
    for e in events:
        counts[str(e.get("evt", "?"))] += 1
    return dict(counts)


def event_tick(evt: Dict[str, Any]) -> int:
    return int(evt.get("tick", 0))


def events_of_type(events: List[Dict[str, Any]], evt_type: str) -> List[Dict[str, Any]]:
    return [e for e in events if e.get("evt") == evt_type]


def compare_events_by_tick(
    ref: List[Dict[str, Any]],
    rust: List[Dict[str, Any]],
    evt_type: str,
    key_fn,
    label: str,
) -> List[str]:
    """Pairwise compare within each shared tick bucket — avoids index cascade after first miss."""
    ref_by_tick: Dict[int, List[Dict[str, Any]]] = defaultdict(list)
    rust_by_tick: Dict[int, List[Dict[str, Any]]] = defaultdict(list)
    for e in ref:
        if e.get("evt") == evt_type:
            ref_by_tick[event_tick(e)].append(e)
    for e in rust:
        if e.get("evt") == evt_type:
            rust_by_tick[event_tick(e)].append(e)

    diffs: List[str] = []
    all_ticks = sorted(set(ref_by_tick) | set(rust_by_tick))
    for tick in all_ticks:
        ref_list = ref_by_tick.get(tick, [])
        rust_list = rust_by_tick.get(tick, [])
        n = min(len(ref_list), len(rust_list))
        for i in range(n):
            a, b = ref_list[i], rust_list[i]
            ak, bk = key_fn(a), key_fn(b)
            if ak != bk:
                diffs.append(f"{label} tick={tick}[{i}] ref={ak} rust={bk}")
        if len(ref_list) != len(rust_list):
            diffs.append(
                f"{label} tick={tick} count ref={len(ref_list)} rust={len(rust_list)}"
            )
    ref_total = len(events_of_type(ref, evt_type))
    rust_total = len(events_of_type(rust, evt_type))
    if ref_total != rust_total:
        diffs.append(f"{label} total count: ref={ref_total} rust={rust_total}")
    return diffs


def compare_shortway(
    ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]
) -> List[str]:
    def key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return (pos_key(evt, "dest"), steps_key(evt), bool(evt.get("ok")))

    return compare_events_by_tick(ref, rust, "shortway", key, "shortway")


def compare_branch(ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]) -> List[str]:
    return compare_events_by_tick(ref, rust, "branch", branch_key, "branch")


def compare_todo_go(ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]) -> List[str]:
    return compare_events_by_tick(ref, rust, "todo_go", todo_go_key, "todo_go")


def compare_go_exec(ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]) -> List[str]:
    def key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return (
            pos_key(evt, "from"),
            pos_key(evt, "to"),
            int(evt.get("diag", 0)),
        )

    return compare_events_by_tick(ref, rust, "go_exec", key, "go_exec")


def branch_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return (
        str(evt.get("branch", "")),
        pos_key(evt, "dest"),
        int(evt.get("must", 0)),
        int(evt.get("max", 0)),
    )


def todo_go_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return (
        str(evt.get("via", "")),
        pos_key(evt, "dest"),
        int(evt.get("must", 0)),
        int(evt.get("max", 0)),
    )


def todo_go_same_contract(a: Dict[str, Any], b: Dict[str, Any]) -> bool:
    return (
        pos_key(a, "dest") == pos_key(b, "dest")
        and int(a.get("must", 0)) == int(b.get("must", 0))
        and int(a.get("max", 0)) == int(b.get("max", 0))
    )


def normalize_todo_go_events(events: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """Collapse C++ duplicate `enter`+`single` pair on the same tick/creature."""
    out: List[Dict[str, Any]] = []
    i = 0
    while i < len(events):
        evt = events[i]
        if (
            i + 1 < len(events)
            and evt.get("evt") == "todo_go"
            and events[i + 1].get("evt") == "todo_go"
            and event_tick(evt) == event_tick(events[i + 1])
            and evt.get("id") == events[i + 1].get("id")
            and evt.get("via") == "enter"
            and events[i + 1].get("via") == "single"
            and todo_go_same_contract(evt, events[i + 1])
        ):
            out.append(events[i + 1])
            i += 2
            continue
        out.append(evt)
        i += 1
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Compare reference vs Rust chase_path JSONL logs")
    parser.add_argument("--ref", type=Path, help="tibia-game-master chase_path.log")
    parser.add_argument("--cip", type=Path, help="deprecated alias for --ref")
    parser.add_argument("--rust", type=Path, required=True, help="TFS-RUST chase_path.log")
    parser.add_argument("--monster", help="Filter to one monster name (case-insensitive)")
    parser.add_argument(
        "--max-tick",
        type=int,
        help="Drop events with tick greater than this (scenario wall_ms budget)",
    )
    parser.add_argument("--json", action="store_true", help="Print JSON report")
    args = parser.parse_args()

    ref_path = args.ref or args.cip
    if ref_path is None:
        parser.error("one of --ref or --cip is required")

    ref_events = filter_max_tick(
        filter_monster(load_events(ref_path), args.monster),
        args.max_tick,
    )
    rust_events = filter_max_tick(
        filter_monster(load_events(args.rust), args.monster),
        args.max_tick,
    )
    ref_events = normalize_todo_go_events(ref_events)
    rust_events = normalize_todo_go_events(rust_events)

    report = {
        "ref_file": str(ref_path),
        "rust_file": str(args.rust),
        "monster": args.monster,
        "max_tick": args.max_tick,
        "ref_summary": summarize(ref_events),
        "rust_summary": summarize(rust_events),
        "branch_diffs": compare_branch(ref_events, rust_events),
        "todo_go_diffs": compare_todo_go(ref_events, rust_events),
        "shortway_diffs": compare_shortway(ref_events, rust_events),
        "go_exec_diffs": compare_go_exec(ref_events, rust_events),
    }

    if args.json:
        print(json.dumps(report, indent=2))
        return 0 if not any(
            report[k]
            for k in (
                "branch_diffs",
                "todo_go_diffs",
                "shortway_diffs",
                "go_exec_diffs",
            )
        ) else 1

    print(f"Reference events: {len(ref_events)}  {report['ref_summary']}")
    print(f"Rust events:      {len(rust_events)}  {report['rust_summary']}")
    if not ref_events:
        print(f"warn: no reference events in {ref_path}", file=sys.stderr)
    if not rust_events:
        print(f"warn: no Rust events in {args.rust}", file=sys.stderr)

    all_diffs = (
        report["branch_diffs"]
        + report["todo_go_diffs"]
        + report["shortway_diffs"]
        + report["go_exec_diffs"]
    )
    if not all_diffs:
        print("ok: branch, todo_go, shortway, and go_exec sequences match (pairwise order)")
        return 0

    print(f"\n{len(all_diffs)} mismatch(es):")
    for line in all_diffs:
        print(f"  - {line}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
