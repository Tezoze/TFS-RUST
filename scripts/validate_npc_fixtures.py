#!/usr/bin/env python3
"""Validate NPC-0 black-box transcript fixtures under tests/fixtures/npc/.

Checks required fields, source paths, event kinds, and reply delay_ms against
772 TalkDelay rules (crnonpl.cc:1088-1113).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
FIXTURE_DIR = REPO / "tests" / "fixtures" / "npc"
BEHAVIOR_DIR = REPO / "data" / "npc" / "behavior"

REQUIRED_TOP = (
    "id",
    "description",
    "npc",
    "sources",
    "rng_seed",
    "round_nr",
    "server_ms",
    "players",
    "npc_state",
    "steps",
    "expected",
    "cpp_refs",
)

REQUIRED_NPC_STATE = (
    "home",
    "radius",
    "state",
    "focus",
    "topic",
    "price",
    "amount",
    "type",
    "data",
    "queue",
)

REQUIRED_PLAYER = ("id", "name", "pos", "hp")

EVENT_KINDS = frozenset(
    {
        "situation",
        "match_rule",
        "state",
        "focus",
        "turn_to",
        "queue",
        "set",
        "say",
        "mutate",
        "todo",
    }
)

SITUATIONS = frozenset(
    {"ADDRESS", "DEFAULT", "BUSY", "VANISH", "ADDRESSQUEUE"}
)

STEP_OPS = frozenset({"say", "wait_rounds", "move_player", "remove_player"})

EXPECTED_FIXTURES = frozenset(
    {
        "greeting_farewell",
        "quentin_heal",
        "zebron_gamble",
        "bank_change",
        "explorer_quest",
        "guard_thais",
        "multi_reply_timing",
        "two_player_busy_queue_vanish",
    }
)


def talk_delay_sequence(texts: list[str]) -> tuple[list[int], int]:
    """Return per-reply absolute TalkDelay values and final TalkDelay after last reply."""
    talk_delay = 1000
    delays: list[int] = []
    for text in texts:
        delays.append(talk_delay)
        byte_len = len(text.encode("latin-1"))
        talk_delay += 3100 + (byte_len // 2) * 100
    return delays, talk_delay


def err(errors: list[str], path: str, msg: str) -> None:
    errors.append(f"{path}: {msg}")


def validate_fixture(path: Path, errors: list[str]) -> None:
    rel = str(path.relative_to(REPO))
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        err(errors, rel, f"invalid JSON: {exc}")
        return

    if not isinstance(data, dict):
        err(errors, rel, "root must be an object")
        return

    for key in REQUIRED_TOP:
        if key not in data:
            err(errors, rel, f"missing required field `{key}`")

    stem = path.stem
    if data.get("id") != stem:
        err(errors, rel, f"`id` must equal filename stem `{stem}`")

    sources = data.get("sources")
    if isinstance(sources, list):
        for src in sources:
            if not (BEHAVIOR_DIR / src).is_file():
                err(errors, rel, f"source not found: data/npc/behavior/{src}")
    else:
        err(errors, rel, "`sources` must be an array")

    npc_state = data.get("npc_state")
    if isinstance(npc_state, dict):
        for key in REQUIRED_NPC_STATE:
            if key not in npc_state:
                err(errors, rel, f"npc_state missing `{key}`")
    else:
        err(errors, rel, "`npc_state` must be an object")

    players = data.get("players")
    player_ids: set[str] = set()
    if isinstance(players, list) and players:
        for i, p in enumerate(players):
            if not isinstance(p, dict):
                err(errors, rel, f"players[{i}] must be an object")
                continue
            for key in REQUIRED_PLAYER:
                if key not in p:
                    err(errors, rel, f"players[{i}] missing `{key}`")
            pid = p.get("id")
            if isinstance(pid, str):
                if pid in player_ids:
                    err(errors, rel, f"duplicate player id `{pid}`")
                player_ids.add(pid)
    else:
        err(errors, rel, "`players` must be a non-empty array")

    steps = data.get("steps")
    if isinstance(steps, list) and steps:
        prev_round = -1
        for i, step in enumerate(steps):
            if not isinstance(step, dict):
                err(errors, rel, f"steps[{i}] must be an object")
                continue
            if "at_round" not in step or "op" not in step:
                err(errors, rel, f"steps[{i}] needs at_round and op")
                continue
            if step["op"] not in STEP_OPS:
                err(errors, rel, f"steps[{i}] unknown op `{step['op']}`")
            if not isinstance(step["at_round"], int):
                err(errors, rel, f"steps[{i}].at_round must be int")
            elif step["at_round"] < prev_round:
                err(errors, rel, f"steps[{i}].at_round not monotonic")
            else:
                prev_round = step["at_round"]
            if step["op"] == "say":
                if step.get("player") not in player_ids:
                    err(errors, rel, f"steps[{i}] say player unknown")
                if not isinstance(step.get("text"), str):
                    err(errors, rel, f"steps[{i}] say needs text")
    else:
        err(errors, rel, "`steps` must be a non-empty array")

    expected = data.get("expected")
    if not isinstance(expected, list) or not expected:
        err(errors, rel, "`expected` must be a non-empty array")
        return

    # Validate events + reply delay chains per contiguous say-group after each situation.
    say_batch: list[dict] = []

    def flush_say_batch(batch: list[dict], context: str) -> None:
        if not batch:
            return
        texts = [e["text"] for e in batch]
        delays, _final = talk_delay_sequence(texts)
        for e, delay in zip(batch, delays):
            if e.get("delay_ms") != delay:
                err(
                    errors,
                    rel,
                    f"{context}: say delay_ms={e.get('delay_ms')} "
                    f"expected {delay} for text {e.get('text')!r}",
                )
            bl = e.get("byte_len")
            actual = len(e["text"].encode("latin-1"))
            if bl != actual:
                err(
                    errors,
                    rel,
                    f"{context}: byte_len={bl} expected {actual} for {e.get('text')!r}",
                )

    for i, ev in enumerate(expected):
        if not isinstance(ev, dict):
            err(errors, rel, f"expected[{i}] must be an object")
            continue
        kind = ev.get("kind")
        if kind not in EVENT_KINDS:
            err(errors, rel, f"expected[{i}] unknown kind `{kind}`")
            continue
        if kind == "situation":
            flush_say_batch(say_batch, f"expected before situation@{i}")
            say_batch = []
            if ev.get("name") not in SITUATIONS:
                err(errors, rel, f"expected[{i}] bad situation `{ev.get('name')}`")
        elif kind == "say":
            if not isinstance(ev.get("text"), str):
                err(errors, rel, f"expected[{i}] say needs text")
            else:
                say_batch.append(ev)
        elif kind == "set":
            if "var" not in ev or "value" not in ev:
                err(errors, rel, f"expected[{i}] set needs var/value")
        elif kind == "queue":
            if ev.get("op") not in {"push", "pop", "dedupe_skip"}:
                err(errors, rel, f"expected[{i}] bad queue op")
        elif kind == "todo":
            if ev.get("op") not in {"wait", "talk", "start"}:
                err(errors, rel, f"expected[{i}] bad todo op")
            if ev.get("op") == "wait" and say_batch:
                # Final wait after a reply batch should equal post-last TalkDelay.
                _delays, final = talk_delay_sequence([e["text"] for e in say_batch])
                if ev.get("delay_ms") != final:
                    err(
                        errors,
                        rel,
                        f"expected[{i}] todo wait delay_ms={ev.get('delay_ms')} "
                        f"expected {final}",
                    )

    flush_say_batch(say_batch, "expected trailing says")

    if not isinstance(data.get("cpp_refs"), list) or not data["cpp_refs"]:
        err(errors, rel, "`cpp_refs` must be a non-empty array")

    if not isinstance(data.get("rng_seed"), int):
        err(errors, rel, "`rng_seed` must be int")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()

    if not FIXTURE_DIR.is_dir():
        print(f"missing {FIXTURE_DIR}", file=sys.stderr)
        return 1

    files = sorted(FIXTURE_DIR.glob("*.json"))
    stems = {p.stem for p in files}
    errors: list[str] = []

    missing = EXPECTED_FIXTURES - stems
    extra = stems - EXPECTED_FIXTURES
    if missing:
        errors.append(f"missing fixtures: {sorted(missing)}")
    if extra:
        errors.append(f"unexpected fixtures: {sorted(extra)}")

    for path in files:
        validate_fixture(path, errors)

    if errors:
        print(f"{len(errors)} validation error(s):", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print(f"ok: {len(files)} fixtures validated under {FIXTURE_DIR.relative_to(REPO)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
