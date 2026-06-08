#!/usr/bin/env python3
"""
Audit 772 BANK Waypoints (objects.srv) vs items.otb ITEM_ATTR_SPEED.

Terminology (do not conflate):
  - Waypoints: BANK attribute on ground item types (objects.srv). Used by
    TShortway::FillMap (pathfinding) and NotifyGo step delay (cract.cc) — same value.
  - Creature GetSpeed(): creature movement stat; divides tile Waypoints for step ms.
  - TFS ITEM_ATTR_SPEED / ItemType::speed: OTB field historically called "ground speed";
    for 772 parity we treat it as the Waypoints source when present.
  - items.xml attribute speed: equipment bonus (abilities.speed), NOT tile Waypoints.

Map conversion (SEC/OTBM) stores only ground item ids per tile; neither Waypoints nor
ITEM_ATTR_SPEED are per-tile map data.

Usage (from repo root):
  python3 scripts/audit_objects_srv_waypoints_vs_otb.py
  cargo test -p tfs-rust-content --test audit_objects_srv_waypoints -- --nocapture
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

    print("Running Rust audit (authoritative OTB loader)...\n")
    proc = subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "tfs-rust-content",
            "--test",
            "audit_objects_srv_waypoints",
            "--",
            "--nocapture",
        ],
        cwd=REPO,
        text=True,
    )
    return proc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
