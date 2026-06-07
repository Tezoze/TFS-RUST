#!/usr/bin/env python3
"""
Patch Tibia 7.72 Tibia.exe on disk (Linux-friendly — no running client needed).

Replaces CipSoft login hostnames, RSA modulus (decimal string), optional login port.

Usage:
  scripts/patch_tibia_client.py /path/to/Tibia.exe
  scripts/patch_tibia_client.py /path/to/Tibia.exe --output /path/to/Tibia.local.exe
  scripts/patch_tibia_client.py --restore /path/to/Tibia.exe.bak

Default host/port/key from env or repo tibia.pem:
  TIBIA_LOGIN_HOST=127.0.0.1  TIBIA_LOGIN_PORT=7171  TIBIA_RSA_PEM=reference/cipsoft-772/client/tibia.pem
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_RSA_PEM = ROOT / "reference" / "cipsoft-772" / "client" / "tibia.pem"

# CipSoft 7.72 login host strings (OTLand / leaked client)
DEFAULT_HOST_STRINGS = (
    b"tibia2.cipsoft.com",
    b"tibia1.cipsoft.com",
    b"server2.tibia.com",
    b"server.tibia.com",
)

RSA_RE = re.compile(rb"\d{200,350}")


def die(msg: str) -> None:
    print(f"error: {msg}", file=sys.stderr)
    sys.exit(1)


def padded_host(host: str, width: int) -> bytes:
    raw = host.encode("ascii")
    if len(raw) >= width:
        if len(raw) > width:
            die(f"host {host!r} longer than slot ({width} bytes)")
        return raw
    return raw + b"\x00" * (width - len(raw))


def rsa_modulus_decimal(pem_path: Path) -> str:
    out = subprocess.check_output(
        ["openssl", "rsa", "-in", str(pem_path), "-modulus", "-noout"],
        text=True,
    ).strip()
    hexmod = out.split("=", 1)[1]
    return str(int(hexmod, 16))


def find_rsa_span(data: bytes) -> tuple[int, int, str]:
    m = RSA_RE.search(data)
    if not m:
        die("RSA decimal string not found in executable")
    old = m.group().decode("ascii")
    return m.start(), m.end(), old


def patch_exe(
    data: bytearray,
    host: str,
    port: int,
    pem: Path,
    patch_port: bool,
) -> dict[str, int]:
    stats = {"hosts": 0, "rsa": 0, "ports": 0}
    new_rsa = rsa_modulus_decimal(pem).encode("ascii")

    rsa_start, rsa_end, old_rsa = find_rsa_span(bytes(data))
    if len(new_rsa) != len(old_rsa):
        die(
            f"RSA length mismatch: pem={len(new_rsa)} exe={len(old_rsa)} "
            "(need same-length modulus for in-place patch)"
        )
    data[rsa_start:rsa_end] = new_rsa
    stats["rsa"] = 1

    for old_host in DEFAULT_HOST_STRINGS:
        repl = padded_host(host, len(old_host))
        count = 0
        start = 0
        while True:
            idx = data.find(old_host, start)
            if idx < 0:
                break
            data[idx : idx + len(old_host)] = repl
            count += 1
            start = idx + len(old_host)
        stats["hosts"] += count

    if patch_port and port != 7171:
        if port <= 0 or port > 0xFFFF:
            die(f"invalid port: {port}")
        port_le = port.to_bytes(2, "little")
        # Conservative: only patch LE port halfwords preceded by 0x0078 (772 login table pattern).
        needle = b"\x78\x00" + port_le
        # If already 7171, search for 03 1c; user wants different port — scan 78 00 ?? ??
        patched = 0
        i = 0
        while i < len(data) - 3:
            if data[i] == 0x78 and data[i + 1] == 0x00:
                data[i + 2 : i + 4] = port_le
                patched += 1
                i += 4
                continue
            i += 1
        stats["ports"] = patched

    return stats


def main() -> None:
    ap = argparse.ArgumentParser(description="Patch Tibia 7.72 client for local login")
    ap.add_argument("exe", type=Path, nargs="?", help="Path to Tibia.exe")
    ap.add_argument(
        "-o",
        "--output",
        type=Path,
        help="Write patched copy (default: patch in place after .bak backup)",
    )
    ap.add_argument("--host", default=None, help="Login host (default: 127.0.0.1)")
    ap.add_argument("--port", type=int, default=None, help="Login port (default: 7171)")
    ap.add_argument("--pem", type=Path, default=None, help="RSA private key PEM")
    ap.add_argument(
        "--patch-port",
        action="store_true",
        help="Patch port bytes (only needed if not 7171)",
    )
    ap.add_argument(
        "--restore",
        type=Path,
        metavar="BACKUP",
        help="Restore Tibia.exe from .bak backup",
    )
    args = ap.parse_args()

    if args.restore:
        bak = args.restore
        exe = bak.with_suffix("") if bak.suffix == ".bak" else Path(str(bak).removesuffix(".bak"))
        if not bak.is_file():
            die(f"backup not found: {bak}")
        shutil.copy2(bak, exe)
        print(f"restored: {exe} <- {bak}")
        return

    if not args.exe:
        ap.print_help()
        die("pass path to Tibia.exe")

    exe = args.exe.expanduser().resolve()
    if not exe.is_file():
        die(f"not found: {exe}")

    host = args.host or __import__("os").environ.get("TIBIA_LOGIN_HOST", "127.0.0.1")
    port = args.port or int(__import__("os").environ.get("TIBIA_LOGIN_PORT", "7171"))
    pem = args.pem or Path(__import__("os").environ.get("TIBIA_RSA_PEM", str(DEFAULT_RSA_PEM if DEFAULT_RSA_PEM.is_file() else ROOT / "tibia.pem")))
    pem = pem.expanduser().resolve()
    if not pem.is_file():
        die(f"PEM not found: {pem}")

    raw = bytearray(exe.read_bytes())
    rsa_start, rsa_end, old_rsa = find_rsa_span(bytes(raw))
    print(f"input:  {exe}")
    print(f"host:   {host}")
    print(f"port:   {port}")
    print(f"rsa:    {pem}")
    print(f"old rsa prefix: {old_rsa[:24]}...")

    stats = patch_exe(raw, host, port, pem, args.patch_port)

    out = args.output.expanduser().resolve() if args.output else exe
    if out == exe:
        bak = exe.with_name(exe.name + ".bak")
        if not bak.exists():
            shutil.copy2(exe, bak)
            print(f"backup: {bak}")
        out.write_bytes(raw)
    else:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(raw)

    print(f"output: {out}")
    print(f"patched: hosts={stats['hosts']} rsa={stats['rsa']} ports={stats['ports']}")
    print("launch patched Tibia.exe — no wine ipchanger needed")


if __name__ == "__main__":
    main()
