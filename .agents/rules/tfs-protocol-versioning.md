---
inclusion: auto
name: tfs-protocol-versioning
description: Two-axis versioning (wire codec + mechanics profile). Where new code goes for 772 vs 1098.
---

# Protocol & Mechanics Versioning

Full matrix: `docs/PROTOCOL_VERSIONING.md`. Config key: `clientVersion` (`772` | `1098`).

## Two independent axes

| Axis | Selector | What differs |
|------|----------|--------------|
| **Wire** | `ProtocolVersion` + `ProtocolCaps` + `ProtocolCodec` | Bytes, opcodes, login, transport |
| **Mechanics** | `MechanicsProfile` + `data/formulas/<version>.lua` | Combat, walk beat, AI knobs, condition ticks |

One binary, both eras — **no** `if version == 772` in core, **no** `condition_772.rs`-style forks.

## Source-of-truth by era

| Building… | 1098 | 772 wire / packets | 772 behavior |
|-----------|------|--------------------|--------------|
| Packets / opcodes | repo-root `src/` | **`gameserver/src/` only** | — |
| Game mechanics | TFS 1.4.2 `src/` | — | `tibia-game-master/src/` (outcomes only) |
| Code shape | TFS idioms | TVP/TFS idioms | TFS APIs, CipSoft numbers via profile |

**772 wire rule:** all 772 packet bytes, opcodes, login, and transport come **exclusively** from `gameserver/src/`. Do **not** use `tibia-game-master` or repo-root `src/` for 772 wire work.

**Clean-room (772 mechanics):** replicate decompile *outcomes*, never transcribe its source. Write Rust in TFS/TVP style.

**All eras:** C++ is the spec for observable behavior; Rust is idiomatic implementation — not a line-for-line port. See `tfs-core.md` §Porting model.

## Where new code goes (R1–R12 summary)

- **Game logic** → `tfs-rust-core` — shared, protocol-free, reads `MechanicsProfile` for era constants
- **Wire bytes** → `tfs-rust-net` codec only — see `tfs-wire-codec.md`
- **Balance literals** → `MechanicsProfile` / `data/formulas/*.lua` — see `tfs-mechanics-profile.md`
- **DB save format** → shared schema — **not** version-gated (except auth: account number vs name)
- **NPC scripts** → TFS Lua only (`data/npc/scripts/`) — no `.ndb` engine in Rust

## Naming bans

- No `*_1098` / `*_772` in public APIs — use neutral `*Wire`, `encode_*`, `Codec1098`/`Codec772` impls
- No version suffix on core functions — era is config + profile, not function name

## Conditions example (TFS structure, era-tuned numbers)

CipSoft uses timer-skills; we use **TFS `ConditionDamage` / `condition.rs`**. Decompile differences (fire 10/8, energy 25/10, poison decay) go into **`MechanicsProfile` / `getConditionTick`**, not a parallel condition system.
