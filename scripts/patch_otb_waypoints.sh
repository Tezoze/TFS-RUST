#!/usr/bin/env bash
# Patch data/items/items.otb from objects.srv:
#   ITEM_ATTR_SPEED ← BANK Waypoints
#   FLAG_BLOCK_SOLID ← Unpass (when missing)
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run -p tfs-rust-content --bin patch-otb-waypoints -- "$@"
