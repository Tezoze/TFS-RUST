# Lua API Completion Plan — Chat Channel Hooks (and the shared surface behind them)

**Goal.** Bring the `tfs-rust-lua` scripting surface up to the point where the `data/scripts/chatchannels/*.lua`
hooks — both the **active** bodies and the currently **commented-out** blocks — run for real, with no
`nil`-method / `nil`-constant failures. The channel scripts are the concrete driver, but every symbol
they need is a shared Lua-API primitive (`Player:getAccountType`, constants, `Condition`, …) that the
broader `luascript.cpp` port needs anyway (`TFS-lua-boundaries.md` §Full API Port Plan). This plan
sequences that subset.

**Non-goal.** This plan does **not** implement anything — it is the roadmap. It also does not port the
entire `luascript.cpp`; only the primitives the channel hooks touch, plus the minimum plumbing those
force (account-type backing, a `Condition` object).

**Reference discipline.** Behavior/values cited from the 772 tree only:
`reference/tvp-772/gameserver/src/{enums.h,const.h,luascript.cpp,player.cpp,chat.cpp}`. The Lua **method
names** are taken from `luascript.cpp` `registerMethod(...)` (the API contract is era-stable; only
values differ). Per `TFS-lua-boundaries.md`: **reads** go through `tfs_rust_common::ScriptContext`,
**mutations** through `LuaMutation` + immediate apply, userdata stores **typed IDs only**.

---

## 0. Current-state audit (what exists vs. what the scripts need)

### 0.1 What the Lua runtime provides today
- **Userdata:** `CreatureRef` (`Creature`/`Player`), `ItemRef`, `ContainerRef`. `Channel` is now a
  plain table (see chat-system-plan CH-8), not userdata.
- **`Creature`/`Player` methods (`userdata/player.rs`):** `getId`, `getName`, `getGuid`, `getSlotItem`,
  `getCapacity`, `getFreeCapacity`, `addItem`, `getItemCount`, `removeItem`, `getItemById`,
  `getDepotChest`, `getInbox`, `getContainerId`/`ById`/`Index`, `feed`, `getFood`. **All inventory /
  capacity / food — no social, no level, no account, no flags, no conditions.**
- **Globals/constants (`runtime.rs::register_event_script_bootstrap`):** class stub tables
  (`Player`, `Creature`, `Game`, …), a **stub** `Condition(type,id)` returning a table with no-op
  `setTicks`/`setParameter` (only there so `data/events/scripts/player.lua`'s `soulCondition` loads),
  `CONDITION_SOUL`, `CONDITIONID_DEFAULT`, `CONDITION_PARAM_SOULGAIN/SOULTICKS`, `RETURNVALUE_NOERROR`,
  `APPLY_SKILL_MULTIPLIER`, `hasEventCallback`/`EventCallback` no-ops, `debugPrint`, `configManager`
  (stubbed getters). **No `ACCOUNT_TYPE_*`, `TALKTYPE_*`, `PlayerFlag_*`, `VOCATION_NONE`, chat
  `RETURNVALUE_*`, `CONDITION_CHANNELMUTEDTICKS`, `CONDITION_PARAM_SUBID/TICKS`.**
- **`ScriptContext` (`tfs-rust-common/src/script_context.rs`):** creature name/guid, item + container +
  inventory reads, `get_config_string`, `get_player_food`. **No level / account-type / vocation /
  flags / condition reads.**
- **`LuaMutation` (`tfs-rust-lua/src/lua_mutation.rs`):** item add/remove/move, depot/inbox, feed.
  **No condition apply/remove, no outbound-message mutation.**

### 0.2 What every channel script needs (grep of the 8 loaded scripts)

**Constants (referenced bare — currently `nil`):**
| Symbol | 772 value | Source |
|---|---|---|
| `ACCOUNT_TYPE_NORMAL/TUTOR/SENIORTUTOR/GAMEMASTER/COMMUNITYMANAGER/GOD` | 1,2,3,4,5,6 | `enums.h:79-85` |
| `TALKTYPE_CHANNEL_Y/O/R1/R2` | 5,12,10,14 | `const.h:66,73,71,74` |
| `PlayerFlag_CanTalkRedChannel` | `1<<22` | `const.h:265` |
| `PlayerFlag_TalkOrangeHelpChannel` | `1<<23` | `const.h:266` |
| `PlayerFlag_CanTalkRedPrivate` | `1<<21` | `const.h:264` |
| `VOCATION_NONE` | 0 | `enums.h:297` |

**Read methods (referenced on `player` — currently `nil`):**
| Call | Backing today | Gap |
|---|---|---|
| `player:getAccountType()` | **none** — `Player` has `group_id`/`account_id`, not account type | Plumb `accounts.type` → `Player.account_type` → `ScriptContext` |
| `player:getLevel()` | `Player.level` exists | Add `ScriptContext::get_player_level` + binding |
| `player:getVocation()` → `:getId()` | `Player.vocation_id` exists | Add lightweight `Vocation` userdata (or int) + binding |
| `player:hasFlag(flag)` | `GameWorld::player_has_flag` exists | Add `ScriptContext::player_has_flag` + binding |
| `player:getName()` | **exists** | — |

**Mutation / outbound methods (referenced — currently `nil`):**
| Call | Gap |
|---|---|
| `player:sendCancelMessage(text)` | Outbound `MESSAGE_STATUS_SMALL`/cancel — new `LuaMutation` (outbound-only, safe) |

### 0.3 What the commented-out blocks additionally need (CH-5 mute logic)
The `-- TODO(chat CH-5)` blocks in `advertising*.lua`, `trade.lua`, `help.lua` add:
| Symbol / call | 772 value / ref | Gap |
|---|---|---|
| `Condition(CONDITION_CHANNELMUTEDTICKS, CONDITIONID_DEFAULT)` | real object, not the soul stub | Real `Condition` userdata + constructor |
| `condition:setParameter(param, value)` | — | `Condition:setParameter` on the real object |
| `CONDITION_CHANNELMUTEDTICKS` | `1<<15` | `enums.h:268` |
| `CONDITIONID_DEFAULT` | `-1` | `enums.h:275` (already registered as `0` — **wrong**, see §4.3) |
| `CONDITION_PARAM_SUBID` | `45` | `enums.h:179` |
| `CONDITION_PARAM_TICKS` | `2` | `enums.h:136` |
| `player:getCondition(type, id, subId)` | `luascript.cpp:2116` `Creature:getCondition` | `ScriptContext` read of active condition |
| `player:addCondition(condition)` | `luascript.cpp:2117` | `LuaMutation` immediate-apply |
| `target:removeCondition(type, id, subId)` | `luascript.cpp:2118` | `LuaMutation` immediate-apply |
| `Player(name)` constructor | `luascript.cpp` `luaPlayerCreate` | resolve by name via `player_by_name` → `CreatureRef` |
| `sendChannelMessage(channelId, type, msg)` | `chat.cpp` channel broadcast | global fn → `GameWorld` channel fan-out |
| `RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE` | `enums.h` `ReturnValue_t` | register the `RETURNVALUE_*` enum block |

### 0.4 Prerequisite (NOT Lua-API, but gates observing any of this)
`game_world_chat.rs::player_talk_to_channel` currently has `// TODO(chat CH-4): Run Lua onSpeak hook`
— **the hooks are loaded but never invoked**, and neither is `canJoin` in `player_open_channel`. So
even the *active* (uncommented) hook bodies do nothing today. This plan's Lua API is only observable
once chat-system-plan **CH-4 hook invocation** lands (`call_channel_on_speak`/`can_join` already exist
in `runtime.rs`; core just needs to call them and honor the return). Sequence LUA-1..LUA-4 to land
**before or with** that CH-4 wiring — flag the dependency, don't duplicate the wiring here.

---

## 1. Architecture (how each category plugs in)

Follows `TFS-lua-boundaries.md` exactly.

### 1.1 Constants — `tfs-rust-lua/src/constants.rs` (new), mirrors `luascript.cpp` `registerConstants`
A single `register_constants(&Lua) -> Result<()>` called once from `LuaRuntime::new`, after the class
stubs. Source values from `tfs_rust_common::enums` where a Rust enum already exists (avoid a second
source of truth that can drift); hardcode with a `// enums.h:NN` cite only where no Rust enum exists.
Grouped like the C++ (`ACCOUNT_TYPE_*`, `TALKTYPE_*`, `PlayerFlag_*`, `VOCATION_*`, `CONDITION_*`,
`CONDITION_PARAM_*`, `CONDITIONID_*`, `RETURNVALUE_*`). This replaces the scattered `globals.set(...)`
constant lines in `register_event_script_bootstrap` (move them here; keep the class-table stubs there).

### 1.2 Reads — extend `ScriptContext` (default-`None` methods, `GameWorld` impls)
Add trait methods (all with safe defaults so `NullEventDispatcher`/tests need no change):
- `get_player_level(id) -> Option<i32>`
- `get_player_account_type(id) -> Option<u8>`
- `get_player_vocation_id(id) -> Option<i32>`
- `player_has_flag(id, flag: u64) -> bool` (default `false`)
- `get_creature_condition(id, ctype: u8, cond_id: i32, sub_id: u32) -> Option<i32>` (remaining ticks, or `None`)

`GameWorld` impls reuse existing helpers (`player_has_flag`, `player_group_flags`, `Player.level`,
`Player.vocation_id`, `condition.rs` active-condition scan). Bindings in `userdata/player.rs` follow
the existing `with_ctx(|ctx| …)` read pattern.

### 1.3 Account type — backing-data plumb (the one real data gap)
`Player` has no account type today. `accounts.type` **is** read in the DB layer
(`account.rs` `SELECT ... type ...`) but dropped after auth. Plan:
1. Carry `accounts.type` through the login path into a new `Player.account_type: u8` field
   (default `ACCOUNT_TYPE_NORMAL = 1`).
2. Expose via `ScriptContext::get_player_account_type`.
This is the only item that touches DB/login, not just Lua — call it out in LUA-2 so it isn't
mistaken for a pure binding.

### 1.4 Vocation object — minimal `Vocation` userdata
Scripts only call `player:getVocation():getId()`. Two options (LUA-2 picks one):
- **(a)** `Vocation` userdata wrapping `vocation_id`, method `getId()` (extensible later for
  `getName`/`getPromotion`). Matches C++ `luaPlayerGetVocation` returning a `Vocation` object.
- **(b)** Return the int id directly and change scripts to `player:getVocation()`. **Rejected** —
  diverges from stock TFS scripts; keep script parity, choose (a).

### 1.5 Mutations — `LuaMutation` additions, immediate apply
Per the rule "if C++ applies before the Lua call returns, Rust must too" — `addCondition`/
`removeCondition`/`sendCancelMessage` are all synchronous in C++ and scripts read back state in the
same callback (e.g. `getCondition` right after `addCondition`), so **immediate apply**, not tick-end:
- `PlayerAddCondition { creature_id, ctype, cond_id, sub_id, ticks }` (or a richer condition payload,
  see §4.2)
- `PlayerRemoveCondition { creature_id, ctype, cond_id, sub_id }`
- `PlayerSendCancelMessage { creature_id, text }` — outbound only (client-visible), technically safe
  to defer, but trivial to apply immediately; keep it simple and immediate.

Wire the applier arm in `tfs-rust-core` (the existing `apply_lua_mutation` registered at startup).

### 1.6 `Condition` object — replace the soul stub with a real userdata
Current `Condition(type,id)` returns a throwaway table (no-op setters) so `player.lua` loads. Replace
with a `ConditionBuilder` userdata that accumulates `{ ctype, cond_id, sub_id, ticks, params }` and is
consumed by `player:addCondition(condition)` (reads the builder's fields into `PlayerAddCondition`).
Keep `setTicks` + `setParameter` working so `player.lua`'s `soulCondition` still loads unchanged
(regression guard — see §4.1).

### 1.7 Global functions — `sendChannelMessage(channelId, type, message)`
Used only in `help.lua`'s `!mute`/`!unmute` broadcast. Register as a Lua global that routes to a
`GameWorld` channel fan-out (reuse `player_talk_to_channel`'s member send path, minus the speaker
membership check — it's a server-originated channel message). Needs the mutation scope (writes
outbound packets). Defer to LUA-4 with the rest of the CH-5 block.

---

## 2. Phased plan

Ordering: constants first (unblocks all enum comparisons in active hook bodies), then reads (unblocks
the active gating logic), then outbound cancel (unblocks the level-1 / mute cancel messages), then the
full CH-5 condition/mute surface (unblocks the commented-out blocks). Each phase leaves the tree
building and the channel scripts loading.

### LUA-1 — Constants registration (unblocks active hook comparisons) — ✅ DONE
1. New `constants.rs` + `register_constants` called from `LuaRuntime::new`.
2. Register: `ACCOUNT_TYPE_*` (`enums.h:79-85`), `TALKTYPE_*` **772 values** (`const.h:61-77` — same
   set `game_world_chat.rs` uses; source from `tfs_rust_common::enums` if a `SpeakClasses` enum is
   added there, else cite), `PlayerFlag_CanTalkRedChannel/TalkOrangeHelpChannel/CanTalkRedPrivate`
   (`const.h:264-266`), `VOCATION_NONE` (`enums.h:297`).
3. Move the existing scattered constant `globals.set` lines out of `register_event_script_bootstrap`
   into `constants.rs`; fix `CONDITIONID_DEFAULT` to `-1` (§4.3).
4. Verify: all 8 channel scripts load; a smoke test evaluating `ACCOUNT_TYPE_GOD == 6` etc. passes.

**Done.** `crates/tfs-rust-lua/src/constants.rs` centralizes all bare enum/flag constants with
772-correct values and `const.h`/`enums.h` cites. Also fixed three pre-existing wrong values
uncovered while moving: `TALKTYPE_CHANNEL_Y/O/R1` (were 7/8/14 → 5/12/10),
`PlayerFlag_CanTalkRedChannel` (was `1<<21` → `1<<22`; `1<<21` is `CanTalkRedPrivate`, now also
registered), `CONDITION_SOUL` (was 0 → `1<<13`), `CONDITION_PARAM_SOULGAIN/SOULTICKS` (were 0 →
12/13). `TALKTYPE_*` are hardcoded with cites rather than sourced from `tfs_rust_common::enums`
because the existing `SpeakType` enum carries 1098 numbering, not 772 — adding a 772-correct
`SpeakClasses` enum is deferred (it would touch `idle_stimulus.rs` callers). Tests:
`constants::tests::register_constants_sets_expected_values`,
`runtime::tests::channel_scripts_load_with_constants` (loads all 8 channel scripts + spot-checks
values). NOTE: pre-existing `player_events_script_loads_with_bootstrap` test was already failing
on `main` (missing `Player:onInventoryUpdate`) — unrelated to LUA-1.

### LUA-2 — Player read methods + account-type backing + `Vocation` object — ✅ DONE
1. `ScriptContext`: add `get_player_level`, `get_player_account_type`, `get_player_vocation_id`,
   `player_has_flag`; `GameWorld` impls.
2. **Account-type plumb (§1.3):** add `Player.account_type`, carry `accounts.type` through login.
3. `Vocation` userdata (§1.4 option a) + `player:getVocation()` binding.
4. `userdata/player.rs` bindings: `getLevel`, `getAccountType`, `getVocation`, `hasFlag`.
5. Verify: unit test a `ScriptContext` fake returning a GM player; assert `getAccountType`/`hasFlag`/
   `getLevel`/`getVocation():getId()` read through.

**Done.** `Player.account_type: u8` plumbed from `accounts.type` (folded into the existing
`premium_ends_at` `SELECT` — one query, no extra round-trip; defaults to
`ACCOUNT_TYPE_NORMAL` (1) on NULL). `ScriptContext` gained four default-`None`/`false`
read methods; `GameWorld` impls reuse `player_has_flag` (`stats.rs`) and the new
`account_type` field. `VocationRef(i32)` userdata (`userdata/vocation.rs`) wraps the raw
`players.vocation` id with `getId()` — extensible for `getName`/`getPromotion` later.
Bindings `getLevel`/`getAccountType`/`getVocation`/`hasFlag` added to `CreatureRef`.
Tests: `userdata::player::tests::player_read_methods_return_gm_values_through_lua`
(fake GM ctx → Lua round-trip) + `player_read_methods_default_none_does_not_panic`
(null ctx degrades gracefully). All 5 `Player { … }` construction sites updated
(login.rs, sim_harness.rs, spell_tests.rs, tests/arena.rs, notifications.rs).
**Verify:** `cargo check` (0 errors), `cargo clippy --all-targets` (0 new warnings),
`cargo test -p tfs-rust-{common,db,core,lua}` (606 + 10 lua tests pass; the one
pre-existing `player_events_script_loads_with_bootstrap` failure is unrelated —
missing `Player:onInventoryUpdate`, fails on `main` too).

### LUA-3 — `player:sendCancelMessage` (unblocks the active level-1 cancel path)
1. `LuaMutation::PlayerSendCancelMessage { creature_id, text }` + `call_lua_send_cancel_message`.
2. Core applier arm → `send_text_message_simple(failure_message_type(), text)` enqueue (reuse the
   exact path `game_world_chat.rs` already uses for mute/yell cancels).
3. `player:sendCancelMessage(text)` binding.
4. Note: stock scripts also pass a `RETURNVALUE_*` enum to `sendCancelMessage` (help.lua commented
   block) — accept both `string` and integer return-value; integer maps via a `ReturnValue`→message
   table (LUA-4 registers the enum; the message mapping can land here as a stub returning the raw code
   until then).

### LUA-4 — Condition API + `Player(name)` + `sendChannelMessage` (unblocks CH-5 commented blocks)
1. Register `CONDITION_CHANNELMUTEDTICKS` (`enums.h:268`), `CONDITION_PARAM_SUBID=45`,
   `CONDITION_PARAM_TICKS=2`, and the `RETURNVALUE_*` block (`enums.h` `ReturnValue_t`).
2. Real `Condition` userdata (§1.6) replacing the soul stub; keep `setTicks`/`setParameter`.
3. `ScriptContext::get_creature_condition` (read remaining ticks by type+id+subid).
4. `LuaMutation::PlayerAddCondition` / `PlayerRemoveCondition` + immediate-apply core arms
   (reuse `condition.rs` `apply_condition` / removal), matching chat-system-plan **CH-5** mute model
   (`CONDITION_CHANNELMUTEDTICKS`, subId = channel id, 120000ms trade/advertising, 3600000ms help).
5. `player:getCondition` / `player:addCondition` / `player:removeCondition` bindings.
6. `Player(name)` constructor global → resolve `player_by_name` → `CreatureRef` or `nil`.
7. `sendChannelMessage(channelId, type, message)` global (§1.7).
8. **Uncomment** the CH-5 blocks in `advertising.lua`, `advertising-rook.lua`, `trade.lua`, `help.lua`
   once 1–7 land; verify each loads and (with CH-4 hook invocation) mutes/broadcasts correctly.

### LUA-5 — Tests
1. `constants.rs` smoke test (all channel scripts load + spot-check enum values).
2. `ScriptContext` fake-backed unit tests for each read method.
3. Mutation round-trip: `addCondition` then `getCondition` in the same scope returns the applied ticks
   (validates immediate-apply, not tick-end deferral — the core invariant from
   `TFS-lua-boundaries.md`).
4. Channel-hook integration (depends on CH-4 invocation): GM speaking `TALKTYPE_CHANNEL_Y` in
   Game-Chat is upgraded to `TALKTYPE_CHANNEL_O`; level-1 non-GM is cancelled.

---

## 3. Dependency & sequencing notes
- **Hard prerequisite for observing any of this:** chat-system-plan **CH-4 hook invocation** (call
  `call_channel_on_speak`/`call_channel_can_join` from core and honor the return / rewritten type).
  LUA-1..LUA-4 can be built independently but are only exercised once that lands. Recommend landing
  LUA-1/LUA-2 alongside CH-4 so the active hooks work end to end, then LUA-3/LUA-4 for the mute blocks.
- **CH-5 (flood mute) already exists in Rust** (`player_remove_message_buffer` applies
  `ConditionType::Muted`). LUA-4's `CONDITION_CHANNELMUTEDTICKS` is the **per-channel** offer-throttle
  mute (distinct from global flood mute) — both use the same `Condition` machinery; don't fork.
- Constants must not drift: prefer sourcing from `tfs_rust_common::enums` (add a `SpeakClasses` /
  `AccountType` enum there if it eases single-sourcing) over re-hardcoding in `constants.rs`.

## 4. Open questions (resolve while implementing)
1. **`Condition` stub regression.** `data/events/scripts/player.lua` builds `soulCondition` via the
   current no-op `Condition`. LUA-4's real `Condition` must keep `setTicks`/`setParameter` working (or
   player.lua breaks at load). Confirm the field set the real builder needs covers both soul and
   channel-muted use before replacing the stub.
2. **`addCondition` payload shape.** Minimum for channel-mute is `{ctype, cond_id, sub_id, ticks}`.
   Soul/other conditions carry more params (`CONDITION_PARAM_SOULGAIN`, …). Decide whether
   `PlayerAddCondition` takes a flat set of known params or an opaque param map — lean flat for now
   (only ticks + subid are used by channels), extend when a second consumer needs more.
3. **`CONDITIONID_DEFAULT` value bug.** Bootstrap currently registers `CONDITIONID_DEFAULT = 0`;
   `enums.h:275` says `-1`. Fix in LUA-1. Audit whether anything already relies on the wrong `0`
   (grep `CONDITIONID_DEFAULT` in `data/`).
4. **Account type vs. group.** Scripts gate on `getAccountType()` (account-level: NORMAL/TUTOR/GM/GOD)
   while this codebase's access checks use `group_id`→`Group.access`. Confirm the DB has a populated
   `accounts.type` and that account type — not group — is the correct gate for these channels (it is,
   per 772 `chat.cpp`), then plumb account type (§1.3) rather than aliasing to group.
5. **`sendChannelMessage` origin.** Server-originated channel messages (help `!mute` broadcast) have no
   speaker creature. Confirm the wire path (`send_to_channel`/`sendChannelMessage`,
   `protocolgame.cpp` `sendToChannel` vs `sendChannelMessage`) and which opcode the anonymous
   broadcast uses in 772 before wiring the global.

## 5. Naming compliance (TFS-Core.md)
No `cip`/`Cip`/`CipSoft` identifiers. Lua method/constant names mirror `luascript.cpp`
`registerMethod`/`registerEnum` exactly (API parity). Rust symbols (`register_constants`,
`get_player_account_type`, `PlayerAddCondition`) are descriptive, not decompile-transliterated.
