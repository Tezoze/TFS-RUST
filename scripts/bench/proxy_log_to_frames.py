#!/usr/bin/env python3
"""Convert packet-proxy text hex logs into one frame per dump.

Packet-proxy lines (tools/packet-proxy/src/logger.rs):
  [{ts}] […] raw N bytes …
    hex: aa bb cc …

Wire-diff loadgen output against **raw / first-packet** dumps, not the
decrypted path — connection.rs currently decrypts with ProtocolCaps V1098
(Adler / cipher offset differ from 772).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def parse_frames(text: str) -> list[dict]:
    frames: list[dict] = []
    header: str | None = None
    chunks: list[str] = []

    def flush() -> None:
        nonlocal header, chunks
        if header is None and not chunks:
            return
        hex_bytes: list[str] = []
        for line in chunks:
            part = line.strip()
            if part.startswith("hex:"):
                part = part[4:].strip()
            hex_bytes.extend(b for b in part.split() if b)
        raw = bytes(int(b, 16) for b in hex_bytes)
        frames.append(
            {
                "header": header or "",
                "n": len(raw),
                "hex": raw.hex(),
            }
        )
        header = None
        chunks = []

    for line in text.splitlines():
        if line.startswith("  hex:"):
            chunks.append(line)
            continue
        if line.startswith("["):
            flush()
            header = line.strip()
            continue
        if line.strip().startswith("hex:") and header is not None:
            chunks.append(line)
            continue
        if header is not None and chunks:
            flush()

    flush()
    return frames


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("log", type=Path, help="packet-proxy text log")
    ap.add_argument("--out", type=Path, default=None, help="JSONL output (default stdout)")
    args = ap.parse_args()
    frames = parse_frames(args.log.read_text(encoding="utf-8", errors="replace"))
    out_fp = args.out.open("w", encoding="utf-8") if args.out else sys.stdout
    try:
        for fr in frames:
            out_fp.write(json.dumps(fr) + "\n")
    finally:
        if args.out:
            out_fp.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
