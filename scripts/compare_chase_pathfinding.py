#!/usr/bin/env python3
"""
Compare CipSoft 7.72 TShortway chase pathing vs TFS-RUST reverse pathfinder.

Faithful port of `tibia-game-master/src/cract.cc` (`TShortway::FillMap`, `Expand`, `Calculate`).
Rust side: `cargo run -p tfs-rust-core --bin path_compare -- <scenario.txt>`

Usage:
  python scripts/compare_chase_pathfinding.py
  python scripts/compare_chase_pathfinding.py --scenario /path/to/scenario.txt
  python scripts/compare_chase_pathfinding.py --build-rust
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Tuple

INT_MAX = 2_147_483_647

# CipSoft monster viewport — `cract.cc` `TShortway` (players use 7).
DEFAULT_VISIBLE = 10
DEFAULT_MAX_STEPS = 3  # `ToDoGo(..., false, 3)` melee chase — `crnonpl.cc` ~2729

# (dx, dy) neighbor order — `cract.cc:141-145`
NEIGHBOR_OFFSETS: Tuple[Tuple[int, int], ...] = (
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
)

DIR_NAMES = {
    (0, -1): "N",
    (0, 1): "S",
    (-1, 0): "W",
    (1, 0): "E",
    (-1, -1): "NW",
    (1, -1): "SW",
    (-1, 1): "NE",
    (1, 1): "SE",
}


@dataclass
class Scenario:
    """Shared chase test grid."""

    name: str
    start: Tuple[int, int]
    target: Tuple[int, int]
    visible: int = DEFAULT_VISIBLE
    max_steps: int = DEFAULT_MAX_STEPS
    default_wp: int = 150
    blocked: set[Tuple[int, int]] = field(default_factory=set)
    waypoints: Dict[Tuple[int, int], int] = field(default_factory=dict)

    def wp_at(self, x: int, y: int) -> int:
        if (x, y) in self.blocked:
            return -1
        return self.waypoints.get((x, y), self.default_wp)


class TShortwayPoint:
    __slots__ = ("x", "y", "waypoints", "waylength", "heuristic", "predecessor", "next_to_expand")

    def __init__(self, x: int, y: int, waypoints: int = -1) -> None:
        self.x = x
        self.y = y
        self.waypoints = waypoints
        self.waylength = INT_MAX
        self.heuristic = INT_MAX
        self.predecessor: Optional[TShortwayPoint] = None
        self.next_to_expand: Optional[TShortwayPoint] = None


class TShortway:
    """CipSoft `TShortway` — `cract.cc:29-262`."""

    def __init__(self, scenario: Scenario) -> None:
        self.scenario = scenario
        self.start_x, self.start_y = scenario.start
        self.visible_x = scenario.visible
        self.visible_y = scenario.visible
        self.min_waypoints = 1000
        self.first_to_expand: Optional[TShortwayPoint] = None
        self.grid: Dict[Tuple[int, int], TShortwayPoint] = {}
        self._fill_map()

    def _fill_map(self) -> None:
        s = self.scenario
        # Matrix spans `-(Visible+1)..+(Visible+1)`; FillMap only sets `±Visible` (`cract.cc:79-114`).
        for rx in range(-(self.visible_x + 1), self.visible_x + 2):
            for ry in range(-(self.visible_y + 1), self.visible_y + 2):
                self.grid[(rx, ry)] = TShortwayPoint(rx, ry, -1)

        for rx in range(-self.visible_x, self.visible_x + 1):
            for ry in range(-self.visible_y, self.visible_y + 1):
                ax = self.start_x + rx
                ay = self.start_y + ry
                wp = s.wp_at(ax, ay)
                if wp > 0 and wp < self.min_waypoints:
                    self.min_waypoints = wp
                self.grid[(rx, ry)].waypoints = wp if wp > 0 else -1

    def _clear_map(self) -> None:
        for rx in range(-self.visible_x, self.visible_x + 1):
            for ry in range(-self.visible_y, self.visible_y + 1):
                node = self.grid[(rx, ry)]
                node.waylength = INT_MAX
                node.heuristic = INT_MAX
                node.predecessor = None
                node.next_to_expand = None

    def _at(self, x: int, y: int) -> TShortwayPoint:
        try:
            return self.grid[(x, y)]
        except KeyError as e:
            raise KeyError(f"out of TShortway matrix at ({x}, {y})") from e

    def _remove_from_expand_list(self, neighbor: TShortwayPoint) -> None:
        prev: Optional[TShortwayPoint] = None
        cur = self.first_to_expand
        while cur is not None and cur is not neighbor:
            prev = cur
            cur = cur.next_to_expand
        if cur is not neighbor:
            return
        if prev is None:
            self.first_to_expand = neighbor.next_to_expand
        else:
            prev.next_to_expand = neighbor.next_to_expand

    def _insert_expand_list(self, neighbor: TShortwayPoint) -> None:
        prev: Optional[TShortwayPoint] = None
        cur = self.first_to_expand
        while cur is not None and cur.heuristic < neighbor.heuristic:
            prev = cur
            cur = cur.next_to_expand
        if prev is None:
            self.first_to_expand = neighbor
        else:
            prev.next_to_expand = neighbor
        neighbor.next_to_expand = cur

    def expand(self, node: TShortwayPoint) -> None:
        """`TShortway::Expand` — `cract.cc:128-204`."""
        self.first_to_expand = node.next_to_expand

        min_neighbor_waylength = node.waylength + node.waypoints
        origin = self._at(0, 0)
        if min_neighbor_waylength >= origin.waylength:
            return

        for ox, oy in NEIGHBOR_OFFSETS:
            neighbor = self._at(node.x + ox, node.y + oy)
            neighbor_waylength = min_neighbor_waylength
            if ox != 0 and oy != 0:
                neighbor_waylength += node.waypoints * 2

            if neighbor_waylength < neighbor.waylength:
                neighbor.waylength = neighbor_waylength
                neighbor.predecessor = node
                if (neighbor.x != 0 or neighbor.y != 0) and neighbor.waypoints != -1:
                    if neighbor.heuristic != INT_MAX:
                        self._remove_from_expand_list(neighbor)
                    distance = abs(neighbor.x) + abs(neighbor.y)
                    neighbor.heuristic = (
                        neighbor.waylength
                        + neighbor.waypoints
                        + self.min_waypoints * (distance - 1)
                    )
                    self._insert_expand_list(neighbor)

    def calculate(
        self, dest_x: int, dest_y: int, must_reach: bool = False, max_steps: int = DEFAULT_MAX_STEPS
    ) -> Optional[List[Tuple[int, int]]]:
        """`TShortway::Calculate` — returns absolute tile waypoints (first steps from monster)."""
        rel_dest_x = dest_x - self.start_x
        rel_dest_y = dest_y - self.start_y

        if rel_dest_x == 0 and rel_dest_y == 0:
            return []

        if abs(rel_dest_x) > self.visible_x or abs(rel_dest_y) > self.visible_y:
            return None

        self._clear_map()
        seed = self._at(rel_dest_x, rel_dest_y)
        seed.waylength = 0
        self.first_to_expand = seed

        while self.first_to_expand is not None:
            self.expand(self.first_to_expand)

        origin = self._at(0, 0)
        if origin.waylength == INT_MAX:
            return None

        cur_distance = max(abs(origin.x - rel_dest_x), abs(origin.y - rel_dest_y))
        node = origin.predecessor
        steps: List[Tuple[int, int]] = []
        remaining = max_steps

        while node is not None and remaining > 0 and (must_reach or cur_distance > 1):
            steps.append((self.start_x + node.x, self.start_y + node.y))
            cur_distance = max(abs(node.x - rel_dest_x), abs(node.y - rel_dest_y))
            node = node.predecessor
            remaining -= 1

        # C++ queues predecessor chain without reversing — first append is the next hop.
        return steps


def tiles_to_dirs(
    start: Tuple[int, int], tiles: Iterable[Tuple[int, int]]
) -> List[str]:
    dirs: List[str] = []
    x, y = start
    for tx, ty in tiles:
        dx, dy = tx - x, ty - y
        name = DIR_NAMES.get((dx, dy))
        if name is None:
            name = f"?({dx},{dy})"
        dirs.append(name)
        x, y = tx, ty
    return dirs


def count_diagonals(dirs: List[str]) -> int:
    return sum(1 for d in dirs if len(d) == 2)


def scenario_to_text(scenario: Scenario) -> str:
    lines = [
        f"name {scenario.name}",
        f"start {scenario.start[0]} {scenario.start[1]}",
        f"target {scenario.target[0]} {scenario.target[1]}",
        f"visible {scenario.visible}",
        f"max_steps {scenario.max_steps}",
        f"default_wp {scenario.default_wp}",
    ]
    for x, y in sorted(scenario.blocked):
        lines.append(f"block {x} {y}")
    for (x, y), wp in sorted(scenario.waypoints.items()):
        lines.append(f"wp {x} {y} {wp}")
    return "\n".join(lines) + "\n"


def path_step_cost(scenario: Scenario, start: Tuple[int, int], dirs: List[str]) -> int:
    """Terrain-weighted step cost — CipSoft cardinal=wp, diagonal=wp*3 from source tile."""
    total = 0
    x, y = start
    for name in dirs:
        wp = scenario.wp_at(x, y)
        if wp <= 0:
            return -1
        if len(name) == 2:
            total += wp * 3
        else:
            total += wp
        dx, dy = 0, 0
        for (ox, oy), label in DIR_NAMES.items():
            if label == name:
                dx, dy = ox, oy
                break
        x, y = x + dx, y + dy
    return total


def rust_bin_path(repo_root: Path) -> Path:
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", repo_root / "target"))
    return target_dir / "debug" / "path_compare"


def run_rust_path_compare(scenario: Scenario, repo_root: Path, build: bool) -> dict:
    bin_path = rust_bin_path(repo_root)
    if build or not bin_path.is_file():
        subprocess.run(
            ["cargo", "build", "-p", "tfs-rust-core", "--bin", "path_compare"],
            cwd=repo_root,
            check=True,
        )
    if not bin_path.is_file():
        raise RuntimeError(
            f"path_compare binary not found at {bin_path} "
            f"(CARGO_TARGET_DIR={os.environ.get('CARGO_TARGET_DIR', '')!r})"
        )
    proc = subprocess.run(
        [str(bin_path)],
        input=scenario_to_text(scenario),
        text=True,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"path_compare failed (exit {proc.returncode}):\n{proc.stderr}\n{proc.stdout}"
        )
    return json.loads(proc.stdout)


def builtin_scenarios() -> List[Scenario]:
    return [
        Scenario(
            name="open_grass_chase",
            start=(10, 10),
            target=(15, 15),
            default_wp=150,
        ),
        Scenario(
            name="obstacle_cardinal_detour",
            start=(1, 10),
            target=(5, 10),
            default_wp=150,
            blocked={(3, 10)},
        ),
        Scenario(
            name="cardinal_trap_needs_diagonal",
            start=(10, 10),
            target=(12, 12),
            default_wp=150,
            blocked={(10, 9), (10, 11), (9, 10), (11, 10)},
        ),
        Scenario(
            name="mixed_terrain_sand_strip",
            start=(10, 10),
            target=(16, 10),
            default_wp=150,
            waypoints={(13, 10): 160, (14, 10): 160, (15, 10): 160},
        ),
        Scenario(
            name="diagonal_kite_target",
            start=(10, 10),
            target=(14, 14),
            default_wp=150,
        ),
    ]


def compare_scenario(scenario: Scenario, repo_root: Path, build_rust: bool) -> dict:
    cip = TShortway(scenario)
    cip_tiles = cip.calculate(scenario.target[0], scenario.target[1], False, scenario.max_steps)
    cip_dirs = tiles_to_dirs(scenario.start, cip_tiles or [])

    rust = run_rust_path_compare(scenario, repo_root, build_rust)
    rust_dirs = rust.get("dirs", [])
    rust_tiles = rust.get("tiles", [])

    dirs_match = cip_dirs == rust_dirs
    tiles_match = list(cip_tiles or []) == [tuple(t) for t in rust_tiles]
    cip_cost = path_step_cost(scenario, scenario.start, cip_dirs) if cip_dirs else 0
    rust_cost = rust.get("total_cost")
    tie_break = (
        cip_tiles is not None
        and rust.get("ok")
        and not dirs_match
        and cip_cost == rust_cost
    )

    return {
        "name": scenario.name,
        "start": scenario.start,
        "target": scenario.target,
        "cipsoft": {
            "ok": cip_tiles is not None,
            "tiles": cip_tiles,
            "dirs": cip_dirs,
            "diagonals": count_diagonals(cip_dirs),
            "total_waylength": cip._at(0, 0).waylength if cip_tiles is not None else None,
            "step_cost": cip_cost,
        },
        "rust": rust,
        "match": {
            "dirs": dirs_match,
            "tiles": tiles_match,
            "both_ok": (cip_tiles is not None) == rust.get("ok", False),
            "tie_break": tie_break,
        },
    }


def print_report(results: List[dict]) -> int:
    failures = 0
    print("=" * 72)
    print("CipSoft TShortway vs TFS-RUST chase path compare")
    print("=" * 72)
    for r in results:
        print(f"\n--- {r['name']} ---")
        print(f"  start={r['start']} target={r['target']}")
        cip = r["cipsoft"]
        rust = r["rust"]
        m = r["match"]

        if not cip["ok"]:
            print("  CipSoft: NOWAY")
        else:
            print(f"  CipSoft: dirs={' '.join(cip['dirs']) or '(empty)'}  diagonals={cip['diagonals']}")
            print(f"           tiles={cip['tiles']}  origin_waylength={cip['total_waylength']}")

        if not rust.get("ok"):
            print(f"  Rust:    NOWAY ({rust.get('error', '')})")
        else:
            print(
                f"  Rust:    dirs={' '.join(rust['dirs']) or '(empty)'}  "
                f"diagonals={rust.get('diagonals', 0)} cost={rust.get('total_cost')}"
            )
            print(f"           tiles={rust['tiles']}")

        if m["both_ok"] and m["dirs"] and m["tiles"]:
            print("  Result:  MATCH")
        elif m.get("tie_break"):
            print(
                f"  Result:  TIE (same step cost {cip.get('step_cost')}, "
                "expand-order differs — BinaryHeap vs linked list)"
            )
        elif cip["ok"] != rust.get("ok"):
            print("  Result:  MISMATCH (one side NOWAY)")
            failures += 1
        else:
            print("  Result:  MISMATCH (path differs)")
            if cip["dirs"] != rust.get("dirs"):
                print(f"           dirs: cip={cip['dirs']} rust={rust.get('dirs')}")
            if cip.get("step_cost") != rust.get("total_cost"):
                print(
                    f"           cost: cip={cip.get('step_cost')} rust={rust.get('total_cost')}"
                )
            failures += 1

    print("\n" + "=" * 72)
    ties = sum(1 for r in results if r["match"].get("tie_break"))
    passed = len(results) - failures
    print(f"Summary: {passed}/{len(results)} exact match, {ties} tie-break, {failures} mismatch")
    return 1 if failures else 0


def exit_code(results: List[dict]) -> int:
    for r in results:
        m = r["match"]
        if not m["both_ok"]:
            return 1
        if not m["dirs"] and not m.get("tie_break"):
            return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scenario", type=Path, help="Single scenario file (optional)")
    parser.add_argument("--build-rust", action="store_true", help="cargo build path_compare first")
    parser.add_argument("--json", action="store_true", help="Print JSON report only")
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]

    if args.scenario:
        text = args.scenario.read_text()
        # Minimal parser for custom scenarios — reuse rust bin format via temp file
        scenarios = [parse_scenario_text(text)]
    else:
        scenarios = builtin_scenarios()

    results = [compare_scenario(s, repo_root, args.build_rust) for s in scenarios]

    if args.json:
        print(json.dumps(results, indent=2))
        return exit_code(results)

    return print_report(results)


def parse_scenario_text(text: str) -> Scenario:
    name = "custom"
    start = (0, 0)
    target = (0, 0)
    visible = DEFAULT_VISIBLE
    max_steps = DEFAULT_MAX_STEPS
    default_wp = 150
    blocked: set[Tuple[int, int]] = set()
    waypoints: Dict[Tuple[int, int], int] = {}

    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        key = parts[0]
        if key == "name" and len(parts) >= 2:
            name = parts[1]
        elif key == "start" and len(parts) >= 3:
            start = (int(parts[1]), int(parts[2]))
        elif key == "target" and len(parts) >= 3:
            target = (int(parts[1]), int(parts[2]))
        elif key == "visible" and len(parts) >= 2:
            visible = int(parts[1])
        elif key == "max_steps" and len(parts) >= 2:
            max_steps = int(parts[1])
        elif key == "default_wp" and len(parts) >= 2:
            default_wp = int(parts[1])
        elif key == "block" and len(parts) >= 3:
            blocked.add((int(parts[1]), int(parts[2])))
        elif key == "wp" and len(parts) >= 4:
            waypoints[(int(parts[1]), int(parts[2]))] = int(parts[3])

    return Scenario(
        name=name,
        start=start,
        target=target,
        visible=visible,
        max_steps=max_steps,
        default_wp=default_wp,
        blocked=blocked,
        waypoints=waypoints,
    )


if __name__ == "__main__":
    sys.exit(main())
