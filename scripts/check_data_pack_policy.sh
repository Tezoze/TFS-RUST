#!/usr/bin/env bash
# Fail closed: no loot-generator Lua call sites; no script-registry XML outside content data.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
data="$root/data"
fail=0

# Stub definition in container.lua is the only allowed hit.
loot_hits="$(grep -RInE --include='*.lua' 'createLootItem|getLootRandom|getLossPercent' "$data" || true)"
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  case "$line" in
    *lib/core/container.lua:*createLootItem*)
      continue
      ;;
    *)
      echo "unexpected loot-generator Lua: $line" >&2
      fail=1
      ;;
  esac
done <<< "$loot_hits"

allowed_xml() {
  local rel="$1"
  [[ "$rel" == globalevents/globalevents.xml ]] && return 0
  [[ "$rel" == items/* ]] && return 0
  [[ "$rel" == XML/* ]] && return 0
  [[ "$rel" == raids/* ]] && return 0
  [[ "$rel" == world/* ]] && return 0
  [[ "$rel" == monster/monsters/* ]] && return 0
  [[ "$rel" == npc/archive/* ]] && return 0
  return 1
}

while IFS= read -r path; do
  rel="${path#"$data"/}"
  if ! allowed_xml "$rel"; then
    echo "script-registry XML outside content allowlist: $rel" >&2
    fail=1
  fi
done < <(find "$data" -type f -name '*.xml' | sort)

exit "$fail"
