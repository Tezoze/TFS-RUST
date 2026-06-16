#!/usr/bin/env bash
# Register local 772 C++ reference trees with code-review-graph.
#
# CRG discovers files via `git ls-files`, so local reference trees (excluded via
# .git/info/exclude) are invisible to the main TFS_RUST graph. Each reference
# checkout gets a nested local-only git repo (src/ only) plus CRG registration.
#
# Called automatically by scripts/setup_reference_local.sh when code-review-graph
# is installed. Safe to re-run (idempotent).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v code-review-graph >/dev/null 2>&1; then
  echo "skip: code-review-graph not installed" >&2
  exit 0
fi

is_registered() {
  local alias="$1"
  code-review-graph repos 2>/dev/null | grep -qF "($alias)"
}

register_tree() {
  local path="$1"
  local alias="$2"

  if [[ ! -d "$path/src" ]]; then
    echo "skip: $path (no src/)"
    return 0
  fi

  pushd "$path" >/dev/null

  if [[ ! -d .git ]]; then
    git init -q
    cat >.gitignore <<'EOF'
build/
.code-review-graph/
EOF
    git add src/ .gitignore
    git commit -qm "Local CRG index only — never push"
    echo "initialized local git in $path (src/ only)"
  else
  # Keep nested index scoped to src/ — drop build artifacts if present.
    cat >.gitignore <<'EOF'
build/
.code-review-graph/
EOF
    git add .gitignore src/ 2>/dev/null || true
    if ! git diff --cached --quiet 2>/dev/null; then
      git commit -qm "src update for CRG" 2>/dev/null || true
    fi
  fi

  if is_registered "$alias"; then
    echo "ok: $alias already registered — updating graph"
    code-review-graph update --base HEAD 2>/dev/null || code-review-graph build
  else
    code-review-graph register . --alias "$alias"
    code-review-graph build
    echo "registered and built: $alias ($path)"
  fi

  popd >/dev/null
}

register_tree "$ROOT/reference/cipsoft-772/tibia-game-master" ref-772-mechanics
register_tree "$ROOT/reference/classic-772/tibia-game-master" ref-772-mechanics-classic
register_tree "$ROOT/reference/tvp-772/gameserver" ref-772-wire

echo
echo "Done. Agents: use cross_repo_search_tool for 772 C++ discovery (ref-772-mechanics, ref-772-wire)."
echo "Shell text search fallback: scripts/ref_grep.sh PATTERN"
