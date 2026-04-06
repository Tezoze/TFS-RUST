# Phase A — Protocol Correctness (Wire-format parity)

**Goal:** OTClient renders the world correctly — items, creatures, floor changes, chat, speed.
**C++ ref files:** `networkmessage.cpp`, `protocolgame.cpp`, `protocolgame.h`
**Estimated effort:** 1–2 days

> **DO THIS FIRST** — unblocks visual correctness for all subsequent phases.

---

- [ ] **A.1** ~~Fix `write_item_template` — append `0x00` duration byte~~ **SKIPPED** — OTClient v8 doesn't consume this byte (GameDisplayItemDuration off by default), so it shifts all later fields causing black screen. Deviation from C++ required for wire compatibility. (`item_encode.rs`)
- [x] **A.2** Add fluid/splash sub-type byte to `write_item_template` (`item_encode.rs`, needs `fluidMap` table)
- [x] **A.3** Fix `sendChangeSpeed` — 2×u16 (baseSpeed/2, speed/2) not 1×u32 (`outgoing_extra.rs`)
- [x] **A.4** Fix `sendChannelMessage` field order to match C++ (`outgoing_extra.rs`)
- [x] **A.5** Rewrite `send_creature_turn` — full `0x6B` + position/ffff + `0x63` known-creature sub-header + direction + walkthrough (`outgoing_extra.rs`)
- [x] **A.6** Fix `sendCancelTarget` — add `u32(0)` after opcode (`outgoing_extra.rs`)
- [x] **A.7** Fix `send_cancel_walk` — pass and write actual player direction (`outgoing_extra.rs`)
- [x] **A.8** Implement `MoveUpCreature` (0xBE) / `MoveDownCreature` (0xBF) for z-axis transitions (`map_description.rs` / new outgoing)
- [x] **A.9** Parse and store OS / OTCv8 detection from first packet (`game_first_packet.rs`, `pending_login.rs`)
- [x] **A.10** Fix known creature eviction — visibility-aware (`map_description.rs`)

---

**Quick wins in this phase (< 30 min each):**
- A.1 — 1 line fix, immediately unblocks map rendering
- A.2 — ~10 lines + lookup table
- A.3 — 3 lines
- A.6 — 1 line
- A.7 — 2 lines
