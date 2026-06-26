#!/usr/bin/env python3
"""Assign unique (x,y) offsets inside each <spawn> for RME map-editor compatibility.

RME allows one creature per tile. TVP / monster.db conversions often emit
amount=N as N identical <monster x="0" y="0"> children — RME keeps the first
and logs "Duplicate creature" for the rest.

This script walks each spawn block and gives every child a distinct offset
within `radius` (Chebyshev / max(|x|,|y|) <= radius). Offsets are assigned in
spiral order from the center: (0,0), (1,0), (1,1), (0,1), …

772 Rust placement (`Classic772Bfs`) still searches from zone center; distinct
offsets mainly fix RME display/editing. TFS 1098 shuffle uses slot position as
search origin — spreading offsets matches FS/RME expectations there too.

Example:
  python3 scripts/spread_spawn_offsets_for_rme.py data/world/spawns.xml -o data/world/spawns-rme.xml
  python3 scripts/spread_spawn_offsets_for_rme.py data/world/spawns.xml --in-place
"""

from __future__ import annotations

import argparse
import shutil
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def _local_tag(tag: str) -> str:
    if "}" in tag:
        tag = tag.rsplit("}", 1)[-1]
    return tag.lower()


def spiral_offsets(max_radius: int):
    """Chebyshev rings: (0,0), then ring 1, ring 2, … up to max_radius."""
    r = max(max_radius, 0)
    yield 0, 0
    for dist in range(1, r + 1):
        for x in range(-dist, dist + 1):
            for y in range(-dist, dist + 1):
                if max(abs(x), abs(y)) == dist:
                    yield x, y


def effective_radius(raw: str | None) -> int:
    if raw is None:
        return 3
    try:
        v = int(raw)
    except ValueError:
        return 3
    if v < 0:
        return 3
    # RME editing is easier with a modest cap; full 50-tile spirals are huge.
    return min(v, 15)


def spread_spawn_block(spawn: ET.Element) -> int:
    radius = effective_radius(spawn.get("radius"))
    centerz = spawn.get("centerz", "7")
    used: set[tuple[int, int]] = set()
    offset_iter = spiral_offsets(radius)
    changed = 0

    for child in spawn:
        if not isinstance(child.tag, str):
            continue
        tag = _local_tag(child.tag)
        if tag not in ("monster", "npc"):
            continue

        try:
            ox = int(child.get("x", "0"))
            oy = int(child.get("y", "0"))
        except ValueError:
            ox, oy = 0, 0

        key = (ox, oy)
        if key in used:
            for cx, cy in offset_iter:
                if (cx, cy) not in used:
                    ox, oy = cx, cy
                    break
            else:
                # Radius exhausted — extend one ring beyond cap for uniqueness.
                dist = radius + 1
                while (ox, oy) in used:
                    ox = dist
                    oy = 0
                    dist += 1
            child.set("x", str(ox))
            child.set("y", str(oy))
            changed += 1

        used.add((ox, oy))
        child.set("z", centerz)

    return changed


def spread_document(root: ET.Element) -> int:
    total = 0
    for spawn in root:
        if isinstance(spawn.tag, str) and _local_tag(spawn.tag) == "spawn":
            total += spread_spawn_block(spawn)
    return total


def serialize_spawns(root: ET.Element) -> str:
    lines = ['<?xml version="1.0"?>', "<spawns>"]
    for spawn in root:
        if _local_tag(spawn.tag) != "spawn":
            continue
        attrs = " ".join(f'{k}="{spawn.get(k)}"' for k in spawn.keys())
        lines.append(f"\t<spawn {attrs}>")
        for child in spawn:
            tag = _local_tag(child.tag)
            child_attrs = " ".join(f'{k}="{child.get(k)}"' for k in child.keys())
            lines.append(f"\t\t<{tag} {child_attrs} />")
        lines.append("\t</spawn>")
    lines.append("</spawns>")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("-o", "--output", type=Path, help="output path")
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="overwrite input (creates .pre-rme-spread.bak first)",
    )
    args = parser.parse_args()

    if not args.input.is_file():
        print(f"error: not found: {args.input}", file=sys.stderr)
        return 1
    if args.in_place and args.output:
        print("error: use --in-place or --output, not both", file=sys.stderr)
        return 1

    tree = ET.parse(args.input)
    root = tree.getroot()
    if _local_tag(root.tag) != "spawns":
        print("error: expected <spawns> root", file=sys.stderr)
        return 1

    changed = spread_document(root)

    if args.in_place:
        backup = args.input.with_suffix(args.input.suffix + ".pre-rme-spread.bak")
        shutil.copy2(args.input, backup)
        out = args.input
        print(f"backup: {backup}")
    elif args.output:
        out = args.output
    else:
        out = args.input.with_name(args.input.stem + "-rme.xml")

    out.write_text(serialize_spawns(root), encoding="utf-8")
    print(f"wrote {out} ({changed} creature offset(s) reassigned)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
