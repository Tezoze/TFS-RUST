---
trigger: glob
globs: crates/tfs-rust-core/**/*.rs, data/formulas/**
---

# Mechanics Profile (772 / 1098)

Mechanics are **shared code**, **era-tuned data**. Full spec: `docs/PROTOCOL_VERSIONING.md` §12, §12.13. Implement in idiomatic Rust for the same observable outcome — not a C++ transliteration (`tfs-core.md`).

## Add or change a mechanic (combat, conditions, walk, AI, spells)

1. **One function** in `tfs-rust-core` — TFS-shaped API (e.g. `ConditionDamage` ticks, not CipSoft timer-skill classes).
2. **Read profile** — `MechanicsProfile` field or Tier-2 Lua hook; **never** bare balance literals in Rust.
3. **Defaults per era** — `data/formulas/1098.lua` (TFS 1.4.2) vs `data/formulas/772.lua` (CipSoft outcomes).
4. **772 behavior cite** — `tibia-game-master/src/` for numbers/outcomes; repo-root `src/` for TFS structure cite.
5. **No `client_version` checks** — profile loaded at startup from `clientVersion` config.

## Conditions (explicit contract)

- Structure: TFS `condition.cpp` / `Condition*` — merge + tick in `condition.rs`.
- 772 differences (fire 10/8, energy 25/10, poison decay, haste via speed delta): **`MechanicsProfile.conditions`** or `getConditionTick(type, round)` hook.
- Do **not** fork `condition_772.rs` or port `TSkillPoison` as a separate system.

## Tier-1 vs Tier-2

| Tier | What | When loaded |
|------|------|-------------|
| **1** | Scalars/tables → `MechanicsProfile` fields | Startup, zero per-tick Lua cost |
| **2** | Optional overrides (`getWeaponDamage`, `getConditionTick`, …) | Native default; Lua only if registered |

## Touch points (examples)

`walk.rs`, `combat/mod.rs`, `condition.rs`, `spell.rs`, `monster_ai.rs`, `spawn_lifecycle.rs`, `pathfinding.rs` — constants from profile, logic shared.

## Do not

- Put opcode bytes or `NetworkMessage` writes in core
- Hardcode `2000` ms attack, `10/8` fire, beat `200` ms — belong in `772.lua` / `1098.lua`
- Copy decompiled C++ source — outcomes only, validated clean-room