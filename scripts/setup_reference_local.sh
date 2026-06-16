#!/usr/bin/env bash
# One-time per-clone setup: keep reference/ out of git but visible to Cursor agents.
#
# - .git/info/exclude       → git never commits reference checkouts (local only)
# - .cursorignore           → tracked hierarchical negation patterns (src/ only)
# - register_reference_graph.sh → code-review-graph for 772 C++ (when installed)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXCLUDE="$ROOT/.git/info/exclude"
MARKER="# Local C++ reference trees — see reference/README.md"

if [[ ! -d "$ROOT/.git" ]]; then
  echo "error: not a git repo: $ROOT" >&2
  exit 1
fi

# Migrate legacy `reference/foo/` exclude lines to `reference/foo/**` so Cursor
# negations in .cursorignore can re-include nested paths.
migrate_exclude_patterns() {
  local tmp
  tmp="$(mktemp)"
  sed \
    -e 's|^reference/classic-772/$|reference/classic-772/**|' \
    -e 's|^reference/cipsoft-772/$|reference/cipsoft-772/**|' \
    -e 's|^reference/tvp-772/$|reference/tvp-772/**|' \
    -e 's|^reference/archives/$|reference/archives/**|' \
    "$EXCLUDE" >"$tmp"
  mv "$tmp" "$EXCLUDE"
}

if grep -qF "$MARKER" "$EXCLUDE" 2>/dev/null; then
  migrate_exclude_patterns
  echo "ok: .git/info/exclude already has reference patterns (migrated to /** if needed)"
else
  cat >>"$EXCLUDE" <<'EOF'

#############
## TFS / OT — local-only (not in shared .gitignore)
#############
# Local C++ reference trees — see reference/README.md
reference/classic-772/**
reference/cipsoft-772/**
reference/tvp-772/**
reference/archives/**
/reference/classic-772/client/tibia.pem
/reference/cipsoft-772/client/tibia.pem
/reference/classic-772/client/Tibia772*.exe
/reference/cipsoft-772/client/Tibia772*.exe
EOF
  echo "added reference patterns to .git/info/exclude"
fi

if [[ ! -f "$ROOT/.cursorignore" ]]; then
  echo "error: missing tracked .cursorignore — pull latest main" >&2
  exit 1
fi

if ! grep -qF 'tibia-game-master/src/' "$ROOT/.cursorignore"; then
  echo "error: .cursorignore missing source-tree negations — pull latest main" >&2
  exit 1
fi
echo "ok: .cursorignore re-includes reference C++ src/ for Cursor tools"

if command -v rg >/dev/null 2>&1; then
  if rg -l 'TShortway' "$ROOT/reference/cipsoft-772/tibia-game-master/src" >/dev/null 2>&1; then
    echo "ok: ripgrep can search reference/cipsoft-772/"
  else
    echo "note: reference/cipsoft-772/ not present or empty — clone checkouts first"
  fi
fi

if [[ -x "$ROOT/scripts/ref_grep.sh" ]]; then
  if "$ROOT/scripts/ref_grep.sh" TShortway --files-with-matches >/dev/null 2>&1; then
    echo "ok: scripts/ref_grep.sh can search reference src/"
  else
    echo "note: scripts/ref_grep.sh found no matches (reference src may be absent)"
  fi
fi

bash "$ROOT/scripts/register_reference_graph.sh"
