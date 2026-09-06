#!/usr/bin/env python3
"""Align forgotten.otbm TILEFLAG_REFRESH (1<<5) with ORIGMAP `.sec` Refresh tiles.

ORIGMAP has 694,625 `Refresh` fields. Sector files are token streams (corpus
`TReadScriptFile`): many fields per line and unpadded offsets (`11-6:`) are
normal, so parsing is token-based, never line-based — a line-based header regex
silently drops ~21k refresh fields and ~95k fields overall.

This rewrites the OTBM in place (via a temp file):
  - insert missing ORIGMAP Refresh tiles (empty, or Content remapped TypeID→server_id)
  - set TILEFLAG_REFRESH on existing OTBM tiles that ORIGMAP marks Refresh
  - clear TILEFLAG_REFRESH on OTBM-only tiles (keep ProtectionZone / etc.)

Usage (from repo root):
  python3 scripts/patch_otbm_refresh_from_origmap.py --dry-run
  python3 scripts/patch_otbm_refresh_from_origmap.py
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

NODE_START = 0xFE
NODE_END = 0xFF
ESCAPE = 0xFD

OTBM_MAP_DATA = 2
OTBM_TILE_AREA = 4
OTBM_TILE = 5
OTBM_ITEM = 6
OTBM_HOUSETILE = 14

OTBM_ATTR_TILE_FLAGS = 3
OTBM_ATTR_ITEM = 9
OTBM_ATTR_COUNT = 15
OTBM_ATTR_CHARGES = 22

TILEFLAG_PROTECTIONZONE = 1 << 0
TILEFLAG_NOPVPZONE = 1 << 2
TILEFLAG_NOLOGOUT = 1 << 3
TILEFLAG_PVPZONE = 1 << 4
TILEFLAG_REFRESH = 1 << 5

# Corpus `TReadScriptFile` is token-based: a BYTES token `X-Y` followed by `:` starts a
# field anywhere in the stream (`map.cc` `LoadSector`). Sector files mix one-tile-per-line
# with many tiles per line and unpadded offsets (`11-6:`), so never parse line-wise.
SEC_COORD = re.compile(r"(\d{1,2})-(\d{1,2})\s*:")
SEC_ITEM_HEAD = re.compile(r"\s*(\d+)(.*)", re.S)
SEC_ITEM_ATTR = re.compile(r"([A-Za-z]+)=(\d+)")

FLAG_BITS = {
    "refresh": TILEFLAG_REFRESH,
    "protectionzone": TILEFLAG_PROTECTIONZONE,
    "nopvpzone": TILEFLAG_NOPVPZONE,
    "nologout": TILEFLAG_NOLOGOUT,
    "pvpzone": TILEFLAG_PVPZONE,
}


@dataclass
class Node:
    typ: int
    props: bytes
    children: list[Node] = field(default_factory=list)


@dataclass(frozen=True)
class SecItem:
    type_id: int
    amount: int | None = None
    charges: int | None = None


@dataclass
class OrigTile:
    flags: int
    items: tuple[SecItem, ...]


def parse_otbm_node(data: bytes, i: int) -> tuple[Node, int]:
    if data[i] != NODE_START:
        raise ValueError(f"expected NODE_START at {i}")
    i += 1
    typ = data[i]
    i += 1
    props = bytearray()
    children: list[Node] = []
    n = len(data)
    while i < n:
        b = data[i]
        if b == NODE_START:
            child, i = parse_otbm_node(data, i)
            children.append(child)
        elif b == NODE_END:
            return Node(typ, bytes(props), children), i + 1
        elif b == ESCAPE:
            i += 1
            if i >= n:
                raise ValueError("dangling OTBM escape")
            props.append(data[i])
            i += 1
        else:
            props.append(b)
            i += 1
    raise ValueError("unterminated OTBM node")


def write_escaped(out: bytearray, raw: bytes) -> None:
    for b in raw:
        if b in (ESCAPE, NODE_START, NODE_END):
            out.append(ESCAPE)
        out.append(b)


def write_otbm_node(out: bytearray, node: Node) -> None:
    out.append(NODE_START)
    out.append(node.typ)
    write_escaped(out, node.props)
    for child in node.children:
        write_otbm_node(out, child)
    out.append(NODE_END)


def load_otbm(path: Path) -> tuple[bytes, Node]:
    data = path.read_bytes()
    if len(data) < 5 or data[4] != NODE_START:
        raise ValueError(f"invalid OTBM header: {path}")
    root, end = parse_otbm_node(data, 4)
    if end != len(data):
        raise ValueError(f"OTBM trailing bytes: consumed {end} of {len(data)}")
    return data[:4], root


def map_data_node(root: Node) -> Node:
    for child in root.children:
        if child.typ == OTBM_MAP_DATA:
            return child
    raise ValueError("OTBM missing MAP_DATA")


def iter_tile_areas(map_data: Node):
    for child in map_data.children:
        if child.typ == OTBM_TILE_AREA:
            yield child


def area_base(area: Node) -> tuple[int, int, int]:
    p = area.props
    if len(p) < 5:
        raise ValueError("invalid TILE_AREA props")
    bx = int.from_bytes(p[0:2], "little")
    by = int.from_bytes(p[2:4], "little")
    return bx, by, p[4]


def tile_header_len(typ: int) -> int:
    return 6 if typ == OTBM_HOUSETILE else 2


def parse_tile_attrs(typ: int, props: bytes) -> tuple[bytes, list[tuple[int, bytes]]]:
    hdr = tile_header_len(typ)
    header = props[:hdr]
    rest = props[hdr:]
    attrs: list[tuple[int, bytes]] = []
    i = 0
    while i < len(rest):
        attr = rest[i]
        i += 1
        if attr == OTBM_ATTR_TILE_FLAGS:
            payload = rest[i : i + 4]
            i += 4
        elif attr == OTBM_ATTR_ITEM:
            payload = rest[i : i + 2]
            i += 2
        else:
            attrs.append((-1, rest[i - 1 :]))
            break
        attrs.append((attr, payload))
    return header, attrs


def tile_flags_from_attrs(attrs: list[tuple[int, bytes]]) -> int:
    for attr, payload in attrs:
        if attr == OTBM_ATTR_TILE_FLAGS and len(payload) == 4:
            return int.from_bytes(payload, "little")
    return 0


def rebuild_tile_props(
    header: bytes, attrs: list[tuple[int, bytes]], flags: int
) -> bytes:
    out = bytearray(header)
    wrote_flags = False
    for attr, payload in attrs:
        if attr == OTBM_ATTR_TILE_FLAGS:
            if flags == 0:
                continue
            out.append(OTBM_ATTR_TILE_FLAGS)
            out.extend(flags.to_bytes(4, "little"))
            wrote_flags = True
            continue
        if attr == -1:
            out.extend(payload)
            continue
        out.append(attr)
        out.extend(payload)
    if flags != 0 and not wrote_flags:
        # Insert flags immediately after the tile header (before ITEM / remainder).
        inserted = bytearray(header)
        inserted.append(OTBM_ATTR_TILE_FLAGS)
        inserted.extend(flags.to_bytes(4, "little"))
        inserted.extend(out[len(header) :])
        return bytes(inserted)
    return bytes(out)


def strip_sec_comments(text: str) -> str:
    return "\n".join(
        line for line in text.splitlines() if not line.lstrip().startswith("#")
    )


def split_sec_tiles(text: str):
    """Yield `(ox, oy, body)` per field.

    `body` runs from after the `X-Y:` token to the next coordinate token that sits at
    brace depth 0, outside quotes, and is preceded by a separator — so `Content={...}`
    with nested container contents or strings never splits a field.
    """
    n = len(text)
    m = SEC_COORD.search(text, 0)
    while m:
        ox, oy = int(m.group(1)), int(m.group(2))
        start = m.end()
        depth = 0
        in_quote = False
        k = start
        while k < n:
            c = text[k]
            if in_quote:
                if c == '"':
                    in_quote = False
            elif c == '"':
                in_quote = True
            elif c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            elif depth == 0 and c.isdigit() and (k == 0 or text[k - 1] in " \t\n,"):
                if SEC_COORD.match(text, k):
                    break
            k += 1
        yield ox, oy, text[start:k]
        m = SEC_COORD.search(text, k) if k < n else None


def split_top_level_commas(inner: str) -> list[str]:
    parts: list[str] = []
    depth = 0
    in_quote = False
    cur: list[str] = []
    for c in inner:
        if in_quote:
            cur.append(c)
            if c == '"':
                in_quote = False
            continue
        if c == '"':
            in_quote = True
            cur.append(c)
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
        if c == "," and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(c)
    if "".join(cur).strip():
        parts.append("".join(cur))
    return parts


def balanced_brace_inner(text: str) -> str:
    """`text` starts at `{`; return the contents of the matching top-level braces."""
    depth = 0
    for idx, c in enumerate(text):
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return text[1:idx]
    return text[1:]


def parse_sec_flags_and_content(rest: str) -> OrigTile:
    flags_part, sep, content_part = rest.partition("Content=")
    flags = 0
    for token in flags_part.split(","):
        name = token.strip().lower()
        if not name:
            continue
        bit = FLAG_BITS.get(name)
        if bit is not None:
            flags |= bit
    items: list[SecItem] = []
    if sep:
        inner = content_part.strip()
        inner = balanced_brace_inner(inner) if inner.startswith("{") else inner
        for raw in split_top_level_commas(inner):
            m = SEC_ITEM_HEAD.match(raw)
            if not m:
                continue
            type_id = int(m.group(1))
            amount = None
            charges = None
            # Only this item's own `Key=Value` attrs; nested `Content={...}` is skipped.
            attrs = m.group(2).split("Content=", 1)[0]
            for kv in SEC_ITEM_ATTR.finditer(attrs):
                key = kv.group(1).lower()
                if key == "amount":
                    amount = int(kv.group(2))
                elif key == "charges":
                    charges = int(kv.group(2))
            items.append(SecItem(type_id, amount, charges))
    return OrigTile(flags=flags, items=tuple(items))


def load_origmap_refresh(sec_dir: Path) -> dict[tuple[int, int, int], OrigTile]:
    out: dict[tuple[int, int, int], OrigTile] = {}
    for path in sec_dir.glob("*.sec"):
        parts = path.stem.split("-")
        if len(parts) != 3:
            continue
        sx, sy, sz = int(parts[0]), int(parts[1]), int(parts[2])
        text = strip_sec_comments(path.read_text(encoding="latin-1"))
        for ox, oy, body in split_sec_tiles(text):
            if ox >= 32 or oy >= 32:
                continue
            tile = parse_sec_flags_and_content(body)
            if tile.flags & TILEFLAG_REFRESH == 0:
                continue
            out[(sx * 32 + ox, sy * 32 + oy, sz)] = tile
    return out


def load_client_to_server(otb_path: Path) -> dict[int, int]:
    scripts_dir = str(REPO / "scripts")
    if scripts_dir not in sys.path:
        sys.path.insert(0, scripts_dir)
    import convert_itemid_to_clientid as conv

    _server_to_client, client_owners = conv.build_server_to_client(otb_path)
    # First server id wins for a client id — TFS `clientIdToServerIdMap` emplace.
    return {cid: sids[0] for cid, sids in client_owners.items() if sids}


def remap_type_id(type_id: int, client_to_server: dict[int, int]) -> int:
    return client_to_server.get(type_id, type_id)


def encode_item_node(server_id: int, item: SecItem) -> Node:
    props = bytearray(server_id.to_bytes(2, "little"))
    if item.amount is not None:
        props.append(OTBM_ATTR_COUNT)
        props.append(item.amount & 0xFF)
    if item.charges is not None:
        props.append(OTBM_ATTR_CHARGES)
        props.extend(int(item.charges).to_bytes(2, "little"))
    return Node(OTBM_ITEM, bytes(props), [])


def encode_new_tile(
    ox: int, oy: int, orig: OrigTile, client_to_server: dict[int, int]
) -> Node:
    flags = orig.flags | TILEFLAG_REFRESH
    props = bytearray((ox, oy))
    props.append(OTBM_ATTR_TILE_FLAGS)
    props.extend(flags.to_bytes(4, "little"))
    children: list[Node] = []
    if orig.items:
        ground = orig.items[0]
        gid = remap_type_id(ground.type_id, client_to_server)
        if ground.amount is None and ground.charges is None:
            props.append(OTBM_ATTR_ITEM)
            props.extend(gid.to_bytes(2, "little"))
        else:
            children.append(encode_item_node(gid, ground))
        for extra in orig.items[1:]:
            children.append(
                encode_item_node(remap_type_id(extra.type_id, client_to_server), extra)
            )
    return Node(OTBM_TILE, bytes(props), children)


def make_tile_area(bx: int, by: int, z: int) -> Node:
    props = bytearray()
    props.extend(bx.to_bytes(2, "little"))
    props.extend(by.to_bytes(2, "little"))
    props.append(z)
    return Node(OTBM_TILE_AREA, bytes(props), [])


def patch_tree(
    root: Node,
    orig: dict[tuple[int, int, int], OrigTile],
    client_to_server: dict[int, int],
) -> dict[str, int]:
    map_data = map_data_node(root)
    areas_by_key: dict[tuple[int, int, int], Node] = {}
    stats: Counter[str] = Counter()
    present: set[tuple[int, int, int]] = set()

    for area in iter_tile_areas(map_data):
        bx, by, z = area_base(area)
        areas_by_key[(bx, by, z)] = area
        for tile in area.children:
            if tile.typ not in (OTBM_TILE, OTBM_HOUSETILE):
                continue
            x = bx + tile.props[0]
            y = by + tile.props[1]
            pos = (x, y, z)
            present.add(pos)
            header, attrs = parse_tile_attrs(tile.typ, tile.props)
            flags = tile_flags_from_attrs(attrs)
            want = pos in orig
            has = flags & TILEFLAG_REFRESH != 0
            if has and not want:
                new_flags = flags & ~TILEFLAG_REFRESH
                tile.props = rebuild_tile_props(header, attrs, new_flags)
                stats["cleared_extra"] += 1
            elif want and has:
                stats["kept"] += 1
            elif want and not has:
                new_flags = flags | orig[pos].flags | TILEFLAG_REFRESH
                tile.props = rebuild_tile_props(header, attrs, new_flags)
                stats["set_on_existing"] += 1
            else:
                stats["untouched"] += 1

    missing = [pos for pos in orig if pos not in present]
    stats["missing_before"] = len(missing)
    for x, y, z in missing:
        bx, by = x & ~255, y & ~255
        key = (bx, by, z)
        area = areas_by_key.get(key)
        if area is None:
            area = make_tile_area(bx, by, z)
            map_data.children.append(area)
            areas_by_key[key] = area
            stats["new_areas"] += 1
        ox, oy = x - bx, y - by
        area.children.append(encode_new_tile(ox, oy, orig[(x, y, z)], client_to_server))
        stats["inserted"] += 1
        if orig[(x, y, z)].items:
            stats["inserted_with_content"] += 1
        else:
            stats["inserted_empty"] += 1
    return dict(stats)


def collect_refresh(root: Node) -> set[tuple[int, int, int]]:
    found: set[tuple[int, int, int]] = set()
    map_data = map_data_node(root)
    for area in iter_tile_areas(map_data):
        bx, by, z = area_base(area)
        for tile in area.children:
            if tile.typ not in (OTBM_TILE, OTBM_HOUSETILE):
                continue
            _, attrs = parse_tile_attrs(tile.typ, tile.props)
            if tile_flags_from_attrs(attrs) & TILEFLAG_REFRESH:
                found.add((bx + tile.props[0], by + tile.props[1], z))
    return found


def default_sec_dir() -> Path:
    return REPO / "reference" / "cipsoft-772" / "runtime" / "map"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--otbm",
        type=Path,
        default=REPO / "data" / "world" / "forgotten.otbm",
    )
    parser.add_argument("--sec-dir", type=Path, default=default_sec_dir())
    parser.add_argument(
        "--otb",
        type=Path,
        default=REPO / "data" / "items" / "items.otb",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="mutate in memory and report; do not write the OTBM",
    )
    args = parser.parse_args()

    if not args.otbm.is_file():
        print(f"missing OTBM: {args.otbm}", file=sys.stderr)
        return 1
    if not args.sec_dir.is_dir():
        print(f"missing ORIGMAP dir: {args.sec_dir}", file=sys.stderr)
        return 1

    print(f"Loading ORIGMAP Refresh from {args.sec_dir} …")
    orig = load_origmap_refresh(args.sec_dir)
    print(f"  ORIGMAP Refresh tiles: {len(orig)}")

    print(f"Loading OTB client→server from {args.otb} …")
    client_to_server = load_client_to_server(args.otb)
    print(f"  client_id entries: {len(client_to_server)}")

    print(f"Parsing {args.otbm} …")
    ident, root = load_otbm(args.otbm)
    before = collect_refresh(root)
    print(f"  OTBM refresh before: {len(before)}")
    orig_pos = set(orig)
    print(
        f"  intersection={len(before & orig_pos)} "
        f"orig_only={len(orig_pos - before)} otbm_only={len(before - orig_pos)}"
    )

    stats = patch_tree(root, orig, client_to_server)
    after = collect_refresh(root)
    print("Patch stats:", dict(stats))
    print(f"  OTBM refresh after: {len(after)}")
    print(
        f"  intersection={len(after & orig_pos)} "
        f"orig_only={len(orig_pos - after)} otbm_only={len(after - orig_pos)}"
    )

    if after != orig_pos:
        print("ERROR: refresh set still diverges from ORIGMAP", file=sys.stderr)
        return 1

    if args.dry_run:
        print("Dry run: OTBM not written.")
        return 0

    out = bytearray(ident)
    write_otbm_node(out, root)
    tmp = args.otbm.with_suffix(args.otbm.suffix + ".tmp")
    tmp.write_bytes(out)
    tmp.replace(args.otbm)
    print(f"Wrote {args.otbm} ({len(out)} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
