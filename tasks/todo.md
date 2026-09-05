# Death metadata and persistence — audit Step 9 (2026-09-05)

`docs/772_PARITY_GAP_AUDIT.md` Step 9. Corpus: `RecordDeath` / `AddKillStatistics` (`crmain.cc:830-860`), `Murderer` (`crplayer.cc:1546`), `GetArmorStrength` flags (`crcombat.cc:295-297`), `TSkillSoulpoints` Cycle/Count/MaxCount (`crskill.cc` / `crcombat.cc:938-955`), `TSkillLevel::Decrease` abort (`crskill.cc:300-303`), `WriteKillStatistics` (`main.cc:394`). Pack surface: TFS `player_deaths` / `kill_statistics` / `/deathlist`; death row stays native (lessons 369/409). Corpse "killed by" last-hit name is already live.

**Architecture:** focused modules, not more `GameWorld` methods. `death_record.rs` (snapshot + persist spawn, VIP-style `Handle::spawn`). `kill_statistics.rs` (in-memory race table + shutdown write). Armor helper next to `slot_type_for_item_type`. Soul columns follow `food_remaining`/`food_level` (772-only extras, not `CONDITION_SOUL` blob). Thin call from `apply_creature_death` / shutdown flush.

- [x] C++ analysis — `RecordDeath` writer order; kill-stat race counters; soul Cycle/Count/MaxCount; armor CLOTHES+ARMOR ≡ `armor>0`+slot; `TSkillLevel::Decrease` abort is LEVEL-only (already on `remove_experience`; Probe has no abort)
- [x] `RecordDeath` — native INSERT `player_deaths` (VIP-style spawn). Corpus remarks; TFS columns; one row with `mostdamage_*` (not two corpus rows). Snapshot OldLevel before skill/exp loss. Store `last_damage_type` on `CreatureBase` for env remarks.
- [x] Kill statistics — in-memory by race name; `AddKillStatistics` on every lethal death; flush at wall-clock minute 55 + shutdown (no boot load). Env name `"(fire/poison/energy)"`. SQLx `kill_statistics` + UNIQUE(name) upsert-add.
- [x] Armor slot — named helper `item_counts_as_armor_at_slot` (CLOTHES=`slot_position` + ARMOR=`armor>0` + BODYPOSITION); same observable as today; cite `crcombat.cc:295-297`
- [x] Soul timer persist — `players.soul_cycle` / `soul_count` / `soul_max_count` like food; not `CONDITION_SOUL`
- [x] Death skill-loss abort — keep on `remove_experience` only; do **not** add to `skill_decrease` / `magic_decrease` (`TSkillProbe::Decrease` has no 100000 abort)
- [x] Tests + audit Step 9 marked done + lesson

# Splash layer + elevation climb — audit Step 8 (2026-09-05)

`docs/772_PARITY_GAP_AUDIT.md` Step 8. Corpus: `CreatePool` (`operate.cc:2596`), `GoExec` climb (`cract.cc:415-431`), `GetHeight` (`info.cc:689`). Write-ups: `docs/772_SPLASH_LAYER_MISMATCH.md`, restore `docs/772_ELEVATION_WALK_PARITY.md` (deleted in `f2123185`).

**Splash — keep sorted `top_items` insert.** Option A (clear `FLAG_ALWAYSONTOP` → `down_items`) stays **rejected**: 772 `0x6A` omits stackpos and the client inserts by `.dat` order (live test). Blood-on-ladders already works. Remaining: `CreatePool` NOROOM when a non-`LIQUIDPOOL` BOTTOM object is present (corpse vs pool); delete any leftover TFS ladder guards; Cip map description must keep splash stackpos matching `0x6A`/`0x6C`.

**Elevation — Part B only (G1–G4).** Part A (7.4 step-up limit) is **not** 772 — do not add `elevationStepLimit`. All 357 `HEIGHT` types in `objects.srv` have `Elevation = 8`.

- [x] G1 — `ItemType::elevation()` returns `8` when `has_height()` and xml `elevation` is 0 (`items.xml` has no keys; OTB has no attr)
- [x] G2 — climb only after same-floor `MovePossible` fails (`cract.cc:415`); hoist probe into `internal_move_creature_step`; player + cardinal only
- [x] G3 — floor bounds `DestZ > 0` / `DestZ < 15` (drop TFS `z != 8` / `z != 7`)
- [x] G4 — walk rejection `NotEnoughRoom` → `NotPossible` (`GoExec` throws `MOVENOTPOSSIBLE`); keep PZ / not-invited throws
- [x] G5 skip — 19 missing OTB `HAS_HEIGHT` (cosmetic, Unmove)
- [x] Splash: `CreatePool` NOROOM on non-pool BOTTOM scenery (not corpses); no Option A; no leftover ladder guards
- [x] Tests (G1–G4 + splash NOROOM + 2-stack still walkable) + restore elevation doc + audit Step 8 done + lesson

# Script numerics — audit Step 7 (2026-09-05)

Tier 3 table in `docs/772_PARITY_GAP_AUDIT.md`. Data-pack Lua only; 1098 extras stay behind `formulas.otherActions`. Cite `moveuse.cc` / `moveuse.dat`.

- [x] `food.lua` — `(cur+add) > 1200` (`moveuse.cc:1842`); exact 1200 allowed
- [x] `birdcage.lua` — empty iff `random(100)<=1 and random(100)<=10` (0.1%); else effect 22
- [x] `waterpipe.lua` — `random(100)<=90` poff on item else player; id 2093; 2099 behind `extraInstruments`
- [x] `music.lua` — didgeridoo chance 10; cornucopia **3957 only** 95% keep+10 grapes else 9+`transform(2681)`; 2369 is horn; bongo/war drum behind `extraInstruments`
- [x] `change_gold.lua` — already gated; 772 does not register coins
- [x] `decayto.lua` — drop cuckoo 1873–1876 (use = time via `watch.lua`)
- [x] `teleport.lua` — drop PZ-lock cancel
- [x] Tests + audit Step 7 marked done; lesson captured

# Monster AI edge paths — audit Step 6 (2026-09-05)

Corpus `TMonster::IdleStimulus` (`crnonpl.cc:2345`). **Exclude** `DistanceFighting` race flag — keep inferring the dist branch from `target_distance > 1 && ThrowPossible`.

- [x] `LifeEndRound` at IdleStimulus entry (`crnonpl.cc:2352`) — `StartLogout(true,true)` via `remove_creature`; drop raid-tick poll in `raid_waves.rs`
- [x] Monsterhome idle despawn (`crnonpl.cc:2408`) — non-summon + `home_radius > 0` + `!MonsterhomeInRange` (axis box `|dx|<=R && |dy|<=R`, `|dz|<=2`); no ATTACKING exemption
- [x] Helper `monsterhome_in_range` (`crnonpl.cc:1515`); `home_radius <= 0` ≡ `Home == 0` → in range
- [x] Tests in `idle_stimulus_tests.rs`; audit Step 6 marked done (DistanceFighting still open)

# Chat parity pass — audit Step 5 (2026-09-05)

Corpus `Talk` / `RecordTalk` / `RecordMessage` (`operate.cc`, `crplayer.cc`). Pack surface stays TFS channels (`data/scripts/chatchannels/*.lua`). Flood decision: **port RecordTalk** (772 corpus for all `clientVersion`); do not keep TFS `5n²` / `maxMessageBuffer` as the live model.

- [x] `chat_talk.rs` — 7×5 same-Z say box, 30×30 yell box (corpus `Talk`); thin call from `broadcast_creature_say_viewport` / `broadcast_creature_yell`
- [x] Trade-channel 2 min gate — `EarliestTradeChannelRound + 120` on pack Trade id **6** (not corpus enum 5 = pack RL-Chat)
- [x] PM spam cap — `RecordMessage` → `"You have addressed too many players."`
- [x] Guild look clause + guild-name channel filter; persist rank/nick from `guild_membership`
- [x] Cancel texts: private-channel premium (`YouNeedPremiumAccount`), invite/exclude info, `EditText` TOOLONG, `UseWithCreature` 7×5 range
- [x] RecordTalk 2.5s sliding window + `MutingEndRound` (replace TFS message-buffer mute)
- [x] Tests + audit Step 5 marked done; lesson captured

# Unified item catalog (RON)

**Status:** converter complete (2026-09-05). Engine cutover later — **unify on client id**, no dual-id alias.

`data/items/clientid_output/items.ron` is the future single catalog for every `clientVersion` (replaces runtime `items.otb` + `items.xml`). Converter: `scripts/convert_itemid_to_clientid.py`.

- [x] Nested `field.*` (`cycles` / `initdamage` / `skippeaceful` / TFS `ticks`/`count`) — runtime `xml_attributes`
- [x] Typed RON fields for every `parseItemNode` + `apply_ability_attribute` key (all versions, omit-if-absent)
- [x] OTB `flags` stay OTB bits only; XML overlays as `ItemType` fields (`block_solid`, `force_use`, `unlay`, …)
- [x] `vocations: [...]` (repeatable XML key)
- [x] Self-check: no dropped pack keys; regenerate `items.ron` (`--self-test`)
- [x] Duplicate client ids: first OTB wins (`clientIdToServerIdMap`); fill missing XML/group
- [ ] **Later campaign** (one cutover, remap data then flip engine):
  1. Lua / monster loot / NPC type / rune / weapon rewriter (extend the converter; dry-run first)
  2. OTBM + house `tile_store` rewrite; PVP field table into client-id space
  3. Rust `ITEM_*` / money / field constants + Lua globals + hardcoded tests
  4. SQL `itemtype` remap (player/depot/inbox/market) + seed
  5. Engine DTO→`ItemType` loader; promote `items.ron`; drop runtime OTB+XML
  6. Delete `client_id_for_server` / `server_id_for_client`; shop/quick-equip/npc_import become identity
  7. Soak: gold 3031, fire decay, doors, loot, shops, depot, waypoints, 6 collision rows

# Native Handler Migration


**Status:** complete.

## Native data/lib API (2026-08-29)

Ported stateful `data/lib/core` helpers to native Rust + Lua-callable bindings.

- [x] `register_data_lib_native` after `load_data_lib` (`run_server.rs`, tests)
- [x] Phase 1a — `game_map_helpers.rs`, `GameWorld.global_storage`, `Game.*` natives
- [x] Phase 1b — native `Combat:getPositions` + `resolve_combat_area_context`
- [x] Phase 1c — `player_money_lib.rs`, `Player.removeTotalMoney` / `canCarryMoney`
- [x] Phase 2 — `creature_lib.rs`, summon/outfit/path/PZ natives on `CreatureRef`
- [x] Phase 2 misc — `Player.getDepotItems`, `Player.getClosestFreePosition` override
- [x] Phase 3 — item look, `Position:isInRange`, `ItemType.usesSlot`, `Vocation.getBase`, `Party.broadcastPartyLoot`, `Tile.relocateTo`
- [x] Pack Lua slimmed/stubbed (`game.lua`, `combat.lua`, `creature.lua`, `item.lua`, …)

**Stay Lua:** `constants.lua`, `storages.lua`, `create_functions.lua`, `container.createLootItem` error stub.

## Party lifecycle — audit §1.2 (2026-08-29)

Wire client opcodes `0xA3`–`0xA8` to corpus `operate.cc:3919–4214`.

- [x] Extend `Party` (`invited`, `PartyShield`); fix `split_shared_experience` (even split, no TFS bonus)
- [x] `party.rs` — invite / revoke / join / pass leadership / leave / disband / share XP
- [x] `skulls.rs` — `player_get_party_mark`, `send_creature_shield_to_conn`
- [x] `game_loop.rs` — dispatch + timed-action whitelist
- [x] Logout forced leave; login `party_shield` from mark
- [x] Unit tests in `party.rs`

## NPC shop runtime — audit §1.3 (2026-09-05)

TFS shop-window pack surface (`0x79`–`0x7C`). 772 dialogue trading stays on `npc/host.rs`.

- [x] `shop.rs` — open/close, look, buy, sell, sale-list refresh
- [x] `Player.shop_owner` + `shop_items`; money + capacity on native buy
- [x] `game_loop.rs` dispatch; logout/remove close shop
- [x] Lua `openShopWindow` / `closeShopWindow` (`npc_shop.rs`)
- [x] Unit tests in `shop.rs`

## VIP runtime — audit §1.4 / step 3b (2026-09-05)

TFS pack surface `Game::playerAddVip` / `playerRemoveVip` / `playerEditVip`. List already loads at login.

- [x] `vip.rs` — add/remove/edit, `getMaxVIPEntries` (group / premium 100 / free 20, hard cap 200)
- [x] `game_loop.rs` dispatch `0xDC`–`0xDE`; `VipLookupFinished` for offline name resolve
- [x] `PlayerStore` INSERT/DELETE/UPDATE `account_viplist` (not on `savePlayer`)
- [x] Login/logout `notifyStatusChange`; era-correct `0xD2` / `0xD3` (`0xD4` logout on 772)
- [x] Unit tests in `vip.rs`; VIP codec goldens
- [x] `account_viplist.icon` decode as `u8` (`TINYINT UNSIGNED`, not `i32`)

## Rune and spell gates — audit Step 4 (2026-09-05)

Corpus `CheckRuneLevel` / `EarliestSpellTime` / `CheckAccount` / `UseMagicItem` target walk (`magic.cc`). Pack surface stays TFS Spell Lua.

- [x] `spell.rs` — `prefer_rune_tile_target`, `rune_magic_level_ok`; `LOWMAGICLEVEL` cancel text
- [x] `player_cast_rune` — ML + `EarliestSpellTime` (+ aggressive PZ) before fire; no consume on fail
- [x] `ActionObjectRef.creature_id` seed from `UseWithCreature`; rune-only stacked-tile preference
- [x] `player_say_spell` — `is_premium` → `YouNeedPremiumAccount` (`CheckAccount`, no `ALL_SPELLS` skip)
- [x] Tests: target helper, rune ML, exhaust, premium say-spell
- [x] Audit Step 4 marked done; lesson captured

## learnSpells config (2026-09-05)

Global `config.lua` `learnSpells` — not per-script `needLearn`. 772 NPC `TeachSpell` is native `NpcDialogue` (not TFS `StdModule.learnSpell`).

- [x] `config.lua.dist` `learnSpells = true` (missing key = false)
- [x] SpellNr ↔ Comment name table; `teach_spell` stores name; NPC `spellKnown`/`spellLevel` resolve nr
- [x] `player_knows_instant`: true → SpellKnown for ex/ut/ad; false → vocation only (ignore `needLearn`)
- [x] House `al*` / level 0 skip the learn gate
- [x] Tests + lesson

## NPC vocation properties (2026-09-05)

`property = "knight"` used 772 profession ids; pack Elite Knight is id 8.

- [x] Map NPC vocation properties from `vocations.lua` names (TFS ids 4|8 = knight)
- [x] Tests + lesson

## 772 classic item look (2026-09-05)

TFS `Item::getDescription` dumps `showattributes` (speed, skills) and absorb % (`ice`/`holy`/`death`). 772 look is name + Arm/Atk/Def/Range + charges + weight.

- [x] `item_look_description(..., classic_look)` — skip speed/skills/absorbs/Hit%/Atk Spd/extra Def
- [x] Gate on `Codec::V772` at look / shop / trade / Lua `getDescription`
- [x] Tests: boots of haste, might ring, plate/sword keep Arm/Atk/Def

## absorbpercentmagic 8.1 types (2026-09-05)

TFS `absorbpercentmagic` synthesizes ice/holy/death (8.1+). Corpus magic is energy/fire/earth. Same for all `clientVersion`.

- [x] `absorbpercentmagic` / `absorbpercentelements` expand 772 types only
- [x] Keep explicit `absorbpercentice` / `holy` / `death` XML keys (pack surface)
- [x] Tests + lesson

## items.xml absorbs from objects.srv (2026-09-05)

Runtime loads `data/items/items.otb` + `data/items/items.xml` only (not merged_objects.srv). Align XML `absorbpercent*` with 772 `ProtectionDamageTypes` + `DamageReduction`.

- [x] Might ring / elven: 25% / 10% on physical+magic+lifedrain (mask 287); bronze manadrain 15%
- [x] Drop TFS-only absorbs on dwarven set / wood cape (srv Armor only, no Protection)
- [x] Pack-xml load test; lesson

## Rings/amulets protection + charges (2026-09-05)

Might ring (and other WearOut jewelry) spawned with `Item::new` so `charges` attr was 0; absorb never wore out. 772 `TotalUses` seeds `RemainingUses`; `crmain.cc` WearOut on absorb.

- [x] `Item::from_item_type` seeds charges from `ItemType.charges` (count 0/1)
- [x] Lua/loot/player-add create paths use it; login hydrates missing charge attr
- [x] Combat absorb decrements jewelry charges / destroys at 0
- [x] Tests + lesson

## Private tell window (2026-09-05)

VIP "Message" / `0x9A` was mixed up with owned private chat rooms (`0xAA` / `0xB2`).

- [x] `player_open_private_channel` always `sendOpenPrivateChannel` `0xAD`
- [x] `player_speak_to` case-insensitive name lookup
- [x] Tests + lesson

## Phase 2 — EventCallback dispatch (ship first)
- [x] Rust-side `has_event_callback` bitset + direct RegistryKey dispatch
- [x] Sync from `EventCallbackData` at end of `load_scripts_interface`

## Phase 1 — MoveEvent aid native path (3000–3123)
- [x] `aid_move_events.rs` + `aid_move_compile.rs` + dispatch hooks + boot log

## Phase 1c — Native moveitem policy
- [x] `player_move_policy.rs` — quest aid, candelabrum, blocking tile, trap (no VM per move)
- [x] `spell_combat_compile.rs` — boot parse Combat specs from spell/rune scripts
- [x] `native_spell_combat.rs` — skip `onCastSpell` VM; call `combat_execute_from_lua` directly
- [x] `fire_on_cast_spell` / `fire_on_cast_rune` try native first; boot log `native_spell_combats`
