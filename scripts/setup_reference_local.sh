#!/usr/bin/env bash
# One-time per-clone setup: keep reference/ out of git but visible to Cursor agents.
#
# - .git/info/exclude  → git never commits reference checkouts (local only)
# - .cursorignore      → tracked negation patterns so Grep/Read/indexing work
#
# C++ graph search: scripts/register_reference_graph.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXCLUDE="$ROOT/.git/info/exclude"
MARKER="# Local C++ reference trees — see reference/README.md"

if [[ ! -d "$ROOT/.git" ]]; then
  echo "error: not a git repo: $ROOT" >&2
  exit 1
fi

if grep -qF "$MARKER" "$EXCLUDE" 2>/dev/null; then
  echo "ok: .git/info/exclude already has reference patterns"
else
  cat >>"$EXCLUDE" <<'EOF'

#############
## TFS / OT — local-only (not in shared .gitignore)
#############
# Local C++ reference trees — see reference/README.md
reference/classic-772/
reference/cipsoft-772/
reference/tvp-772/
reference/archives/
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

if ! grep -qF '!reference/cipsoft-772/' "$ROOT/.cursorignore"; then
  echo "error: .cursorignore missing reference negation patterns — pull latest main" >&2
  exit 1
fi
echo "ok: .cursorignore re-includes reference/ for Cursor tools"

if command -v rg >/dev/null 2>&1; then
  if rg -l 'TShortway' "$ROOT/reference/cipsoft-772/tibia-game-master/src" >/dev/null 2>&1; then
    echo "ok: ripgrep can search reference/cipsoft-772/"
  else
    echo "note: reference/cipsoft-772/ not present or empty — clone checkouts first"
  fi
fi

echo
echo "Optional: register C++ trees with code-review-graph:"
echo "  scripts/register_reference_graph.sh"
