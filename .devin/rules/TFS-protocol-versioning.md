---
trigger: always_on
description: >
globs:
---

# Protocol & Mechanics Versioning

Config key: `clientVersion` (`772` | `1098`). **Primary development target: 772** (`config.lua`).

## Primary goal

**772 first:** decompile **outcomes** + `gameserver` **wire**, on a **TFS-style domain** (data pack / Lua / XML stay TFS-compatible), implemented in **idiomatic Rust**. 1098 stays available through the same axes.

## Two independent axes

| Axis | Selector | What differs |
|------|----------|--------------|
| **Wire** | `ProtocolVersion` + `ProtocolCaps` + `ProtocolCodec` | Bytes, opcodes, login, transport |
| **Mechanics** | `MechanicsProfile` + `data/formulas/<version>.lua` | Combat, walk beat, AI knobs, condition ticks |

One binary, both eras — **no** `if version == 772` in core, **no** `condition_772.rs`-style forks. Domain stays TFS-shaped; only profile/codec switch.

## Source-of-truth by era

| Building… | **772 (primary)** | 1098 (supported) |
|-----------|-------------------|-------------------|
| Packets / opcodes | **`gameserver/src/` only** | repo-root `src/` |
| Game mechanics (outcomes) | `tibia-game-master/src/` (**outcomes only**) | TFS 1.4.2 `src/` |
| Domain shape | **TFS style** (`data/`, Lua APIs, cylinders, conditions) | TFS style |
| Rust implementation | **Idiomatic Rust** (not C++ transliteration) | Idiomatic Rust |

**772 wire rule:** all 772 packet bytes, opcodes, login, and transport come **exclusively** from `gameserver/src/`. Do **not** use `tibia-game-master` or repo-root `src/` for 772 wire work.

**Clean-room (772 mechanics):** replicate decompile *outcomes*, never transcribe its source. Keep **TFS-style domain** so the data pack works; write **idiomatic Rust** behind it. Era knobs in profile.

**Conflict rule:** TFS 1.4.2 vs decompile **numbers** → **772 profile follows the decompile**; keep TFS domain shape — do not fork a decompile-shaped parallel system.

## Where new code goes (R1–R12 summary)

- **Game logic** → `tfs-rust-core` — shared, protocol-free, reads `MechanicsProfile` for era constants
- **Wire bytes** → `tfs-rust-net` codec only — see `@.cursor/rules/TFS-wire-codec.mdc`
- **Balance literals** → `MechanicsProfile` / `data/formulas/*.lua` — see `@.cursor/rules/TFS-mechanics-profile.mdc`
- **DB save format** → shared schema — **not** version-gated (except auth: account number vs name)
- **NPC scripts** → TFS Lua only (`data/npc/scripts/`) — no `.ndb` engine in Rust

## Naming bans

- No `*_1098` / `*_772` in public APIs — use neutral `*Wire`, `encode_*`, `Codec1098`/`Codec772` impls
- No version suffix on core functions — era is config + profile, not function name

## Conditions example (TFS structure, 772-tuned numbers)

Decompile uses timer-skills; we use **TFS `ConditionDamage` / `condition.rs`**. Decompile differences (fire 10/8, energy 25/10, poison decay) go into **`MechanicsProfile` / `getConditionTick`**, not a parallel condition system.
