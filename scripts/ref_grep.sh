#!/usr/bin/env bash
# Text search in 772 C++ reference src/ trees.
#
# Use this instead of `rtk grep` on reference/ — RTK's -l is --max-len (not
# ripgrep's file-list), and bare grep fails on directories.
#
# Usage:
#   scripts/ref_grep.sh PATTERN [extra rg args...]
#   scripts/ref_grep.sh TShortway -C 2
#   scripts/ref_grep.sh 'IdleStimulus' --files-with-matches
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ $# -lt 1 ]]; then
  echo "usage: ref_grep.sh PATTERN [rg args...]" >&2
  exit 2
fi

PATTERN=$1
shift

RG_BIN="${RG:-}"
if [[ -z "$RG_BIN" ]]; then
  for candidate in \
    /usr/share/cursor/resources/app/node_modules/@vscode/ripgrep/bin/rg \
    "$(command -v rg 2>/dev/null || true)"; do
    if [[ -n "$candidate" && -x "$candidate" ]]; then
      RG_BIN=$candidate
      break
    fi
  done
fi

if [[ -z "$RG_BIN" ]]; then
  echo "error: ripgrep (rg) not found" >&2
  exit 1
fi

DIRS=()
for d in \
  "$ROOT/reference/cipsoft-772/tibia-game-master/src" \
  "$ROOT/reference/classic-772/tibia-game-master/src" \
  "$ROOT/reference/tvp-772/gameserver/src"; do
  [[ -d "$d" ]] && DIRS+=("$d")
done

if [[ ${#DIRS[@]} -eq 0 ]]; then
  echo "error: no reference src/ trees found — run scripts/setup_reference_local.sh" >&2
  exit 1
fi

exec "$RG_BIN" --no-ignore -n "$PATTERN" "${DIRS[@]}" "$@"
