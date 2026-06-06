# TODO

## Bcrypt password migration — done
- [x] `tfs-rust-db/src/password.rs`: verify, hash, upgrade-on-login, unit tests
- [x] Refactor `account.rs` auth paths; wire `PasswordHashConfig` through net + `run_server`
- [x] SQLx migration widen `accounts.password`; update `schema.sql`, `config.lua`
- [x] `cargo test` / `cargo check` / clippy

## Config-and-save plan — Step 2 (run_server wiring)
- [x] Wire `run_server::run` listener bind addresses to `NetConfig` with env overrides preserved.
- [x] Wire database URL resolution to `DbConfig` when `DATABASE_URL` is unset.
- [x] Keep `TFS_GAME_PORT` / `TFS_PUBLIC_IP` env override behavior; fallback to `NetConfig`.
- [x] Add warning path for differing `statusProtocolPort` (listener not yet implemented).
- [x] Run `cargo check -p tfs-rust-core`.
- [x] Mark Step 2 complete in `tasks/config-and-save-plan.md`.

## Phase C (inventory / Lua) — done
- Runtime equipment slots, capacity, `internal_move_item` container↔inventory, quick-equip, equip/deequip `EventDispatcher` hooks, Lua `ItemRef` + `Player` inventory methods (`register_game_lua_item_hooks` + script cookie in `login.rs`).

## P1 — Player inventory cylinder queries — done
- [x] `player_query_remove`, `player_query_max_count`, `player_query_destination` (`player_inventory_query_add.rs`)
- [x] Wire `resolve_move_destination` + `internal_move_item` query chain (`container_ops.rs`, `game_world.rs`)
- [x] Unit tests: `player_max_count_index`, slot range constants
- [x] Update `docs/INVENTORY_STATUS.md`

## Terrain look parity — done
- [x] `LookTarget` enum + `internal_get_thing_look` (`STACKPOS_LOOK`, `game.cpp` ~223)
- [x] `Tile::top_visible_look_target` / `getTopVisibleThing` (`tile.cpp` ~322)
- [x] `player_look_at` ground + immovable terrain descriptions; `can_see_position` gate
- [x] Unit tests: `tile::look_tests`, `item_look::ground_water_description`
- [x] Update `docs/INVENTORY_STATUS.md`, `tasks/lessons.md`

## P4 — Depot & inbox runtime — done
- [x] Item typing: `is_depot()`, `depot_id` attributes, `DEPOT` tile flag, item constants
- [x] `player_depot.rs`: getInbox, getDepotChest, getDepotLocker, getMaxDepotItems, isNearDepotBox, last_depot_id
- [x] `load_depot_table` + `load_inbox_table`; login wiring; store inbox `ContainerType::StoreInbox`
- [x] Depot locker open branch in `container_ui.rs`; locker/inbox `queryAdd` rules
- [x] `DepotIsFull` in `container_ops.rs`; `depotFreeLimit` / `depotPremiumLimit` config
- [x] Live depot/inbox serialization in `game_world_save.rs`
- [x] Depot-owner container refresh in `player_inventory_notifications.rs`
- [x] Unit tests + `docs/INVENTORY_STATUS.md` update

## Ladder / grate UseItem teleport — done
- [x] `Tile::item_id_for_use` (`getUseItem` parity) + sprite-id fallback on map tiles
- [x] Defer `UseItem` on `nextAction` (not silent game-loop drop / cancel)
- [x] Immediate flush for `UseItem` / teleport packets
- [x] Unit tests: `tile::look_tests`, `game_loop::timed_action_gate_tests`

## P5 — Lua Container + Inventory API — done
- [x] Tranche 0: `find_item_of_type`, `ScriptContext` cylinder reads, `lua_script_*`, `fire_on_player_equip*`
- [x] Tranche 1: Player `getItemById`, container id round-trip bindings
- [x] Tranche 2: `getDepotChest`, `getInbox`, expanded `addItem`
- [x] Tranche 3: `Container` userdata (inherits Item)
- [x] Tranche 4: `item:moveTo`, `item:remove`
- [x] Tranche 5: item parent/position/attrs
- [x] Tranche 6: MoveEvents XML loader + equip dispatch (defer tile MoveEvents + `MoveEvent():register()`)
- [x] `docs/INVENTORY_STATUS.md`, `tasks/lessons.md` updated

## Phase D.1 — Generalize walk engine (Monster/NPC) — done
- [x] Route walk timing through `CreatureBase`; `step_speed_for_walk` (player clamp vs base speed)
- [x] `tile_query_add_monster` / `tile_query_add_npc` / `tile_query_add_creature`; `creature_can_stand_for_pathfind`
- [x] `internal_move_creature_step` (player height walk only); spectator `0x6D` via `creature_wire_id`
- [x] `creature_queue_walk_step` + `monster_walk_step_broadcasts_spectator_move` test
- [x] `cargo test -p tfs-rust-core monster_walk`

## Phase D.5 — Follow-on-target-move repath — done
- [x] `has_follow_path` gate in `monster_on_follow_creature_moved` (`creature.cpp` ~619)
- [x] Acceptance test `monster_repaths_when_follow_target_moves`
- [x] Fix `compute_look_toward_target` offset args (lesson #25 contract)
- [x] `cargo test --workspace` green

## Throw destination validation parity (B.5)
- [ ] Confirm C++ reference behavior for `Game::playerMoveItem` throw gating.
- [ ] Add Rust throw-destination validation before `internal_move_item`.
- [ ] Verify compile for `tfs-rust-core` after patch.

## Bugfix: inventory container move disappearance
- [ ] Confirm failure path in `internal_move_item` for `* -> Container` branches.
- [ ] Ensure destination can accept non-merge insert before source removal.
- [ ] Keep behavior parity for merge moves and normal successful inserts.
- [ ] Run `cargo check -p tfs-rust-core`.

## Audit: item parsers parity
- [x] Audit `items.otb` parser attribute/flag coverage vs TFS C++.
- [x] Audit `items.xml` parser key coverage vs TFS C++.
- [x] Write findings report with severity and recommended fixes.

## Implement: item parsers full parity pass
- [ ] Expand OTB `apply_attr` coverage for parity-relevant `itemattrib_t` fields.
- [ ] Expand XML typed key mapping (`apply_xml_attribute`) for runtime-relevant keys and aliases.
- [ ] Parse non-empty `<attribute>` blocks (`Event::Start`) including nested `field` sub-attributes.
- [ ] Switch container identity truth to OTB group (`group == ITEM_GROUP_CONTAINER`).
- [ ] Add parity diagnostics for unknown keys/types and duplicate XML item definitions.
- [ ] Add regression tests for OTB decode coverage, XML parity keys, nested attribute parsing, and container truth.
- [ ] Run `cargo test -p tfs-rust-content`, `cargo check`, and `cargo test -p tfs-rust-core`.

## Phase A0 — Protocol version scaffolding (Track A)
- [x] `ProtocolVersion`, `ProtocolCaps`, `ProtocolCaps::for_version` in `tfs-rust-common`
- [x] `clientVersion` in `config.lua.dist`; `resolve_protocol_version` + `TFS_PROTOCOL_VERSION` override
- [x] Thread `protocol_version` + `protocol_caps` on `GameWireConfig` / `LoginWireConfig`
- [x] Unit tests: `protocol_caps.rs` (1098 matrix + 772 invariants) + config tests
- [x] `cargo check --workspace`, `cargo test -p tfs-rust-common protocol_caps`, `cargo clippy`

## Phase A1 — Codec seam (10.98 only, Track A)
- [x] `codec/{mod,wire,v1098}.rs`: `ProtocolCodec`, `Codec1098`, `Codec` enum, `from_version` (772 rejected until A5)
- [x] `PlayerStatsWire`, `PlayerSkillsWire`, `ItemTemplateArgs`; deprecated shims in `outgoing_extra.rs`
- [x] `map_description.rs`: tile/map/move encoders take `&Codec`
- [x] `GameWorld.codec` + `enqueue_encoded`; `run_server` / `test_world` init
- [x] §3.5 rewire: `game_world`, `login_out`, `walk`, `container_ui`, `game_world_inventory`, `spawn_lifecycle`, `player_inventory_notifications`
- [x] Golden tests: `protocol_compat.rs` + `map_description.rs` via `Codec1098`
- [x] `cargo check --workspace`, `cargo test -p tfs-rust-net`

## Phase A3 — Transport capability gating (Track A) — done
C++ refs — 772: `gameserver/src/protocol.cpp` `XTEA_decrypt` (no checksum, `len-4`), `networkmessage.h`
`INITIAL_BUFFER_POSITION = 4`, `connection.cpp` (no `onConnect` challenge). 1098: repo-root
`src/protocol.cpp` `XTEA_decrypt` (4-byte Adler header, `len-6`), `networkmessage.h` IBP = 8,
`connection.cpp` checksum read.
- [x] A3.1 Plumb `ProtocolCaps` into `decrypt_xtea_game_body` / `encrypt_xtea_game_frame` (caps already on wire config).
- [x] A3.2 Gate Adler header read/write + buffer offset by `caps.adler_checksum` / `caps.initial_buffer_position` (cipher offset = `IBP - 4`).
- [x] A3.3 XTEA recv slack `-4` vs `-6` subsumed by the caps-driven cipher offset (772 = 0, 1098 = 4).
- [x] A3.4 Gate pre-login `0x1F` challenge send by `caps.prelogin_challenge`; only verify echo when sent (`Option<GameChallenge>`).
- [x] A3.5 Update callers: `server.rs` (game + login), packet-proxy `decrypt.rs`/`connection.rs`, tests.
- [x] Tests: XTEA frame encode→decode round-trip under both caps profiles; 772 no-checksum; cross-profile guard; 1098 unchanged.
- [x] Gate: `cargo check --workspace`, `cargo test -p tfs-rust-net -p tfs-rust-common` green (clippy errors are pre-existing baseline, unchanged by A3).

## Phase A4 — Login capability gating (772 / 1098) — done

Goal: login parse/encode branch on caps; DB gains account-number auth; 1098 byte-identical.
C++ refs — 772: `gameserver/src/protocolgame.cpp` `onRecvFirstMessage`, `protocollogin.cpp`
`onRecvFirstMessage`/`getCharacterList`/`disconnectClient`, `iologindata.cpp` `gameworldAuthentication`/
`loginserverAuthentication` (`accounts.id`). 1098: repo-root `src/protocollogin.cpp`, `protocolgame.cpp`.

- [x] A4.1 `LoginIdentity` enum (`AccountName(String)` 1098 | `AccountNumber(u32)` 772) in `game_first_packet.rs`; thread `&ProtocolCaps` into parse.
- [x] A4.2 Branch credential-block parse: 1098 session key (`acc\npass\ntoken\ntime` + char + challenge); 772 inline `[u8 gm][u32 acct][string char][string pass]` (game) / `[u32 acct][string pass]` (login). Split testable `parse_{game,login}_credentials`.
- [x] A4.3 DB `loginserver_authentication_by_number` / `gameworld_authentication_by_number` (`accounts.id`).
- [x] A4.4 `build_login_success`/`build_login_error` caps-gated: 772 = no `0x28`, per-char `name+server+u32 ip+u16 port`, `u16` premium days, `0x0A` error; 1098 byte-identical (legacy shim retained).
- [x] A4.5 `0x28` session-key send gated (772 omits); self-appear opcode already version-keyed (A2).
- [x] server.rs branches DB auth on `LoginIdentity`; packet-proxy threads 1098 caps.
- [x] Tests: 1098 encode/parse unchanged + new 772 credential/char-list units. `cargo check/clippy/test -p tfs-rust-net -p tfs-rust-common -p tfs-rust-db` green.

## Phase A5 — Implement `Codec772` (772 wire layouts) — done

C++ refs (772 wire — `gameserver/src/` ONLY):
- `networkmessage.cpp` `addItem` (2-byte min, no MARK/anim/desc/duration); fluid via `tools.cpp` `getLiquidColor`.
- `protocolgame.cpp` `AddCreature` (~2051), `AddPlayerStats` (~2090), `AddPlayerSkills` (~2118),
  `AddOutfit`, `AddCreatureLight` (~2149), `sendContainer`/`sendAddContainerItem`/`sendUpdateContainerItem`,
  tile item senders (~1591), `sendAddCreature` self branch (0x0A self-appear, ~1694), `sendCreatureTurn`,
  `sendCancelWalk`, `RemoveTileThing`.

- [x] A5.0 Widened neutral `AddCreatureWire.speed_half` → `step_speed` (full `getStepSpeed()`; design §9.5). 1098 codec writes `/2`, 772 writes full. New `ContainerOpenWire` (1098 writes unlock/pagination/size/firstIndex; 772 omits).
- [x] A5.1 `write_item_template` / `item_template_wire_len` — 772 (count + `getLiquidColor`; no mark/anim/desc).
- [x] A5.2 `write_add_creature` / `add_creature_wire_len` — 772 (no creature-type/emblem/bubble/MARK/helpers/walkthrough; full step speed; raw light).
- [x] A5.3 `write_outfit` — 772 (no addons, no mount; lookTypeEx path).
- [x] A5.4 `encode_player_stats` — 772 (`u16` cap=free/100, `u32` exp w/ overflow→0, no base-magic/stamina/speed block).
- [x] A5.5 `encode_player_skills` — 772 (7 × `u8` level + `u8`%).
- [x] A5.6 container open via `encode_container_open` (`0x6E` no unlock/pagination/size/firstIndex); add `0x70` no slot; update `0x71` `u8` slot; inventory `0x78`.
- [x] A5.7 tile item add/update/remove, add-tile-creature, creature light/turn, cancel-walk — 772.
- [x] A5.8 `encode_self_appear_login` — 772 (`0x0A` + id + `u16` beat + `u8` canReportBugs).
- [x] A5.9 `encode_basic_data` / `encode_remove_tile_creature_by_id` — 772 has none → empty msg; empty-skip guard in `enqueue_outgoing`.
- [x] A5.10 Wired `Codec::V772` into enum + every `delegate_codec!` arm + `from_version(772) => Ok(V772)`.
- [x] A5.11 Golden tests: `mod v772` in `protocol_compat.rs` (item/creature/outfit/stats/skills/self-appear/container/tile/light/turn/cancel + empty guard) + 1098 container-open regression.
- Deferred: `sendIcons 0xA2` `u8`-vs-`u16` (built by `send_icons` helper, not yet a codec method); OTClient-772 `0x6A` stackpos byte omitted (canonical 7.72 client).

## Phase A6 — Wire it up & document — done (live smoke test pending 772 content)

- [x] A6.1 `from_version(772)` end-to-end; no `Codec1098`/`*_1098` direct imports in core (grep clean); all §3.5 sites route through `world.codec`.
- [~] A6.2 Live 7.72 smoke test deferred (needs a real client + 772 content). Wire frozen as goldens vs `gameserver/src/`. Login-choreography caps-gating (OTCv8-only preamble) to skip for 772 before live test.
- [x] A6.3 Updated `docs/PROJECT_STATUS.md`, `tasks/lessons.md`, `PROTOCOL_VERSIONING_IMPLEMENTATION_PLAN.md`, `codec/v772.rs` C++ refs.
- [x] A6.4 Flagged 772 content prerequisite (items.otb/.spr/.dat/OTBM) as separate follow-up.
- [x] Gate: `cargo check/test --workspace`, `cargo clippy --workspace --all-targets` green; 1098 goldens unchanged.

## Track B — Mechanics (`MechanicsProfile` + `data/formulas/`)

Source of truth: `tibia-game-master/src/` (CipSoft outcomes, R12) for 772 behavior; cite TFS structure
(`gameserver/src/`, repo-root `src/`) for style. Behavior stays 1098 until B5. Every extracted constant
becomes a `MechanicsProfile` field / `data/formulas/<v>.lua` value (R11) — never a bare Rust literal.

### Phase B0 — `MechanicsProfile` + Lua loader (no behavior change)
- [x] B0.1 `crates/tfs-rust-core/src/formulas.rs`: `MechanicsProfile` (Copy Tier-1 data) + enums
      (`PathCostModel`, `ArmorReduction`, `WeakestTargetMetric`, `DamageFormula`, `LevelExpModel`,
      `SpawnNearPlayer`, `FightModes`, `ConditionTicks`) + `for_version(v)`.
- [x] B0.2 `data/formulas/1098.lua` (TFS 1.4.2 defaults) + `data/formulas/772.lua` (CipSoft defaults).
- [x] B0.3 Tier-1 loader (`load_mechanics`) via standalone `mlua::Lua`; missing file → `for_version`.
- [x] B0.4 `FormulaHooks` (Tier-2): owns formulas `Lua`; per-hook `Option`; native fast path.
- [x] B0.5 Thread `Mechanics` onto `GameWorld` (game thread); `run_server`/`test_world`.
- [x] B0.6 Tests: 1098 == defaults; missing-file fallback; 772 knobs; partial overlay; nested cond; Tier-2 used when registered.

### Phase B1 — Movement & scheduling
- [x] B1.1 `walk.rs` step quantization reads `profile.step_beat_ms` (50 ms both eras; TVP authority for 772) — not `beat_ms` (772 loop timer only).
- [x] B1.2 Kept TFS speed/curve; profile only the quantizer. Test `beat_quantization_is_profile_driven`.
- [x] B1.3 Tier-2 `getStepDuration` honored if registered (`tier2_step_duration_hook_overrides_native`).

### Phase B2 — Pathfinding
- [x] B2.1 `get_path_matching` edge cost via `path_step_cost(profile.path_cost, …)`: 1098 fixed 10/25; 772 terrain-weighted + diagonal 3×.
- [x] B2.2 Algorithm/search box shared; only `path_step_cost` diverges. `ground_cost` closure + `tile_ground_speed` threaded through both callers (`monster_ai`, `walk`). Tests `path_step_cost_*`.

### Phase B3 — Monster AI
- [x] B3.1 Weakest-target metric via profile (`monster_weakest_opponent` + `TargetSearchType::HealthLow`; current HP 772 / max HP 1098). Test `weakest_opponent_metric_follows_profile`.
- [x] B3.2 Distance-keep via profile (`monster_effective_target_distance`: hardcoded 4 for 772 / per-type for 1098) at all 4 extraction sites. Test `effective_target_distance_follows_profile`.
- [x] B3.4 Spawn-near-player policy via profile (`poll_spawn_respawns`: stall on `Block` 1098 / never stall on `RadiusShrink` 772).

### Phase B4 — Combat / skills / conditions / magic (formula engine on the skeleton)
- [x] B4.1 Attack/defense cadence (`attack_speed_ms`: flat 2000 ms 772 vs vocation 1098) + Tier-2 `getAttackSpeed`; `defense_gate_ms`.
- [x] B4.2 Melee `max(0, Atk−Def)` then armor (`melee_damage_after_defense_and_armor`); `probe_value` weapon/defense + Tier-2 `getWeaponDamage`/`getDefense`.
- [x] B4.3 Armor reduction mode (`armor_reduction`: randomized 772 / full 1098) + Tier-2 `getArmorReduction`.
- [x] B4.4 Fight-mode modifiers from profile (CipSoft ±20/40/80; TFS 1.2/0.8) — `apply_attack_mode`/`apply_defense_mode`.
- [x] B4.5 Exp distribution (`distribute_experience`, `pvp_exp_cap`) + `experience_for_level` polynomial + `req_skill_tries` geometric + Tier-2 hooks.
- [x] B4.6 Condition ticks via `profile.conditions` (`condition_tick` + `condition::dot_tick_for_condition`; fire 10/8, energy 25/10) + Tier-2 `getConditionTick`.
- [x] B4.7 Spell damage `2*lvl+3*ml` clamp flags (`spell_damage` + `spell::spell_damage_scaled`) + Tier-2 `getSpellDamage`.
- [x] B4.x 15 golden numeric tests under both profiles (`combat/math.rs`, `condition.rs`), validated vs CipSoft / TFS values.
- Note: math lives in `combat/math.rs` as pure, profile-driven, Tier-2-hookable fns; combat *execution* loop (`combat/mod.rs`) is still skeleton (design §12.7/§12.9) and will call these once wired.

### Phase B5 — Flip & validate the 772 profile
- [x] B5.1 `clientVersion = 772` loads `data/formulas/772.lua` (codec flips via A6). `tests/mechanics_formulas.rs` validates shipped 772/1098 files == era defaults end-to-end.
- [x] B5.2 Lessons captured (`tasks/lessons.md` #30); CipSoft↔TFS deviations documented (exp polynomial shared; beat/attack/armor/fight-mode differ).

Gate each phase: `cargo check -p tfs-rust-core && cargo clippy -p tfs-rust-core --all-targets && cargo test -p tfs-rust-core`; 1098 outcomes unchanged.

## 772 formulas API follow-up
- [x] Add concrete Tier-2 Lua hook templates in `data/formulas/772.lua` so shard owners can tune actual formulas directly.
- [x] Make 772 attack cadence draw from vocation/weapon speed (`vocations.xml` path) instead of fixed `attackSpeedMs = 2000`.
- [x] Update/adjust combat formula tests for the new 772 attack-speed behavior.
- [x] Run `cargo test -p tfs-rust-core combat::math` and capture any follow-up lesson.

## 772 monster distance-fighting follow-up
- [x] Keep CipSoft-style distance-fighting branch behavior, but center range decisions on monster XML `targetDistance` (no hardcoded 4).
- [x] When a ranged-distance monster cannot currently use a ranged attack, collapse chase path target range to melee (`max_target_dist = 1`) instead of kiting away.
- [x] Add regression test to lock the non-attackable ranged chase clamp.

## 772 floor-change invisible-monster — fixed
- [x] Root cause: erroneous extra `u8` stackpos on 772 standalone `0x6A` (OTCv8 772 omits stackpos; `GameTileAddThingWithStackpos` is 841+).
- [x] Fix: `Codec772` never writes stackpos on `0x6A`; golden + integration tests.
- [x] Reverted workaround: `evict_known_creatures_in_viewport`, `purge_creature_wire_id_from_all_conns`.

## P2 — 772 beat-driven game loop (MVP) — done
- [x] `todo_queue.rs` — `ToDoQueue` min-heap + unit tests; wired into `GameWorld`
- [x] `CreatureBase::next_wakeup` / `last_step_server_ms` for logical-time walk scheduling
- [x] `run_game_loop_772` — `beat_ms` timer, cmd drain, `advance_beat_772`, `FlushPolicy::BeatEndOnly`
- [x] `run_game_loop_1098` extracted; shared `process_game_command` + immediate movement flush
- [x] `walk.rs` — ToDoQueue scheduling when `beat_driven_loop`; skip Tokio walk polling on 772
- [x] `run_server.rs` — `walk_wake_tx = None` + loop branch on `StepSpeedModel::CipSoft`
- [x] `resolve_migrations_dir()` — runtime migrations path when repo mount differs from compile-time
- [x] Staggered ~1000 ms subsystem counters (`subsystem_counters_772.rs` + `advance_beat_772`)
- [ ] Deferred: multi-beat lag catch-up when beat alarms pile up

## `walk.rs` layout split — done
- [x] Extract `walk/walk_timing.rs` (~380 lines) — speed/timing pure functions
- [x] Extract `walk/walk_tile.rs` (~462 lines) — `Tile::queryAdd`, destination resolution
- [x] Anchor `walk/mod.rs` — direction utils, dispatch, `impl GameWorld`, tests; re-export `wire_step_speed` / `WalkSpeedRole`
- [x] `cargo check -p tfs-rust-core`; `cargo test -p tfs-rust-core walk::` (14 tests)

## P8 — IdleStimulus Phase A (772 drain-triggered Go/chase) — done
- [x] `creature_todo.rs` — `CreatureAction::Go`, per-creature queue on `CreatureBase`
- [x] `idle_stimulus.rs` — `idle_stimulus`, `monster_idle_stimulus`, `request_idle_stimulus`
- [x] `walk/mod.rs` — 772 monster execute via action queue; idle on drain; player path unchanged
- [x] `monster_ai.rs` / `creature_think.rs` — skip think chase/repath when `beat_driven_loop`
- [x] `monster_events.rs` / `monster_targets.rs` — 772 repath defers to idle
- [x] Unit tests: `idle_stimulus::tests` (4), existing beat_driven + creature_think green
- [ ] Phase B deferred: `CreatureAction::Attack`, retire 772 `creature_on_attacking` from think
