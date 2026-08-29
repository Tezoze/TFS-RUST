# 772 Parity Gap Audit — Remaining Work

**Date:** August 2026
**Scope:** Mechanics/corpus parity only. Wire codec, query manager, login, and DB infrastructure were explicitly out of scope for this pass.
**Method:** Function-level sweep of the decompile corpus (`reference/cipsoft-772/tibia-game-master/src/`) against `crates/` and `data/`, one auditor per corpus system cluster.

## Reading this document

Status is judged **by observable behavior, not by naming**. Large parts of the port intentionally wear the TFS pack surface (`ConditionDamage`, `Combat:execute`, `Action()`/`MoveEvent()`, monster Lua defs) while reproducing corpus outcomes. A behavior implemented under a TFS-shaped name is **DONE**, not a gap.

Likewise, several corpus subsystems are deliberately **not** ported as engines — the `moveuse.dat` rule VM being the largest. Those are recorded in [Intentional deviations](#intentional-deviations) rather than counted against completeness.

## Completeness snapshot

| System | Corpus files | Estimate | State |
|---|---|---|---|
| Player / skills / combat | `crplayer.cc`, `crskill.cc`, `crcombat.cc`, `crmain.cc` | ~85% | Strongest area. Remaining items are metadata and fidelity, not mechanics. |
| Monster / NPC AI | `crnonpl.cc`, `cract.cc` | ~85–90% | Idle-stimulus engine, spawn, chase, casting, loot, NPC dialogue all live. |
| Magic / spells | `magic.cc` | ~75–85% | Impacts, shapes, `ComputeDamage`, fields done. Rune pre-cast gates missing. |
| Map | `map.cc`, `info.cc` | ~78% | Stacking, flags, throw LOS, decay cron done. Live sector refresh missing. |
| Houses | `houses.cc` | ~75% | Ownership, rent, lists, doors, eviction, **in-game sell via trade** done. Policy evictions and transfer missing. |
| Move / use | `moveuse.cc`, `objects.cc` | ~70% | Typed handlers, doors, fields, tools done. Mail missing; script numerics drift. |
| Chat / channels | `operate.cc`, `crplayer.cc` | ~68% | Say/whisper/yell/private/channels live. Flood model and yell radius diverge. |
| Player operations | `operate.cc` | ~68% | Trade done (1.1). Party, shop, VIP still parse but are never dispatched. |
| Info / script | `info.cc`, `script.cc`, `config.cc` | ~55% | Mostly replaced by OTBM + `config.lua` + Lua by design. |

Note that `communication.cc` turned out to contain no chat mechanics at all — it is socket, login, and waiting-list only (`communication.hh:24-64`). The 772 talk and channel behavior lives in `operate.cc` (`Talk`, `TChannel`, `OpenChannel`) and `crplayer.cc` (`RecordTalk`, `LeaveAllChannels`). Cite those instead.

---

## Tier 1 — Subsystems that are absent

These are whole features where the client sends a request and nothing happens. They dominate the remaining work.

Three items below share the same failure shape: the packet parses correctly in `crates/tfs-rust-net/src/game_parse.rs`, produces a `GamePacket` variant, and then falls into the catch-all at `crates/tfs-rust-core/src/game_loop.rs:1150` where it is traced and dropped. **§1.1 trade is complete** — see below.

### 1.1 Player-to-player trade — **DONE**

**Corpus:** `TCreature::ToDoTrade` / `Trade` (`cract.cc:653-725`, `:1202-1256`), `TPlayer::InspectTrade` / `AcceptTrade` / `RejectTrade` (`crplayer.cc:811-1000`), invalidation via `NotifyTrades` (`operate.cc:990-1023`, called from move paths). Wire out via TVP `sendTradeItemRequest` / `sendCloseTrade` (`gameserver/src/protocolgame.cpp`).

**Rust (August 2026):**

- Focused module [`crates/tfs-rust-core/src/trade.rs`](../crates/tfs-rust-core/src/trade.rs) — `TradeRegistry` / `TradeSide` on `GameWorld`, not extra `Player` fields.
- Packets `0x7D`–`0x80` dispatched in [`game_loop.rs`](../crates/tfs-rust-core/src/game_loop.rs); `RequestTrade` queues `TDTrade` via ToDo (walk-to-reach prepends `Go`); look/accept/reject immediate.
- Wire: `ProtocolCodec::encode_trade_item_request` / `encode_close_trade` (`0x7D`/`0x7E`/`0x7F`); golden bytes in [`protocol_compat.rs`](../crates/tfs-rust-net/tests/protocol_compat.rs) v772.
- `NotifyTrades` on item move (`game_world_item_move.rs`), inventory update/remove, walk out of range, logout/takeover/remove.
- Auto-stack skip via `player_trade_item` in `query_add.rs`.
- **House pack surface:** `house:startTrade` → `LuaMutation::HouseStartTrade`; dual `ITEM_DOCUMENT_RO` counter-offer; `house_set_owner` on dual accept — unblocks [`!sellhouse`](../data/scripts/talkactions/players/sellhouse.lua).

**Tests:** `cargo test -p tfs-rust-core --lib trade` (9 tests); protocol goldens; house cancel-code tests.

**Corpus notes (lesson 411):** partner-trading string `"This person is already trading."`; cancel always `"Trade cancelled."`; reject is asymmetric; max 100 nested objects; Chebyshev ≤2 + LOS.

### 1.2 Party lifecycle

**Corpus:** `InviteToParty` / `RevokeInvitation` / `JoinParty` / `PassLeadership` / `LeaveParty` / `DisbandParty` and `IsInvitedToParty` / `GetParty` (`operate.hh:189-196`, bodies at `operate.cc:3919-4214`). `TParty` holds leader, member vector, and invited-player vector (`operate.hh:68-82`).

**Rust:** `crates/tfs-rust-core/src/party.rs` is 70 lines of pure data — `new`, `add_member`, `remove_member`, `transfer_leadership`, `split_shared_experience`. No packet handlers for `0xA3`–`0xA8`. The party skull helpers in `player/combat/skulls.rs:610` are marked `#[allow(dead_code)]` because nothing drives them.

**Needed:** the six lifecycle operations, invited-player tracking, party skull/emblem broadcast, and the `CREATURE_PARTY_CHANGED` notify (`operate.hh:15`). The PvP same-party XP skip already works (`death.rs:250-256`), so shared XP wiring is partly in place.

### 1.3 NPC shop runtime

**Rust:** `LookInShop` / `PlayerPurchase` / `PlayerSale` / `CloseShop` parse at `game_parse.rs:64-81`. `crates/tfs-rust-content/src/npcs/shop.rs` is a 32-line data model. `updateSaleShopList` is a stub (`player/inventory/notifications.rs:195`).

**Needed:** `shop_owner` assignment on NPC focus, buy/sell with capacity and money checks, and sale-list refresh on inventory change. Note that 772 itself drives vendors through `.npc` behaviour rules rather than a shop opcode, so this is a TFS pack surface obligation rather than strict corpus parity — but the pack ships shop NPCs and the client shop UI is unusable without it.

### 1.4 VIP runtime

**Rust:** the list loads from the DB at login (`login_out.rs:640`), but `VipAdd` / `VipRemove` / `VipEdit` have no handlers, so the list is a read-only snapshot that cannot be edited in-session.

**Needed:** the three handlers plus DB persist, honoring `max_vip_entries` from `groups.xml`.

### 1.5 Mail

**Corpus:** `SendMail` / `SendMails` (`moveuse.cc:712-919`) — parses addressee and town from the letter text, delivers to the recipient's depot when online, queues when offline, and stamps the letter on send.

**Rust:** no equivalent found in `crates/`. Mailboxes exist in the map content and currently do nothing.

### 1.6 Live sector refresh

**Corpus:** `SectorRefreshable` / `RefreshSector` / `RefreshMap` / `RefreshCylinders` / `ApplyPatch` / `ApplyPatches` and `ProcessCronSystem` (`operate.hh:158-165`). `RefreshSector` (`map.cc:1307-1350`) tests the sector's `MapFlags & 0x01` (`map.cc:1320`), strips non-creature objects, and reloads from a patch stream.

**Rust:** the refresh flag is read from OTBM at load but never acted on at runtime; `Game.refreshMap` returns 0 and logs (lesson 411). The decay cron half of `ProcessCronSystem` **is** implemented (`game_world_tick.rs:91-99` + `decay_apply.rs`); only the sector-reload half is missing.

---

## Tier 2 — Behavioral gaps inside shipped systems

### Magic / spells

- **Rune use has no pre-cast gates.** `CheckRuneLevel` (`magic.cc:662-679`, called from `UseMagicItem:4085`) enforces magic level before a rune fires. Rust stores `rune_magic_level` (`items.rs:89-106`, `spell.rs:362-365`) but uses it only for look text (`item_look.rs:500-531`); `player_cast_rune` (`container_ui.rs:1021-1095`) has no level check. The `EarliestSpellTime` check at `magic.cc:4087` is also missing — Rust only applies exhaustion *after* the cast.
- **Premium spells are not enforced.** `CheckAccount` tests flag bit 2 (`magic.cc:625-641`). `InstantSpellDef.is_premium` is populated from Lua (`spell.rs:237`) but never read in `player_say_spell` (`game_world_chat.rs:250-493`).
- **Rune target selection picks the wrong creature on stacked tiles.** `UseMagicItem` (`magic.cc:4062-4082`) prefers a non-self target for aggressive runes and self for non-aggressive; `resolve_creature_at_action_target` (`container_ui.rs:1112+`) takes `creatures.first()`.
- **Healing does not clear paralyze natively.** `THealingImpact` and `Heal` reset `SKILL_GO_STRENGTH` when the delta is negative (`magic.cc:203-205`, `:2113-2115`). Rust relies on individual scripts setting `COMBAT_PARAM_DISPEL`.
- **AoE radii use TFS matrices where the corpus uses rings.** Ultimate explosion is r=6 in the corpus (`magic.cc:3485-3487`) but `AREA_CIRCLE5X5` in `ultimate_explosion.lua:7` — note `AREA_CIRCLE6X6` already exists at `areas.lua:177`. Poison storm is r=8 (`magic.cc:3536-3539`) against `AREA_CIRCLE5X5`. Cancel invisibility is r=4 skipping origin (`magic.cc:2353-2450`) against `AREA_CIRCLE3X3`.
- **Berserk uses a different formula path.** Corpus case 80 is `(Level * ComputeDamage(...)) / 25` with mana `Level*4` (`magic.cc:3557-3562`); `berserk.lua:10` routes through `computeSkillDamage`.
- **Mana fluid roll differs.** `DrinkPotion` uses `ComputeDamage(NULL, 0, 100, 50)` (`magic.cc:4328-4333`); `fluids.lua:61-62` uses `math.random(50,150)`.

### Player operations and chat

- **Yell radius is TFS, not corpus.** `Talk` searches r=30 and filters a 30×30 box (`operate.cc:2357-2397`); `broadcast_creature_yell` uses `spectator_players_in_box(18,14,true)` (`game_world_spectators.rs:428-429`).
- **Say radius is not the asymmetric 7×5 box.** Corpus filters `DistanceX>7 || DistanceY>5` on the same Z (`operate.cc:2372-2374`); Rust uses generic `can_see` (~8×6).
- **Flood mute uses a different algorithm.** `RecordTalk` is a 2.5-second sliding window with a round-based `MutingEndRound` (`crplayer.cc:1741-1755`); `player_remove_message_buffer` is the TFS count-based `5n²` seconds model (`game_world_chat.rs:1097-1162`).
- **Trade-channel rate limit is missing entirely.** `EarliestTradeChannelRound + 120` gates channel 5 (`operate.cc:2263-2270`) — a two-minute limit between trade offers. No channel-5 gate exists in `game_world_chat.rs`.
- **Private-message spam cap is missing.** `RecordMessage` (`operate.cc:2316-2322`) produces `"You have addressed too many players."`.
- **Guild clause absent from player look.** `operate.cc:1900-1927` appends guild, rank, and title; `player_look_description` (`game_world_inventory.rs:1597`) omits it. Channel talk is likewise missing the guild-name filter (`operate.cc:2445-2448`).
- **Several failures are silent where the corpus sends text.** Private-channel creation without premium returns quietly (`game_world_chat.rs:679-681`) instead of `NOPREMIUMACCOUNT` (`operate.cc:3543-3545`); channel invite/exclude info texts are TODO (`:746-747`, `:802`); `EditText` accepts over-long input instead of `TOOLONG` (`container_ui.rs:876-877` vs `operate.cc:2654-2656`); `UseWithCreature` out of the 7×5 range drops silently (`game_loop.rs:925-929`).
- **`LookInBattleList`, `JoinAggression`, and `CloseNpcChannel` are unhandled** — parsed, then dropped.
- **Lua channel hooks are stubbed.** `canJoin` / `onJoin` / `onSpeak` from `data/scripts/chatchannels/*.lua` are not wired (`game_world_chat.rs:557-638`).

### Monster / NPC AI

- **Monsters outside their monsterhome never despawn.** `IdleStimulus` calls `MonsterhomeInRange` and triggers `StartLogout` when false (`crnonpl.cc:2408-2414`). Rust only blocks movement via `monster_move_possible_planning` (`monster_ai.rs:1855-1858`), so a monster that gets stuck off-home stays forever.
- **`LifeEndRound` is checked in the wrong place.** The corpus tests it at the top of `IdleStimulus` (`crnonpl.cc:2352-2356`). The field exists (`creature/monster.rs:300`) but is only polled by the raid tick (`raid_waves.rs:310-321`).
- **No explicit `DistanceFighting` race flag.** The corpus reads it from `RaceData` (`crmain.cc:1253`, `:1498`) and branches at `crnonpl.cc:2837-2868`. Rust infers the distance branch from `target_distance > 1 && ThrowPossible` (`monster_ai.rs:217-226`). This currently produces correct results for the shipped pack, but it is a data-shape mismatch waiting to bite.
- **Four NPC behaviour actions are unimplemented:** `Bless` (7 call sites), `Town` (9), `String` assignment (595), `Promote` (4) — see `tasks/npc-corpus-inventory.md:85-88`.
- **NPC `Summon()` does not bind a master.** `npc/host.rs:134-144` creates a detached monster.

### Move / use

- **`MOVEMENTEVENT` on item cylinder transfer has no hook.** `moveuse.cc:2263-2287` fires when a flagged item moves between containers; the corpus uses it for quest items in chests. No equivalent in the item-move path.
- **`UseChangeObject` UNLAY shuffle is not replicated.** When a transform target is `UNLAY`, the corpus relocates stack objects to an adjacent passable tile (`moveuse.cc:2184-2204`) — distinct from `ClearField`, which *is* ported (`clear_field.rs:30+`). Currently only doors get the treatment (`doors.rs:164`).
- **`UseAnnouncer` cases 1 and 3 are missing** — full in-world date string (`moveuse.cc:1891-1898`) and the blessings list from quest values 101–105 (`:1909-1944`). Case 2 (time) and case 4 (spellbook) are done.
- **Level/quest door denial text is hardcoded.** The corpus reads the item's info string via `GetInfo(Door)` (`moveuse.cc:2075`, `:2111`); `doors.rs:196`, `:218` use fixed strings, which loses map-specific messages.

### Map / houses

- **Splash and pool items are on the wrong layer.** They belong on BOTTOM (`CreatePool` scans BOTTOM, `operate.cc:2585+`) but OTB `FLAG_ALWAYSONTOP` routes them into `top_items`, so they render above creatures. Already written up in `docs/772_SPLASH_LAYER_MISMATCH.md`; content-side guards currently paper over it.
- **Elevation climb defects remain.** Four are enumerated in `docs/772_ELEVATION_WALK_PARITY.md` §4/§5 against `walk/walk_tile.rs`.
- **House policy evictions are absent:** `EvictFreeAccounts` (`houses.cc:1139+`), `EvictDeletedCharacters` (`:1173+`), `EvictExGuildLeaders` (`:1199+`).
- **`TransferHouses` (`houses.cc:1029+`) and `StartAuctions` (`houses.cc:1334+`) are not ported.** Auction *settlement* is (`house/auction.rs:18-36`), on the assumption MyAAC writes the bid columns — worth confirming that schema matches the `FinishAuctions` payment check.
- **Corpus `MayOpenDoor` parses access rules from the door's own text** (`houses.cc:562-619`). Rust uses DB `door_lists` (`house/mod.rs:210-224`), which is the TFS shape; confirm it covers every 772 door.
- **`IsPremiumArea` (`map.cc:2430-2453`) has no equivalent** — undetermined whether the shard needs it.

### Player / combat

This system is in the best shape; what remains is mostly bookkeeping.

- **No death metadata.** `RecordDeath` and `AddKillStatistics` (`crmain.cc:830-860`) plus the `Murderer` field (`crplayer.cc:1546`) have no counterpart — there is no DB death row and no kill statistics. Only the last-hit name reaches the corpse description.
- **Armor slot check is a proxy.** `crcombat.cc:295-297` gates on the CLOTHES and ARMOR flags; `values.rs:288-291` substitutes `armor > 0`.
- **Soul timer does not persist.** `soul` is saved (`game_world_save.rs:139`) but `soul_cycle` / `count` / `max_count` are session-only, so the timer resets on relog.
- **Attack rearm snapback is incomplete.** No player `CreatureMoveStimulus` snapback when the chase target walks away (`crmain.cc:920-965`); tracked as L3/S5 in `docs/SNAPBACK_KNOCKBACK_AUDIT.md`.
- **Death skill-loss abort quirk.** `TSkillLevel::Decrease` aborts when `Amount > Exp && Exp > 100000` (`crskill.cc:300-303`); Rust applies this only on `remove_experience` (`player.rs:476-477`), not in the death skill loop.
- **Latent TFS leak in party XP.** `split_shared_experience` (`party.rs:63-69`) adds a party bonus that 772 `DistributeExperiencePoints` does not have (`crcombat.cc:906-921`). Harmless today because callers pass `None`, but it should be removed or gated to 1098 before party work lands.
- **`WriteKillStatistics` (`main.cc:394`) is not ported.**

---

## Tier 3 — Script probability and threshold drift

Small, low-risk, and independently verifiable. Each is a data-pack edit.

| Item | Corpus | Current | File |
|---|---|---|---|
| Food cap boundary | `(cur+add) > Max` (`moveuse.cc:1842`) | `>= 1200` — rejects exactly 1200 | `food.lua:53` |
| Birdcage empty chance | nested roll = 0.1% | `random(100)==1` = 1% | `birdcage.lua:4` |
| Waterpipe puff target | 90% item / 10% player | 33% / 67% | `waterpipe.lua:4-7` |
| Didgeridoo success | 10% | 20% | `music.lua:27` |
| Cornucopia grape keep | 95% | 80% | `music.lua:21`, `:29` |
| Change gold | absent on 772 | still registers | `change_gold.lua` |
| Cuckoo clock | use announces time only | also toggles | `decayto.lua:3-4` |
| Teleport PZ cancel | not in corpus | cancels in PZ | `teleport.lua:18-21` |

These are already listed in `tasks/other-actions-plan.md` step 5; this audit confirms them against the corpus.

---

## Intentional deviations

Recorded so future audits do not re-file them as gaps.

- **`moveuse.dat` rule engine is not ported.** `HandleEvent` / `CheckCondition` (26 condition types) / `ExecuteAction` (38 action types) (`moveuse.cc:86-350`, `:946-1531`) are replaced by TFS `Action()` / `MoveEvent()` Lua plus native handlers, per `tasks/movements-plan.md:148`. **The conversion has been done, and systematically** — see [Coverage of the converted dat rules](#coverage-of-the-converted-dat-rules) below. Coordinate-pinned rules became action-id-keyed `MoveEvent` scripts, with the coordinate living in the OTBM as an action id.
- **`playerSpeed = "balanced"`** in `data/formulas/772.lua:49` is a deliberate shard-tuning choice, not a parity bug. The corpus formula is linear `2*Go + 80` (`crskill.cc:667`), available as `playerSpeed = "772"` if strict parity is ever wanted.
- **`script.cc` binary script I/O** is replaced by OTBM + Lua.
- **Rule violation reporting** (corpus channel 3, `operate.cc:3222+`) is an explicit non-goal.
- **Critical hits and stamina do not exist in the 772 corpus.** The stamina DB field is persisted for TFS compatibility but has no gameplay effect.
- **Party channel** exists in Rust (`chat.rs:108`) but not in the corpus public-channel enum (`operate.hh:26-36`) — treat as a gated TFS extra.
- **`change_target` interval/chance, `<elements>` modifiers, `static_attack_chance`, `immunity_outfit`** are TFS XML fields with no corpus equivalent; stored but inert on 772.
- **1098-era opcodes** (market, modal window, wrap, browse field, mount, quest log) are correctly unhandled on 772; they should be version-gated rather than left to the catch-all.

---

## Recommended next steps

Ordered by gameplay impact per unit of effort. Steps 2–3 are the bulk of what "feature complete" means from a player's seat.

### ~~Step 1 — Player trade~~ **Done (audit 1.1, August 2026)**

Shipped in `trade.rs`: four packet handlers, ToDo `TDTrade`, wire encode, `NotifyTrades`, walk cancel, `house:startTrade` / `!sellhouse` on the same engine. See [§1.1](#11-player-to-player-trade--done).

### Step 2 — Party lifecycle

Second largest. `party.rs` already holds the data model; add the six operations from `operate.cc:3919-4214`, invited-player tracking, and the party skull broadcast that `skulls.rs:610` is waiting for. Remove the non-corpus XP bonus at `party.rs:63-69` while in there.

### Step 3 — NPC shop and VIP runtime

Both are small handler-wiring jobs against existing data models. Shop needs `shop_owner` plus buy/sell and the `updateSaleShopList` stub at `notifications.rs:195`; VIP needs three handlers and a DB persist.

### Step 4 — Rune and spell gates

Cheap and high-fidelity. Add `CheckRuneLevel` and the `EarliestSpellTime` check to `player_cast_rune`, the premium check to `player_say_spell`, and aggressive/non-aggressive target preference to `resolve_creature_at_action_target`.

### Step 5 — Chat parity pass

Yell radius to 30×30, explicit 7×5 say box, trade-channel two-minute gate, PM spam cap, guild clause in look and channel filter, and the missing cancel messages. Decide separately whether to port the `RecordTalk` sliding-window flood model or gate the TFS buffer behind `MechanicsProfile`.

### Step 6 — Monster AI edge paths

Monsterhome idle despawn, `LifeEndRound` at the idle entry point, and an explicit `DistanceFighting` flag on `MonsterType`.

### Step 7 — Script numerics

The whole Tier 3 table in one pass. Independently testable, no engine risk.

### Step 8 — Splash layer and elevation

Both have standing write-ups (`772_SPLASH_LAYER_MISMATCH.md`, `772_ELEVATION_WALK_PARITY.md` §4/§5). Fixing the splash layer also lets the content-side guards be deleted.

### Step 9 — Death metadata and persistence

`RecordDeath` DB row, kill statistics, armor slot flags, soul timer fields, and the skill-decrease abort quirk.

### Step 10 — Longer tail

Mail (`SendMail`), live sector refresh, house policy evictions and transfer, `MOVEMENTEVENT` hook, `UseAnnouncer` cases 1 and 3, UNLAY shuffle, and NPC `Bless` / `Town` / `String` / `Promote`.

### Step 11 — Spot-check fidelity of the converted dat rules

Not a coverage hunt — the conversion is done (see below). What remains is verifying that each converted rule is faithful, and that the action ids are actually placed on the right OTBM tiles. Start with the `level_2_bridge.lua` offset question recorded below.

---

## Coverage of the converted dat rules

The `moveuse.dat` Collision table was converted into action-id-keyed `MoveEvent` scripts rather than left unimplemented. Evidence:

- **124 distinct action ids, contiguous `3000`–`3123` with no gaps**, across 130 files in `data/scripts/movements/`. A contiguous block is a deliberate allocation pass, not incremental growth.
- **999 hardcoded real-map coordinates** in those scripts, confirming the OTBM is a real-map replica and the corpus coordinates carried across. `tasks/movements-plan.md:142` ("772 Collision-by-coord is not an OT map") is therefore too pessimistic as written.
- Coordinate crosswalk between dat Collision sections and script coordinates lands at 80–100% for most quest and teleporter sections, and exactly 100% for Paradox Tower, Annihilator, Dark Cathedral, Sacrificial Basins and Stones, Desert Quest, Edron Demons, Lighthouse Thais, and Teleporters Absolute/Annihilator.

Worked example — Rookgaard premium bridge. Corpus (`moveuse.dat:1159-1162`):

```
Collision, IsType (Obj1,452), IsPosition (Obj1,[32057,32192,07]), IsPlayer (Obj2), HasRight (Obj2,PREMIUM_ACCOUNT) -> NOP
Collision, IsType (Obj1,452), IsPosition (Obj1,[32057,32192,07]) -> MoveTop(Obj1,[32060,32192,07]), EffectOnMap([32060,32192,07],13)
```

Script (`data/scripts/movements/map/rookgaard/premium_bridge.lua:3-8`) reproduces it exactly — `x + 3`, same `y`, `z = 07`, effect 13 — with the `NOP` fallthrough ladder expressed as an inverted `if not isPremium()`.

### Rule categories by disposition

- **Coordinate-pinned (647 of 828 Collision rules, 561 distinct coordinates)** — converted to aid scripts as above.
- **Type-keyed (181 rules)** — handled natively. Trap Damage (35 rules) via `trap.lua` + `magic_field.rs`; Liquid Deletions (70) and Teleporters Relative (71) via `tile_specials.rs`; Dustbins (1) likewise. These sections contain no coordinates at all, which is why the crosswalk reports none for them.
- **Genuinely uncovered:** `Collision/Mailboxes` (2 rules), which needs `SendMail` — already tracked as Tier 1 item 1.5.

### Residual risk

The open question is no longer whether a handler exists, but whether each conversion is faithful and whether the action id is placed on the correct OTBM tile. A script registered on aid 3051 does nothing if no tile carries 3051.

One concrete discrepancy to check first. Corpus level-2 bridge (`moveuse.dat:1155-1158`) relocates to the **same x**, `y + 1`, `z 07`:

```
Collision, IsType (Obj1,452), IsPosition (Obj1,[32092,32175,06]) -> MoveTop(Obj1,[32092,32176,07])
Collision, IsType (Obj1,452), IsPosition (Obj1,[32091,32175,06]) -> MoveTop(Obj1,[32091,32176,07])
```

`level_2_bridge.lua:5` uses `x = item:getPosition().x - 1`. That is correct only if the aid-3051 tiles sit one tile east of the corpus coordinates; otherwise every drop lands one tile west. Note the sibling `premium_bridge.lua` uses a pure relative offset with no such shift. Resolve by dumping the aid-3051 tile coordinates from the OTBM and comparing against `32091`/`32092`.

A useful general check is the same dump for all 124 aids: any aid with a registered script but no tile in the map is a dead script, and any tile carrying an aid outside `3000`–`3123` is an unhandled trigger.

---

## Verification

```
rtk cargo check --workspace
rtk cargo clippy --workspace --all-targets
rtk cargo test --workspace
rtk cargo run -p tfs-rust-lua --bin emit-lua-defs -- --check
bash scripts/check_data_pack_policy.sh
```

Per-area suites worth running while working the steps above:

```
rtk cargo test -p tfs-rust-core --lib trade
rtk cargo test -p tfs-rust-core --lib idle_stimulus
rtk cargo test -p tfs-rust-core --lib monster_ai
rtk cargo test -p tfs-rust-core --lib player::combat
rtk cargo test -p tfs-rust-net --test protocol_compat
```
