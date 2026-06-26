#!/usr/bin/env python3
"""Convert TVP flat spawn XML (<tvpspawn …/>) to TFS nested format (<spawn> children).

Matches `crates/tfs-rust-content/src/spawns.rs` `parse_tvp_spawn_element`:
each tvpspawn row becomes one <spawn> zone; `amount` expands to N child entries at x=0,y=0.

Example:
  python3 scripts/convert_tvp_spawns_to_tfs.py data/world/spawns.xml --in-place
"""

from __future__ import annotations

import argparse
import shutil
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def _local_tag(tag: str) -> str:
    if "}" in tag:
        return tag.rsplit("}", 1)[-1]
    return tag


def _child_attrs(
    *,
    name: str,
    centerz: str,
    spawntime: str,
    direction: str | None,
    kind: str,
) -> dict[str, str]:
    attrs: dict[str, str] = {
        "name": name,
        "x": "0",
        "y": "0",
        "z": centerz,
        "spawntime": spawntime,
    }
    if kind == "npc" and direction is not None:
        attrs["direction"] = direction
    return attrs


def convert_tvpspawn(elem: ET.Element) -> ET.Element:
    """Turn one <tvpspawn> element into a nested <spawn> block."""
    spawn = ET.Element("spawn")
    for key in ("centerx", "centery", "centerz", "radius"):
        value = elem.get(key)
        if value is not None:
            spawn.set(key, value)

    centerz = elem.get("centerz", "7")
    spawntime = elem.get("spawntime", "60")
    direction = elem.get("direction")
    amount = max(int(elem.get("amount", "1") or "1"), 1)

    monster_name = elem.get("monstername")
    npc_name = elem.get("npcname")

    if monster_name:
        for _ in range(amount):
            child = ET.SubElement(spawn, "monster")
            child.attrib.update(
                _child_attrs(
                    name=monster_name,
                    centerz=centerz,
                    spawntime=spawntime,
                    direction=direction,
                    kind="monster",
                )
            )
    elif npc_name:
        for _ in range(amount):
            child = ET.SubElement(spawn, "npc")
            child.attrib.update(
                _child_attrs(
                    name=npc_name,
                    centerz=centerz,
                    spawntime=spawntime,
                    direction=direction,
                    kind="npc",
                )
            )

    return spawn


def convert_document(root: ET.Element) -> tuple[int, int]:
    """Replace <tvpspawn> children under <spawns> with nested <spawn> blocks."""
    if _local_tag(root.tag) != "spawns":
        raise ValueError(f"expected <spawns> root, got <{_local_tag(root.tag)}>")

    converted = 0
    skipped = 0
    new_children: list[ET.Element] = []

    for child in list(root):
        if not isinstance(child.tag, str):
            continue
        tag = _local_tag(child.tag)
        if tag.lower() == "tvpspawn":
            spawn = convert_tvpspawn(child)
            if len(spawn):
                new_children.append(spawn)
                converted += 1
            else:
                skipped += 1
        elif tag.lower() == "spawn":
            new_children.append(child)
        else:
            skipped += 1

    root[:] = new_children
    return converted, skipped


def _serialize_spawns(root: ET.Element) -> str:
    """Emit TFS-style spawn XML with tab indents (matches forgotten-spawn.xml)."""
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


def convert_file(src: Path, dst: Path) -> tuple[int, int]:
    tree = ET.parse(src)
    root = tree.getroot()
    counts = convert_document(root)
    dst.write_text(_serialize_spawns(root), encoding="utf-8")
    return counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="source spawn XML (TVP or mixed)")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="output path (default: stdout unless --in-place)",
    )
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="overwrite input file (writes .tvp.bak backup first)",
    )
    args = parser.parse_args()

    if not args.input.is_file():
        print(f"error: not found: {args.input}", file=sys.stderr)
        return 1

    if args.in_place and args.output:
        print("error: use either --in-place or --output, not both", file=sys.stderr)
        return 1

    if args.in_place:
        backup = args.input.with_suffix(args.input.suffix + ".tvp.bak")
        shutil.copy2(args.input, backup)
        out = args.input
        print(f"backup: {backup}")
    elif args.output:
        out = args.output
    else:
        out = None

    converted, skipped = convert_file(args.input, out or Path("/tmp/tfs-spawn-convert.xml"))
    summary = f"converted {converted} tvpspawn row(s)"
    if skipped:
        summary += f", skipped {skipped} other node(s)"

    if out is None:
        print(Path("/tmp/tfs-spawn-convert.xml").read_text(), end="")
    else:
        print(f"wrote {out} ({summary})")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
