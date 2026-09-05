#!/usr/bin/env python3
"""Convert server item ids to client ids using items.otb (Sarah Wesker xmlConverter, offline).

The original TFS talkaction called ItemType(id):getClientId() on a live server and
rewrote XML `id` / `fromId` / `toId` attributes. This script builds the same map
from items.otb (ITEM_ATTR_SERVERID → ITEM_ATTR_CLIENTID) and can rewrite:

  - items.ron  — unified typed catalog (future engine file for every clientVersion)
  - items.otb  — optional rewrite (ITEM_ATTR_SERVERID → client id)
  - extra XML  — any files under --xml-dir (same id/fromId/toId attrs as the Lua)

Covers `loadFromOtb` + `parseItemNode` + abilities, including nested `field.*`
(`cycles` / `initdamage` / `skippeaceful`). `--self-test` checks the schema
against `items_xml_keys.rs`.

Unmapped ids and client_id 0 are left unchanged (Lua skips those). Writes to an
output directory by default; never overwrites inputs unless --in-place.

Examples:
  python3 scripts/convert_itemid_to_clientid.py
  python3 scripts/convert_itemid_to_clientid.py --dry-run
  python3 scripts/convert_itemid_to_clientid.py --xml-dir data/xmlConverter/input
  python3 scripts/convert_itemid_to_clientid.py --self-test --skip-otb
"""

from __future__ import annotations

import argparse
import csv
import re
import shutil
import sys
import xml.etree.ElementTree as ET
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

NODE_START = 0xFE
NODE_END = 0xFF
ESCAPE = 0xFD
ROOT_ATTR_VERSION = 0x01
ITEM_ATTR_SERVERID = 0x10
ITEM_ATTR_CLIENTID = 0x11
ITEM_ATTR_SPEED = 0x14
ITEM_ATTR_LIGHT2 = 0x2A
ITEM_ATTR_TOPORDER = 0x2B
ITEM_ATTR_WAREID = 0x2D

# itemgroup_t — src/itemloader.h
GROUP_NAMES = {
    1: "Ground",
    2: "Container",
    3: "Weapon",
    4: "Ammunition",
    5: "Armor",
    6: "Charges",
    7: "Teleport",
    8: "MagicField",
    9: "Writeable",
    10: "Key",
    11: "Splash",
    12: "Fluid",
    13: "Door",
    14: "Deprecated",
}

# itemflags_t bits `ItemType` accessors read (`otb.rs`). Other bits are stored raw
# on `ItemType.flags` but have no loader accessors — emitted as FlagN if set.
FLAG_NAMES = (
    (0, "BlockSolid"),
    (1, "BlockProjectile"),
    (2, "BlockPathFind"),
    (3, "HasHeight"),
    (4, "Useable"),
    (5, "Pickupable"),
    (6, "Moveable"),
    (7, "Stackable"),
    (13, "AlwaysOnTop"),
    (14, "Readable"),
    (15, "Rotatable"),
    (16, "Hangable"),
    (17, "Vertical"),
    (18, "Horizontal"),
    (20, "AllowDistRead"),
    (23, "LookThrough"),
    (24, "Animation"),
    (26, "ForceUse"),
)

# Lua: id="N" or fromId="N" / toId="N" (TFS items.xml uses lowercase fromid/toid).
ID_ATTR_RE = re.compile(
    r'\b((?:fromid|toid|fromId|toId|id)\s*=\s*)(["\'])(\d+)\2',
    re.IGNORECASE,
)

# items.xml <attribute key="…" value="N"/> whose value is a server item id.
ITEM_ID_XML_KEYS = frozenset(
    {
        "decayto",
        "destroyto",
        "rotateto",
        "transformto",
        "transformequipto",
        "transformdeequipto",
        "writeonceitemid",
        "maletransformto",
        "femaletransformto",
        "malesleeper",
        "femalesleeper",
    }
)
ITEM_ATTR_VALUE_RE = re.compile(
    r'(<attribute\b[^>]*\bkey\s*=\s*)(["\'])([^"\']+)\2([^>]*\bvalue\s*=\s*)(["\'])(\d+)\5',
    re.IGNORECASE,
)


@dataclass
class OtbNode:
    node_type: int
    flags: int
    attrs: list[tuple[int, bytes]]
    children: list[OtbNode] = field(default_factory=list)


@dataclass
class ConvertStats:
    xml_replacements: int = 0
    xml_unmapped: int = 0
    otb_nodes_rewritten: int = 0
    otb_nodes_unchanged: int = 0
    ron_items: int = 0
    collisions: list[str] = field(default_factory=list)


@dataclass
class OtbItem:
    server_id: int
    client_id: int
    group: int
    flags: int
    speed: int | None = None
    light_level: int | None = None
    light_color: int | None = None
    always_on_top_order: int | None = None
    ware_id: int | None = None


@dataclass
class CatalogItem:
    id: int
    name: str = ""
    article: str = ""
    plural: str = ""
    editorsuffix: str = ""
    group: int = 0
    flags: int = 0
    speed: int | None = None
    light_level: int | None = None
    light_color: int | None = None
    always_on_top_order: int | None = None
    ware_id: int | None = None
    vocations: list[str] = field(default_factory=list)
    attributes: dict[str, str] = field(default_factory=dict)
    otb_applied: bool = False


class OtbError(RuntimeError):
    pass


def _need(data: bytes, pos: int, n: int = 1) -> None:
    if pos + n > len(data):
        raise OtbError(f"unexpected EOF at {pos} (need {n} byte(s), have {len(data) - pos})")


def read_data_byte(data: bytes, pos: int) -> tuple[int, int]:
    """Read one logical byte, consuming an ESCAPE prefix when present. C++ fileloader.cpp."""
    _need(data, pos)
    if data[pos] == ESCAPE:
        pos += 1
        _need(data, pos)
        return data[pos], pos + 1
    return data[pos], pos + 1


def read_data_u16(data: bytes, pos: int) -> tuple[int, int]:
    lo, pos = read_data_byte(data, pos)
    hi, pos = read_data_byte(data, pos)
    return lo | (hi << 8), pos


def read_data_u32(data: bytes, pos: int) -> tuple[int, int]:
    b0, pos = read_data_byte(data, pos)
    b1, pos = read_data_byte(data, pos)
    b2, pos = read_data_byte(data, pos)
    b3, pos = read_data_byte(data, pos)
    return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24), pos


def write_data_byte(out: bytearray, value: int) -> None:
    b = value & 0xFF
    if b in (ESCAPE, NODE_START, NODE_END):
        out.append(ESCAPE)
    out.append(b)


def write_data_u16(out: bytearray, value: int) -> None:
    write_data_byte(out, value & 0xFF)
    write_data_byte(out, (value >> 8) & 0xFF)


def write_data_u32(out: bytearray, value: int) -> None:
    write_data_byte(out, value & 0xFF)
    write_data_byte(out, (value >> 8) & 0xFF)
    write_data_byte(out, (value >> 16) & 0xFF)
    write_data_byte(out, (value >> 24) & 0xFF)


def parse_node(data: bytes, pos: int) -> tuple[OtbNode, int]:
    _need(data, pos)
    if data[pos] != NODE_START:
        raise OtbError(f"expected NODE_START at {pos}, got {data[pos]:#04x}")
    pos += 1
    node_type, pos = read_data_byte(data, pos)
    flags, pos = read_data_u32(data, pos)
    attrs: list[tuple[int, bytes]] = []
    while pos < len(data) and data[pos] not in (NODE_START, NODE_END):
        attr_type, pos = read_data_byte(data, pos)
        size, pos = read_data_u16(data, pos)
        payload = bytearray()
        for _ in range(size):
            b, pos = read_data_byte(data, pos)
            payload.append(b)
        attrs.append((attr_type, bytes(payload)))
    children: list[OtbNode] = []
    while pos < len(data) and data[pos] == NODE_START:
        child, pos = parse_node(data, pos)
        children.append(child)
    _need(data, pos)
    if data[pos] != NODE_END:
        raise OtbError(f"expected NODE_END at {pos}, got {data[pos]:#04x}")
    pos += 1
    return OtbNode(node_type, flags, attrs, children), pos


def write_node(out: bytearray, node: OtbNode) -> None:
    out.append(NODE_START)
    write_data_byte(out, node.node_type)
    write_data_u32(out, node.flags)
    for attr_type, payload in node.attrs:
        write_data_byte(out, attr_type)
        write_data_u16(out, len(payload))
        for b in payload:
            write_data_byte(out, b)
    for child in node.children:
        write_node(out, child)
    out.append(NODE_END)


def parse_otb_forest(data: bytes) -> tuple[bytes, list[tuple[bytes, OtbNode | None]]]:
    """Return (identifier, sequence of leftover bytes and/or root nodes)."""
    if len(data) < 4:
        raise OtbError("items.otb too small for 4-byte identifier")
    ident = data[:4]
    if ident not in (b"OTBI", b"\x00\x00\x00\x00"):
        raise OtbError("items.otb must start with OTBI (or wildcard \\0\\0\\0\\0)")
    chunks: list[tuple[bytes, OtbNode | None]] = []
    pos = 4
    while pos < len(data):
        if data[pos] == NODE_START:
            node, pos = parse_node(data, pos)
            chunks.append((b"", node))
        else:
            chunks.append((bytes([data[pos]]), None))
            pos += 1
    return ident, chunks


def attr_u16(payload: bytes) -> int | None:
    if len(payload) < 2:
        return None
    return payload[0] | (payload[1] << 8)


def iter_item_nodes(node: OtbNode):
    yield node
    for child in node.children:
        yield from iter_item_nodes(child)


def build_server_to_client(otb: Path) -> tuple[dict[int, int], dict[int, list[int]]]:
    """server_id → client_id. Duplicate server ids keep the first node (TFS load order)."""
    data = otb.read_bytes()
    _, chunks = parse_otb_forest(data)
    mapping: dict[int, int] = {}
    client_owners: dict[int, list[int]] = defaultdict(list)
    for _, node in chunks:
        if node is None:
            continue
        for item in iter_item_nodes(node):
            server_id = None
            client_id = None
            for attr_type, payload in item.attrs:
                if attr_type == ITEM_ATTR_SERVERID:
                    server_id = attr_u16(payload)
                elif attr_type == ITEM_ATTR_CLIENTID:
                    client_id = attr_u16(payload)
            if not server_id:
                continue
            cid = client_id or 0
            if server_id not in mapping:
                mapping[server_id] = cid
            if cid:
                client_owners[cid].append(server_id)
    return mapping, client_owners


def lookup_client_id(mapping: dict[int, int], server_id: int) -> int | None:
    """None means leave unchanged (missing, 0, or already the client id with no row)."""
    if server_id == 0:
        return None
    client_id = mapping.get(server_id)
    if client_id is None or client_id == 0:
        return None
    return client_id


def convert_id_attrs(line: str, mapping: dict[int, int], stats: ConvertStats) -> str:
    def repl(match: re.Match[str]) -> str:
        prefix, quote, raw = match.group(1), match.group(2), match.group(3)
        server_id = int(raw)
        client_id = lookup_client_id(mapping, server_id)
        if client_id is None:
            if server_id != 0:
                stats.xml_unmapped += 1
            return match.group(0)
        if client_id == server_id:
            return match.group(0)
        stats.xml_replacements += 1
        return f"{prefix}{quote}{client_id}{quote}"

    return ID_ATTR_RE.sub(repl, line)


def convert_item_xml_values(line: str, mapping: dict[int, int], stats: ConvertStats) -> str:
    def repl(match: re.Match[str]) -> str:
        key = match.group(3).strip().lower()
        if key not in ITEM_ID_XML_KEYS:
            return match.group(0)
        server_id = int(match.group(6))
        client_id = lookup_client_id(mapping, server_id)
        if client_id is None:
            if server_id != 0:
                stats.xml_unmapped += 1
            return match.group(0)
        if client_id == server_id:
            return match.group(0)
        stats.xml_replacements += 1
        return (
            f"{match.group(1)}{match.group(2)}{match.group(3)}{match.group(2)}"
            f"{match.group(4)}{match.group(5)}{client_id}{match.group(5)}"
        )

    return ITEM_ATTR_VALUE_RE.sub(repl, line)


def convert_xml_text(text: str, mapping: dict[int, int], stats: ConvertStats, *, items_xml: bool) -> str:
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    for line in lines:
        line = convert_id_attrs(line, mapping, stats)
        if items_xml:
            line = convert_item_xml_values(line, mapping, stats)
        out.append(line)
    return "".join(out)


def rewrite_otb_node(node: OtbNode, mapping: dict[int, int], stats: ConvertStats) -> None:
    server_id = None
    client_id = None
    server_idx = None
    for i, (attr_type, payload) in enumerate(node.attrs):
        if attr_type == ITEM_ATTR_SERVERID:
            server_id = attr_u16(payload)
            server_idx = i
        elif attr_type == ITEM_ATTR_CLIENTID:
            client_id = attr_u16(payload)
    if server_idx is not None and server_id and client_id:
        new_id = lookup_client_id(mapping, server_id)
        if new_id is not None and new_id != server_id:
            node.attrs[server_idx] = (
                ITEM_ATTR_SERVERID,
                bytes((new_id & 0xFF, (new_id >> 8) & 0xFF)),
            )
            stats.otb_nodes_rewritten += 1
        else:
            stats.otb_nodes_unchanged += 1
    elif server_id:
        stats.otb_nodes_unchanged += 1
    for child in node.children:
        rewrite_otb_node(child, mapping, stats)


def convert_otb_bytes(data: bytes, mapping: dict[int, int], stats: ConvertStats) -> bytes:
    ident, chunks = parse_otb_forest(data)
    out = bytearray(ident)
    for leftover, node in chunks:
        if node is None:
            out.extend(leftover)
            continue
        rewrite_otb_node(node, mapping, stats)
        write_node(out, node)
    return bytes(out)


def _u16(payload: bytes) -> int | None:
    return attr_u16(payload)


def decode_otb_item(node: OtbNode) -> OtbItem | None:
    server_id = 0
    client_id = 0
    speed = None
    light_level = None
    light_color = None
    always_on_top_order = None
    ware_id = None
    for attr_type, payload in node.attrs:
        if attr_type == ITEM_ATTR_SERVERID:
            server_id = _u16(payload) or 0
        elif attr_type == ITEM_ATTR_CLIENTID:
            client_id = _u16(payload) or 0
        elif attr_type == ITEM_ATTR_SPEED:
            speed = _u16(payload)
        elif attr_type == ITEM_ATTR_LIGHT2 and len(payload) >= 4:
            light_level = payload[0] | (payload[1] << 8)
            light_color = payload[2] | (payload[3] << 8)
        elif attr_type == ITEM_ATTR_TOPORDER and payload:
            always_on_top_order = payload[0]
        elif attr_type == ITEM_ATTR_WAREID:
            ware_id = _u16(payload)
    if not server_id:
        return None
    return OtbItem(
        server_id=server_id,
        client_id=client_id,
        group=node.node_type,
        flags=node.flags,
        speed=speed,
        light_level=light_level,
        light_color=light_color,
        always_on_top_order=always_on_top_order,
        ware_id=ware_id,
    )


def parse_otb_version(root: OtbNode) -> tuple[int, int, int, str]:
    for attr_type, payload in root.attrs:
        if attr_type != ROOT_ATTR_VERSION or len(payload) < 12:
            continue
        major = int.from_bytes(payload[0:4], "little")
        minor = int.from_bytes(payload[4:8], "little")
        build = int.from_bytes(payload[8:12], "little")
        desc = payload[12:].split(b"\x00", 1)[0].decode("latin-1", "replace")
        return major, minor, build, desc
    return 0, 0, 0, ""


def load_otb_items(data: bytes) -> tuple[list[OtbItem], tuple[int, int, int, str]]:
    _, chunks = parse_otb_forest(data)
    items: list[OtbItem] = []
    version = (0, 0, 0, "")
    for _, node in chunks:
        if node is None:
            continue
        version = parse_otb_version(node)
        for child in node.children:
            decoded = decode_otb_item(child)
            if decoded is not None:
                items.append(decoded)
    return items, version


def remap_id(mapping: dict[int, int], server_id: int) -> int:
    client_id = lookup_client_id(mapping, server_id)
    return client_id if client_id is not None else server_id


def remap_xml_value(mapping: dict[int, int], key: str, value: str) -> str:
    if key not in ITEM_ID_XML_KEYS:
        return value
    try:
        server_id = int(value)
    except ValueError:
        return value
    return str(remap_id(mapping, server_id))


def apply_otb_to_catalog(item: CatalogItem, otb: OtbItem) -> None:
    """First OTB node for a client id wins (`clientIdToServerIdMap` emplace).

    Later duplicates only fill gaps: group 0 → real group (container coffin/tree),
    and missing speed/light/toporder/ware_id. Flags are never OR’d or overwritten.
    """
    if not item.otb_applied:
        item.group = otb.group
        item.flags = otb.flags
        item.speed = otb.speed
        item.light_level = otb.light_level
        item.light_color = otb.light_color
        item.always_on_top_order = otb.always_on_top_order
        item.ware_id = otb.ware_id
        item.otb_applied = True
        return
    if item.group == 0 and otb.group != 0:
        item.group = otb.group
    if item.speed is None and otb.speed is not None:
        item.speed = otb.speed
    if item.light_level is None and otb.light_level is not None:
        item.light_level = otb.light_level
    if item.light_color is None and otb.light_color is not None:
        item.light_color = otb.light_color
    if item.always_on_top_order is None and otb.always_on_top_order is not None:
        item.always_on_top_order = otb.always_on_top_order
    if not item.ware_id and otb.ware_id:
        item.ware_id = otb.ware_id


def parse_xml_items(xml_path: Path, mapping: dict[int, int]) -> dict[int, CatalogItem]:
    root = ET.fromstring(xml_path.read_text(encoding="latin-1"))
    by_id: dict[int, CatalogItem] = {}
    for el in root:
        if not isinstance(el.tag, str) or el.tag.lower() != "item":
            continue
        raw_id = el.get("id")
        raw_from = el.get("fromid") or el.get("fromId")
        raw_to = el.get("toid") or el.get("toId")
        ids: list[int] = []
        if raw_id is not None:
            ids.append(remap_id(mapping, int(raw_id)))
        elif raw_from is not None and raw_to is not None:
            start, end = int(raw_from), int(raw_to)
            if start > end:
                start, end = end, start
            ids.extend(remap_id(mapping, sid) for sid in range(start, end + 1))
        else:
            continue
        attributes: dict[str, str] = {}
        vocations: list[str] = []
        for child in el:
            if not isinstance(child.tag, str) or child.tag.lower() != "attribute":
                continue
            key = (child.get("key") or "").strip().lower()
            if not key:
                continue
            value = remap_xml_value(mapping, key, child.get("value") or "")
            if key == "vocation":
                token = value.strip()
                if token:
                    vocations.append(token)
            else:
                attributes[key] = value
            # Nested attrs — `Items::parseItemNode` stores `field.cycles` etc.
            # (`apply_nested_xml_attribute`). Needed for magic-field runtime.
            for inner in child:
                if not isinstance(inner.tag, str) or inner.tag.lower() != "attribute":
                    continue
                inner_key = (inner.get("key") or "").strip().lower()
                if not inner_key:
                    continue
                composite = f"{key}.{inner_key}"
                attributes[composite] = remap_xml_value(
                    mapping, composite, inner.get("value") or ""
                )
        name = el.get("name") or ""
        article = el.get("article") or ""
        plural = el.get("plural") or ""
        editorsuffix = el.get("editorsuffix") or ""
        for item_id in ids:
            existing = by_id.get(item_id)
            if existing is None:
                by_id[item_id] = CatalogItem(
                    id=item_id,
                    name=name,
                    article=article,
                    plural=plural,
                    editorsuffix=editorsuffix,
                    vocations=list(vocations),
                    attributes=dict(attributes),
                )
                continue
            # Same client id as an earlier server id: keep first name/suffix,
            # fill missing XML keys (`clientIdToServerIdMap` first-wins).
            if not existing.name and name:
                existing.name = name
            if not existing.article and article:
                existing.article = article
            if not existing.plural and plural:
                existing.plural = plural
            for voc in vocations:
                if voc not in existing.vocations:
                    existing.vocations.append(voc)
            for key, value in attributes.items():
                existing.attributes.setdefault(key, value)
    return by_id


def merge_xml_and_otb(
    xml_items: dict[int, CatalogItem],
    otb_items: list[OtbItem],
    mapping: dict[int, int],
) -> list[CatalogItem]:
    by_id = dict(xml_items)
    for otb in otb_items:
        unified = remap_id(mapping, otb.server_id)
        item = by_id.get(unified)
        if item is None:
            item = CatalogItem(id=unified)
            by_id[unified] = item
        apply_otb_to_catalog(item, otb)
    return [by_id[i] for i in sorted(by_id)]


def ron_str(value: str) -> str:
    escaped = (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )
    return f'"{escaped}"'


def ron_ident(value: str) -> str:
    token = value.strip()
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", token):
        return token[:1].upper() + token[1:]
    return ron_str(token)


def xml_bool(value: str) -> bool | None:
    token = value.strip().lower()
    if token in ("1", "true", "yes"):
        return True
    if token in ("0", "false", "no"):
        return False
    return None


def xml_int(value: str) -> int | None:
    try:
        return int(value.strip())
    except ValueError:
        return None


# Nested `<attribute key="field">` children → RON `Field(...)` members.
# Runtime keys: `field.cycles` / `field.initdamage` / `field.skippeaceful` (`magic_field.rs`).
# TFS also uses ticks/count/damage (`apply_nested_xml_attribute`).
FIELD_CHILD_FIELDS: dict[str, tuple[str, str]] = {
    "initdamage": ("init_damage", "int"),
    "cycles": ("cycles", "int"),
    "skippeaceful": ("skip_peaceful", "bool"),
    "ticks": ("ticks", "int"),
    "count": ("count", "int"),
    "damage": ("damage", "int"),
    "start": ("start", "int"),
}

# XML attribute key → (ron field, "int" | "bool" | "ident" | "str").
# Covers `items_xml_keys.rs` `KNOWN_XML_KEYS` plus aliases. `field`, `speed`, and
# `vocation` are formatted separately (nested Field, bonus_speed, vocations list).
XML_RON_FIELDS: tuple[tuple[str, str, str], ...] = (
    ("type", "type", "ident"),
    ("description", "description", "str"),
    ("elevation", "elevation", "int"),
    ("weight", "weight", "int"),
    ("slottype", "slot_type", "ident"),
    ("weapontype", "weapon_type", "ident"),
    ("ammotype", "ammo_type", "ident"),
    ("attack", "attack", "int"),
    ("defense", "defense", "int"),
    ("extradef", "extra_def", "int"),
    ("extradefense", "extra_def", "int"),
    ("armor", "armor", "int"),
    ("attackspeed", "attack_speed", "int"),
    ("range", "range", "int"),
    ("charges", "charges", "int"),
    ("hitchance", "hit_chance", "int"),
    ("maxhitchance", "max_hit_chance", "int"),
    ("decayto", "decay_to", "int"),
    ("duration", "duration", "int"),
    ("destroyto", "destroy_to", "int"),
    ("rotateto", "rotate_to", "int"),
    ("transformto", "transform_to", "int"),
    ("transformequipto", "transform_equip_to", "int"),
    ("transformdeequipto", "transform_deequip_to", "int"),
    ("writeonceitemid", "write_once_item_id", "int"),
    ("malesleeper", "male_sleeper", "int"),
    ("femalesleeper", "female_sleeper", "int"),
    ("maletransformto", "male_transform_to", "int"),
    ("femaletransformto", "female_transform_to", "int"),
    ("floorchange", "floor_change", "ident"),
    ("fluidsource", "fluid_source", "ident"),
    ("corpsetype", "corpse_type", "ident"),
    ("containersize", "container_size", "int"),
    ("maxtextlen", "max_text_len", "int"),
    ("partnerdirection", "partner_direction", "ident"),
    ("runespellname", "rune_spell_name", "str"),
    ("shoottype", "shoot_type", "ident"),
    ("effect", "effect", "ident"),
    ("ammospecialeffect", "ammo_special_effect", "int"),
    ("ammoeffectstrength", "ammo_effect_strength", "int"),
    ("throwspecialeffect", "throw_special_effect", "int"),
    ("throweffectstrength", "throw_effect_strength", "int"),
    ("blocking", "block_solid", "bool"),
    ("blockprojectile", "block_projectile", "bool"),
    ("blockpathfind", "block_path_find", "bool"),
    ("moveable", "moveable", "bool"),
    ("movable", "moveable", "bool"),
    ("allowpickupable", "allow_pickupable", "bool"),
    ("pickupable", "allow_pickupable", "bool"),
    ("allowdistread", "allow_dist_read", "bool"),
    ("readable", "readable", "bool"),
    ("writeable", "writeable", "bool"),
    ("forceuse", "force_use", "bool"),
    ("unlay", "unlay", "bool"),
    ("chest", "chest", "bool"),
    ("showcount", "show_count", "bool"),
    ("showcharges", "show_charges", "bool"),
    ("showduration", "show_duration", "bool"),
    ("showattributes", "show_attributes", "bool"),
    ("stopduration", "stop_duration", "bool"),
    ("replaceable", "replaceable", "bool"),
    ("walkstack", "walk_stack", "bool"),
    ("storeitem", "store_item", "bool"),
    ("forceserialize", "force_serialize", "bool"),
    ("forcesave", "force_serialize", "bool"),
    ("replacemagicfields", "replace_magic_fields", "bool"),
    ("specialfieldblockpath", "special_field_block_path", "bool"),
    ("leveldoor", "level_door", "int"),
    ("levelrequired", "level_required", "int"),
    ("magiclevelrequired", "magic_level_required", "int"),
    ("supply", "supply", "str"),
    ("poisondamagecycles", "poison_damage_cycles", "int"),
    ("invisible", "invisible", "bool"),
    ("manashield", "mana_shield", "bool"),
    ("healthgain", "health_gain", "int"),
    ("healthticks", "health_ticks", "int"),
    ("managain", "mana_gain", "int"),
    ("manaticks", "mana_ticks", "int"),
    ("skillfist", "skill_fist", "int"),
    ("skillclub", "skill_club", "int"),
    ("skillsword", "skill_sword", "int"),
    ("skillaxe", "skill_axe", "int"),
    ("skilldist", "skill_dist", "int"),
    ("skillfish", "skill_fish", "int"),
    ("skillshield", "skill_shield", "int"),
    ("criticalhitamount", "critical_hit_amount", "int"),
    ("criticalhitchance", "critical_hit_chance", "int"),
    ("lifeleechamount", "life_leech_amount", "int"),
    ("lifeleechchance", "life_leech_chance", "int"),
    ("manaleechamount", "mana_leech_amount", "int"),
    ("manaleechchance", "mana_leech_chance", "int"),
    ("maxhitpoints", "max_hitpoints", "int"),
    ("maxhitpointspercent", "max_hitpoints_percent", "int"),
    ("maxmanapoints", "max_manapoints", "int"),
    ("maxmanapointspercent", "max_manapoints_percent", "int"),
    ("magicpoints", "magic_points", "int"),
    ("magiclevelpoints", "magic_points", "int"),
    ("magicpointspercent", "magic_points_percent", "int"),
    ("absorbpercentall", "absorb_percent_all", "int"),
    ("absorbpercentallelements", "absorb_percent_all_elements", "int"),
    ("absorbpercentelements", "absorb_percent_elements", "int"),
    ("absorbpercentmagic", "absorb_percent_magic", "int"),
    ("absorbpercentenergy", "absorb_percent_energy", "int"),
    ("absorbpercentfire", "absorb_percent_fire", "int"),
    ("absorbpercentpoison", "absorb_percent_poison", "int"),
    ("absorbpercentearth", "absorb_percent_earth", "int"),
    ("absorbpercentice", "absorb_percent_ice", "int"),
    ("absorbpercentholy", "absorb_percent_holy", "int"),
    ("absorbpercentdeath", "absorb_percent_death", "int"),
    ("absorbpercentlifedrain", "absorb_percent_life_drain", "int"),
    ("absorbpercentmanadrain", "absorb_percent_mana_drain", "int"),
    ("absorbpercentdrown", "absorb_percent_drown", "int"),
    ("absorbpercentphysical", "absorb_percent_physical", "int"),
    ("absorbpercenthealing", "absorb_percent_healing", "int"),
    ("absorbpercentundefined", "absorb_percent_undefined", "int"),
    ("fieldabsorbpercentenergy", "field_absorb_percent_energy", "int"),
    ("fieldabsorbpercentfire", "field_absorb_percent_fire", "int"),
    ("fieldabsorbpercentpoison", "field_absorb_percent_poison", "int"),
    ("fieldabsorbpercentearth", "field_absorb_percent_earth", "int"),
    ("suppressdrunk", "suppress_drunk", "bool"),
    ("suppressenergy", "suppress_energy", "bool"),
    ("suppressfire", "suppress_fire", "bool"),
    ("suppresspoison", "suppress_poison", "bool"),
    ("suppressdrown", "suppress_drown", "bool"),
    ("suppressphysical", "suppress_physical", "bool"),
    ("suppressfreeze", "suppress_freeze", "bool"),
    ("suppressdazzle", "suppress_dazzle", "bool"),
    ("suppresscurse", "suppress_curse", "bool"),
    ("elementice", "element_ice", "int"),
    ("elementearth", "element_earth", "int"),
    ("elementfire", "element_fire", "int"),
    ("elementenergy", "element_energy", "int"),
    ("elementdeath", "element_death", "int"),
    ("elementholy", "element_holy", "int"),
)

# Keys formatted outside the generic XML_RON_FIELDS loop.
XML_SPECIAL_KEYS = frozenset({"field", "speed", "vocation"})


def item_flags(item: CatalogItem) -> list[str]:
    """OTB `itemflags_t` bits `ItemType` accessors read. XML overlays are separate fields."""
    flags: list[str] = []
    named_mask = 0
    for bit, name in FLAG_NAMES:
        named_mask |= 1 << bit
        if item.flags & (1 << bit):
            flags.append(name)
    leftover = item.flags & ~named_mask
    bit = 0
    rest = leftover
    while rest:
        if rest & 1:
            flags.append(f"Flag{bit}")
        rest >>= 1
        bit += 1
    return flags


def format_xml_field(kind: str, value: str) -> str | None:
    if kind == "int":
        parsed = xml_int(value)
        return None if parsed is None else str(parsed)
    if kind == "bool":
        parsed = xml_bool(value)
        if parsed is None:
            return None
        return "true" if parsed else "false"
    if kind == "ident":
        return ron_ident(value)
    return ron_str(value)


def format_field_ron(item: CatalogItem, consumed: set[str]) -> list[str]:
    kind = item.attributes.get("field")
    nested: list[tuple[str, str]] = []
    for key, value in item.attributes.items():
        if key.startswith("field."):
            nested.append((key[6:], value))
            consumed.add(key)
    if kind is not None:
        consumed.add("field")
    if kind is None and not nested:
        return []
    if kind is not None and not nested:
        rendered = format_xml_field("ident", kind)
        if rendered is None:
            return []
        return [f"\t\t\tfield: {rendered},"]
    lines = ["\t\t\tfield: Field("]
    if kind is not None:
        rendered = format_xml_field("ident", kind)
        if rendered is not None:
            lines.append(f"\t\t\t\tkind: {rendered},")
    for child_key, child_value in nested:
        spec = FIELD_CHILD_FIELDS.get(child_key)
        if spec is None:
            field_name = child_key.replace("-", "_")
            rendered = format_xml_field("int", child_value)
            if rendered is None:
                rendered = format_xml_field("bool", child_value)
            if rendered is None:
                rendered = ron_str(child_value)
        else:
            field_name, kind_name = spec
            rendered = format_xml_field(kind_name, child_value)
            if rendered is None:
                rendered = ron_str(child_value)
        lines.append(f"\t\t\t\t{field_name}: {rendered},")
    lines.append("\t\t\t),")
    return lines


def format_item_ron(item: CatalogItem) -> list[str]:
    lines = ["\t\tItem("]
    lines.append(f"\t\t\tid: {item.id},")
    if item.name:
        lines.append(f"\t\t\tname: {ron_str(item.name)},")
    if item.article:
        lines.append(f"\t\t\tarticle: {ron_str(item.article)},")
    if item.plural:
        lines.append(f"\t\t\tplural: {ron_str(item.plural)},")
    if item.editorsuffix:
        lines.append(f"\t\t\teditor_suffix: {ron_str(item.editorsuffix)},")

    group_name = GROUP_NAMES.get(item.group)
    if group_name:
        lines.append(f"\t\t\tgroup: {group_name},")

    flags = item_flags(item)
    if flags:
        lines.append(f"\t\t\tflags: [{', '.join(flags)}],")

    # `ItemType.speed` / `waypoints_raw` — emit for Ground even when 0 (void).
    if item.speed is not None:
        lines.append(f"\t\t\tspeed: {item.speed},")
    elif item.group == 1:
        lines.append("\t\t\tspeed: 0,")
    if item.light_level is not None:
        lines.append(f"\t\t\tlight_level: {item.light_level},")
    if item.light_color is not None:
        lines.append(f"\t\t\tlight_color: {item.light_color},")
    if item.always_on_top_order is not None:
        lines.append(f"\t\t\talways_on_top_order: {item.always_on_top_order},")
    if item.ware_id:
        lines.append(f"\t\t\tware_id: {item.ware_id},")

    consumed: set[str] = set(XML_SPECIAL_KEYS)
    xml_speed = item.attributes.get("speed")
    if xml_speed is not None:
        parsed = xml_int(xml_speed)
        if parsed is not None:
            lines.append(f"\t\t\tbonus_speed: {parsed},")

    if item.vocations:
        names = ", ".join(ron_str(v) for v in item.vocations)
        lines.append(f"\t\t\tvocations: [{names}],")

    lines.extend(format_field_ron(item, consumed))

    emitted_fields: set[str] = set()
    extra: list[tuple[str, str]] = []
    for key, ron_field, kind in XML_RON_FIELDS:
        if ron_field in emitted_fields:
            continue
        value = item.attributes.get(key)
        if value is None:
            continue
        consumed.add(key)
        rendered = format_xml_field(kind, value)
        if rendered is None:
            extra.append((key, value))
            continue
        if ron_field == "type" and group_name and rendered == group_name:
            emitted_fields.add(ron_field)
            continue
        emitted_fields.add(ron_field)
        lines.append(f"\t\t\t{ron_field}: {rendered},")

    for key, value in item.attributes.items():
        if key in consumed or key in {k for k, _, _ in XML_RON_FIELDS}:
            continue
        extra.append((key, value))

    if extra:
        lines.append("\t\t\textra: {")
        for key, value in extra:
            lines.append(f"\t\t\t\t{ron_str(key)}: {ron_str(value)},")
        lines.append("\t\t\t},")

    lines.append("\t\t),")
    return lines


def write_unified_ron(
    items: list[CatalogItem], version: tuple[int, int, int, str]
) -> str:
    major, minor, build, desc = version
    lines = [
        "// Unified item catalog for every clientVersion. `id` is the client id.",
        "// Replaces runtime items.otb + items.xml. OTB: group, flags (itemflags_t),",
        "// speed (ITEM_ATTR_SPEED), light_*, ware_id, always_on_top_order.",
        "// XML: parseItemNode + abilities as typed fields; magic fields use Field(kind, …).",
        "// `flags` are OTB bits only; XML overlays are separate (block_solid, force_use, …).",
        "// `bonus_speed` is items.xml equipment speed. Unknown keys in `extra`.",
        f"// Source {desc} major={major} minor={minor} build={build}.",
        "ItemCatalog(",
        "\titems: [",
    ]
    for index, item in enumerate(items):
        if index:
            lines.append("")
        lines.extend(format_item_ron(item))
    lines.append("\t],")
    lines.append(")")
    lines.append("")
    return "\n".join(lines)


def export_unified_ron(
    xml_path: Path | None,
    otb_path: Path,
    mapping: dict[int, int],
    dest: Path,
    stats: ConvertStats,
    *,
    dry_run: bool,
) -> None:
    otb_items, version = load_otb_items(otb_path.read_bytes())
    xml_items = parse_xml_items(xml_path, mapping) if xml_path is not None else {}
    catalog = merge_xml_and_otb(xml_items, otb_items, mapping)
    stats.ron_items = len(catalog)
    for err in validate_catalog(catalog, otb_items, mapping):
        print(f"warning: {err}", file=sys.stderr)
    text = write_unified_ron(catalog, version)
    if dry_run:
        return
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(text, encoding="utf-8")


def known_xml_keys_from_rust() -> set[str]:
    path = REPO / "crates" / "tfs-rust-content" / "src" / "items_xml_keys.rs"
    text = path.read_text(encoding="utf-8")
    start = text.index("const KNOWN_XML_KEYS")
    end = text.index("];", start)
    return set(re.findall(r'"([a-z0-9]+)"', text[start:end]))


def mapped_xml_keys() -> set[str]:
    return {key for key, _, _ in XML_RON_FIELDS} | set(XML_SPECIAL_KEYS)


def schema_gaps() -> list[str]:
    return sorted(known_xml_keys_from_rust() - mapped_xml_keys())


def extra_attribute_keys(item: CatalogItem) -> list[str]:
    mapped = mapped_xml_keys()
    extras: list[str] = []
    for key in item.attributes:
        if key.startswith("field."):
            continue
        if key in mapped:
            continue
        extras.append(key)
    return extras


def first_otb_by_client(
    otb_items: list[OtbItem], mapping: dict[int, int]
) -> dict[int, OtbItem]:
    first: dict[int, OtbItem] = {}
    for otb in otb_items:
        uid = remap_id(mapping, otb.server_id)
        first.setdefault(uid, otb)
    return first


def validate_catalog(
    catalog: list[CatalogItem],
    otb_items: list[OtbItem] | None = None,
    mapping: dict[int, int] | None = None,
) -> list[str]:
    errors: list[str] = []
    gaps = schema_gaps()
    if gaps:
        errors.append(f"RON schema missing parser keys: {gaps}")
    extras = sorted({key for item in catalog for key in extra_attribute_keys(item)})
    if extras:
        errors.append(f"untyped XML keys landed in extra: {extras}")
    nested_fields = [
        item
        for item in catalog
        if any(k.startswith("field.") for k in item.attributes)
    ]
    if not nested_fields:
        errors.append("no nested field.* attributes (magic-field cycles/initdamage missing)")
    else:
        sample = nested_fields[0]
        text = "\n".join(format_item_ron(sample))
        if "field: Field(" not in text:
            errors.append(f"id {sample.id} has field.* but RON is not Field(...)")
        if "field.cycles" in sample.attributes and "cycles:" not in text:
            errors.append(f"id {sample.id} dropped field.cycles")
        if "field.initdamage" in sample.attributes and "init_damage:" not in text:
            errors.append(f"id {sample.id} dropped field.initdamage")
    skip_item = next(
        (item for item in catalog if "field.skippeaceful" in item.attributes),
        None,
    )
    if skip_item is not None:
        text = "\n".join(format_item_ron(skip_item))
        if "skip_peaceful:" not in text:
            errors.append(f"id {skip_item.id} dropped field.skippeaceful")
    if otb_items is not None and mapping is not None:
        by_id = {item.id: item for item in catalog}
        for uid, otb in first_otb_by_client(otb_items, mapping).items():
            item = by_id.get(uid)
            if item is None:
                errors.append(f"first OTB server {otb.server_id} client {uid} missing from catalog")
                continue
            if item.flags != otb.flags:
                errors.append(
                    f"id {uid}: flags {item.flags} != first OTB server {otb.server_id} flags {otb.flags}"
                )
            if item.group != otb.group and not (otb.group == 0 and item.group != 0):
                errors.append(
                    f"id {uid}: group {item.group} != first OTB server {otb.server_id} group {otb.group}"
                )
    return errors


def is_items_xml(path: Path) -> bool:
    return path.name.lower() == "items.xml"


def convert_xml_file(
    src: Path,
    dst: Path,
    mapping: dict[int, int],
    stats: ConvertStats,
    *,
    dry_run: bool,
) -> None:
    # iso-8859-1 / latin-1: 1:1 with file bytes, preserves the items.xml header encoding.
    text = src.read_text(encoding="latin-1")
    converted = convert_xml_text(text, mapping, stats, items_xml=is_items_xml(src))
    if dry_run:
        return
    dst.parent.mkdir(parents=True, exist_ok=True)
    dst.write_text(converted, encoding="latin-1")


def convert_otb_file(
    src: Path,
    dst: Path,
    mapping: dict[int, int],
    stats: ConvertStats,
    *,
    dry_run: bool,
) -> None:
    data = src.read_bytes()
    converted = convert_otb_bytes(data, mapping, stats)
    if dry_run:
        return
    dst.parent.mkdir(parents=True, exist_ok=True)
    dst.write_bytes(converted)


def dump_map_csv(path: Path, mapping: dict[int, int], client_owners: dict[int, list[int]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["server_id", "client_id", "changed", "duplicate_client"])
        for server_id in sorted(mapping):
            client_id = mapping[server_id]
            writer.writerow(
                [
                    server_id,
                    client_id,
                    int(client_id not in (0, server_id)),
                    int(len(client_owners.get(client_id, [])) > 1),
                ]
            )


def report_collisions(client_owners: dict[int, list[int]], stats: ConvertStats) -> None:
    for client_id, servers in sorted(client_owners.items()):
        unique = sorted(set(servers))
        if len(unique) > 1:
            stats.collisions.append(
                f"client_id {client_id} is shared by server ids {unique} "
                "(catalog keeps first OTB / first XML; later rows fill missing group/attrs)"
            )


def default_output_dir(in_place: bool) -> Path | None:
    if in_place:
        return None
    return REPO / "data" / "items" / "clientid_output"


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--otb",
        type=Path,
        default=REPO / "data" / "items" / "items.otb",
        help="items.otb used as the ItemType map and conversion source",
    )
    parser.add_argument(
        "--xml",
        type=Path,
        default=REPO / "data" / "items" / "items.xml",
        help="items.xml to rewrite (omit conversion with --skip-xml)",
    )
    parser.add_argument(
        "--xml-dir",
        type=Path,
        help="extra directory of XML files (Lua xmlConverter input folder)",
    )
    parser.add_argument(
        "-o",
        "--output-dir",
        type=Path,
        help="directory for converted files (default: data/items/clientid_output)",
    )
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="overwrite sources (writes .bak copies first)",
    )
    parser.add_argument(
        "--skip-ron",
        action="store_true",
        help="do not write items.ron (unified typed catalog)",
    )
    parser.add_argument(
        "--skip-otb",
        action="store_true",
        help="do not rewrite items.otb (still load it for the map and RON)",
    )
    parser.add_argument(
        "--dump-map",
        type=Path,
        help="write server_id,client_id CSV",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="parse and count replacements without writing files",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="validate RON schema + nested field attrs against the XML parser keys",
    )
    args = parser.parse_args()

    if args.in_place and args.output_dir:
        print("error: use either --in-place or --output-dir, not both", file=sys.stderr)
        return 1

    if not args.otb.is_file():
        print(f"error: items.otb not found: {args.otb}", file=sys.stderr)
        return 1

    mapping, client_owners = build_server_to_client(args.otb)
    changed = sum(1 for sid, cid in mapping.items() if cid and cid != sid)
    print(
        f"loaded {len(mapping)} OTB server ids from {args.otb} "
        f"({changed} differ from client id, "
        f"{sum(1 for cid in mapping.values() if cid == 0)} have client_id 0)"
    )

    stats = ConvertStats()
    report_collisions(client_owners, stats)
    for line in stats.collisions:
        print(f"warning: {line}", file=sys.stderr)

    if args.dump_map:
        dump_map_csv(args.dump_map, mapping, client_owners)
        print(f"wrote map: {args.dump_map}")

    if args.self_test:
        xml_src = args.xml if args.xml.is_file() else None
        if xml_src is None:
            print(f"error: items.xml not found: {args.xml}", file=sys.stderr)
            return 1
        otb_items, _ = load_otb_items(args.otb.read_bytes())
        catalog = merge_xml_and_otb(parse_xml_items(xml_src, mapping), otb_items, mapping)
        errors = validate_catalog(catalog, otb_items, mapping)
        if errors:
            for err in errors:
                print(f"error: {err}", file=sys.stderr)
            return 1
        print(f"self-test ok ({len(catalog)} items)")

    output_dir = args.output_dir if args.output_dir is not None else default_output_dir(args.in_place)

    def dest_for(src: Path) -> Path:
        if args.in_place:
            return src
        assert output_dir is not None
        if args.xml_dir and src.is_relative_to(args.xml_dir):
            return output_dir / src.relative_to(args.xml_dir)
        return output_dir / src.name

    def maybe_backup(src: Path) -> None:
        if not args.in_place or args.dry_run:
            return
        backup = src.with_name(src.name + ".bak")
        if not backup.exists():
            shutil.copy2(src, backup)
            print(f"backup: {backup}")

    if not args.skip_ron:
        xml_src = args.xml if args.xml.is_file() else None
        if xml_src is None:
            print(f"warning: items.xml not found ({args.xml}); RON will be OTB-only", file=sys.stderr)
        if args.in_place:
            ron_dest = (xml_src or args.otb).with_name("items.ron")
        else:
            assert output_dir is not None
            ron_dest = output_dir / "items.ron"
        export_unified_ron(
            xml_src,
            args.otb,
            mapping,
            ron_dest,
            stats,
            dry_run=args.dry_run,
        )
        action = "would write" if args.dry_run else "wrote"
        print(f"{action} RON: {ron_dest} ({stats.ron_items} items)")

    xml_dir_count = 0
    if args.xml_dir:
        if not args.xml_dir.is_dir():
            print(f"error: --xml-dir not a directory: {args.xml_dir}", file=sys.stderr)
            return 1
        for src in sorted(args.xml_dir.rglob("*.xml")):
            if not src.is_file():
                continue
            maybe_backup(src)
            convert_xml_file(src, dest_for(src), mapping, stats, dry_run=args.dry_run)
            xml_dir_count += 1
        print(f"{'would convert' if args.dry_run else 'converted'} {xml_dir_count} file(s) under {args.xml_dir}")

    if not args.skip_otb:
        maybe_backup(args.otb)
        convert_otb_file(args.otb, dest_for(args.otb), mapping, stats, dry_run=args.dry_run)
        action = "would write" if args.dry_run else "wrote"
        print(f"{action} OTB: {dest_for(args.otb)}")

    dup_clients = sum(1 for ids in client_owners.values() if len(set(ids)) > 1)
    print(
        f"xml replacements={stats.xml_replacements} xml unmapped={stats.xml_unmapped} "
        f"otb rewritten={stats.otb_nodes_rewritten} otb unchanged={stats.otb_nodes_unchanged} "
        f"unified items={stats.ron_items} duplicate client ids={dup_clients}"
    )
    if args.dry_run:
        print("dry-run: no files written")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
