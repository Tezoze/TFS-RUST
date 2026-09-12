#!/usr/bin/env bash
# Build Crypto++ and pugixml into $TFS_TVP_772_DIR/deps when distro packages
# are not installed (no sudo). Used by scripts/build_tvp.sh.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# shellcheck source=lib/reference_paths.sh
. "$ROOT/scripts/lib/reference_paths.sh"
reference_paths_init "$ROOT"

DEPS="${TVP772}/deps"
SRC="${DEPS}/src"
mkdir -p "$SRC" "$DEPS"

need_cryptopp=1
need_pugi=1
if [[ -f /usr/include/cryptopp/cryptlib.h ]] && { ldconfig -p 2>/dev/null | grep -q libcryptopp || [[ -f /usr/lib/libcryptopp.so ]]; }; then
  need_cryptopp=0
fi
if [[ -f /usr/include/pugixml.hpp ]] && { ldconfig -p 2>/dev/null | grep -q libpugixml || [[ -f /usr/lib/libpugixml.so ]]; }; then
  need_pugi=0
fi
if [[ -f "$DEPS/include/cryptopp/cryptlib.h" && -f "$DEPS/lib/libcryptopp.a" ]]; then
  need_cryptopp=0
fi
if [[ -f "$DEPS/include/pugixml.hpp" && -f "$DEPS/lib/libpugixml.a" ]]; then
  need_pugi=0
fi

if [[ "$need_cryptopp" -eq 0 && "$need_pugi" -eq 0 ]]; then
  echo "install_tvp_deps: already present (system or $DEPS)" >&2
  exit 0
fi

fetch() {
  local url="$1" dest="$2"
  if [[ -f "$dest" ]]; then
    return 0
  fi
  echo "install_tvp_deps: fetch $url" >&2
  curl -fsSL -o "$dest" "$url"
}

NPROC="$(nproc)"

if [[ "$need_cryptopp" -eq 1 ]]; then
  fetch "https://github.com/weidai11/cryptopp/archive/refs/tags/CRYPTOPP_8_9_0.tar.gz" "$SRC/cryptopp-8.9.0.tar.gz"
  rm -rf "$SRC/cryptopp-CRYPTOPP_8_9_0"
  tar -xzf "$SRC/cryptopp-8.9.0.tar.gz" -C "$SRC"
  echo "install_tvp_deps: building Crypto++ 8.9.0" >&2
  make -C "$SRC/cryptopp-CRYPTOPP_8_9_0" -j"$NPROC" libcryptopp.a
  make -C "$SRC/cryptopp-CRYPTOPP_8_9_0" install PREFIX="$DEPS"
fi

if [[ "$need_pugi" -eq 1 ]]; then
  fetch "https://github.com/zeux/pugixml/archive/refs/tags/v1.14.tar.gz" "$SRC/pugixml-1.14.tar.gz"
  rm -rf "$SRC/pugixml-1.14"
  tar -xzf "$SRC/pugixml-1.14.tar.gz" -C "$SRC"
  echo "install_tvp_deps: building pugixml 1.14" >&2
  cmake -S "$SRC/pugixml-1.14" -B "$SRC/pugixml-1.14/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$DEPS" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DBUILD_SHARED_LIBS=OFF
  cmake --build "$SRC/pugixml-1.14/build" -j"$NPROC"
  cmake --install "$SRC/pugixml-1.14/build"
fi

echo "install_tvp_deps: ok prefix=$DEPS" >&2
