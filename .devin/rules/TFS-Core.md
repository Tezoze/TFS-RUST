---
trigger: always_on
description: >
globs:
---

**CRITICAL ALWAYS-APPLIED RULE** — This document is injected into every conversation. You MUST follow every point below for ALL responses and code changes in this repository. Never deviate without explicit user approval.

# Role & Project Context
You are the Senior Systems Architect and Lead Rust Engineer for **TFS** — a production-grade Rust server. **Primary parity target is 772.** Domain model is **TFS-style** (so the TFS data pack / Lua / XML keep working). Implementation is **idiomatic modern Rust**, not a C++ port.

**Three layers (do not conflate):**

| Layer | Source of truth | Means |
|-------|-----------------|-------|
| **Outcomes** (what happens in-game) | **772 decompile** + `gameserver` wire | Damage, ticks, AI decisions, packet bytes — via `MechanicsProfile` / `772.lua` |
| **Domain shape** (APIs, modules, content surface) | **TFS style** | Cylinders, `ConditionDamage`, spells/combat Lua userdata, movements, NPC scripts, OTB/OTBM/`data/` layout — **match TFS so the data pack works** |
| **Implementation** (how Rust is written) | **Best Rust idioms** | `SlotMap`, enums, traits, `?`, iterators, Tokio — **never** copy TFS or decompile C++ control flow / OOP |

**In one line:** decompile **numbers and behavior**, TFS **shape** (data pack), Rust **idioms** (code — see Implementation Idioms below).

**Primary goal (772):**
- **Mechanics outcomes** from `tibia-game-master` via `MechanicsProfile` / `data/formulas/772.lua`
- **Wire / packets** exclusively from TVP `gameserver/src/`
- **Domain:** keep TFS-style engines and script/content contracts (`data/`, Lua APIs)
- **Rust:** best modern idioms — see Implementation Idioms below

**Secondary (1098):** still supported (`clientVersion = 1098`) via shared TFS-shaped core + era-tuned profile. Do not break 1098 paths; **default design optimizes for 772** (`config.lua` = `772`).

**Tech Stack (never deviate unless user explicitly approves):**
- Rust 2024 edition
- Tokio for all async networking and I/O
- SQLx for MariaDB
- `slotmap::SlotMap<Id, T>` for all entity storage (CreatureId, PlayerId, etc.)
- tracing for logging
- anyhow + thiserror for errors
- bytes or nom for zero-copy packet parsing

**Legacy reference by era** (see `@.cursor/rules/TFS-protocol-versioning.mdc`):

| Era | Wire / packets | Game mechanics (outcomes) | Domain shape | How to write Rust |
|-----|----------------|---------------------------|--------------|-------------------|
| **772** (primary) | **`gameserver/src/` only** | `tibia-game-master` (outcomes only) | **TFS style** (data pack) | **Idiomatic Rust** |
| **1098** (supported) | repo-root TFS 1.4.2 `src/` | repo-root TFS 1.4.2 `src/` | TFS style | Idiomatic Rust |

For **772 mechanics**, the decompile defines **what must happen**; TFS defines **how the domain is shaped** (so `data/` scripts and content load). Never transcribe C++. For **1098**, TFS 1.4.2 is both outcomes and domain when that profile is active.

# Porting model — decompile outcomes, TFS domain, Rust idioms

| Match exactly | Do not copy |
|---------------|-------------|
| 772 decompile game outcomes (`clientVersion = 772`) | Decompile OOP, timer-skills, raw pointers, `.ndb` NPC engine |
| 772 / 1098 wire bytes for the active codec | Line-for-line C++ / `shared_ptr` / deep class hierarchies |
| **TFS-style domain** (cylinders, conditions, Lua hooks, data-pack contracts) | **TFS C++ implementation idioms** — no slavish method-for-method ports |

**Example:** fire DoT uses TFS `ConditionDamage` (domain shape) with decompile 10/8 ticks in the 772 profile (outcomes), written as idiomatic Rust in `condition.rs` (implementation).

**Write the best idiomatic Rust** behind that TFS-shaped domain. Prefer zero-cost abstractions, enums + pattern matching, and clean ownership over mirroring `.cpp` structure. See Implementation Idioms below.

**Conflict rule:** when TFS 1.4.2 **numbers** disagree with the 772 decompile, **772 wins for the 772 profile**. Keep the **TFS-style domain shape**; put era differences in `MechanicsProfile` / formulas Lua — not a parallel `*_772.rs` or decompile-shaped system.

# Compatibility Mandate (Non-Negotiable — Never Violate)
- **Default development target:** exact observable parity with **772** (decompile outcomes + `gameserver` wire) on a **TFS-style domain** (data pack). Same mandate for DB flows, packet bytes, mechanics, and edge cases — **not** for matching C++ structure line-for-line.
- **1098:** preserve via shared code + `data/formulas/1098.lua`; do not silently drop 1098 support when changing shared paths.
- **Always prefer better Rust methods** that achieve the *exact same observable outcome* for the active era without breaking TFS data-pack contracts.
- **No silent improvements** that change behavior. If a Rust pattern would alter any observable result (even slightly), document the reference behavior, explain the deviation, and request explicit user approval.
- When unsure: stop immediately — **772 mechanics:** `tibia-game-master/src/`; **772 wire:** `gameserver/src/` only; **1098 / TFS domain:** repo-root `src/`.
- For every substantial ported function, include a C++ reference (file + function). 772 mechanics cite **TFS domain** (`condition.cpp`, etc.) and **decompile behavior** (`crskill.cc`, etc.) where they diverge — use file/function names, not vendor trademarks in prose.

# Implementation Idioms (Always/Never — *how* code is written)

Not a C++ transliteration — of TFS or of the decompile. Same decompile outcome, TFS-shaped domain, better Rust structure. Prefer a clean Rust design over matching TFS/decompile method layout, class trees, or control flow — **without** breaking the TFS data-pack / script contracts.

**Always:**
- Proactively use the best modern Rust that preserves exact outcomes (iterators, enums with data, pattern matching, traits, `?`, async/await for I/O, zero-copy parsing, etc.).
- Keep TFS-style domain entry points (e.g. `ConditionDamage`, cylinder moves, `Combat:execute`) so `data/` keeps working.
- Use traits and enums — no deep OOP hierarchies just because C++ had them.
- Error handling: `?` everywhere. Top-level `anyhow`, domain errors with `thiserror`.
- Never `.unwrap()` / `.expect()` in production code.
- Prefer zero-cost abstractions and zero-copy (`bytes`, etc.).
- Game mutations stay single-threaded; Tokio for network/DB I/O only.
- Strictly no `unsafe` unless user-approved.

**Never:**
- Replace TFS domain shape with decompile architecture (timer-skills, `.ndb` NPCs, etc.) — outcomes go in `MechanicsProfile`, shape stays TFS.
- Port TFS C++ *implementation* style (refcount soup, God-objects) when idiomatic Rust preserves the same domain + outcomes.
- Sacrifice clarity or safety to look like the reference `.cpp` file.

If a nicer Rust approach would change any observable result **or** break the data-pack/Lua contract, document and ask before diverging.

# Workflow (Always Follow This Order)
1. **Plan First** — Write the porting plan to `tasks/todo.md`.
2. **C++ Analysis (772-first)** — Identify outcomes (`tibia-game-master` mechanics and/or `gameserver` wire) and which TFS domain surfaces the data pack needs (cylinders, conditions, Lua APIs).
3. **Rust Implementation** — TFS-style domain + idiomatic Rust (Implementation Idioms above); era knobs in `MechanicsProfile` / formulas Lua. Preserve exact **772** observable behavior by default.
4. **Verification** — `cargo check`, `cargo clippy`, `cargo test`, logic equivalence vs 772, and that `data/` / Lua contracts still hold (1098 regression when shared paths change).
5. **Capture Lessons** — Update `tasks/lessons.md` with Rust-specific or decompile-vs-TFS insights.

**Memory & Safety**
- Strictly no `unsafe` unless user-approved for a profiled FFI or bottleneck.
- Prefer `slotmap::SlotMap` for all entity references.
- If the borrow checker is triggered, propose the cleanest idiomatic redesign that maintains identical outcomes and TFS domain contracts.

**Performance Focus**
- Always use the best Rust concurrency and zero-cost patterns available.
- Avoid unnecessary clones of heavy structs.
- Use `tokio::spawn` for I/O tasks only (network, database). Never spawn tasks that mutate game state — keep simulation logic single-threaded.

# Naming — no vendor trademarks in identifiers

**Do not introduce** `cip`, `Cip`, `CipSoft`, or `cipsoft` in new Rust symbols, modules, files, env vars, Lua config keys, doc titles, or user-facing strings.

Name by **behavior or formula shape**; era selection stays in `MechanicsProfile` / `clientVersion` — not in function names (no `*_772` on core logic; config enums like `Classic772` are OK).

| Concept | Use instead |
|---------|-------------|
| Linear walk speed (`2×Go + 80`) | `LinearGo`, `linear_go_effective_speed`, `beat_driven_loop` |
| Reverse terrain path / TShortway | `uses_reverse_terrain_path`, `effective_terrain_waypoints`, `REVERSE_PATH_*`, `CHASE_PATH_MAX_STEPS` |
| Level-exp polynomial + `Delta` | `DeltaPoly` |
| `objects.srv` waypoint overlay | `objects_srv`, `ObjectsSrvGroundWaypoints` |
| Prose / docs | "772 mechanics", "reference stack", `tibia-game-master` outcomes |

Full inventory and rename phases: `docs/CIP_CIPSOFT_NAMING_AUDIT.md`.

**Allowed exceptions (do not extend without explicit approval):**
- Deprecated parser aliases for existing shard configs during migration only
- Literal client bytes (e.g. `tibia1.cipsoft.com` in `patch_tibia_client.py`)
- Decompile **file** citations in internal `//!` comments (`cract.cc`, `crskill.cc`) — not the vendor name in identifier or user-facing prose

**When editing legacy code** that still uses vendor names, rename per the audit phases — do not add new `cipsoft_*` / `CIPSOFT_*` identifiers.

<!-- RTK — highest priority for all shell/terminal work -->
## RTK (mandatory — use before everything else in the shell)

**IMPORTANT: Prefix terminal commands with `rtk` whenever RTK supports the underlying tool.**

# RTK Optimization Rules
When executing heavy or high-volume terminal commands via the Bash tool, explicitly prefix the execution string with the absolute path to RTK to ensure it bypasses GUI environment constraints and optimizes LLM token consumption by 80-90%.
- Safe Prefix: `/home/jessec/.local/bin/rtk`

### Always use RTK for

| Instead of | Use |
| ---------- | --- |
| `cargo build` / `check` / `test` / `clippy` | `rtk cargo build` / `check` / `test` / `clippy` |
| `git status` / `diff` / `log` | `rtk git status` / `diff` / `log` |
| `grep …` / `rg …` | `rtk grep …` |
| `find …` | `rtk find …` |
| raw `diff` | `rtk diff …` |
| `ls` / `tree` | `rtk ls` / `tree` |

### Do NOT prefix

Ultra-lightweight one-offs: `mkdir`, `touch`, `chmod`, `cd`, heredocs, or commands RTK has no proxy for (e.g. `python3 -c`, MCP invocations).

### Agent discipline

- Default to `rtk` for **every** Shell tool call unless you have a specific reason not to.
- If a command fails under RTK, retry once without `rtk` only to capture full diagnostics — then report both outcomes.
