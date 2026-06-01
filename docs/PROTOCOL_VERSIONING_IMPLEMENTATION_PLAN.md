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

**Not yet started:** Entire Track B (`MechanicsProfile` / `data/formulas/`). **Done:** A0–A6 (full
wire track — 772 is connectable; 1098 byte-identical).

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

### Phase A4 — Login capability gating `[x]`

**Goal:** login parse/encode branch on `account_name_login` / `session_key_login`; DB gains an
account-number auth path; self-appear opcode gated.

**C++ ref:** 772 `gameserver/src/protocollogin.cpp` (`onRecvFirstMessage`: `u32` accountNumber +
`string` password; `getCharacterList`: char = name + serverName + `u32` IP + `u16` port, premium
`u16` days; `disconnectClient` = `0x0A`) + `gameserver/src/protocolgame.cpp` `onRecvFirstMessage`
(gm flag + `u32` accountNumber + char + password) + `gameserver/src/iologindata.cpp`
(account number = `accounts.id`); 1098 repo-root `src/`. Design §2.2, §2.6.

**Files (done):**
- `crates/tfs-rust-net/src/game_first_packet.rs` — added `LoginIdentity` enum
  (`AccountName(String)` 1098 | `AccountNumber(u32)` 772); `parse_first_client_packet` /
  `parse_first_game_packet` now take `&ProtocolCaps`. RSA-offset candidates + checksum handling are
  caps-driven (1098 set unchanged; 772 adds checksum-free game off 5 / login off 17). Credential
  parse split into testable `parse_game_credentials` / `parse_login_credentials`.
- `crates/tfs-rust-net/src/protocol_login_out.rs` — `LoginSuccess` neutral struct +
  `build_login_success(caps, &LoginSuccess)` and `build_login_error(caps, msg)`. 1098 path byte-
  identical to the legacy `build_login_success_packet` (kept as a thin shim). 772 path: no `0x28`,
  per-char `name+server+u32 ip+u16 port`, `u16` premium days, `0x0A` error opcode.
- `crates/tfs-rust-db/src/account.rs` — added `loginserver_authentication_by_number` /
  `gameworld_authentication_by_number` (`accounts.id`); refactored shared verify/premium helpers.
- `crates/tfs-rust-net/src/server.rs` — login + game handlers pass caps to parse and branch DB auth
  on `LoginIdentity`; emit via the new caps-gated builders.
- `tools/packet-proxy/src/connection.rs` — threads 1098 caps into `parse_first_game_packet`.

**Tasks:**
- [x] A4.1 `LoginIdentity` enum (name | number), filled from caps.
- [x] A4.2 Branch credential parse (1098 session key vs 772 inline acct-number + char + password).
- [x] A4.3 DB account-number auth queries (`accounts.id`, bind-parameterized).
- [x] A4.4 Char-list + premium encode per caps (`u16` days vs `u8` flag + `u32` ts).
- [x] A4.5 `0x28` session-key send gated (772 omits it); self-appear opcode already version-keyed (A2).

**Tests (done):** lib unit tests — 1098/772 game + login credential parse, 772 char-list layout,
`0x0A`/`0x0B` error opcode, premium-days math, `inet_addr` LE bytes, and a 1098
success-byte-identical regression vs the legacy builder. `protocol_compat.rs` 1098 goldens unchanged.

**Gate:** ✅ `cargo check --workspace`, `cargo clippy --workspace --all-targets`,
`cargo test -p tfs-rust-net -p tfs-rust-common -p tfs-rust-db` green. 1098 login bytes unchanged.

> **Deferred to A5/A6 (intentional):** exact 772 first-packet *framing* offsets (the `0x0A`/`0x01`
> proto-id byte handling and prelude widths) are best-effort from `gameserver/src/` and tagged
> `// PROTOCOL:`; confirm against a live 7.72 capture before flipping `clientVersion = 772`. The 772
> self-appear *payload* (`0x0A` + `u16` beat + `u8` canReportBugs) lands with `Codec772` in A5.9.

---

### Phase A5 — Implement `Codec772` `[x]`

**Goal:** full 772 byte layouts behind the existing `ProtocolCodec` trait; `Codec` enum gains
`V772(Codec772)` and `from_version(772)` stops erroring. **Done.**

**C++ ref (772 wire — `gameserver/src/` ONLY, cited in `codec/v772.rs` module header):**
- `networkmessage.cpp ~L82` `addItem` (2-byte min: `u16 clientId` [+`u8 count`] [+`u8 liquidColor`],
  no MARK/animation/description/duration). Fluid via `tools.cpp ~L20` `getLiquidColor` — confirmed a
  **distinct switch**, not the 10.x `FLUID_MAP` (e.g. `6 → 4`, not `9`).
- `protocolgame.cpp ~L2051` `AddCreature` (no creature-type byte, no guild emblem, no speech bubble, no
  MARK, no helpers, no walkthrough; **full `getStepSpeed()`**; raw light, no access-player `0xFF`).
- `AddOutfit ~L2128` (no addons, no mount; `lookType==0` → `u16 lookTypeEx`).
- `AddPlayerStats ~L2090` (`u16` cap = `freeCapacity/100`, `u32` exp w/ overflow→0, `u8`+`u8`% magic
  level, no base-magic/stamina/speed/training block).
- `AddPlayerSkills ~L2118` (7 skills × `u8` level + `u8`%).
- `sendContainer 0x6E ~L1326` (cid+item+name+`u8` cap+`u8` hasParent+`u8` count+items; no unlock/
  pagination/`u16` size/firstIndex). `sendAddContainerItem 0x70 ~L1871` (no slot index).
  `sendUpdateContainerItem 0x71` (`u8` slot). Tile senders ~L1591; self-appear `0x0A` ~L1730.

**Files (done):**
- `crates/tfs-rust-net/src/codec/v772.rs` (new) — `Codec772` impl of every `ProtocolCodec` method,
  narrowing the neutral wire structs to 772 widths. Module header cites each `gameserver/src/` ref.
- `crates/tfs-rust-net/src/codec/mod.rs` — `mod v772; pub use v772::Codec772;`, `Codec::V772` arm on
  the enum + `caps()` + every `delegate_codec!` match + the `ProtocolCodec for Codec` block;
  `from_version(772) => Ok(V772(Codec772))`.
- `crates/tfs-rust-net/src/codec/wire.rs` — **widened** `AddCreatureWire.speed_half → step_speed`
  (full `getStepSpeed()`; 1098 codec halves, 772 writes full — design §9.5). New neutral
  `ContainerOpenWire` (max-width `sendContainer`; 1098 writes unlock/pagination/size/firstIndex, 772
  omits) routed via `encode_container_open`.
- `crates/tfs-rust-core/src/login_out.rs` — fills `step_speed` (full) instead of pre-narrowed half.
- `crates/tfs-rust-core/src/container_ui.rs` — builds `ContainerOpenWire` → `codec.encode_container_open`
  (was the 1098-only `send_container_open` helper, now `#[deprecated]`).
- `crates/tfs-rust-core/src/game_world.rs` — `enqueue_outgoing` drops empty packets (772 has no
  `sendBasicData` / by-id tile removal; those encoders return an empty message).

**Tasks:**
- [x] A5.1 `write_item_template` / `item_template_wire_len` — 772 (count + `getLiquidColor`; no mark/anim/desc).
- [x] A5.2 `write_add_creature` / `add_creature_wire_len` — 772 known/unknown headers; full step speed; raw light.
- [x] A5.3 `write_outfit` — 772 (no addons/mount; lookTypeEx path).
- [x] A5.4 `encode_player_stats` — 772 widths (`u16` cap=free/100, `u32` exp, overflow→0).
- [x] A5.5 `encode_player_skills` — 772 (7 × `u8`/`u8`%).
- [x] A5.6 container family (`sendContainer` via `ContainerOpenWire`, `add` no-slot, `update` `u8` slot) — 772 shapes.
- [x] A5.7 tile/inventory item + creature add/remove + creature light/turn + cancel-walk — 772.
- [x] A5.8 `encode_self_appear_login` — 772 (`0x0A` + id + `u16` beat + `u8` canReportBugs).
- [x] A5.9 Wire `Codec772` into the `Codec` enum and unblock `from_version(772)`.

> **Notes / deferred:** (1) `sendIcons 0xA2` (`u8` vs `u16`) is built by the standalone
> `outgoing_extra::send_icons` helper (a raw 1098 builder, not a `ProtocolCodec` method) — it is *not*
> yet caps-gated; tracked for a follow-up when icons routes through the codec. (2) The OTClient-on-772
> `stackpos` byte on `0x6A` is **omitted** (canonical 7.72 client), same way OTCv8 quirks are flagged
> for 1098 — confirm against a live capture before relying on OTClient-772. (3) `canReportBugs`
> defaults to 0 (account type not in the neutral self-appear signature).

**Tests (done):** `crates/tfs-rust-net/tests/protocol_compat.rs` gained a `mod v772` sibling (R10) with
golden bytes for item (plain/stackable/fluid + wire-len sync), outfit (looktype/item-outfit), creature
(known/unknown + wire-len), stats (+ exp overflow), skills, self-appear, container open (+ capacity
cap), tile/inventory/container item, remove-tile-thing, creature light/turn, cancel-walk, and the
empty-packet guard. Added a 1098 `container_open_1098_layout` regression for the refactored path.

**Gate:** ✅ 1098 goldens unchanged; 772 goldens match `gameserver/src/` (cited). `cargo test
--workspace`, `cargo clippy --workspace --all-targets` green.

---

### Phase A6 — Wire it up & document `[x]`

**Goal:** `clientVersion = 772` selects `Codec772` end-to-end; documented deviations. **Done** (live
client smoke test pending — needs a real 7.72 client, see A6.2).

**Tasks:**
- [x] A6.1 `resolve_protocol_version` → `Codec::from_version(772)` → per-connection codec path works.
      Verified **no** `Codec1098` / `*_1098` direct imports in core (grep clean); all §3.5 emission
      call sites route through `world.codec` (`game_world.rs`, `login_out.rs`, `walk.rs`,
      `container_ui.rs`, `game_world_inventory.rs`, `player_inventory_notifications.rs`,
      `spawn_lifecycle.rs`, `player_ping.rs`).
- [~] A6.2 Smoke-test with a real 7.72 client (login → world render → walk → container). **Deferred** —
      requires a live 7.72 client + 772 content (A6.4). Wire layouts are frozen as golden tests vs
      `gameserver/src/` in the meantime. Remaining login-choreography caps-gating (OTCv8-only preamble
      packets `0x43`/extended-opcode in the 1098 login burst) should be skipped for 772 before a live
      test — tracked here.
- [x] A6.3 Update `docs/PROJECT_STATUS.md`, `tasks/lessons.md`, `PROTOCOL_VERSIONING_IMPLEMENTATION_PLAN.md`,
      module C++ ref headers (`codec/v772.rs`).
- [x] A6.4 Flagged content/asset prerequisite (772 `items.otb`/`.spr`/`.dat`/OTBM) as a separate
      follow-up (design §11) — wire alone won't run a 772 server. **Update (June 2026):** the
      `items.otb` version gate now accepts the 772 pair (OTB major 2 / minor `CLIENT_VERSION_800` 7)
      as well as 1098 (major 3 / minor ≥ 57) — `crates/tfs-rust-content/src/otb.rs`, lessons #31.
      Remaining 772 assets (`.spr`/`.dat`/OTBM client ids) still per §11.

**Gate:** ✅ 1098 fully unaffected (goldens unchanged); 772 codec selectable and golden-verified. Live
772 client render is the remaining manual step (blocked on 772 content).

---

## Track B — Mechanics (`MechanicsProfile` + `data/formulas/`)

Source of truth: **`tibia-game-master/src/`** (CipSoft outcomes, clean-room R12) for behavior; cite
TFS structure (`gameserver/src/`, repo-root `src/`) for style. Behavior stays 1098 until B5. Each
extracted constant becomes a `MechanicsProfile` field / `data/formulas/<version>.lua` value (R11) —
never a bare Rust literal.

Verification gate per phase: `cargo check -p tfs-rust-core && cargo clippy -p tfs-rust-core --all-targets && cargo test -p tfs-rust-core`.

### Phase B0 — `MechanicsProfile` + Lua loader (no behavior change) `[x]`

**Done.** `MechanicsProfile` (Copy Tier-1) + `Mechanics { profile, hooks }` + `FormulaHooks` (Tier-2,
owns its `Lua`) in `crates/tfs-rust-core/src/formulas.rs`; `load_mechanics(data_dir, version)` overlays
`data/formulas/<v>.lua` onto `MechanicsProfile::for_version` (missing/partial file → era defaults).
Threaded onto `GameWorld.mechanics` (game thread), wired in `run_server`/`test_world`. Enums:
`PathCostModel`, `ArmorReduction`, `WeakestTargetMetric`, `DistanceKeep`, `DamageFormula`,
`LevelExpModel`, `SpawnNearPlayer`, plus `FightModes`/`ConditionTicks`/`SpellCoeff`/`TickSpec`. Tests:
1098 == defaults, 772 == CipSoft knobs, missing-file fallback, partial overlay, nested condition table,
Tier-2 used when registered.

---

### Phase B1 — Movement & scheduling `[x]`

**Done.** `walk.rs` step-duration helpers (`get_step_duration`, `get_step_duration_ms_with_direction`,
`get_walk_delay`, `get_event_step_ticks`) now take `&Mechanics`; the final quantizer uses
`profile.beat_ms` (50 TFS / 200 CipSoft) instead of a hardcoded `50.0`. TFS speed/log-curve kept; only
the quantizer is profiled. Tier-2 `getStepDuration(speed, ground, diagonal)` honored before the native
path. Tests: `beat_quantization_is_profile_driven` (772 = same raw value rounded to 200, `>=` 1098,
within one beat), `tier2_step_duration_hook_overrides_native`.

**CipSoft ref:** `cract.cc:1462` `NotifyGo` (`Delay=(wp*1000)/GetSpeed()`, `BeatCount=ceil`), `config.cc`
`Beat=200`, `crmain.cc:445` `GetSpeed`. **TFS:** `creature.cpp` `getStepDuration` (ceil to 50).

---

### Phase B2 — Pathfinding `[x]`

**Done.** `get_path_matching` gained a `cost_model: PathCostModel` + `ground_cost(pos)` closure.
`path_step_cost(Fixed, …)` = TFS 10/25 (1098, byte-identical); `path_step_cost(TerrainWeighted, …)` =
CipSoft current-tile waypoints, diagonal `×3` (`cract.cc:136–155` `TShortway::Expand` — the per-step
cost is the **current** node's `Waypoints`, not the destination's). Both callers (`monster_ai`,
`walk`) pass `self.mechanics.profile.path_cost` + a `tile_ground_speed` ground-cost closure. Tests:
`path_step_cost_fixed_is_tfs_10_25`, `path_step_cost_terrain_weighted_uses_ground_and_diagonal_3x`.

**CipSoft ref:** `cract.cc:7–262` `TShortway`. **TFS:** `map.cpp:689` `getPathMatching`.

---

### Phase B3 — Monster AI `[~]`

**Done (profile-gated knobs):** B3.1 weakest-target metric (`monster_weakest_opponent` +
`TargetSearchType::HealthLow`; current HP 772 / max HP 1098), B3.2 distance-keep
(`monster_effective_target_distance` applied at all 4 `m.target_distance` extraction sites; fixed 4 for
772 / per-type for 1098), B3.4 spawn-near-player (`poll_spawn_respawns` stalls on `Block` 1098 / never
stalls on `RadiusShrink` 772). Tests: `weakest_opponent_metric_follows_profile`,
`effective_target_distance_follows_profile`.

**Deferred (not era-divergent enough to gate now):** B3.3 lose-target roll + Strategy[3]=RANDOM 4th
bucket — the port's `monster_on_think_target` change-roll already covers the observable behavior; the
explicit CipSoft 4-bucket roulette is content/AI tuning, not a profile flag. B3.5 NPCs stay Lua-only
(design §12.6) — no Rust `.ndb` engine; unchanged.

**CipSoft ref:** `crnonpl.cc` `IdleStimulus`/`Strategy`/`IsFleeing`/distance-4/`MonsterhomeInRange`.

---

### Phase B4 — Combat, skills, conditions, magic `[x]`

**Done (formula engine).** Combat execution (`combat/mod.rs`) and condition ticking (Phase G) are still
skeleton, so B4 builds the *math* as pure, profile-driven, Tier-2-hookable fns in
`crates/tfs-rust-core/src/combat/math.rs`, ready to be called when the loops land. Each fn checks its
Tier-2 hook first (native fast path when unregistered).

- B4.1 `attack_speed_ms` (flat 2000 ms 772 vs vocation value 1098) + `defense_gate_ms` + Tier-2 `getAttackSpeed`.
- B4.2 `probe_value` (`((rand%100+rand%100)/2)*(attack*(skill*5+50))/10000`), `weapon_damage`,
  `defense_value`, `melee_damage_after_defense_and_armor` (`max(0,Atk−Def)` then armor) + Tier-2
  `getWeaponDamage`/`getDefense`.
- B4.3 `armor_reduction` (randomized `(A/2)+rand%(A/2)` 772 / full 1098) + Tier-2 `getArmorReduction`.
- B4.4 fight-mode multipliers (`apply_attack_mode`/`apply_defense_mode`; CipSoft 1.2/0.6/0.6/1.8, TFS
  1.2/0.8/0.8/1.2).
- B4.5 `distribute_experience` (20-slot proportional), `pvp_exp_cap` (11/10), `experience_for_level`
  (shared polynomial `(((L-6)*L+17)*L-12)/6 * delta`), `req_skill_tries` (geometric) + Tier-2 hooks.
- B4.6 `condition_tick` + `condition::dot_tick_for_condition` (fire 10/8, energy 25/10 from
  `profile.conditions`) + Tier-2 `getConditionTick`.
- B4.7 `spell_damage` + `spell::spell_damage_scaled` (`2*lvl+3*ml` % with `&4`/`&8` clamp flags) +
  Tier-2 `getSpellDamage`.

15 golden numeric tests under both profiles validate against the cited CipSoft / TFS values.

**Key finding:** TFS `getExpForLevel` (`player.h:171`) is the *same* polynomial as CipSoft
(`crskill.cc:352`) with `Delta=100` — not a separate cubic. Both `LevelExpModel` variants share it;
only `level_exp_delta` differs.

**CipSoft ref:** `crcombat.cc` (`GetAttackDamage`/`GetDefendDamage`/`GetArmorStrength`/`CloseAttack`),
`crskill.cc:535` `ProbeValue` / `:352` level exp / `:1064,1090` DoT, `magic.cc:784` `ComputeDamage`.

---

### Phase B5 — Flip & validate the 772 profile `[~]`

**Done (load-path validation).** `clientVersion = 772` selects `Codec772` (A6) **and** loads
`data/formulas/772.lua` into the 772 `MechanicsProfile` (B0 loader). `tests/mechanics_formulas.rs`
loads the *shipped* `772.lua`/`1098.lua` and asserts they equal the built-in era defaults end-to-end
(catches drift in the deployed files). Lessons captured (`tasks/lessons.md` #30).

**Remaining (needs a live shard / captured CipSoft values):** numeric validation of step cadence,
combat damage, and DoT ticks against real CipSoft captures — the formulas match the decompile's
*written* constants (cited + unit-tested), but end-to-end behavioral validation against a running 772
client is blocked on 772 content (same gate as A6.2). The combat/condition *execution* loops
(`combat/mod.rs`, Phase G) must also be built before in-game numbers can be observed.

**Tasks:**
- [x] B5.1 772 loads `data/formulas/772.lua`; shipped-file == era-defaults integration test.
- [x] B5.2 Lessons + CipSoft↔TFS deviations documented.
- [~] B5.3 Live numeric validation vs CipSoft captures — deferred (needs 772 content + combat loop).

---

## Dependency graph

```
A0 [x] ─► A1 [x] ─► A2 [x] ─► A3 [x] ─► A4 [x] ─► A5 [x] ─► A6 [x]   (wire: connectable 772, 1098-behaving)
                                   │
B0 [x] ─► B1 [x] ─► B2 [x] ─► B3 [~] ─► B4 [x] ─► B5 [~]   (mechanics: 772 profile + formula engine; B0 needs only A0)
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
