#!/usr/bin/env bash
# Flamegraph the Tier 2 scale_sweep binary under load.
#
# Requires:
#   pacman -S perf
#   cargo install flamegraph
#
# Usage from repo root:
#   ./scripts/profile_sim.sh --axis monsters --points 200 --beats 200
# Extra args are forwarded to scale_sweep. Output:
#   results/flamegraph_<axis>_<n>.svg

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v perf >/dev/null 2>&1; then
  echo "profile_sim: perf not found. Install with: pacman -S perf" >&2
  exit 1
fi

if ! command -v cargo-flamegraph >/dev/null 2>&1; then
  echo "profile_sim: cargo-flamegraph not found. Install with: cargo install flamegraph" >&2
  exit 1
fi

AXIS="monsters"
N="200"
PASSTHRU=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --axis)
      AXIS="${2:-monsters}"
      PASSTHRU+=("$1" "$2")
      shift 2
      ;;
    --points)
      N="${2%%,*}"
      PASSTHRU+=("$1" "$2")
      shift 2
      ;;
    *)
      PASSTHRU+=("$1")
      shift
      ;;
  esac
done

if [[ ${#PASSTHRU[@]} -eq 0 ]] || [[ ! " ${PASSTHRU[*]} " =~ " --axis " ]]; then
  PASSTHRU=(--axis "$AXIS" --points "$N" --beats 200 --warmup 20 "${PASSTHRU[@]+"${PASSTHRU[@]}"}")
fi

OUT_DIR="${ROOT}/results"
mkdir -p "$OUT_DIR"
OUT_SVG="${OUT_DIR}/flamegraph_${AXIS}_${N}.svg"

export CARGO_PROFILE_RELEASE_DEBUG=true
echo "profile_sim: cargo flamegraph -p tfs-rust-sim --bin scale_sweep -- ${PASSTHRU[*]}" >&2
echo "profile_sim: writing ${OUT_SVG}" >&2

# flamegraph writes cargo-flamegraph.perf / flamegraph.svg in cwd unless -o is set.
cargo flamegraph -p tfs-rust-sim --bin scale_sweep -o "$OUT_SVG" -- "${PASSTHRU[@]}"
