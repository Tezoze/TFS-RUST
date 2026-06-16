#!/usr/bin/env python3
"""
Print 772 objects.srv ↔ OTB flag correlation stats.

Wrapper around the Rust audit test (authoritative OTB + items.xml loader).

Usage (from repo root):
  python3 scripts/audit_otb_objects_srv_flags.py
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()

    print("Running Rust flag correlation audit...\n")
    proc = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "tfs-rust-content",
            "audit_objects_srv_flag_correlation",
            "--",
            "--nocapture",
        ],
        cwd=REPO,
        text=True,
    )
    return proc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
