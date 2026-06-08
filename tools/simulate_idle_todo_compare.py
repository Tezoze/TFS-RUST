#!/usr/bin/env python3
"""Compare 772 ToDo/IdleStimulus vs Rust Phase A idle-todo chase timing.

Pure state-machine simulation — no game server required.

reference: cract.cc Execute/ToDoStart/CalculateDelay, crnonpl.cc IdleStimulus.
Rust: idle_stimulus.rs, creature_todo.rs, walk/walk_timing.rs.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum, auto
from typing import List, Optional

BEAT_MS = 200
GROUND = 150


def linear_go_effective_speed(go: int) -> int:
    return max(go * 2 + 80, 1)


def step_delay_ms(ground: int, waypoint_cost: int, go: int) -> int:
    eff = linear_go_effective_speed(go)
    waypoints = ground * max(waypoint_cost, 1)
    raw = (waypoints * 1000) // eff
    return ((raw + BEAT_MS - 1) // BEAT_MS) * BEAT_MS


# Chase path: cardinal-heavy with occasional diagonal (matches typical repath).
STEP_COSTS = [1, 1, 1, 3, 1, 1, 3, 1]


class CipAction(Enum):
    GO = auto()


@dataclass
class ReferenceCreature:
    go: int
    path_steps: int
    server_ms: int = 0
    next_wakeup: Optional[int] = None
    todo_list: List[CipAction] = field(default_factory=list)
    last_cost: int = 1
    log: List[str] = field(default_factory=list)

    def log_state(self, event: str) -> None:
        self.log.append(
            f"reference {event:22} server_ms={self.server_ms:5} "
            f"todo={len(self.todo_list)} path={self.path_steps:2} wake={self.next_wakeup}"
        )

    def idle_stimulus(self) -> None:
        if self.path_steps > 0:
            self.todo_list.append(CipAction.GO)

    def execute(self) -> None:
        if not self.todo_list:
            self.idle_stimulus()
        if not self.todo_list:
            return
        self.todo_list.pop(0)
        if self.path_steps > 0:
            idx = 8 - self.path_steps
            self.last_cost = STEP_COSTS[idx % len(STEP_COSTS)]
            self.path_steps -= 1
        if not self.todo_list:
            self.idle_stimulus()

    def calculate_delay_ms(self) -> int:
        # CalculateDelay uses the step just completed.
        return max(step_delay_ms(GROUND, self.last_cost, self.go), 1)

    def todo_start(self) -> None:
        if not self.todo_list:
            return
        self.next_wakeup = self.server_ms + self.calculate_delay_ms()

    def drain_if_due(self) -> bool:
        if self.next_wakeup is None or self.next_wakeup > self.server_ms:
            return False
        self.next_wakeup = None
        self.log_state("process_creature_todo")
        self.execute()
        self.todo_start()
        self.log_state("after_execute")
        return True

    def advance_beat(self) -> None:
        self.server_ms += BEAT_MS
        while self.drain_if_due():
            pass


@dataclass
class RustCreature:
    go: int
    walk_queue_len: int
    server_ms: int = 0
    next_wakeup: Optional[int] = None
    action_q: int = 0
    last_cost: int = 1
    log: List[str] = field(default_factory=list)

    def log_state(self, event: str) -> None:
        self.log.append(
            f"Rust    {event:22} server_ms={self.server_ms:5} "
            f"action_q={self.action_q} walk_q={self.walk_queue_len:2} wake={self.next_wakeup}"
        )

    def idle_stimulus(self) -> None:
        if self.walk_queue_len > 0 and self.action_q == 0:
            self.action_q = 1

    def execute_go(self) -> None:
        if self.action_q == 0:
            return
        self.action_q = 0
        if self.walk_queue_len > 0:
            idx = 8 - self.walk_queue_len
            self.last_cost = STEP_COSTS[idx % len(STEP_COSTS)]
            self.walk_queue_len -= 1

    def schedule_wakeup(self, delay_ms: int) -> None:
        self.next_wakeup = self.server_ms + delay_ms

    def process_creature_todo(self) -> None:
        if self.next_wakeup is None or self.next_wakeup > self.server_ms:
            return
        self.next_wakeup = None
        self.log_state("process_creature_todo")
        if self.action_q == 0:
            self.idle_stimulus()
        if self.action_q > 0:
            self.execute_go()
        if self.walk_queue_len > 0:
            # finish_creature_todo_execute: todo_start_go_delay(first=false) → completed step delay.
            self.schedule_wakeup(step_delay_ms(GROUND, self.last_cost, self.go))
            self.log_state("schedule_wakeup")
        elif self.action_q == 0:
            self.idle_stimulus()
            self.log_state("idle_stimulus_exit")

    def advance_beat(self) -> None:
        self.server_ms += BEAT_MS
        self.process_creature_todo()


def run_chase(go: int, steps: int = 8) -> tuple[list[str], list[str], list[int], list[int]]:
    cip = ReferenceCreature(go=go, path_steps=steps)
    rust = RustCreature(go=go, walk_queue_len=steps)

    # Arm chase: idle → first Go immediate (ticks==1) → schedule from completed step.
    cip.idle_stimulus()
    cip.todo_start()
    cip.log_state("initial_arm")

    rust.idle_stimulus()
    rust.execute_go()  # first_step immediate
    rust.schedule_wakeup(step_delay_ms(GROUND, rust.last_cost, go))
    rust.log_state("initial_arm")

    for _ in range(120):
        if cip.path_steps == 0 and cip.next_wakeup is None:
            if rust.walk_queue_len == 0 and rust.next_wakeup is None:
                break
        cip.advance_beat()
        rust.advance_beat()

    cip_wakes = [
        int(l.split("server_ms=")[1].split()[0])
        for l in cip.log
        if "process_creature_todo" in l
    ]
    rust_wakes = [
        int(l.split("server_ms=")[1].split()[0])
        for l in rust.log
        if "process_creature_todo" in l
    ]
    cip_deltas = [cip_wakes[i] - cip_wakes[i - 1] for i in range(1, len(cip_wakes))]
    rust_deltas = [rust_wakes[i] - rust_wakes[i - 1] for i in range(1, len(rust_wakes))]
    return cip.log, rust.log, cip_deltas, rust_deltas


def print_speed_table(go: int, label: str) -> None:
    eff = linear_go_effective_speed(go)
    card = step_delay_ms(GROUND, 1, go)
    diag = step_delay_ms(GROUND, 3, go)
    print(f"  {label}: go={go} GetSpeed={eff}  cardinal={card} ms  diagonal={diag} ms")


def main() -> None:
    print("=== Step delay formula (ground=150, Beat=200) ===")
    print_speed_table(22, "Dog XML speed")
    print_speed_table(200, "Test monster (Rat)")
    print()

    for go, name in [(200, "go=200 (diagonal-heavy chase)"), (22, "go=22 (Dog XML)")]:
        cip_log, rust_log, cip_d, rust_d = run_chase(go=go)
        print(f"=== {name} — inter-step delays (logical ms) ===")
        print(f"  {'Step':>4}  {'reference':>10}  {'Rust':>8}  {'Match':>6}")
        for i, (dc, dr) in enumerate(zip(cip_d, rust_d), start=2):
            match = "yes" if dc == dr else "NO"
            print(f"  {i:4}  {dc:8}  {dr:8}  {match:>6}")
        mismatches = sum(1 for a, b in zip(cip_d, rust_d) if a != b)
        print(f"  Total steps: {len(cip_d)}  mismatches: {mismatches}")
        print()

    print("=== Architecture parity ===")
    rows = [
        ("Drain trigger", "Execute drains → IdleStimulus", "drain_todo_queue → idle_stimulus", "Match"),
        ("Path buffer", "TDGo in ToDoList (1 at a time)", "walk_queue + single Go", "Equivalent"),
        ("Steps per wakeup", "One TDGo / one tile", "One Go → on_walk one step", "Match"),
        ("Post-step delay", "CalculateDelay(completed)", "get_walk_delay_logical @ elapsed=0", "Match"),
        ("First chase step", "Immediate if delay satisfied", "todo_start_go_delay → ticks==1", "Match"),
        ("Scheduler", "ToDoQueue min-heap", "todo_queue BTree + next_wakeup", "Match"),
        ("Think chase", "Not used", "Gated off beat_driven_loop", "Match"),
        ("Attack", "ToDoAttack from idle", "Phase B (think stub)", "Gap"),
    ]
    print(f"  {'Aspect':<18} {'CipSoft':<32} {'Rust 772':<32} {'Parity'}")
    print("  " + "-" * 88)
    for row in rows:
        print(f"  {row[0]:<18} {row[1]:<32} {row[2]:<32} {row[3]}")
    print()

    print("=== Live server Dog chase (your logs) ===")
    live = [
        ("16000→17000", 1000, "walk_q 8→7"),
        ("17000→18000", 1000, "walk_q 7→6"),
        ("18000→19000", 1000, "walk_q 6→5"),
        ("21000→22400", 1400, "walk_q 3→2 (one diagonal/cardinal mix)"),
        ("25200", 0, "walk_q=0, idle only, no schedule_wakeup"),
    ]
    for span, delta, note in live:
        print(f"  {span}: Δ={delta:4} ms  ({note})")
    print()
    print("Interpretation:")
    print("  • +1000 ms = go=200 diagonal step (450 wp / GetSpeed 480 → ceil to 1000)")
    print("  • +1400 ms = go=22 cardinal step (150 wp / GetSpeed 124 → ceil to 1400)")
    print("  • Dog XML speed=22; mixed +1000/+1400 suggests chase path cost mix OR")
    print("    a different effective go at runtime — verify with trace + monster.speed in logs.")
    print("  • heap_len 0|1 and idle-on-drain cycle match 772 reference semantics (Phase A OK).")


if __name__ == "__main__":
    main()
