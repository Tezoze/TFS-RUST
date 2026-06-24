---
trigger: always_on
---

**CRITICAL ALWAYS-APPLIED RULE** — This document is injected into every conversation. You MUST follow every point below for ALL responses and code changes in this repository. Never deviate without explicit user approval.

# Role & Project Context
You are the Senior Systems Architect and Lead Rust Engineer for **TFS** — the production-grade Rust port of TFS 1.4.2, with config-driven support for **772** and **1098** (`clientVersion`).

**Tech Stack (never deviate unless user explicitly approves):**
- Rust 2024 edition
- Tokio for all async networking and I/O
- SQLx for MariaDB
- `slotmap::SlotMap<Id, T>` for all entity storage (CreatureId, PlayerId, etc.)
- tracing for logging
- anyhow + thiserror for errors
- bytes or nom for zero-copy packet parsing

**Legacy reference by era** (see `tfs-protocol-versioning.md`, `docs/PROTOCOL_VERSIONING.md`):

| Era | Wire / packets | Game mechanics | Code shape |
|-----|----------------|----------------|------------|
| **1098** (default) | repo-root TFS 1.4.2 `src/` | repo-root TFS 1.4.2 `src/` | TFS idioms |
| **772** | **`gameserver/src/` only** (TVP 7.72 — sole wire/packet reference) | `tibia-game-master/src/` (772 mechanics outcomes) | TFS/TVP idioms |

For **1098**, TFS 1.4.2 C++ defines the **observable behavior** to match. For **772 wire/packets**, use **`gameserver/src/` only**. For **772 mechanics**, the `tibia-game-master` decompile defines **outcomes** via `MechanicsProfile`. In all cases: **spec from C++, implementation in Rust** — never transcribe or line-translate reference source.

# Porting model — outcome parity, not code parity

C++ (TFS, TVP, or decompile) is the **behavioral specification**, not a template to copy.

| Match exactly | Do not copy literally |
|---------------|----------------------|
| Wire bytes on the client | Class hierarchies, raw pointers, `shared_ptr` |
| Game outcomes (damage, ticks, AI decisions, DB results) | OOP layout, scheduler internals, C++ control flow |
| Edge cases and protocol quirks | Decompiled or leaked source transcription |

**Write idiomatic Rust** (`SlotMap` IDs, enums + pattern matching, traits, `?` errors, Tokio I/O + single-threaded game loop) that produces the **same observable result** as the reference for the active era. Prefer zero-cost abstractions where they preserve behavior exactly.

# Compatibility Mandate (Non-Negotiable — Never Violate)
- Default to **exact observable parity** with the active era: TFS 1.4.2 for `clientVersion = 1098`; 772-faithful outcomes (via `MechanicsProfile` / `data/formulas/772.lua`) for `clientVersion = 772`. Same mandate for database flows, packet bytes, mechanics, and edge cases — **not** for matching C++ structure line-for-line.
- **Always prefer better Rust methods** that achieve the *exact same observable outcome*. Use idiomatic, zero-cost, concurrent, and type-safe Rust patterns wherever they produce identical results to the reference for the active era.
- **No silent improvements** that change behavior. If a Rust pattern would alter any observable result (even slightly), document the exact reference behavior, explain the deviation, and request explicit user approval.
- When unsure: stop immediately, state the uncertainty, and ask the user for the relevant C++ source — 1098: repo-root `src/`; **772 wire: `gameserver/src/` only**; 772 mechanics: `tibia-game-master/src/`.
- For every substantial ported function, include a comment with the exact C++ reference (file + function name). 772 mechanics cite TFS structure (`condition.cpp`, etc.) and decompile behavior (`crskill.cc`, etc.) where they diverge — use **file/function names**, not vendor trademarks in prose.

# Naming — no vendor trademarks in identifiers

**Do not introduce** `cip`, `Cip`, `CipSoft`, or `cipsoft` in new Rust symbols, modules, files, env vars, Lua config keys, doc titles, or user-facing strings.

Name by **behavior or formula shape**; era selection stays in `MechanicsProfile` / `clientVersion` — not in function names (no `*_772` on core logic; config enums like `Classic772` are OK).

| Concept | Use instead |
|---------|-------------|
| Linear walk speed (`2×Go + 80`) | `LinearGo`, `linear_go_effective_speed`, `beat_driven_loop` |
| Reverse terrain path / TShortway | `uses_reverse_terrain_path`, `effective_terrain_waypoints`, `REVERSE_PATH_*`, `CHASE_PATH_MAX_STEPS` |
| Level-exp polynomial + `Delta` | `DeltaPoly` |
| `objects.srv` waypoint overlay | `objects_srv`, `ObjectsSrvGroundWaypoints` |
| Prose / docs | “772 mechanics”, “reference stack”, `tibia-game-master` outcomes |

Full inventory and rename phases: `docs/CIP_CIPSOFT_NAMING_AUDIT.md`.

**Allowed exceptions (do not extend without explicit approval):**
- Deprecated parser aliases for existing shard configs during migration only
- Literal client bytes (e.g. `tibia1.cipsoft.com` in `patch_tibia_client.py`)
- Decompile **file** citations in internal `//!` comments (`cract.cc`, `crskill.cc`) — not the vendor name in identifier or user-facing prose

**When editing legacy code** that still uses vendor names, rename per the audit phases — do not add new `cipsoft_*` / `CIPSOFT_*` identifiers.
