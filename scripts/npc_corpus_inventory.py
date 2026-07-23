#!/usr/bin/env python3
"""Generate a full-corpus inventory of data/npc/behavior legacy NPC scripts.

NPC-0: freeze identifiers, actions, expression functions, substitutions, include
edges, encodings, and constructs unsupported by 772 `TBehaviourDatabase`
(`crnonpl.cc` ctor / `readValue` / property table).

Usage:
  python3 scripts/npc_corpus_inventory.py          # regenerate artifacts
  python3 scripts/npc_corpus_inventory.py --check   # assert committed JSON matches
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
BEHAVIOR_DIR = REPO / "data" / "npc" / "behavior"
REF_NPC_DIR = REPO / "reference" / "cipsoft-772" / "runtime" / "npc"
OUT_JSON = REPO / "tasks" / "npc-corpus-inventory.json"
OUT_MD = REPO / "tasks" / "npc-corpus-inventory.md"

EXPECTED_NPC = 337
EXPECTED_NDB = 39
EXPECTED_INCLUDES = 165

# Identifiers are lowercased by `TReadScriptFile::getIdentifier` (`script.cc`).
KNOWN_PROPERTIES = frozenset(
    {
        "address",
        "busy",
        "vanish",
        "male",
        "female",
        "knight",
        "paladin",
        "sorcerer",
        "druid",
        "premium",
        "promoted",
        "pzblock",
        "nonpvp",
        "pvpenforced",
    }
)

KNOWN_ACTIONS = frozenset(
    {
        "topic",
        "price",
        "amount",
        "type",
        "data",
        "hp",
        "poison",
        "burning",
        "setquestvalue",
        "effectme",
        "effectopp",
        "profession",
        "teachspell",
        "summon",
        "create",
        "delete",
        "createmoney",
        "deletemoney",
        "queue",
        "teleport",
        "startposition",
        "idle",
        "nop",
    }
)

KNOWN_EXPR = frozenset(
    {
        "topic",
        "price",
        "amount",
        "level",
        "magiclevel",
        "hp",
        "poison",
        "burning",
        "count",
        "countmoney",
        "type",
        "data",
        "spellknown",
        "spelllevel",
        "random",
        "questvalue",
    }
)

# Wrapper / metadata keys in split behavior files (not dialogue vocabulary).
META_KEYS = frozenset({"behavior", "behaviour"})

INCLUDE_RE = re.compile(r'@"([^"]+)"')
IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
SUB_RE = re.compile(r"%([NATPnatp12])")
SPECIAL_OPS = ("!", "*", "$", "%1", "%2", "->", "@include")


def strip_comments_and_strings(text: str) -> str:
    """Remove `#` line comments and quoted strings (preserve structure with placeholders)."""
    out: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "#":
            while i < n and text[i] != "\n":
                i += 1
            continue
        if c == '"':
            i += 1
            while i < n:
                if text[i] == "\\" and i + 1 < n:
                    i += 2
                    continue
                if text[i] == '"':
                    i += 1
                    break
                i += 1
            out.append('""')
            continue
        out.append(c)
        i += 1
    return "".join(out)


def detect_encoding(raw: bytes) -> str:
    try:
        raw.decode("utf-8")
        return "utf-8"
    except UnicodeDecodeError:
        return "non-utf8"


def detect_newlines(raw: bytes) -> str:
    if b"\r\n" in raw:
        return "crlf"
    if b"\n" in raw:
        return "lf"
    return "none"


def scan_file(path: Path) -> dict:
    raw = path.read_bytes()
    text = raw.decode("latin-1")
    encoding = detect_encoding(raw)
    newlines = detect_newlines(raw)

    includes = [m.group(1) for m in INCLUDE_RE.finditer(text)]
    subs: Counter[str] = Counter()
    for m in SUB_RE.finditer(text):
        # Preserve case for %N/%A/%P/%T; normalize %1/%2.
        ch = m.group(1)
        if ch in "12":
            subs[f"%{ch}"] += 1
        else:
            subs[f"%{ch.upper()}"] += 1

    cleaned = strip_comments_and_strings(text)
    cleaned = INCLUDE_RE.sub("", cleaned)

    funcs: Counter[str] = Counter()
    assigns: Counter[str] = Counter()
    bare: Counter[str] = Counter()
    raw_spellings: dict[str, set[str]] = defaultdict(set)
    special_counts: Counter[str] = Counter()

    if "!" in cleaned:
        special_counts["!"] = cleaned.count("!")
    if "*" in cleaned:
        # `*` repeat action and `*` multiply in expressions — count raw stars.
        special_counts["*"] = cleaned.count("*")
    # `$` word-boundary markers live inside keyword strings (stripped from cleaned).
    if "$" in text:
        special_counts["$"] = text.count("$")
    if "->" in cleaned:
        special_counts["->"] = cleaned.count("->")
    if includes:
        special_counts["@include"] = len(includes)
    for m in re.finditer(r"%[12]", cleaned):
        special_counts[m.group(0)] += 1

    for m in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", cleaned):
        raw = m.group(1)
        key = raw.lower()
        funcs[key] += 1
        raw_spellings[key].add(raw)

    for m in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*=", cleaned):
        raw = m.group(1)
        key = raw.lower()
        assigns[key] += 1
        raw_spellings[key].add(raw)

    for m in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\b(?!\s*[=(])", cleaned):
        raw = m.group(1)
        key = raw.lower()
        if key in META_KEYS:
            continue
        bare[key] += 1
        raw_spellings[key].add(raw)

    return {
        "path": path.name,
        "kind": path.suffix.lstrip("."),
        "encoding": encoding,
        "newlines": newlines,
        "byte_len": len(raw),
        "includes": includes,
        "substitutions": dict(subs),
        "functions": dict(funcs),
        "assignments": dict(assigns),
        "bare_identifiers": dict(bare),
        "raw_spellings": {k: sorted(v) for k, v in raw_spellings.items()},
        "special": dict(special_counts),
    }


def merge_counters(files: list[dict], field: str) -> Counter[str]:
    total: Counter[str] = Counter()
    for f in files:
        total.update(f[field])
    return total


def merge_spellings(files: list[dict]) -> dict[str, list[str]]:
    out: dict[str, set[str]] = defaultdict(set)
    for f in files:
        for k, vals in f["raw_spellings"].items():
            out[k].update(vals)
    return {k: sorted(v) for k, v in sorted(out.items())}


def classify_unsupported(
    funcs: Counter[str],
    assigns: Counter[str],
    bare: Counter[str],
    spellings: dict[str, list[str]],
    encodings: list[dict],
) -> list[dict]:
    """List constructs that do not map to 772 parser vocabulary."""
    unsupported: list[dict] = []

    for name, count in sorted(funcs.items()):
        if name in META_KEYS:
            continue
        if name not in KNOWN_ACTIONS and name not in KNOWN_EXPR:
            unsupported.append(
                {
                    "kind": "function_call",
                    "canonical": name,
                    "raw_spellings": spellings.get(name, []),
                    "occurrences": count,
                    "note": "not in 772 action/expression tables (crnonpl.cc)",
                }
            )

    for name, count in sorted(assigns.items()):
        if name in META_KEYS:
            continue
        if name not in KNOWN_ACTIONS and name not in KNOWN_EXPR:
            unsupported.append(
                {
                    "kind": "assignment",
                    "canonical": name,
                    "raw_spellings": spellings.get(name, []),
                    "occurrences": count,
                    "note": "not in 772 SET_VARIABLE / SET_SKILL action ids",
                }
            )

    # Bare identifiers that look like intended properties but are unknown.
    for name, count in sorted(bare.items()):
        if name in KNOWN_PROPERTIES or name in KNOWN_ACTIONS or name in KNOWN_EXPR:
            continue
        if name in META_KEYS:
            continue
        # Ignore very rare noise / single-letter specials from operators.
        if len(name) <= 1:
            continue
        # Likely TeachSpell(String) / SpellKnown(String) operand — covered as assignment.
        if name == "string":
            continue
        # Bare RHS action shape (like Idle/NOP/Queue) that is not in 772 tables.
        kind = "unknown_action" if name in {"promote", "bless", "town"} else "bare_identifier"
        unsupported.append(
            {
                "kind": kind,
                "canonical": name,
                "raw_spellings": spellings.get(name, []),
                "occurrences": count,
                "note": "not in 772 action/property tables (crnonpl.cc)",
            }
        )

    for entry in encodings:
        if entry["encoding"] != "utf-8":
            unsupported.append(
                {
                    "kind": "encoding",
                    "canonical": entry["path"],
                    "raw_spellings": [],
                    "occurrences": 1,
                    "note": f"file is {entry['encoding']} (latin-1 decode used for inventory)",
                }
            )

    # Case-variant spellings that still lower to known ids — informational, not blockers.
    case_variants: list[dict] = []
    for canon, raws in spellings.items():
        if canon in KNOWN_ACTIONS | KNOWN_EXPR | KNOWN_PROPERTIES:
            if len(raws) > 1 or (len(raws) == 1 and raws[0] != canon and raws[0] != canon.capitalize()):
                # Only flag when mixed case forms exist beyond Title case.
                if len(set(r.lower() for r in raws)) == 1 and len(raws) > 1:
                    case_variants.append(
                        {
                            "kind": "case_variant",
                            "canonical": canon,
                            "raw_spellings": raws,
                            "occurrences": len(raws),
                            "note": "identifiers are lowercased at parse; all forms are supported",
                        }
                    )

    return unsupported, case_variants


def build_inventory() -> dict:
    npc_files = sorted(BEHAVIOR_DIR.glob("*.npc"))
    ndb_files = sorted(BEHAVIOR_DIR.glob("*.ndb"))
    if len(npc_files) != EXPECTED_NPC or len(ndb_files) != EXPECTED_NDB:
        raise SystemExit(
            f"coverage mismatch: expected {EXPECTED_NPC} npc + {EXPECTED_NDB} ndb, "
            f"got {len(npc_files)} npc + {len(ndb_files)} ndb under {BEHAVIOR_DIR}"
        )

    scanned = [scan_file(p) for p in npc_files + ndb_files]
    include_edges = []
    missing_includes = []
    for f in scanned:
        for target in f["includes"]:
            include_edges.append({"from": f["path"], "to": target})
            resolved = (BEHAVIOR_DIR / target).resolve()
            if not resolved.is_file():
                missing_includes.append({"from": f["path"], "to": target})

    if len(include_edges) != EXPECTED_INCLUDES:
        raise SystemExit(
            f"include edge count mismatch: expected {EXPECTED_INCLUDES}, got {len(include_edges)}"
        )

    funcs = merge_counters(scanned, "functions")
    assigns = merge_counters(scanned, "assignments")
    bare = merge_counters(scanned, "bare_identifiers")
    subs = merge_counters(scanned, "substitutions")
    special = merge_counters(scanned, "special")
    spellings = merge_spellings(scanned)

    properties_seen = {
        k: bare[k] for k in sorted(KNOWN_PROPERTIES) if bare.get(k, 0) > 0
    }

    unsupported, case_variants = classify_unsupported(
        funcs,
        assigns,
        bare,
        spellings,
        [{"path": f["path"], "encoding": f["encoding"]} for f in scanned],
    )

    ref_npc = sorted(REF_NPC_DIR.glob("*.npc")) if REF_NPC_DIR.is_dir() else []
    ref_ndb = sorted(REF_NPC_DIR.glob("*.ndb")) if REF_NPC_DIR.is_dir() else []

    inventory = {
        "schema_version": 1,
        "authority": "data/npc/behavior",
        "reference_crosscheck": {
            "path": "reference/cipsoft-772/runtime/npc",
            "npc_count": len(ref_npc),
            "ndb_count": len(ref_ndb),
            "note": "counts only; data/npc/behavior is inventory authority",
        },
        "coverage": {
            "npc_files": len(npc_files),
            "ndb_files": len(ndb_files),
            "include_directives": len(include_edges),
            "unique_include_targets": len({e["to"] for e in include_edges}),
            "files_scanned": sorted(f["path"] for f in scanned),
        },
        "encodings": {
            "utf-8": sum(1 for f in scanned if f["encoding"] == "utf-8"),
            "non-utf8": sorted(f["path"] for f in scanned if f["encoding"] != "utf-8"),
            "newlines": {
                "lf": sum(1 for f in scanned if f["newlines"] == "lf"),
                "crlf": sum(1 for f in scanned if f["newlines"] == "crlf"),
                "none": sum(1 for f in scanned if f["newlines"] == "none"),
            },
        },
        "include_edges": sorted(include_edges, key=lambda e: (e["from"], e["to"])),
        "missing_includes": missing_includes,
        "known_vocabulary": {
            "properties": sorted(KNOWN_PROPERTIES),
            "actions": sorted(KNOWN_ACTIONS),
            "expression_values": sorted(KNOWN_EXPR),
            "cpp_refs": [
                "reference/cipsoft-772/tibia-game-master/src/crnonpl.cc",
                "reference/cipsoft-772/tibia-game-master/src/enums.hh",
                "reference/cipsoft-772/tibia-game-master/src/script.cc (getIdentifier lowercases)",
            ],
        },
        "identifiers": {
            "functions": dict(sorted(funcs.items())),
            "assignments": dict(sorted(assigns.items())),
            "properties_seen": properties_seen,
            "raw_spellings": spellings,
        },
        "substitutions": dict(sorted(subs.items())),
        "special_tokens": dict(sorted(special.items())),
        "unsupported_constructs": unsupported,
        "case_variants": case_variants,
    }
    return inventory


def write_md(inv: dict) -> str:
    lines = [
        "# NPC corpus inventory (NPC-0)",
        "",
        f"Authority: `{inv['authority']}`",
        "",
        "## Coverage",
        "",
        f"- `.npc` files: **{inv['coverage']['npc_files']}**",
        f"- `.ndb` fragments: **{inv['coverage']['ndb_files']}**",
        f"- Include directives (`@\"…\"`): **{inv['coverage']['include_directives']}** "
        f"(unique targets: {inv['coverage']['unique_include_targets']})",
        f"- Reference cross-check (`{inv['reference_crosscheck']['path']}`): "
        f"{inv['reference_crosscheck']['npc_count']} npc / "
        f"{inv['reference_crosscheck']['ndb_count']} ndb",
        "",
        "## Encodings",
        "",
        f"- utf-8: {inv['encodings']['utf-8']}",
        f"- non-utf8: {', '.join(inv['encodings']['non-utf8']) or '(none)'}",
        f"- newlines: {inv['encodings']['newlines']}",
        "",
        "## Substitutions",
        "",
    ]
    for k, v in inv["substitutions"].items():
        lines.append(f"- `{k}`: {v}")
    lines += ["", "## Special tokens", ""]
    for k, v in inv["special_tokens"].items():
        lines.append(f"- `{k}`: {v}")
    lines += ["", "## Functions (call sites)", ""]
    for k, v in inv["identifiers"]["functions"].items():
        lines.append(f"- `{k}`: {v}")
    lines += ["", "## Assignments", ""]
    for k, v in inv["identifiers"]["assignments"].items():
        lines.append(f"- `{k}`: {v}")
    lines += ["", "## Properties seen", ""]
    for k, v in inv["identifiers"]["properties_seen"].items():
        lines.append(f"- `{k}`: {v}")
    lines += ["", "## Unsupported / ambiguous constructs", ""]
    if not inv["unsupported_constructs"]:
        lines.append("(none)")
    else:
        for u in inv["unsupported_constructs"]:
            spell = ", ".join(u["raw_spellings"]) if u["raw_spellings"] else "—"
            lines.append(
                f"- **{u['kind']}** `{u['canonical']}` "
                f"(raw: {spell}; n={u['occurrences']}): {u['note']}"
            )
    lines += ["", "## Case variants (supported after lowercase)", ""]
    if not inv["case_variants"]:
        lines.append("(none notable)")
    else:
        for u in inv["case_variants"]:
            lines.append(
                f"- `{u['canonical']}`: {', '.join(u['raw_spellings'])}"
            )
    lines.append("")
    return "\n".join(lines)


def canonical_json(obj: dict) -> str:
    return json.dumps(obj, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify committed inventory JSON matches a fresh scan",
    )
    args = parser.parse_args()

    inv = build_inventory()
    payload = canonical_json(inv)

    if args.check:
        if not OUT_JSON.is_file():
            print(f"missing {OUT_JSON}", file=sys.stderr)
            return 1
        existing = OUT_JSON.read_text(encoding="utf-8")
        if existing != payload:
            print(
                f"{OUT_JSON} is stale; run scripts/npc_corpus_inventory.py to regenerate",
                file=sys.stderr,
            )
            return 1
        print(
            f"ok: {inv['coverage']['npc_files']} npc + "
            f"{inv['coverage']['ndb_files']} ndb + "
            f"{inv['coverage']['include_directives']} includes; "
            f"{len(inv['unsupported_constructs'])} unsupported entries"
        )
        return 0

    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(payload, encoding="utf-8")
    OUT_MD.write_text(write_md(inv), encoding="utf-8")
    print(f"wrote {OUT_JSON.relative_to(REPO)}")
    print(f"wrote {OUT_MD.relative_to(REPO)}")
    print(
        f"coverage {inv['coverage']['npc_files']}+{inv['coverage']['ndb_files']}+"
        f"{inv['coverage']['include_directives']}; "
        f"unsupported={len(inv['unsupported_constructs'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
