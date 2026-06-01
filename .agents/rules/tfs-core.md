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
| **772** | **`gameserver/src/` only** (TVP 7.72 — sole wire/packet reference) | `tibia-game-master/src/` (CipSoft outcomes only) | TFS/TVP idioms |

For **1098**, TFS 1.4.2 C++ defines the **observable behavior** to match. For **772 wire/packets**, use **`gameserver/src/` only**. For **772 mechanics**, CipSoft decompile defines **outcomes** via `MechanicsProfile`. In all cases: **spec from C++, implementation in Rust** — never transcribe or line-translate reference source.

# Porting model — outcome parity, not code parity

C++ (TFS, TVP, or decompile) is the **behavioral specification**, not a template to copy.

| Match exactly | Do not copy literally |
|---------------|----------------------|
| Wire bytes on the client | Class hierarchies, raw pointers, `shared_ptr` |
| Game outcomes (damage, ticks, AI decisions, DB results) | OOP layout, scheduler internals, C++ control flow |
| Edge cases and protocol quirks | Decompiled or leaked source transcription |

**Write idiomatic Rust** (`SlotMap` IDs, enums + pattern matching, traits, `?` errors, Tokio I/O + single-threaded game loop) that produces the **same observable result** as the reference for the active era. Prefer zero-cost abstractions where they preserve behavior exactly.

# Compatibility Mandate (Non-Negotiable — Never Violate)
- Default to **exact observable parity** with the active era: TFS 1.4.2 for `clientVersion = 1098`; CipSoft-faithful outcomes (via `MechanicsProfile` / `data/formulas/772.lua`) for `clientVersion = 772`. Same mandate for database flows, packet bytes, mechanics, and edge cases — **not** for matching C++ structure line-for-line.
- **Always prefer better Rust methods** that achieve the *exact same observable outcome*. Use idiomatic, zero-cost, concurrent, and type-safe Rust patterns wherever they produce identical results to the reference for the active era.
- **No silent improvements** that change behavior. If a Rust pattern would alter any observable result (even slightly), document the exact reference behavior, explain the deviation, and request explicit user approval.
- When unsure: stop immediately, state the uncertainty, and ask the user for the relevant C++ source — 1098: repo-root `src/`; **772 wire: `gameserver/src/` only**; 772 mechanics: `tibia-game-master/src/`.
- For every substantial ported function, include a comment with the exact C++ reference (file + function name). 772 mechanics cite both TFS structure (`condition.cpp`, etc.) and CipSoft behavior (`crskill.cc`, etc.) where they diverge.