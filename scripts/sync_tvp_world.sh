#!/usr/bin/env bash
# Point TVP at the same OTBM bytes as Rust (`data/world/forgotten.otbm`).
#
# TVP still loads `data/world/map.otbm` (`config.lua` mapName = "map"). Both
# files already name `spawns.xml` / `houses.xml` internally. Houses already
# match; spawn XML stays TVP `tvpspawn` (native loader). `enableMapDataFiles`
# must stay false or `gamedata/map.tvpm` overlays a different tile set.
#
# Usage from repo root: ./scripts/sync_tvp_world.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=lib/reference_paths.sh
. "$ROOT/scripts/lib/reference_paths.sh"
reference_paths_init "$ROOT"

SRC="$ROOT/data/world/forgotten.otbm"
DST="${TVP772}/gameserver/data/world/map.otbm"
CFG="${TVP772}/gameserver/config.lua"

if [[ ! -f "$SRC" ]]; then
  echo "sync_tvp_world: missing $SRC" >&2
  exit 1
fi
if [[ ! -d "$(dirname "$DST")" ]]; then
  echo "sync_tvp_world: missing TVP world dir $(dirname "$DST")" >&2
  exit 1
fi

same=0
if [[ -f "$DST" ]] && cmp -s "$SRC" "$DST"; then
  same=1
fi

if [[ "$same" -eq 0 ]]; then
  if ln -f "$SRC" "$DST" 2>/dev/null; then
    echo "sync_tvp_world: hardlink $SRC → $DST" >&2
  else
    cp -f "$SRC" "$DST"
    echo "sync_tvp_world: copied $SRC → $DST" >&2
  fi
else
  echo "sync_tvp_world: OTBM already identical" >&2
fi

if [[ -f "$CFG" ]]; then
  if grep -qE '^enableMapDataFiles[[:space:]]*=' "$CFG"; then
    sed -i -E 's|^enableMapDataFiles[[:space:]]*=.*|enableMapDataFiles = false|' "$CFG"
  else
    printf '\nenableMapDataFiles = false\n' >> "$CFG"
  fi
fi

echo "sync_tvp_world: $(sha256sum "$SRC" "$DST")" >&2
