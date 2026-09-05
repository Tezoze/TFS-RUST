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
