#!/usr/bin/env bash
# Build TVP 7.72 gameserver (Release + IPO when CMake reports it supported).
#
# Usage from repo root:
#   ./scripts/build_tvp.sh
#   ./scripts/build_tvp.sh --o3          # last-wins -O3 (documents TVP's unconditional -Og)
#
# Binary: $TVP772/gameserver/build/tfs
# C++: reference/tvp-772/gameserver/CMakeLists.txt (CMAKE_BUILD_TYPE=Release, IPO 80-89).
# Asymmetric flag: non-Win32 add_compile_options includes -Og even under Release
# (CMakeLists.txt:16-18). Default build leaves that as TVP wrote it. --o3 appends
# -O3 via CMAKE_CXX_FLAGS_RELEASE so a sensitivity run can measure the delta.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=lib/reference_paths.sh
. "$ROOT/scripts/lib/reference_paths.sh"
reference_paths_init "$ROOT"

FORCE_O3=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --o3) FORCE_O3=1; shift ;;
    --help|-h)
      cat <<'EOF'
Build TVP 7.72 gameserver (Release + IPO when supported).

Usage:
  ./scripts/build_tvp.sh
  ./scripts/build_tvp.sh --o3

--o3 rewrites generated -Og to -O3 (TVP CMakeLists adds -Og even in Release).
EOF
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

# GCC LTO writes huge temps; a 16G /tmp tmpfs fills and the link dies.
# Keep TMPDIR on the workspace disk (override with TFS_TVP_TMPDIR).
export TMPDIR="${TFS_TVP_TMPDIR:-$ROOT/.tmp/tvp-build}"
mkdir -p "$TMPDIR"
echo "build_tvp: TMPDIR=$TMPDIR" >&2

echo "build_tvp: ensuring Crypto++ / pugixml" >&2
"$ROOT/scripts/install_tvp_deps.sh"

SRC="${TVP772}/gameserver"
if [[ ! -f "$SRC/CMakeLists.txt" ]]; then
  echo "build_tvp: TVP tree missing at $SRC" >&2
  echo "  expected reference/tvp-772/gameserver (or set TFS_TVP_772_DIR)" >&2
  exit 1
fi

DEPS="${TVP772}/deps"
BUILD="$SRC/build"
mkdir -p "$BUILD"
cd "$BUILD"

CMAKE_ARGS=(-DCMAKE_BUILD_TYPE=Release -DSKIP_GIT=ON)
if [[ -d "$DEPS/include" ]]; then
  CMAKE_ARGS+=(
    -DCMAKE_PREFIX_PATH="$DEPS"
    -DCrypto++_INCLUDE_DIR="$DEPS/include"
    -DCrypto++_LIBRARIES="$DEPS/lib/libcryptopp.a"
    -DPUGIXML_INCLUDE_DIR="$DEPS/include"
    -DPUGIXML_LIBRARIES="$DEPS/lib/libpugixml.a"
  )
fi
if [[ "$FORCE_O3" -eq 1 ]]; then
  # Last -O* on the gcc line wins; TVP still injects -Og via add_compile_options.
  # Putting -O3 at the end of CMAKE_CXX_FLAGS_RELEASE is not enough (COMPILE_OPTIONS
  # come after). Rewrite generated flags after configure.
  CMAKE_ARGS+=(-DCMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG")
fi

echo "build_tvp: cmake ${CMAKE_ARGS[*]} $SRC" >&2
cmake "${CMAKE_ARGS[@]}" "$SRC"
if [[ "$FORCE_O3" -eq 1 ]]; then
  echo "build_tvp: rewriting -Og → -O3 in generated flags" >&2
  find "$BUILD" \( -name 'flags.make' -o -name 'build.ninja' \) -print0 \
    | xargs -0 -r sed -i 's/-Og/-O3/g'
fi
cmake --build . -j"$(nproc)"

BIN="$BUILD/tfs"
if [[ ! -x "$BIN" ]]; then
  echo "build_tvp: expected executable $BIN" >&2
  exit 1
fi
echo "build_tvp: ok $BIN" >&2
