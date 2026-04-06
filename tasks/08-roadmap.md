# Australis — Execution Order, Quick Wins & Metrics

---

## Recommended Execution Order

```
Phase A (Protocol Fixes)          ← DO FIRST — unblocks visual correctness
  ↓
Phase B (Items & Containers)      ← Foundation for everything
  ↓
Phase C (Inventory & Equipment)   ← Players can hold things
  ↓
Phase D (Monster/NPC Walk + AI)   ← World comes alive
  ↓
Phase E (Combat)                  ← Players can fight
  ↓
Phase F (Chat & Social)           ← Players can communicate
  ↓
Phase G (Conditions)              ← Combat effects tick correctly
  ↓
Phase H (Spells & Runes)          ← Magic works
  ↓
Phase I (NPC & Player Trade)      ← Economy works
  ↓
Phase J (Lua Scripting)           ← Custom content works
  ↓
Phase K (Persistence & Shutdown)  ← Server is durable
  ↓
Phase L (Remaining Systems)       ← Feature-complete
```

**Note:** Phases F and G can be parallelised with D/E if desired. Phase J (Lua) is the largest single effort and could be started earlier in reduced form (just `onLogin`/`onDeath` hooks) to unblock script-dependent content.

---

## Quick Wins (< 30 min each, do right now)

1. **A.1** — Add `0x00` duration byte to `write_item_template` (1 line, unblocks map rendering)
2. **A.2** — Add fluid sub-type byte (10 lines + lookup table)
3. **A.3** — Fix `sendChangeSpeed` to 2×u16 (3 lines)
4. **A.6** — Fix `sendCancelTarget` — add `u32(0)` (1 line)
5. **A.7** — Fix `send_cancel_walk` direction (2 lines)
6. **L.7** — Dynamic world light (5 lines in `world_light.rs`)

---

## Metrics Summary

| Category | Ported | Total (est.) | % |
|---|---|---|---|
| Client→Server opcodes parsed | 60+ | ~60 | **~100%** |
| Server→Client opcodes implemented | ~18 | ~40 | **~45%** |
| Game packet handlers (with real logic) | 6 (walk, turn, auto-walk, stop, ping, extended) | ~50 | **~12%** |
| C++ source lines (`.cpp` only) | — | ~1.6M chars / ~45k lines | — |
| Rust source lines (all crates) | — | ~12k lines | — |
| Major systems functional | 3 (networking, walking, map loading) | ~20 | **~15%** |

---

## Bottom Line

The foundation is solid — architecture, networking, map loading, and player walking are well-engineered and battle-tested. The next highest-leverage work is **Phase A** (protocol wire-format fixes) because it unblocks the client from rendering the world correctly, followed by **Phase B** (items/containers) which is the prerequisite for nearly every remaining game system.
