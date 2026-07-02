---
trigger: always_on
description: Two-axis versioning (wire codec + mechanics profile). Where new code goes for 772 vs 1098.
globs:
---

# Protocol & Mechanics Versioning

Config key: `clientVersion` (`772` | `1098`). **Default era is 772** (`config.lua`). 1098 is the secondary supported era.

## Two independent axes

| Axis | Selector | What differs |
|------|----------|--------------|
| **Wire** | `ProtocolVersion` + `ProtocolCaps` + `ProtocolCodec` | Bytes, opcodes, login, transport |
| **Mechanics** | `MechanicsProfile` + `data/formulas/<version>.lua` | Combat, walk beat, AI knobs, condition ticks |

One binary, both eras — **no** `if version == 772` in core, **no** `condition_772.rs`-style forks.

## Source-of-truth by era

| Building… | 772 (default) | 1098 |
|-----------|---------------|------|
| Packets / opcodes | `gameserver/src/` (primary); `tibia-game-master/src/` and repo-root `src/` as secondary cross-refs | repo-root `src/` |
| Game mechanics | `tibia-game-master/src/` (outcomes); `gameserver/src/` for wire-adjacent constants | TFS 1.4.2 `src/` |
| Code shape | TFS/TVP idioms | TFS idioms |
| Balance literals | `MechanicsProfile` / `data/formulas/772.lua` | `MechanicsProfile` / `data/formulas/1098.lua` |

**772 wire rule (relaxed):** TVP `gameserver/src/` is the **primary** wire reference. The decompile (`tibia-game-master/src/`) and repo-root `src/` may be freely consulted as secondary/cross-reference sources — especially when TVP is incomplete or ambiguous (e.g. `config.cc:Beat=200` owns the beat duration constant that TVP hardcodes without explanation). **Cite which source informed each decision.** Do not silently mix without noting which tree is authoritative for a given byte/value.

**Clean-room (772 mechanics):** replicate decompile *outcomes*, never transcribe its source. Write Rust in TFS/TVP style.

**All eras:** C++ is the spec for observable behavior; Rust is idiomatic implementation — not a line-for-line port. See `@.devin/rules/TFS-Core.md` §Porting model.

## Where new code goes (R1–R12 summary)

- **Game logic** → `tfs-rust-core` — shared, protocol-free, reads `MechanicsProfile` for era constants
- **Wire bytes** → `tfs-rust-net` codec only — see `@.devin/rules/TFS-wire-codec.md`
- **Balance literals** → `MechanicsProfile` / `data/formulas/*.lua` — see `@.devin/rules/TFS-mechanics-profile.md`
- **DB save format** → shared schema — **not** version-gated (except auth: account number vs name)
- **NPC scripts** → TFS Lua only (`data/npc/scripts/`) — no `.ndb` engine in Rust

## Naming bans

- No `*_1098` / `*_772` in public APIs — use neutral `*Wire`, `encode_*`, `Codec1098`/`Codec772` impls
- No version suffix on core functions — era is config + profile, not function name

## Conditions example (TFS structure, era-tuned numbers)

CipSoft uses timer-skills; we use **TFS `ConditionDamage` / `condition.rs`**. Decompile differences (fire 10/8, energy 25/10, poison decay) go into **`MechanicsProfile` / `getConditionTick`**, not a parallel condition system.
