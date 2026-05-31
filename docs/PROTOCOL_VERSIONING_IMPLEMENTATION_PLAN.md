# Protocol Versioning — Implementation Plan (772 / 1098)

**Companion to:** `docs/PROTOCOL_VERSIONING.md` (design). This is the *execution* plan: concrete,
phase-by-phase, file-by-file tasks with C++ references, tests, and verification gates.

**Status legend:** `[x]` done · `[~]` partial · `[ ]` not started.

**Two parallel tracks** (see design §6):
- **Track A — Wire/protocol.** Makes byte output version-selectable. Required to *connect* a 772 client.
- **Track B — Mechanics.** Makes 772 *behavior* selectable via `MechanicsProfile`. Required for the
  shard to *behave* like 772. Independent of A; validated on the 1098 shard before any flip.

**Non-negotiable rules carried into every phase** (design §10):
- R1 mechanics are protocol-free · R2 only net knows bytes · R3 outgoing = neutral `XxxWire` + encoder ·
  R4 incoming = semantic `GamePacket` · R5 opcodes centralized & version-keyed · R8 capabilities not
  scattered `if version` · R10 version-structured golden tests · R11 no magic numbers in mechanics ·
  R12 behavior from decompile, code from ourselves.
- **Reference trees by era** (design §12, `tfs-cpp-references`):
  - **772 wire** → `gameserver/src/` ONLY (never `tibia-game-master`, never repo-root `src/`).
  - **772 mechanics/outcomes** → `tibia-game-master/src/` (CipSoft decompile; clean-room outcomes only).
  - **1098 wire + mechanics** → repo-root `src/` (TFS 1.4.2).
- Every ported function carries a C++ ref comment (file + function). 772 mechanics cite both TFS
  structure and CipSoft behavior where they diverge.

---

## 0. Current state (verified against the tree)

What already exists so later phases don't re-do it:

| Piece | Location | State |
|---|---|---|
| `ProtocolVersion`, `ProtocolCaps`, `ProtocolCaps::for_version` (772+1098) | `crates/tfs-rust-common/src/protocol_version.rs` | `[x]` A0 |
| `clientVersion` config + `TFS_PROTOCOL_VERSION` env + validation | `crates/tfs-rust-core/src/config.rs` (`resolve_protocol_version`) | `[x]` A0 |
| `protocol_version` / `protocol_caps` threaded into `*WireConfig` | `crates/tfs-rust-net/src/server.rs` | `[x]` A0 |
| `ProtocolCodec` trait + `Codec1098` + zero-cost `Codec` enum dispatcher | `crates/tfs-rust-net/src/codec/{mod,v1098,wire}.rs` | `[x]` A1 |
| Neutral wire structs (`PlayerStatsWire`, `PlayerSkillsWire`, `ItemTemplateArgs`, `AddCreatureWire`, `OutfitWire`, `ItemStack`/`ItemWire`) | `codec/wire.rs`, `creature_encode.rs`, `map_description.rs` | `[x]` A1 |
| `Codec` stored on game thread; emission call sites route through it | `crates/tfs-rust-core/src/game_world.rs` (`codec` field) etc. | `[x]` A1 |
| `Codec::from_version` errors on 772 ("Phase A5") | `codec/mod.rs` | `[x]` guard in place |
| `ProtocolCaps` invariants test | `crates/tfs-rust-common/tests/protocol_caps.rs` | `[x]` |
| Golden-byte 1098 tests | `crates/tfs-rust-net/tests/protocol_compat.rs`, `tests/map_description.rs` | `[x]` (1098 only) |

**Not yet started:** A2 (per-version opcodes), A3 (transport gating), A4 (login gating), A5
(`Codec772`), A6 (wire-up). Entire Track B (`MechanicsProfile` / `data/formulas/`).

> **Gap to close in A2** even though A0/A1 are done: `ProtocolCaps` currently models per-field booleans
> but `Codec1098` does **not yet read them** — it writes 1098 layout unconditionally. A3/A4/A5 will make
> the codec consult `caps()`. That's expected; A1's mandate was "identical bytes," not "caps-driven."

---

## Track A — Wire / protocol

Each phase compiles green, keeps 1098 byte-identical, and adds 772 only at A5. Verification gate after
every phase: `cargo check --workspace && cargo clippy --workspace --all-targets && cargo test -p tfs-rust-net && cargo test -p tfs-rust-common`.

### Phase A2 — Per-version opcodes & semantic ops `[x]`

**Goal:** opcode values become version-keyed data, never inlined; client→server decode is semantic.

**C++ ref:** 1098 repo-root `src/protocolgame.cpp` `parsePacket`; 772 `gameserver/src/protocolgame.cpp`
`parsePacket` (switch ~L466–528) + send-opcode constants. Design §2.7, §4.3.

**Files (done):**
- `crates/tfs-rust-common/src/protocol_opcodes.rs` — added `client::v772` (rule-violation trio
  `0x9B/0x9C/0x9D`), version-keyed `client::is_supported(op, version)` (1098 set − `V772_REMOVED` +
  `V772_ADDED`), and `server::self_appear(version)` sourced from `ProtocolCaps`.
- `crates/tfs-rust-net/src/game_parse.rs` — `parse_game_packet` / `parse_game_opcode` now take
  `ProtocolVersion` and reject opcodes the active era doesn't dispatch.
- `crates/tfs-rust-net/src/protocol_game.rs` — `game_command_from_payload` +
  `forward_game_packets[_xtea]` thread `ProtocolVersion`.
- `crates/tfs-rust-net/src/server.rs` — passes `wire.protocol_version` into the forwarder.
- `crates/tfs-rust-net/src/codec/v1098.rs` — self-appear opcode via `server::self_appear`.

**Tasks:**
- [x] A2.1 Inventory raw opcode literals (audited; legacy 1098/OTCv8 send builders in
      `outgoing_extra.rs` left as-is — they are 1098-only and move to `Codec1098`/`Codec772` in A5,
      not A2).
- [x] A2.2 Version-keyed send-opcode accessor (`server::self_appear`); migrated the codec call site.
      Bytes unchanged.
- [x] A2.3 Added 772 incoming opcode classification cited to `gameserver/src/protocolgame.cpp`
      `parsePacket`. Era-exclusive 1098 blocks (shop/market/quest/mount/equip/wrap/seek-browse/
      VIP-edit + `0xF2`) flagged via `V772_REMOVED`; 772 rule-violation trio via `V772_ADDED`.
- [x] A2.4 Threaded `ProtocolVersion` into dispatch; 1098 parse path byte-for-byte unchanged.

**Tests (done):** `protocol_compat.rs` 1098 module unchanged (regression gate green).
`protocol_caps.rs` gained `server_self_appear_opcode_is_version_keyed` (0x0A vs 0x17) and
`client_opcode_support_matrix`. `protocol_game_dispatch.rs` gained 772-rejects-1098-opcode and
772-accepts-rule-violation cases.

**Gate:** ✅ all existing tests pass; no behavior change for 1098.

> **Deferred to A5 (intentional):** semantic `GamePacket` parsing for the 772 rule-violation trio
> (`0x9B/0x9C/0x9D`) and per-version *server* opcode tables beyond self-appear. A2 establishes the
> version-keyed dispatch seam + the one genuinely-divergent send opcode; the rest of the send opcodes
> share byte values across eras (only layout differs, which is the codec's job in A5).

---

### Phase A3 — Transport capability gating `[x]`

**Goal:** Adler32, pre-login challenge, and XTEA slack honor `ProtocolCaps`. 1098 defaults intact.

**C++ ref:** 772 `gameserver/src/protocol.cpp` `XTEA_decrypt` (no checksum, `getLength() - 4`),
`networkmessage.h` (no checksum, `INITIAL_BUFFER_POSITION = 4`), `connection.cpp` (no `onConnect`
challenge); 1098 repo-root `src/protocol.cpp` `XTEA_decrypt` (`getLength() - 6`), `networkmessage.h`
(`INITIAL_BUFFER_POSITION = 8`, 4-byte checksum), `connection.cpp` (reads + verifies checksum). Design
§2.1.

**Files (done):**
- `crates/tfs-rust-net/src/protocol_game.rs` — `decrypt_xtea_game_body` / `encrypt_xtea_game_frame` now
  take `&ProtocolCaps`. Cipher region offset = `caps.initial_buffer_position - 4` (4 for 1098, 0 for
  772 = checksum width); Adler header read/write gated on `caps.adler_checksum`. `forward_game_packets_xtea`
  threads caps.
- `crates/tfs-rust-net/src/game_challenge.rs` — `send_game_challenge(&mut w, &caps)` returns
  `Option<GameChallenge>`; emits `0x1F` only when `caps.prelogin_challenge`.
- `crates/tfs-rust-net/src/server.rs` — game + login connections pass caps to encrypt/decrypt; challenge
  echo verified only when a challenge was issued.
- `tools/packet-proxy/src/{decrypt,connection}.rs` — caller threads 1098 caps (proxy logs 10.98 only).

**Tasks:**
- [x] A3.1 Plumb `ProtocolCaps` into the decrypt/encrypt fns (passed from the conn's wire config).
- [x] A3.2 Gate Adler header read/write + buffer offset by `adler_checksum` / `initial_buffer_position`.
- [x] A3.3 XTEA recv slack `-4` vs `-6` subsumed by the caps-driven cipher offset — the C++ slack is a
      `getLength()`/`getBufferPosition()` artifact, not a value to subtract in Rust (decrypt processes the
      exact 8-byte-aligned region). See `tasks/lessons.md` #19.
- [x] A3.4 Gate the pre-login challenge send by `prelogin_challenge`.

**Tests:** `tests/xtea_game_body.rs` — round-trip under both caps profiles (1098 with Adler, 772 no
checksum) + a cross-profile guard (1098 decode of a 772 frame must not silently reproduce the payload).
Inline `protocol_game::encrypt_tests` gained the 772 no-checksum round-trip. 1098 round-trip unchanged.

**Gate:** ✅ 1098 frames byte-identical (offset 4 == old hardcoded `body[4..]`); 772-caps round-trip
succeeds in unit test (no live client yet). `cargo check --workspace`, `cargo test -p tfs-rust-net
-p tfs-rust-common` green. (Pre-existing clippy `too_many_arguments` / `items_after_test_module` baseline
in the crate is unchanged by A3.)

---

#### A3 historical detail (original plan)

**Files:**
- `crates/tfs-rust-net/src/protocol_game.rs` — `decrypt_xtea_game_body` / `encrypt_xtea_game_frame`:
  read `caps.adler_checksum`, `caps.initial_buffer_position`, `caps.xtea_length_slack` instead of the
  hardcoded `-6` / Adler header.
- `crates/tfs-rust-net/src/server.rs` — `handle_game_connection`: send the `0x1F` challenge only when
  `caps.prelogin_challenge`.
- `crates/tfs-rust-net/src/game_challenge.rs`, `adler.rs` — algorithms stay shared (R9); only *whether*
  applied moves behind caps.

**Tasks:**
- [ ] A3.1 Plumb `ProtocolCaps` into the decrypt/encrypt fns (they already have access to the conn's
      wire config — pass caps through).
- [ ] A3.2 Gate Adler header read/write + buffer offset by `adler_checksum` / `initial_buffer_position`.
- [ ] A3.3 Gate XTEA recv slack `-4` vs `-6` by `xtea_length_slack`.
- [ ] A3.4 Gate the pre-login challenge send by `prelogin_challenge`.

**Tests:** `crates/tfs-rust-net` round-trip — XTEA frame encode→decode under both caps profiles
(with/without Adler, slack 4 vs 6). 1098 round-trip identical to before.

**Gate:** 1098 frames byte-identical; 772-caps round-trip succeeds in unit test (no live client yet).

---

### Phase A4 — Login capability gating `[ ]`

**Goal:** login parse/encode branch on `account_name_login` / `session_key_login`; DB gains an
account-number auth path; self-appear opcode gated.

**C++ ref:** 772 `gameserver/src/protocollogin.cpp` + `protocolgame.cpp` (`u32` account number, inline
GM flag + account + char + password, char-list entry = name + world + `u32` IP + `u16` port, premium
`u16` days, self-appear `0x0A` + `u16` beat + `u8` canReportBugs); 1098 repo-root `src/`. Design §2.2,
§2.6.

**Files:**
- `crates/tfs-rust-net/src/game_first_packet.rs` — `parse_login_first` / `parse_game_first`: branch
  account identity + credential block + 2FA presence on caps.
- `crates/tfs-rust-net/src/protocol_login_out.rs` — char-list shape + premium representation per caps.
- `crates/tfs-rust-net/src/pending_login.rs` — carry account-number identity variant.
- `crates/tfs-rust-net/src/server.rs` — session-key packet `0x28` only when `session_key_login`.
- `crates/tfs-rust-db/src/...` (auth) — add `gameworld_authentication_by_number` /
  `loginserver_authentication_by_number`; keep name-based path for 1098 (design §3.4). This is the only
  version-specific DB surface (auth identity, char-list query, premium repr).
- self-appear: route through codec `encode_self_appear_login` using `caps.self_appear_opcode`.

**Tasks:**
- [ ] A4.1 Model login identity as an enum (`AccountName(String)` | `AccountNumber(u32)`); fill from caps.
- [ ] A4.2 Branch credential parse (session key string vs inline GM+acct+char+pass).
- [ ] A4.3 DB account-number auth query (compile-time checked `query_as!`, `tfs-database` rules).
- [ ] A4.4 Char-list + premium encode per caps (`u16` days vs `u8` flag + `u32` ts).
- [ ] A4.5 Gate `0x28` session-key send; gate self-appear opcode via caps.

**Tests:** login encode golden bytes for 1098 unchanged. Add 772 char-list/self-appear golden once A5
land (or stub with caps + neutral struct now, byte-assert in A5).

**Gate:** 1098 login flow unchanged end-to-end (smoke via `examples/game_login_smoke.rs`).

---

### Phase A5 — Implement `Codec772` `[ ]`

**Goal:** full 772 byte layouts behind the existing `ProtocolCodec` trait; `Codec` enum gains
`V772(Codec772)` and `from_version(772)` stops erroring.

**C++ ref (772 wire — `gameserver/src/` ONLY, cite file+function in module header):**
- `networkmessage.cpp ~L82–106` `addItem` (2-byte min: `u16 clientId` [+`u8 count`] [+`u8 liquidColor`],
  no MARK/animation/description/duration). Verify fluid via `getLiquidColor()` in `tools.cpp` — **do not**
  reuse the 10.x `FLUID_MAP` without confirming.
- `protocolgame.cpp ~L2051` `AddCreature` (no creature-type byte, no guild emblem, no speech bubble, no
  MARK, no helpers, no walkthrough; **full `getStepSpeed()`**, i.e. `speed_halved = false`).
- `AddOutfit` (no addons, no mount; `lookType==0` → `u16 lookTypeEx`).
- `AddPlayerStats ~L2090` (`u16` cap = `freeCapacity/100`, `u32` exp, `u8`+`u8`% magic level, no
  xp-rate/stamina block).
- `AddPlayerSkills` (7 skills × `u8` level + `u8`%).
- `sendIcons 0xA2` (`u8`, not `u16`).
- `sendContainer 0x6E` (cid+item+name+`u8` cap+`u8` hasParent+`u8` count+items; no unlock/pagination/
  `u16` size/firstIndex). `sendAddContainerItem 0x70` (no slot index). Container slot updates `u8`.

**Files:**
- `crates/tfs-rust-net/src/codec/v772.rs` (new) — `Codec772` impl of every `ProtocolCodec` method,
  narrowing the neutral wire structs to 772 widths. Module header cites each `gameserver/src/` ref.
- `crates/tfs-rust-net/src/codec/mod.rs` — add `mod v772; pub use v772::Codec772;`, add `Codec::V772`
  arm to the enum + every `delegate_codec!` match + `from_version(772) => Ok(V772(..))`.
- If 772 needs fields not in the neutral structs, **widen the wire struct** (never pre-narrow in core,
  design §9.5) — e.g. `OutfitWire` already carries mount; `Codec772` just omits it.

**Tasks:**
- [ ] A5.1 `write_item_template` / `item_template_wire_len` — 772 (count + liquidColor only; verify
      `getLiquidColor`).
- [ ] A5.2 `write_add_creature` / `add_creature_wire_len` — 772 known/unknown headers; full step speed.
- [ ] A5.3 `write_outfit` — 772 (no addons/mount; lookTypeEx path).
- [ ] A5.4 `encode_player_stats` — 772 widths (`u16` cap, `u32` exp).
- [ ] A5.5 `encode_player_skills` — 772 (7 × `u8`/`u8`%).
- [ ] A5.6 container family (`sendContainer`/`add`/`update`, `u8` slot) — 772 shapes.
- [ ] A5.7 tile/inventory item + creature add/remove + creature light/turn + cancel-walk — 772.
- [ ] A5.8 `encode_self_appear_login` — 772 (`0x0A` + `u16` beat + `u8` canReportBugs).
- [ ] A5.9 Wire `Codec772` into the `Codec` enum and unblock `from_version(772)`.

**Tests:** `crates/tfs-rust-net/tests/protocol_compat.rs` — add a `mod v772` sibling to the 1098 module
(R10). Golden bytes for item (stackable/fluid/plain), creature (known/unknown), outfit (looktype/
item-outfit), stats, skills, container open, self-appear. Where exact bytes are uncertain, capture from
`gameserver/` via `tools/packet-proxy` and freeze as fixtures (design §7) — **never guess**.

**Gate:** 1098 goldens unchanged; 772 goldens match `gameserver/src/` (cited). `cargo test -p tfs-rust-net`.

---

### Phase A6 — Wire it up & document `[ ]`

**Goal:** `clientVersion = 772` selects `Codec772` end-to-end; documented deviations.

**Tasks:**
- [ ] A6.1 Confirm `resolve_protocol_version` → `Codec::from_version(772)` → per-connection codec path
      works with no remaining `*_1098` direct imports in core (grep the §3.5 emission call sites:
      `game_world.rs`, `login_out.rs`, `walk.rs`, `container_ui.rs`, `game_world_inventory.rs`,
      `player_inventory_notifications.rs`, `spawn_lifecycle.rs`, `player_ping.rs`).
- [ ] A6.2 Smoke-test with a real 7.72 client (login → world render → walk → container).
- [ ] A6.3 Update `docs/PROJECT_STATUS.md`, `tasks/lessons.md`, module C++ ref headers.
- [ ] A6.4 Flag content/asset prerequisite (772 `items.otb`/`.spr`/`.dat`/OTBM) in `tfs-rust-content`
      as a separate follow-up (design §11) — wire alone won't run a 772 server.

**Gate:** 772 client connects and renders correctly; 1098 fully unaffected.

---

## Track B — Mechanics (`MechanicsProfile` + `data/formulas/`)

Source of truth: **`tibia-game-master/src/`** (CipSoft outcomes, clean-room R12) for behavior; cite
TFS structure (`gameserver/src/`, repo-root `src/`) for style. Behavior stays 1098 until B5. Each
extracted constant becomes a `MechanicsProfile` field / `data/formulas/<version>.lua` value (R11) —
never a bare Rust literal.

Verification gate per phase: `cargo check -p tfs-rust-core && cargo clippy -p tfs-rust-core --all-targets && cargo test -p tfs-rust-core`.

### Phase B0 — `MechanicsProfile` + Lua loader (no behavior change) `[ ]`

**Goal:** add the profile value + version-keyed Lua loader; 1098 values match today's constants.

**Files:**
- `crates/tfs-rust-core/src/formulas.rs` (new) — `MechanicsProfile` struct (mirror of `ProtocolCaps`)
  with §12.11 knobs: `beat_ms`, path-cost model enum, attack-speed source, armor-reduction mode,
  fight-mode modifiers, weakest-target metric, distance-keep range, damage-formula selector,
  condition-tick constants (fire 10/8, energy 25/10, poison decay), spawn-near-player policy, exp
  attribution window, PvP exp cap, spell coefficients, level-exp polynomial coeffs. Plus
  `MechanicsProfile::for_version(v)` built-in defaults, and a Tier-2 hook registry
  (`Option<LuaFn>` per hook, design §12.13 table).
- `data/formulas/1098.lua` (new) — TFS 1.4.2 defaults (`beatMs=50`, `attackSpeedMs=0`, `armor="full"`, …).
- `data/formulas/772.lua` (new) — CipSoft defaults (`beatMs=200`, `attackSpeedMs=2000`,
  `armor="randomized"`, fight modes, condition ticks).
- `crates/tfs-rust-core/src/config.rs` / `run_server.rs` — load `data/formulas/<clientVersion>.lua` at
  startup into `MechanicsProfile`; missing file → `for_version` defaults. Thread onto the game thread
  alongside `Codec` (game-thread-only, `tfs-threading`).
- `crates/tfs-rust-lua/...` — register (but no-op) the Tier-2 override hooks.

**Tasks:**
- [ ] B0.1 Define `MechanicsProfile` + enums + `for_version`.
- [ ] B0.2 Author both `data/formulas/*.lua` (Tier-1 constants + commented Tier-2 examples).
- [ ] B0.3 Loader (reuse existing Lua runtime; resolve by `clientVersion`; fallback to defaults).
- [ ] B0.4 Thread profile onto game thread; expose to mechanics modules read-only.
- [ ] B0.5 Bind Tier-2 hook registry (registered, unregistered = native fast path).

**Tests:** unit-test loaded 1098 profile == today's hardcoded constants; missing-file fallback ==
`for_version`; 772 file loads the CipSoft knobs.

**Gate:** zero behavior change (1098 profile drives identical outcomes). Mechanics modules still use
their current literals — B1–B4 migrate them.

---

### Phase B1 — Movement & scheduling `[ ]`

**CipSoft ref:** `cract.cc` `TCreature::Execute`/`CalculateDelay`/`NotifyGo`, `crmain.cc:445` `GetSpeed`.
**TFS structure:** `creature.cpp:185`. Design §12.1–§12.2.

**Files:** `crates/tfs-rust-core/src/walk.rs`, `creature_think.rs`.

**Tasks:**
- [ ] B1.1 Beat quantization (200 vs 50 ms) reads `profile.beat_ms`.
- [ ] B1.2 Step-delay formula reads profile; keep TFS speed formula (`2*(base+var)+80`), profile only
      the quantizer (design §12.2: `wp=100,speed=200` → 772 600 ms vs 1098 500 ms).
- [ ] B1.3 Optional `getStepDuration` / `getCreatureSpeed` Tier-2 hooks honored if registered.

**Tests:** unit-test step-duration under both profiles matches the §12.2 worked example.

---

### Phase B2 — Pathfinding `[ ]`

**CipSoft ref:** `cract.cc:7–262` `TShortway` (reverse A*, terrain-weighted `WAYPOINTS`, diagonal 3×).
**TFS structure:** `map.cpp:689` `getPathMatching`. Design §12.3.

**Files:** `crates/tfs-rust-core/src/pathfinding.rs`.

**Tasks:**
- [ ] B2.1 Add a path-cost model selector to `get_path_matching` (`profile.path_cost`): 1098 keeps
      fixed 10/25 (+creature 30, field 180); 772 = terrain-speed-weighted, diagonal 3× tile cost.
- [ ] B2.2 Keep the algorithm shared; only the edge-cost function diverges.

**Tests:** path over mixed terrain differs between profiles per §12.3; 1098 path unchanged.

---

### Phase B3 — Monster AI `[ ]`

**CipSoft ref:** `crnonpl.cc` `IdleStimulus`/`Strategy[4]`/`IsFleeing`/distance-4/`MonsterhomeInRange`.
**TFS structure:** `monster.cpp`. Design §12.4–§12.5.

**Files:** `crates/tfs-rust-core/src/monster_ai.rs`, `monster_distance_step.rs`, `spawn_lifecycle.rs`,
`spawn.rs`.

**Tasks:**
- [ ] B3.1 Weakest-target metric via profile: current HP (772) vs max HP (1098).
- [ ] B3.2 Distance-keep range via profile: hardcoded 4 (772) vs per-type `target_distance` (1098).
- [ ] B3.3 Lose-target roll + Strategy[3]=RANDOM 4th bucket per CipSoft.
- [ ] B3.4 Spawn-near-player policy: radius-shrink (772) vs block (1098); respawn scaling already shared.
- [ ] B3.5 **NPCs stay Lua-only** for both eras (design §12.6) — no `.ndb` engine in Rust. 772 NPC
      content (`gameserver/data/npc/behavior/*.ndb` → `data/npc/scripts/*.lua`) is out-of-band content
      work, NOT a Rust task here.

**Tests:** target selection on wounded creatures differs by metric; distance-step behavior per profile.

---

### Phase B4 — Combat, skills, conditions, magic `[ ]`

**CipSoft ref:** `crcombat.cc` (attack 2000 ms +200 lead-in, defense 2000 ms gate, `ProbeValue` damage,
randomized armor `(A/2)+rand(A/2)`, fight-mode %, 20-slot exp distribution, PvP cap 11/10, 60-round
window), `crskill.cc` (level-exp polynomial, geometric skill tries, DoT timer-skills fire 10/8 energy
25/10 poison decay), `magic.cc:776` (spell `2*lvl+3*ml`). **TFS structure:** `weapons.cpp`,
`creature.cpp:500–533`, `vocation.cpp`, `condition.cpp:1330`, `spells.cpp`. Design §12.7–§12.10.

**Files:** `crates/tfs-rust-core/src/combat/mod.rs`, `combat/pvp.rs`, `condition.rs`, `spell.rs`,
skills/exp module (`creature/vocation.rs` + new as needed). Note: combat damage and condition ticks are
**skeleton/merge-only today** (design §12.7, §12.9) — this phase builds the math on top.

**Tasks:**
- [ ] B4.1 Attack/defense cadence via profile (flat 2000 ms 772 vs `getAttackSpeed` 1098) +
      `getAttackSpeed` Tier-2 hook.
- [ ] B4.2 Melee damage `max(0, Attack−Defense)` then armor; `getWeaponDamage` (`ProbeValue`) +
      `getDefense` hooks; validate vs captured CipSoft values (clean-room).
- [ ] B4.3 Armor reduction mode via profile (randomized 772 vs full 1098) + `getArmorReduction` hook.
- [ ] B4.4 Fight-mode modifiers from profile (CipSoft ±20/40/80, balanced 0%; TFS 1.2×).
- [ ] B4.5 Exp distribution (20-slot proportional, PvP cap, attribution window) + `getExperienceForLevel`
      polynomial + `getReqSkillTries` geometric hooks.
- [ ] B4.6 Condition ticks (Phase G dependency): implement DoT ticking using
      `profile.conditions` (fire 10/8, energy 25/10, poison decay) + `getConditionTick` hook.
- [ ] B4.7 Spell damage `2*lvl+3*ml` with clamp flags + `getSpellDamage` hook.

**Tests:** golden numeric tests for damage/armor/exp/skill-tries/condition-tick under both profiles,
validated against captured CipSoft outputs (772) and current TFS values (1098).

---

### Phase B5 — Flip & validate the 772 profile `[ ]`

**Tasks:**
- [ ] B5.1 `clientVersion = 772` loads `data/formulas/772.lua`; validate step cadence, monster behavior,
      combat numbers, DoT ticks against CipSoft constants.
- [ ] B5.2 Capture lessons (`tasks/lessons.md`); document chosen CipSoft↔TFS deviations.

**Gate:** 1098 shard outcomes unchanged; 772 shard matches CipSoft within validated tolerances.

---

## Dependency graph

```
A0 [x] ─► A1 [x] ─► A2 [x] ─► A3 [x] ─► A4 ─► A5 ─► A6   (wire: connectable 772, 1098-behaving)
                                   │
B0 ──► B1 ──► B2 ──► B3 ──► B4 ──► B5                  (mechanics: 772 behavior; B0 needs only A0)
```

- Track A and Track B are independent after A0. B0–B4 can be built/validated on the 1098 shard before
  A5/A6 land.
- A6 (connectable 772) + B5 (772 behavior) together = a faithful 772 shard.
- **Out of scope here (content track):** 772 assets (`items.otb`/`.spr`/`.dat`/OTBM) and 772 NPC Lua
  conversion (design §11, §12.6) — required to actually *run* a 772 server, tracked separately.

## Per-phase verification checklist (apply every phase)

1. `cargo check --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test -p tfs-rust-net -p tfs-rust-common` (Track A) / `cargo test -p tfs-rust-core` (Track B)
4. 1098 golden bytes / outcomes **unchanged** (regression gate).
5. New version-specific code carries a C++ ref comment (correct tree per era) and a `// PROTOCOL:` /
   profile tag where applicable.
6. New constants live in `protocol_opcodes.rs` / `ProtocolCaps` / `MechanicsProfile` — never inlined.
