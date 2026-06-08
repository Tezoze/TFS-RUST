#!/usr/bin/env bash
# Patch data/items/items.otb ITEM_ATTR_SPEED from CipSoft objects.srv Waypoints.
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run -p tfs-rust-content --bin patch-otb-waypoints -- "$@"
