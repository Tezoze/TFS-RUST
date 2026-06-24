#!/usr/bin/env python3
"""
Summarize monster AI trace divergence between C++ reference and TFS-RUST.

Builds on the JSONL event stream from chase-path debug (branch → todo_go →
shortway → go_exec). Reports count deltas, branch mix, pairwise match rates,
and grouped mismatch examples.

Usage:
  python3 scripts/summarize_chase_gaps.py \\
    --ref log/chase_path_cip.log \\
    --rust log/chase_path_rust.log \\
    --monster rat
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

# Reuse compare helpers when run as a script from repo root.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from compare_chase_live_logs import (  # noqa: E402
    branch_key,
    compare_branch,
    compare_go_exec,
    compare_shortway,
    compare_todo_go,
    filter_max_tick,
    filter_monster,
    load_events,
    monster_name_matches,
    normalize_todo_go_events,
    pos_key,
    steps_key,
    summarize,
    todo_go_key,
)

EventList = List[Dict[str, Any]]


def build_id_to_name(events: EventList) -> Dict[int, str]:
    """Map creature id → display name from JSONL headers and payload references."""
    mapping: Dict[int, str] = {}
    for evt in events:
        raw_id = evt.get("id")
        name = evt.get("name")
        if raw_id is not None and name:
            mapping[int(raw_id)] = str(name)
    # Player/opponent ids often appear only in payload fields (no header line).
    for evt in events:
        self_id = int(evt.get("id", 0))
        for field, role in (
            ("attacker_id", "attacker"),
            ("killer_id", "attacker"),
            ("target_id", "target"),
        ):
            raw = evt.get(field)
            if raw is None:
                continue
            rid = int(raw)
            if rid != 0 and rid != self_id and rid not in mapping:
                mapping[rid] = role
    return mapping


def normalize_creature_role(
    raw_id: int,
    id_to_name: Dict[int, str],
    self_id: int,
) -> str:
    """Semantic role for cross-stack ID compare (slotmap vs C++ sequential)."""
    if raw_id == 0:
        return "none"
    if raw_id == self_id:
        return "self"
    name = id_to_name.get(raw_id, "")
    if not name:
        return f"unknown:{raw_id}"
    role = name.strip().lower()
    if role.startswith("a "):
        role = role[2:]
    # Payload-inferred roles (attacker/target) pass through as-is.
    if role in ("attacker", "target", "killer"):
        return "attacker" if role == "killer" else role
    return role


def damage_stimulus_key(
    evt: Dict[str, Any], id_to_name: Dict[int, str]
) -> Tuple[Any, ...]:
    self_id = int(evt.get("id", 0))
    return (
        str(evt.get("old_state", "")),
        str(evt.get("new_state", "")),
        normalize_creature_role(int(evt.get("attacker_id", 0)), id_to_name, self_id),
        int(evt.get("damage", 0)),
        int(evt.get("had_target", 0)),
    )


def creature_death_key(
    evt: Dict[str, Any],
    id_to_name: Dict[int, str],
    *,
    strict_corpse: bool,
) -> Tuple[Any, ...]:
    self_id = int(evt.get("id", 0))
    key: Tuple[Any, ...] = (
        normalize_creature_role(int(evt.get("killer_id", 0)), id_to_name, self_id),
        int(evt.get("experience", 0)),
    )
    if strict_corpse:
        key = key + (int(evt.get("corpse_id", 0)),)
    return key


def normalize_spell_label(raw: str) -> str:
    if raw.startswith("damage:"):
        return "damage"
    return raw


def spell_cast_key(evt: Dict[str, Any], id_to_name: Dict[int, str]) -> Tuple[Any, ...]:
    self_id = int(evt.get("id", 0))
    return (
        normalize_spell_label(str(evt.get("spell", ""))),
        normalize_creature_role(int(evt.get("target_id", 0)), id_to_name, self_id),
        str(evt.get("shape", "")),
        int(evt.get("range", 0)),
    )


def by_evt(events: EventList, evt: str) -> EventList:
    return [e for e in events if e.get("evt") == evt]


def branch_counts(events: EventList) -> Counter[str]:
    return Counter(str(e.get("branch", "?")) for e in by_evt(events, "branch"))


def via_counts(events: EventList) -> Counter[str]:
    return Counter(str(e.get("via", "?")) for e in by_evt(events, "todo_go"))


def diag_go_stats(events: EventList) -> Tuple[int, int]:
    go = by_evt(events, "go_exec")
    diag = sum(1 for e in go if int(e.get("diag", 0)) != 0)
    return len(go), diag


def pairwise_match_rate(
    ref: EventList, rust: EventList, key_fn
) -> Tuple[int, int, float]:
    n = min(len(ref), len(rust))
    if n == 0:
        return 0, 0, 0.0
    matches = sum(1 for i in range(n) if key_fn(ref[i]) == key_fn(rust[i]))
    return matches, n, 100.0 * matches / n


def pairwise_match_rate_cross(
    ref: EventList,
    rust: EventList,
    ref_key_fn,
    rust_key_fn,
) -> Tuple[int, int, float]:
    n = min(len(ref), len(rust))
    if n == 0:
        return 0, 0, 0.0
    matches = sum(1 for i in range(n) if ref_key_fn(ref[i]) == rust_key_fn(rust[i]))
    return matches, n, 100.0 * matches / n


def go_exec_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return (pos_key(evt, "from"), pos_key(evt, "to"), int(evt.get("diag", 0)))


def combat_state_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return (
        str(evt.get("monster_state", "")),
        str(evt.get("chase_mode", "")),
    )


def attack_enqueue_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return (
        int(evt.get("wait_ms", 0)),
        int(evt.get("needs_close_step", 0)),
        str(evt.get("close_chase", "")),
    )


def melee_hit_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return (
        int(evt.get("attack", 0)),
        int(evt.get("defense", 0)),
        int(evt.get("damage", 0)),
        int(evt.get("hp_before", 0)),
        int(evt.get("hp_after", 0)),
    )


def ranged_hit_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return melee_hit_key(evt)


def shortway_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
    return (
        pos_key(evt, "dest"),
        steps_key(evt),
        bool(evt.get("ok")),
        int(evt.get("visible", 0)),
        int(evt.get("min_wp", 0)),
    )


def first_divergence(
    ref: EventList, rust: EventList, evt: str, key_fn, label: str
) -> Optional[str]:
    ref_e = by_evt(ref, evt)
    rust_e = by_evt(rust, evt)
    n = min(len(ref_e), len(rust_e))
    for i in range(n):
        if key_fn(ref_e[i]) != key_fn(rust_e[i]):
            return f"{label}[{i}]: ref={key_fn(ref_e[i])}  rust={key_fn(rust_e[i])}"
    if len(ref_e) != len(rust_e):
        return f"{label} count: ref={len(ref_e)} rust={len(rust_e)}"
    return None


def sample_mismatches(diffs: List[str], limit: int = 5) -> List[str]:
    return diffs[:limit]


def build_report(
    ref_events: EventList,
    rust_events: EventList,
    monster: Optional[str],
    *,
    ref_id_to_name: Dict[int, str],
    rust_id_to_name: Dict[int, str],
    strict_corpse: bool = False,
) -> Dict[str, Any]:
    def ref_damage_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return damage_stimulus_key(evt, ref_id_to_name)

    def rust_damage_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return damage_stimulus_key(evt, rust_id_to_name)

    def ref_death_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return creature_death_key(evt, ref_id_to_name, strict_corpse=strict_corpse)

    def rust_death_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return creature_death_key(evt, rust_id_to_name, strict_corpse=strict_corpse)

    def ref_spell_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return spell_cast_key(evt, ref_id_to_name)

    def rust_spell_key(evt: Dict[str, Any]) -> Tuple[Any, ...]:
        return spell_cast_key(evt, rust_id_to_name)

    ref_sum = summarize(ref_events)
    rust_sum = summarize(rust_events)

    evt_types = [
        "branch",
        "todo_go",
        "shortway",
        "go_exec",
        "combat_state",
        "attack_enqueue",
        "melee_hit",
        "ranged_hit",
        "spell_cast",
        "damage_stimulus",
        "creature_death",
    ]
    count_delta = {
        evt: {
            "ref": ref_sum.get(evt, 0),
            "rust": rust_sum.get(evt, 0),
            "delta": rust_sum.get(evt, 0) - ref_sum.get(evt, 0),
        }
        for evt in evt_types
    }

    match_rates = {}
    single_key_fns = {
        "branch": branch_key,
        "todo_go": todo_go_key,
        "shortway": shortway_key,
        "go_exec": go_exec_key,
        "combat_state": combat_state_key,
        "attack_enqueue": attack_enqueue_key,
        "melee_hit": melee_hit_key,
        "ranged_hit": ranged_hit_key,
    }
    for evt, key_fn in single_key_fns.items():
        m, n, pct = pairwise_match_rate(
            by_evt(ref_events, evt), by_evt(rust_events, evt), key_fn
        )
        match_rates[evt] = {"matched": m, "paired": n, "pct": round(pct, 1)}

    cross_key_fns = {
        "spell_cast": (ref_spell_key, rust_spell_key),
        "damage_stimulus": (ref_damage_key, rust_damage_key),
        "creature_death": (ref_death_key, rust_death_key),
    }
    for evt, (ref_fn, rust_fn) in cross_key_fns.items():
        m, n, pct = pairwise_match_rate_cross(
            by_evt(ref_events, evt), by_evt(rust_events, evt), ref_fn, rust_fn
        )
        match_rates[evt] = {"matched": m, "paired": n, "pct": round(pct, 1)}

    ref_go, ref_diag = diag_go_stats(ref_events)
    rust_go, rust_diag = diag_go_stats(rust_events)

    branch_diffs = compare_branch(ref_events, rust_events)
    todo_diffs = compare_todo_go(ref_events, rust_events)
    sw_diffs = compare_shortway(ref_events, rust_events)
    go_diffs = compare_go_exec(ref_events, rust_events)
    combat_diffs = compare_combat_state(ref_events, rust_events)
    melee_diffs = compare_melee_hit(ref_events, rust_events)
    spell_diffs = compare_spell_cast(
        ref_events, rust_events, ref_id_to_name, rust_id_to_name
    )
    stimulus_diffs = compare_damage_stimulus(
        ref_events, rust_events, ref_id_to_name, rust_id_to_name
    )
    death_diffs = compare_creature_death(
        ref_events,
        rust_events,
        ref_id_to_name,
        rust_id_to_name,
        strict_corpse=strict_corpse,
    )
    ranged_diffs = compare_ranged_hit(ref_events, rust_events)

    first = {
        "branch": first_divergence(
            ref_events, rust_events, "branch", branch_key, "branch"
        ),
        "todo_go": first_divergence(
            ref_events, rust_events, "todo_go", todo_go_key, "todo_go"
        ),
        "shortway": first_divergence(
            ref_events, rust_events, "shortway", shortway_key, "shortway"
        ),
        "go_exec": first_divergence(
            ref_events, rust_events, "go_exec", go_exec_key, "go_exec"
        ),
    }

    return {
        "monster": monster,
        "ref_total": len(ref_events),
        "rust_total": len(rust_events),
        "count_delta": count_delta,
        "ref_branches": dict(branch_counts(ref_events)),
        "rust_branches": dict(branch_counts(rust_events)),
        "ref_todo_via": dict(via_counts(ref_events)),
        "rust_todo_via": dict(via_counts(rust_events)),
        "diagonal_go_exec": {
            "ref": {"total": ref_go, "diag": ref_diag},
            "rust": {"total": rust_go, "diag": rust_diag},
        },
        "pairwise_match_pct": match_rates,
        "first_divergence": first,
        "mismatch_counts": {
            "branch": len(branch_diffs),
            "todo_go": len(todo_diffs),
            "shortway": len(sw_diffs),
            "go_exec": len(go_diffs),
            "combat_state": len(combat_diffs),
            "melee_hit": len(melee_diffs),
            "ranged_hit": len(ranged_diffs),
            "spell_cast": len(spell_diffs),
            "damage_stimulus": len(stimulus_diffs),
            "creature_death": len(death_diffs),
        },
        "samples": {
            "branch": sample_mismatches(branch_diffs),
            "todo_go": sample_mismatches(todo_diffs),
            "shortway": sample_mismatches(sw_diffs),
            "go_exec": sample_mismatches(go_diffs),
            "combat_state": sample_mismatches(combat_diffs),
            "melee_hit": sample_mismatches(melee_diffs),
            "ranged_hit": sample_mismatches(ranged_diffs),
            "spell_cast": sample_mismatches(spell_diffs),
            "damage_stimulus": sample_mismatches(stimulus_diffs),
            "creature_death": sample_mismatches(death_diffs),
        },
    }


def print_report(report: Dict[str, Any]) -> None:
    monster = report.get("monster") or "(all)"
    print(f"=== Chase AI gap summary — {monster} ===\n")

    print("Event totals")
    print(f"  {'evt':<10} {'ref':>6} {'rust':>6} {'delta':>7}")
    print(f"  {'-' * 10} {'-' * 6} {'-' * 6} {'-' * 7}")
    for evt, row in report["count_delta"].items():
        sign = "+" if row["delta"] >= 0 else ""
        print(f"  {evt:<10} {row['ref']:>6} {row['rust']:>6} {sign}{row['delta']:>6}")

    print(f"\n  all events: ref={report['ref_total']}  rust={report['rust_total']}")

    print("\nBranch mix (IdleStimulus arms)")
    ref_b = report["ref_branches"]
    rust_b = report["rust_branches"]
    all_branches = sorted(set(ref_b) | set(rust_b))
    if all_branches:
        print(f"  {'branch':<16} {'ref':>6} {'rust':>6}")
        for b in all_branches:
            print(f"  {b:<16} {ref_b.get(b, 0):>6} {rust_b.get(b, 0):>6}")
    else:
        print("  (no branch events)")

    print("\ntodo_go via")
    ref_v = report["ref_todo_via"]
    rust_v = report["rust_todo_via"]
    all_via = sorted(set(ref_v) | set(rust_v))
    if all_via:
        print(f"  {'via':<10} {'ref':>6} {'rust':>6}")
        for v in all_via:
            print(f"  {v:<10} {ref_v.get(v, 0):>6} {rust_v.get(v, 0):>6}")

    diag = report["diagonal_go_exec"]
    print("\nDiagonal go_exec")
    print(
        f"  ref:  {diag['ref']['diag']}/{diag['ref']['total']} "
        f"({100 * diag['ref']['diag'] / max(diag['ref']['total'], 1):.1f}%)"
    )
    print(
        f"  rust: {diag['rust']['diag']}/{diag['rust']['total']} "
        f"({100 * diag['rust']['diag'] / max(diag['rust']['total'], 1):.1f}%)"
    )

    print("\nCombat trace (E0–E6)")
    for evt in (
        "combat_state",
        "attack_enqueue",
        "melee_hit",
        "ranged_hit",
        "spell_cast",
        "damage_stimulus",
        "creature_death",
    ):
        row = report["count_delta"].get(evt, {"ref": 0, "rust": 0})
        print(f"  {evt:<16} ref={row['ref']:>4}  rust={row['rust']:>4}")

    print("\nPairwise sequence match (ordered, index-aligned)")
    for evt, row in report["pairwise_match_pct"].items():
        print(f"  {evt:<14} {row['matched']}/{row['paired']} = {row['pct']}%")

    print("\nFirst divergence")
    for evt, line in report["first_divergence"].items():
        if line:
            print(f"  {evt}: {line}")
        else:
            print(f"  {evt}: (none within paired prefix)")

    print("\nMismatch volume")
    total_m = sum(report["mismatch_counts"].values())
    for evt, n in report["mismatch_counts"].items():
        print(f"  {evt}: {n}")
    print(f"  total reported: {total_m}")

    print("\nSample mismatches (first 5 per type)")
    for evt, samples in report["samples"].items():
        if not samples:
            continue
        print(f"  [{evt}]")
        for line in samples:
            print(f"    - {line}")


def compare_melee_hit(
    ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]
) -> List[str]:
    diffs: List[str] = []
    ref_m = [e for e in ref if e.get("evt") == "melee_hit"]
    rust_m = [e for e in rust if e.get("evt") == "melee_hit"]
    if len(ref_m) != len(rust_m):
        diffs.append(f"melee_hit count: ref={len(ref_m)} rust={len(rust_m)}")
    n = min(len(ref_m), len(rust_m))
    for i in range(n):
        a, b = ref_m[i], rust_m[i]
        if int(a.get("damage", 0)) != int(b.get("damage", 0)):
            diffs.append(
                f"melee_hit[{i}] damage ref={a.get('damage')} rust={b.get('damage')} "
                f"(atk ref={a.get('attack')} rust={b.get('attack')})"
            )
    return diffs


def compare_ranged_hit(
    ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]
) -> List[str]:
    diffs: List[str] = []
    ref_m = [e for e in ref if e.get("evt") == "ranged_hit"]
    rust_m = [e for e in rust if e.get("evt") == "ranged_hit"]
    if len(ref_m) != len(rust_m):
        diffs.append(f"ranged_hit count: ref={len(ref_m)} rust={len(rust_m)}")
    n = min(len(ref_m), len(rust_m))
    for i in range(n):
        if ranged_hit_key(ref_m[i]) != ranged_hit_key(rust_m[i]):
            diffs.append(
                f"ranged_hit[{i}] ref={ranged_hit_key(ref_m[i])} rust={ranged_hit_key(rust_m[i])}"
            )
    return diffs


def compare_spell_cast(
    ref: List[Dict[str, Any]],
    rust: List[Dict[str, Any]],
    ref_id_to_name: Dict[int, str],
    rust_id_to_name: Dict[int, str],
) -> List[str]:
    diffs: List[str] = []
    ref_s = [e for e in ref if e.get("evt") == "spell_cast"]
    rust_s = [e for e in rust if e.get("evt") == "spell_cast"]
    if len(ref_s) != len(rust_s):
        diffs.append(f"spell_cast count: ref={len(ref_s)} rust={len(rust_s)}")
    n = min(len(ref_s), len(rust_s))
    for i in range(n):
        ref_k = spell_cast_key(ref_s[i], ref_id_to_name)
        rust_k = spell_cast_key(rust_s[i], rust_id_to_name)
        if ref_k != rust_k:
            diffs.append(f"spell_cast[{i}] ref={ref_k} rust={rust_k}")
    return diffs


def compare_damage_stimulus(
    ref: List[Dict[str, Any]],
    rust: List[Dict[str, Any]],
    ref_id_to_name: Dict[int, str],
    rust_id_to_name: Dict[int, str],
) -> List[str]:
    diffs: List[str] = []
    ref_s = [e for e in ref if e.get("evt") == "damage_stimulus"]
    rust_s = [e for e in rust if e.get("evt") == "damage_stimulus"]
    if len(ref_s) != len(rust_s):
        diffs.append(f"damage_stimulus count: ref={len(ref_s)} rust={len(rust_s)}")
    n = min(len(ref_s), len(rust_s))
    for i in range(n):
        ref_k = damage_stimulus_key(ref_s[i], ref_id_to_name)
        rust_k = damage_stimulus_key(rust_s[i], rust_id_to_name)
        if ref_k != rust_k:
            diffs.append(f"damage_stimulus[{i}] ref={ref_k} rust={rust_k}")
    return diffs


def compare_creature_death(
    ref: List[Dict[str, Any]],
    rust: List[Dict[str, Any]],
    ref_id_to_name: Dict[int, str],
    rust_id_to_name: Dict[int, str],
    *,
    strict_corpse: bool,
) -> List[str]:
    diffs: List[str] = []
    ref_s = [e for e in ref if e.get("evt") == "creature_death"]
    rust_s = [e for e in rust if e.get("evt") == "creature_death"]
    if len(ref_s) != len(rust_s):
        diffs.append(f"creature_death count: ref={len(ref_s)} rust={len(rust_s)}")
    n = min(len(ref_s), len(rust_s))
    for i in range(n):
        ref_k = creature_death_key(
            ref_s[i], ref_id_to_name, strict_corpse=strict_corpse
        )
        rust_k = creature_death_key(
            rust_s[i], rust_id_to_name, strict_corpse=strict_corpse
        )
        if ref_k != rust_k:
            diffs.append(f"creature_death[{i}] ref={ref_k} rust={rust_k}")
    return diffs


def compare_combat_state(
    ref: List[Dict[str, Any]], rust: List[Dict[str, Any]]
) -> List[str]:
    diffs: List[str] = []
    ref_s = [e for e in ref if e.get("evt") == "combat_state"]
    rust_s = [e for e in rust if e.get("evt") == "combat_state"]
    if len(ref_s) != len(rust_s):
        diffs.append(f"combat_state count: ref={len(ref_s)} rust={len(rust_s)}")
    n = min(len(ref_s), len(rust_s))
    for i in range(n):
        a, b = ref_s[i], rust_s[i]
        key = (
            str(a.get("monster_state", "")),
            str(a.get("chase_mode", "")),
        )
        key_b = (
            str(b.get("monster_state", "")),
            str(b.get("chase_mode", "")),
        )
        if key != key_b:
            diffs.append(f"combat_state[{i}] ref={key} rust={key_b}")
    return diffs


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Summarize C++ vs Rust monster AI trace divergence"
    )
    parser.add_argument("--ref", type=Path, required=True, help="C++ chase JSONL")
    parser.add_argument("--rust", type=Path, required=True, help="Rust chase JSONL")
    parser.add_argument("--monster", help="Filter monster name (e.g. rat)")
    parser.add_argument(
        "--max-tick",
        type=int,
        help="Drop events with tick greater than this (scenario wall_ms budget)",
    )
    parser.add_argument(
        "--lockstep",
        action="store_true",
        help="Exit 1 when any movement/combat sequence mismatch exists within --max-tick",
    )
    parser.add_argument(
        "--strict-corpse",
        action="store_true",
        help="Include corpse_id in creature_death lockstep compare (default: ignore)",
    )
    parser.add_argument("--json", action="store_true", help="Print JSON report")
    args = parser.parse_args()

    ref_all = load_events(args.ref)
    rust_all = load_events(args.rust)
    ref_id_to_name = build_id_to_name(ref_all)
    rust_id_to_name = build_id_to_name(rust_all)

    ref_events = normalize_todo_go_events(
        filter_max_tick(filter_monster(ref_all, args.monster), args.max_tick)
    )
    rust_events = normalize_todo_go_events(
        filter_max_tick(filter_monster(rust_all, args.monster), args.max_tick)
    )

    if not ref_events:
        print(f"warn: no reference events in {args.ref}", file=sys.stderr)
    if not rust_events:
        print(f"warn: no Rust events in {args.rust}", file=sys.stderr)

    report = build_report(
        ref_events,
        rust_events,
        args.monster,
        ref_id_to_name=ref_id_to_name,
        rust_id_to_name=rust_id_to_name,
        strict_corpse=args.strict_corpse,
    )

    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print_report(report)

    # Non-zero if any pairwise mismatch exists in the paired prefix or counts differ.
    has_gap = any(report["mismatch_counts"][k] > 0 for k in report["mismatch_counts"])
    if args.lockstep and has_gap:
        return 2
    return 1 if has_gap else 0


if __name__ == "__main__":
    raise SystemExit(main())
