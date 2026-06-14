#!/usr/bin/env bash
# Register local 772 C++ reference trees with code-review-graph.
#
# CRG discovers files via `git ls-files`, so local reference trees (excluded via
# .git/info/exclude) are invisible to the main TFS_RUST graph. Each reference
# local-only git repo (never pushed) plus a CRG registration.
#
# C++ reference: code-review-graph uses git ls-files — see reference/README.md
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

register_tree() {
  local path="$1"
  local alias="$2"

  if [[ ! -d "$path" ]]; then
    echo "skip: $path (not present)"
    return 0
  fi

  pushd "$path" >/dev/null

  if [[ ! -d .git ]]; then
    git init -q
    git add -A
    git commit -qm "Local CRG index only — never push"
    echo "initialized local git in $path"
  fi

  code-review-graph register . --alias "$alias"
  code-review-graph build
  echo "registered and built: $alias ($path)"

  popd >/dev/null
}

register_tree "$ROOT/reference/cipsoft-772/tibia-game-master" ref-772-mechanics
register_tree "$ROOT/reference/classic-772/tibia-game-master" ref-772-mechanics-classic
register_tree "$ROOT/reference/tvp-772/gameserver" ref-772-wire

echo
echo "Done. Agents: use cross_repo_search_tool for 772 C++ reference lookups."
echo "Main-repo graph tools still cover Rust/crates only."
