#!/usr/bin/env python3
"""Content-equivalence gate: compare two tfs-loadgen JSON reports.

Run every isolation role at 5 bots against both servers and assert comparable
observable outcomes (damage numbers, unique creatures, packets/action). Exact
combat equality is not required — server-side RNG differs — but a large relative
delta means the 300-bot chart is not measuring the same workload.

Usage:
  python3 scripts/bench/check_equivalence.py --rust rust.json --tvp tvp.json
  python3 scripts/bench/check_equivalence.py --dir results/<ts>/equivalence
  python3 scripts/bench/check_equivalence.py --self-test
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# Relative deltas that still count as "comparable" at 5 bots.
THRESH_DAMAGE = 0.20
THRESH_CREATURES = 0.15
THRESH_PACKETS = 0.25
THRESH_MAGIC = 0.25
THRESH_DISCARD = 0.25
THRESH_WALK_REJECT = 0.25
THRESH_DROP = 0.25
ROLES = ("walker", "melee", "caster", "rune", "aoe_rune", "noise")


def rel_delta(a: float, b: float) -> float:
    if a == 0.0 and b == 0.0:
        return 0.0
    return abs(a - b) / max(abs(a), abs(b), 1.0)


def packets_per_action(report: dict) -> float:
    sends = float(report.get("sends") or 0)
    if sends <= 0:
        return 0.0
    return float(report.get("bytes_in") or 0) / sends


def discarded_ratio(report: dict) -> float:
    bytes_in = float(report.get("bytes_in") or 0)
    if bytes_in <= 0:
        return 0.0
    return float(report.get("bytes_discarded") or 0) / bytes_in


def walk_rejection_rate(report: dict) -> float:
    walk = report.get("walk") or {}
    samples = float(walk.get("samples") or 0)
    if samples <= 0:
        return 0.0
    return float(walk.get("rejections") or 0) / samples


def drop_rate(report: dict) -> float:
    bots = float(report.get("bots") or 0)
    if bots <= 0:
        return 0.0
    return float(report.get("disconnects") or 0) / bots


def compare_pair(rust: dict, tvp: dict, *, label: str) -> list[str]:
    fails: list[str] = []

    def check(name: str, a: float, b: float, thresh: float, *, skip_zero: bool = False) -> None:
        if skip_zero and a == 0 and b == 0:
            return
        d = rel_delta(a, b)
        if d > thresh:
            fails.append(
                f"{label}: {name} rust={a} tvp={b} rel_delta={d:.3f} > {thresh:.2f}"
            )

    rust_creatures = float(rust.get("unique_creatures") or 0)
    tvp_creatures = float(tvp.get("unique_creatures") or 0)
    # Field / step 0x84 is not combat when neither side saw a creature.
    if rust_creatures != 0 or tvp_creatures != 0:
        check(
            "damage_sum",
            float(rust.get("damage_sum") or 0),
            float(tvp.get("damage_sum") or 0),
            THRESH_DAMAGE,
            skip_zero=True,
        )
    check(
        "unique_creatures",
        rust_creatures,
        tvp_creatures,
        THRESH_CREATURES,
        skip_zero=True,
    )
    check(
        "bytes_in_per_send",
        packets_per_action(rust),
        packets_per_action(tvp),
        THRESH_PACKETS,
    )
    check(
        "magic_effects",
        float(rust.get("magic_effects") or 0),
        float(tvp.get("magic_effects") or 0),
        THRESH_MAGIC,
        skip_zero=True,
    )
    check(
        "sends",
        float(rust.get("sends") or 0),
        float(tvp.get("sends") or 0),
        THRESH_PACKETS,
    )
    check(
        "bytes_discarded_per_in",
        discarded_ratio(rust),
        discarded_ratio(tvp),
        THRESH_DISCARD,
        skip_zero=True,
    )
    check(
        "walk_rejections_per_sample",
        walk_rejection_rate(rust),
        walk_rejection_rate(tvp),
        THRESH_WALK_REJECT,
        skip_zero=True,
    )
    check(
        "disconnects_per_bot",
        drop_rate(rust),
        drop_rate(tvp),
        THRESH_DROP,
        skip_zero=True,
    )
    check(
        "reconnects",
        float(rust.get("reconnects") or 0),
        float(tvp.get("reconnects") or 0),
        THRESH_DROP,
        skip_zero=True,
    )
    return fails


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def compare_dir(root: Path) -> list[str]:
    fails: list[str] = []
    found = 0
    for role in ROLES:
        rust_p = root / "rust" / f"{role}.json"
        tvp_p = root / "tvp" / f"{role}.json"
        if not rust_p.is_file() or not tvp_p.is_file():
            continue
        found += 1
        fails.extend(compare_pair(load_json(rust_p), load_json(tvp_p), label=role))
    if found == 0:
        fails.append(f"no role JSON pairs under {root} (expected rust/<role>.json + tvp/<role>.json)")
    return fails


def self_test() -> int:
    ok = {
        "sends": 100,
        "bytes_in": 10000,
        "damage_sum": 1000,
        "unique_creatures": 20,
        "magic_effects": 50,
        "bytes_discarded": 0,
        "walk": {"samples": 100, "rejections": 10},
        "bots": 5,
        "disconnects": 0,
        "reconnects": 0,
    }
    close = dict(ok)
    close["damage_sum"] = 1100
    close["bytes_in"] = 10500
    close["walk"] = {"samples": 100, "rejections": 12}
    assert not compare_pair(ok, close, label="ok")
    bad = dict(ok)
    bad["damage_sum"] = 5000
    assert compare_pair(ok, bad, label="bad")
    discard_bad = dict(ok)
    discard_bad["bytes_discarded"] = 5000
    assert compare_pair(ok, discard_bad, label="discard")
    reject_bad = dict(ok)
    reject_bad["walk"] = {"samples": 100, "rejections": 40}
    assert compare_pair(ok, reject_bad, label="reject")
    drop_bad = dict(ok)
    drop_bad["disconnects"] = 3
    assert compare_pair(ok, drop_bad, label="drop")
    reconnect_bad = dict(ok)
    reconnect_bad["reconnects"] = 5
    assert compare_pair(ok, reconnect_bad, label="reconnect")
    both_zero_drop = dict(ok)
    other_zero_drop = dict(ok)
    other_zero_drop["bots"] = 8
    assert not compare_pair(both_zero_drop, other_zero_drop, label="drop-zero")
    both_zero = dict(ok)
    both_zero["walk"] = {"samples": 80, "rejections": 0}
    other_zero = dict(ok)
    other_zero["walk"] = {"samples": 100, "rejections": 0}
    assert not compare_pair(both_zero, other_zero, label="reject-zero")
    field_noise = dict(ok)
    field_noise["unique_creatures"] = 0
    field_noise["damage_sum"] = 11
    field_tvp = dict(field_noise)
    field_tvp["damage_sum"] = 0
    assert not compare_pair(field_noise, field_tvp, label="field-noise")
    me_bad = dict(field_noise)
    me_bad["magic_effects"] = 34
    me_tvp = dict(field_noise)
    me_tvp["magic_effects"] = 0
    me_tvp["damage_sum"] = 0
    assert compare_pair(me_bad, me_tvp, label="me-zero")
    print("check_equivalence: self-test ok")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rust", type=Path, help="single rust loadgen JSON")
    parser.add_argument("--tvp", type=Path, help="single tvp loadgen JSON")
    parser.add_argument("--dir", type=Path, help="equivalence dir with rust/ + tvp/ role JSON")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    fails: list[str] = []
    if args.rust and args.tvp:
        fails.extend(compare_pair(load_json(args.rust), load_json(args.tvp), label="pair"))
    elif args.dir:
        fails.extend(compare_dir(args.dir))
    else:
        print("need --rust/--tvp or --dir (or --self-test)", file=sys.stderr)
        return 2

    if fails:
        print("EQUIVALENCE FAIL", file=sys.stderr)
        for line in fails:
            print(f"  {line}", file=sys.stderr)
        return 1
    print("equivalence ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
