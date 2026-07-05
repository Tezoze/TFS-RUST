# Chat System — Implementation Plan (772 wire, TFS structure)

**Goal.** Wire the full player chat pipeline — say/whisper/yell/private message/broadcast/channels
(guild, party, help, private) — end to end: client packet → game-thread dispatch → channel/spectator
fan-out → wire encode. **Wire bytes and mechanics outcomes match the TVP 7.72 decompile**
(`reference/tvp-772/gameserver/src/{chat.cpp,chat.h,talkaction.h,protocolgame.cpp,game.cpp,player.cpp}`).
Code shape follows TFS structure (`ChatChannel`/`Chat` class shapes, `Condition`-based mute/yell
timers) per `TFS-Core.md` §Porting model — outcomes ported, not transcribed.

**Active target:** `clientVersion = 772`. 1098 shares the same `GameWorld` dispatch through
`MechanicsProfile`/`ProtocolCodec`; this plan adds no `if version == 772` branches to
`tfs-rust-core`. Wire divergences (if any) are isolated to `tfs-rust-net::codec::{v772,v1098}`.

---

## 0. Current-state audit (read before implementing — avoids duplicate work)

This is further along than it looks. **Wire parsing for every chat-related client packet already
exists**; what's missing is almost entirely the game-thread dispatch layer.

### 0.1 Already done — incoming wire
- `crates/tfs-rust-common/src/protocol_opcodes.rs::client` has every opcode byte needed, verified
  1:1 against `reference/tvp-772/gameserver/src/protocolgame.cpp` `parsePacket` (~L499–518):
  `SAY=0x96`, `REQUEST_CHANNELS=0x97`, `OPEN_CHANNEL=0x98`, `CLOSE_CHANNEL=0x99`,
  `OPEN_PRIVATE_CHANNEL=0x9A`, `CREATE_PRIVATE_CHANNEL=0xAA`, `CHANNEL_INVITE=0xAB`,
  `CHANNEL_EXCLUDE=0xAC`.
- `crates/tfs-rust-common/src/game_packet.rs::GamePacket` already has fully-parsed variants:
  `Say(SayPayload { speak_class, channel_id, receiver, text })`, `RequestChannels`,
  `OpenChannel { channel_id }`, `CloseChannel { channel_id }`, `OpenPrivateChannel { receiver }`,
  `CreatePrivateChannel`, `ChannelInvite { name }`, `ChannelExclude { name }`. No new wire parsing
  work needed for these opcodes.

### 0.2 Already done — outgoing wire (partial)
`crates/tfs-rust-net/src/outgoing_extra.rs` has:
- `send_creature_say` — `sendCreatureSay` (`protocolgame.cpp:1422`), `0xAA`.
- `send_to_channel` — `sendToChannel` (`protocolgame.cpp:1442`), `0xAA`.
- `send_private_message_speech` — `sendPrivateMessage` (`protocolgame.cpp:1465`), `0xAA`.
- `send_channel_message` — `sendChannelMessage` (`protocolgame.cpp:1306`), `0xAA`.
- `outgoing.rs::send_text_message` / `outgoing_extra.rs::send_text_message_simple` — `sendTextMessage`
  (`0xB4`) for `MESSAGE_STATUS_SMALL` cancel/info text (mute messages, "sent to X", etc.).

**Missing outgoing wire** (none of these exist yet — add to `outgoing_extra.rs`):
| Function to add | C++ ref | Opcode |
|---|---|---|
| `send_channels_dialog` | `sendChannelsDialog` (`protocolgame.cpp:1282`) | `0xAB` |
| `send_channel` (open-channel ack) | `sendChannel` (`protocolgame.cpp:1297`) | `0xAC` |
| `send_create_private_channel` | `sendCreatePrivateChannel` (`protocolgame.cpp:1273`) | `0xB2` |
| `send_close_private` | `sendClosePrivate` (`protocolgame.cpp:1265`) | `0xB3` |
| `send_open_private_channel` | `sendOpenPrivateChannel` (`protocolgame.cpp:1111`) | `0xAD` |

Note the byte reuse across directions is intentional and matches the decompile: client
`CREATE_PRIVATE_CHANNEL`/`CHANNEL_INVITE`/`CHANNEL_EXCLUDE` are `0xAA`/`0xAB`/`0xAC` **incoming**,
while `sendChannelsDialog`/`sendChannel` reuse `0xAB`/`0xAC` **outgoing** — same table, opposite
directions, exactly as in `protocolgame.cpp`. Don't "fix" this into distinct constants.

### 0.3 Already done — spectator fan-out primitive
`crates/tfs-rust-core/src/game_world_spectators.rs::broadcast_creature_say_viewport` already
implements the `TALKTYPE_SAY`/`TALKTYPE_MONSTER_SAY` viewport fan-out (grid-based spectator lookup,
per-viewer statement-id alloc, era-aware codec dispatch via `CreatureSayWire`). This is used today
only by monster/idle-stimulus talk (`idle_stimulus.rs:1330,2218`). **Reuse this for `TALKTYPE_SAY`**;
it does **not** yet support yell's wider viewport (772 `internalCreatureSay`: normal range uses
`Map::maxClientViewportX/Y` via `map.getSpectators(..., false, false, ...)`, yell uses the wider
`(true, false, 18, 18, 14, 14)` — `game.cpp:3518-3524`). See CH-2 for the yell-range gap.

### 0.4 Already done — supporting registries
- `crates/tfs-rust-core/src/guild.rs::GuildRegistry` — `player_guild: HashMap<CreatureId, u32>`,
  usable directly for `CHANNEL_GUILD` membership/MOTD gating.
- `crates/tfs-rust-core/src/party.rs::Party` / `PartyInviteState` — usable directly for
  `CHANNEL_PARTY` membership.
- `tfs_rust_common::enums::ConditionType` **already has** `Muted = 16`, `ChannelMutedTicks = 17`,
  `YellTicks = 18` in the enum (`enums.rs:54-56`) — ported ahead of use, matching TFS
  `CONDITION_MUTED`/`CONDITION_CHANNELMUTEDTICKS`/`CONDITION_YELLTICKS`. **Zero call sites reference
  them today** (`grep` confirms) — no mute/flood system exists yet. This plan is the first consumer.

### 0.5 Not started — everything else
- No `Chat`/`ChatChannel`/`PrivateChatChannel` equivalent exists in `tfs-rust-core`.
- `handle_game_packet` in `crates/tfs-rust-core/src/game_loop.rs` has **no match arm** for
  `GamePacket::Say`, `RequestChannels`, `OpenChannel`, `CloseChannel`, `OpenPrivateChannel`,
  `CreatePrivateChannel`, `ChannelInvite`, or `ChannelExclude` — they all fall into the catch-all
  `_ => trace!(..., "game packet — simulation Phase 9+")` at `game_loop.rs:511`. This is the
  primary gap this plan closes.
- No flood/mute system (`Player::isMuted`/`addMessageBuffer`/`removeMessageBuffer` equivalents,
  `player.cpp:1335-1380`) exists on `creature/player.rs::Player`.
- No word-based spell/talkaction dispatch (`Game::playerSaySpell`, `game.cpp:3375-3398`) exists —
  `spell.rs` only has instant-spell gating (`can_cast_instant`), not say-triggered casting. Out of
  scope here (§1); this plan adds the **call site** so it's a one-line swap-in later.
- No `EventDispatcher::on_creature_say` / `on_hear` hooks (`Events::eventCreatureOnHear`,
  `game.cpp:3542`) — needed for future talkactions/creaturescripts scripting, not for this plan's
  core delivery, but the trait should gain no-op defaults now so call sites don't need revisiting.
- `data/talkactions/talkactions.xml` is an empty placeholder (`<talkactions></talkactions>`); no
  Lua talkactions runtime exists in `tfs-rust-lua` (per `TFS-lua-boundaries.md` §Full API Port
  Plan step 3, still pending). Out of scope (§1).
- No self-registering Lua object (`Action()`/`TalkAction()`-style userdata + `:register()`) is wired
  in `tfs-rust-lua` yet at all — `crates/tfs-rust-lua/src/userdata/` only has `container.rs`/`item.rs`/
  `player.rs`. The `data/scripts/actions/**/*.lua` and `data/scripts/talkactions/**/*.lua` files that
  already use this convention are **inert placeholders today** (nothing loads them). This plan's
  `Channel` object (§2.1) will be the **first** implementation of this pattern in `tfs-rust-lua` — not
  a small XML-loader tweak. Budget CH-4 accordingly.

### 0.6 Done this session — `data/chatchannels/` moved under `data/scripts/`
Per user direction: `data/chatchannels/` relocated to `data/scripts/chatchannels/` (flat, no nested
`scripts/` subfolder — matches `data/scripts/talkactions/<category>/*.lua` shape minus the category
split, since channels don't need one). `chatchannels.xml` was **deleted** (git-staged as a rename +
delete, not a plain new-file add, so history is preserved). Current tree:
```
data/scripts/chatchannels/
├── advertising.lua
├── advertising-rook.lua
├── englishchat.lua
├── gamechat.lua
├── gamemaster.lua
├── help.lua
├── realchat.lua
├── ruleviolations.lua      (excluded, §1 RVR non-goal — leave the file, never load it)
├── trade.lua
├── tutor.lua
└── worldchat.lua
```
All 11 files still contain **old-style bare globals** (`function onSpeak(...)`, `function canJoin(...)`)
— none have been rewritten to the new `Channel(id, name)` self-registering convention yet. That
rewrite is CH-4 work (§3), now that the convention itself is decided (§2.1).

---

## 1. Scope & non-goals

**In scope (this plan):**
1. `TALKTYPE_SAY` — local say, viewport broadcast (reuse `broadcast_creature_say_viewport`).
2. `TALKTYPE_WHISPER` — narrow-range say with "pspsps" garbling outside 1-tile range.
3. `TALKTYPE_YELL` — wide-range say, level gate, `CONDITION_YELLTICKS` 30s exhaust, uppercasing.
4. `TALKTYPE_PRIVATE` / `TALKTYPE_PRIVATE_RED` — player-to-player tell.
5. `TALKTYPE_BROADCAST` — GM broadcast to all online players.
6. `TALKTYPE_CHANNEL_Y/O/R1/R2` — default channel talk (guild, party, help, trade, game-chat, RL-chat)
   + player-created private channels.
7. Channel lifecycle: request channel list, open/close channel, open/create/invite/exclude private
   channel.
8. Flood protection: `CONDITION_MUTED` mute-on-flood (`MessageBufferCount`/`muteCountMap` exponential
   backoff), `removeMessageBuffer`/`addMessageBuffer` tick hook.
9. Missing outgoing wire functions (§0.2 table) + codec wiring.
10. `EventDispatcher::on_creature_say`/`on_hear` no-op hook additions (call sites only — Lua body is
    out of scope).

**Explicit non-goals (flag, don't silently build):**
- **Rule Violation Report (RVR) GM system** — `TALKTYPE_RVR_*`, opcodes `0x9B-0x9D`/`0xAE-0xB1`,
  `CHANNEL_RULE_REP`. This is a legacy CipSoft-only GM reporting tool superseded by modern
  report/bug-report flows. Recommend explicit user sign-off before porting; not included in the
  phases below. `parseProcessRuleViolationReport`/`parseCloseRuleViolationReport` opcodes already
  have `GamePacket::RuleViolationReport` parsing (unrelated newer report system) — do not conflate
  the two.
- **Word-based spell casting / talkactions Lua scripting** — `Game::playerSaySpell` dispatch order
  is respected (call site added, see CH-1 step 2) but the spell-words and talkaction lookup tables
  themselves are a separate `tfs-rust-lua` phase (`TFS-lua-boundaries.md` step 3).
- **NPC chat integration** — `creature/npc.rs::on_say` already has a stub; wiring player `Say` into
  NPC trade/dialog is a separate NPC-behavior phase, not blocked by this plan but not delivered by it.
- **Guild channel MOTD scheduler** (`g_scheduler.addEvent(150ms, sendGuildMotd)`, `chat.cpp:75`) —
  deferred; CH-4 notes the call site but ships a synchronous send instead of the 150ms-delayed one
  unless the user wants exact timing parity (would need the existing `Scheduler`, see
  `scheduler.rs`).

---

## 2. Architecture

### 2.1 New module: `crates/tfs-rust-core/src/chat.rs`
TFS-shaped (`ChatChannel`/`Chat`), not a line-port of `chat.cpp`:

```rust
// C++ reference: chat.h/chat.cpp (ChatChannel, PrivateChatChannel, Chat).
pub struct ChatChannel {
    pub id: u16,
    pub name: String,
    pub public_channel: bool,
    pub users: HashSet<CreatureId>,       // C++ UsersMap keyed by player id; SlotMap key suffices
}

pub struct PrivateChatChannel {
    pub base: ChatChannel,
    pub owner: CreatureId,
    pub invited: HashSet<u32>,            // player guid, matches C++ InvitedMap key
}

#[derive(Default)]
pub struct ChatRegistry {
    pub normal_channels: HashMap<u16, ChatChannel>,      // CHANNEL_GUILD/PARTY + Lua-defined statics from data/scripts/chatchannels
    pub private_channels: HashMap<u16, PrivateChatChannel>, // dynamic ids, CHANNEL_PRIVATE base + counter
    pub next_private_channel_id: u16,
}
```

Static channel ids from the decompile (`const.h:302-305`): `CHANNEL_GUILD=0x00`,
`CHANNEL_PARTY=0x01`, `CHANNEL_RULE_REP=0x02` (excluded, §1), `CHANNEL_PRIVATE=0xFFFF` (base for
dynamic private channels — TFS allocates real ids above this range for concrete instances; confirm
exact allocation scheme against `Chat::createChannel` before implementing CH-4). Guild/party/private
channels are created **dynamically** at runtime (per guild/party/player) and stay Rust-native —
never script-defined. The **static** public/GM/tutor channels (Tutor=3, Game-Chat=4, RL-Chat=5,
Trade=6, Help=7, Gamemaster=8) move to self-registering Lua (decided below).

**Decided (locked in, supersedes earlier drafts of this plan) — no XML, self-registering `Channel`
Lua object, real script execution.** Two things changed from the original draft:

1. `data/chatchannels/chatchannels.xml` is **deleted** (already done, see §0.6). There is no static
   channel manifest at all. The loader directory-scans `data/scripts/chatchannels/*.lua` and executes
   every file it finds; each file declares its own id/name and registers itself. Dropping a file out
   of that directory disables the channel — no index to keep in sync, matching the
   `data/scripts/actions|talkactions` convention already established for those (currently unloaded)
   systems.
2. The `canJoin`/`onSpeak` hooks in these scripts are **real logic**, not stubs (verified this pass —
   every file implements at least one of TFS's four `ChatChannel` script hooks,
   `executeCanJoinEvent`/`executeOnJoinEvent`/`executeOnLeaveEvent`/`executeOnSpeakEvent`,
   `chat.cpp:119-239`), so this plan executes them for real via `tfs-rust-lua` rather than
   hand-porting equivalent Rust predicates. This pulls part of the "self-registering Lua object"
   infra (§0.5) forward into this plan — `Channel` is the **pilot** implementation of the
   `Action`/`TalkAction`-style pattern, ahead of those two systems.

**Lua-side convention** (mirrors `local action = Action(); action:id(x); action:register()` /
`local talkaction = TalkAction("!word"); talkaction:register()` exactly — see
`data/scripts/actions/quests/botanist_container.lua`, `data/scripts/talkactions/players/buypremium.lua`):

```lua
-- data/scripts/chatchannels/gamechat.lua (public channel)
local channel = Channel(4, "Game-Chat")
channel:public(true)

function channel.onSpeak(player, type, message)
	local playerAccountType = player:getAccountType()
	if player:getLevel() == 1 and playerAccountType < ACCOUNT_TYPE_GAMEMASTER then
		player:sendCancelMessage("You may not speak into channels as long as you are on level 1.")
		return false
	end
	if type == TALKTYPE_CHANNEL_Y then
		if playerAccountType >= ACCOUNT_TYPE_GAMEMASTER then
			type = TALKTYPE_CHANNEL_O
		end
	elseif type == TALKTYPE_CHANNEL_O then
		if playerAccountType < ACCOUNT_TYPE_GAMEMASTER then
			type = TALKTYPE_CHANNEL_Y
		end
	end
	return type
end

channel:register()
```

```lua
-- data/scripts/chatchannels/gamemaster.lua (private, canJoin-gated, no :public() → defaults false)
local channel = Channel(8, "Gamemaster")

function channel.canJoin(player)
	return player:getAccountType() >= ACCOUNT_TYPE_GAMEMASTER
end

function channel.onSpeak(player, type, message)
	-- ... (same TALKTYPE_Y/O/R1 upgrade logic as today's bare gamemaster.lua, unchanged)
end

channel:register()
```

Constructor `Channel(id, name)` carries identity (matches `ChatChannel(channelId, channelName)`,
`chat.h:19-20`); `:public(bool)` is a fluent setter (matches `talkaction:separator(" ")`); hooks are
plain table-field functions (matches `function action.onUse(...)`/`function talkaction.onSay(...)`);
`:register()` finalizes — same 4-part shape as the existing (currently-inert) `Action`/`TalkAction`
convention, applied here for the first time end-to-end.

**Rust side — new `tfs-rust-lua` module + userdata, mirrors `move_events.rs`'s registry-builder
shape:**

```rust
// crates/tfs-rust-lua/src/userdata/channel.rs
//! `Channel` — self-registering chat-channel definition (revscriptsys-style pilot; no
//! CipSoft/TFS precedent for the *registration mechanism* itself — TFS 1.4.2 still loads
//! chatchannels.xml even in the revscriptsys era, confirmed against repo-root `src/chat.cpp`
//! `Chat::load`). Hook *contracts* (`onSpeak`/`canJoin`/`onJoin`/`onLeave`) mirror
//! `chat.cpp` `executeOnSpeakEvent`/`executeCanJoinEvent`/`executeOnJoinEvent`/`executeOnLeaveEvent`.

pub struct ChannelHandle {
    pub id: u16,
    pub name: String,
    pub public: Cell<bool>,
}

impl UserData for ChannelHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("public", |_, this, is_public: bool| {
            this.public.set(is_public);
            Ok(())
        });
        // Reads `onSpeak`/`canJoin`/`onJoin`/`onLeave` off the same Lua table this userdata
        // lives under (all optional), wraps any that exist as `CallbackRef`s via
        // `LuaRuntime::register_callback`, and pushes a `ChatChannelDef` into the loader's
        // pending-channel buffer (drained after the directory scan completes).
        methods.add_method("register", |lua, this, ()| { /* ... */ Ok(()) });
    }
}
```

```rust
// crates/tfs-rust-lua/src/chat_channels.rs (directory-scan loader, mirrors move_events.rs)
pub struct ChatChannelDef {
    pub id: u16,
    pub name: String,
    pub public: bool,
    pub on_speak: Option<CallbackRef>,
    pub can_join: Option<CallbackRef>,
    pub on_join: Option<CallbackRef>,
    pub on_leave: Option<CallbackRef>,
}

/// No manifest file — every `.lua` directly under `data/scripts/chatchannels/` self-registers.
pub fn load_chat_channel_scripts(
    runtime: &mut LuaRuntime,
    data_dir: &Path,
) -> Result<Vec<ChatChannelDef>, LuaError> {
    let dir = data_dir.join("scripts/chatchannels");
    for entry in std::fs::read_dir(&dir)?.filter_map(Result::ok) {
        if entry.path().extension().is_some_and(|e| e == "lua") {
            runtime.load_script(&entry.path().display().to_string())?;
        }
    }
    Ok(runtime.drain_pending_chat_channels()) // populated by ChannelHandle::register
}
```

`GameWorld::chat: ChatRegistry` consumes the returned `Vec<ChatChannelDef>` at startup (`run_server.rs`,
alongside guild/party registry init) to seed `normal_channels`. `canJoin`/`onSpeak` execute through
`LuaRuntime::call_creature_callback`-style dispatch (same error-handling contract as
`TFS-lua-boundaries.md` §Error Handling — Lua failures log and fall back to the C++ default, i.e.
`canJoin` absent ⇒ always joinable, `onSpeak` absent ⇒ type unchanged, matching `chat.cpp:139-163`'s
`canJoinEvent == -1` early-return-true / analogous behavior).

`ruleviolations.lua` stays on disk (git history preserved) but is **never loaded** — excluded per §1's
RVR non-goal. Either skip it by filename in the loader (simplest, slightly special-cased) or leave it
until RVR is explicitly greenlit and it registers `CHANNEL_RULE_REP` like everything else at that
point. Flag this as a to-confirm detail in CH-4, not a blocker.

### 2.2 `GameWorld` field
Add `pub chat: chat::ChatRegistry` to `GameWorld` (game-thread only, no `Send`/`Sync` needed —
follows `TFS-threading.md`). Loaded at startup alongside guild/party registries in `run_server.rs`.

### 2.3 Dispatch module: `crates/tfs-rust-core/src/game_world_chat.rs`
Mirrors `game_world_spectators.rs`'s pattern (`impl GameWorld` extension file). Houses:
- `player_say` — `Game::playerSay` (`game.cpp:3208-3281`) dispatch switch on `speak_class`.
- `player_whisper` — `Game::playerWhisper` (`game.cpp:3400-3422`).
- `player_yell` — `Game::playerYell` (`game.cpp:3424-3453`).
- `player_speak_to` — `Game::playerSpeakTo` (`game.cpp:3455-3479`).
- `player_broadcast_message` — `Game::playerBroadcastMessage` (`game.cpp:2005-2018`).
- `player_talk_to_channel` — `Chat::talkToChannel` + `ChatChannel::talk` (`chat.cpp:107-117`).
- `player_request_channels` / `player_open_channel` / `player_close_channel` —
  `Game::playerRequestChannels/OpenChannel/CloseChannel` (`game.cpp:2083-2120`).
- `player_create_private_channel` / `player_channel_invite` / `player_channel_exclude` /
  `player_open_private_channel` — `game.cpp:2020-2140`.
- Flood gate: `player_add_message_buffer` (tick hook) / `player_remove_message_buffer` (per-say hook)
  — `Player::addMessageBuffer`/`removeMessageBuffer` (`player.cpp:1350-1380`).

### 2.4 `handle_game_packet` wiring (`game_loop.rs`)
Add match arms for `GamePacket::Say(_)`, `RequestChannels`, `OpenChannel`, `CloseChannel`,
`OpenPrivateChannel`, `CreatePrivateChannel`, `ChannelInvite`, `ChannelExclude` — each resolves
`cid` via `world.conn_to_creature` (existing pattern, e.g. `GamePacket::Attack` arm) and calls the
matching `game_world_chat.rs` method. Remove these variants from the `_ => trace!` catch-all.

Also remove `GamePacket::Say(_)` from the `game_packet_requires_timed_action` **exclusion** list
(`game_loop.rs:135`) only if 772 parity requires an action-queue gate on say — the decompile's
`ProtocolGame::parseSay` (`protocolgame.cpp:949-959`) does **not** route through `ToDoAdd`/action
queue for OTClientV8, but the real client path does `addActionToDo` + `startToDo`. Confirm against
`connections.rs::packet_counts_as_action` before changing this — likely **no change needed** since
say already appears in the "no timed action" exclusion list and that matches the OTC-first design
already used elsewhere in this codebase (see `parseSay`'s `otclientV8` branch).

### 2.5 EventDispatcher additions (`event_dispatcher.rs`)
```rust
/// TFS `Creature::onCreatureSay` — hear callback, e.g. NPC/creaturescript. Default no-op.
fn on_creature_say(&self, _hearer: CreatureId, _speaker: CreatureId, _speak_type: u8, _text: &str) {}
/// TFS `Events::eventCreatureOnHear` (`game.cpp:3542`) — script-side hear hook, excludes self.
fn on_hear(&self, _hearer: CreatureId, _speaker: CreatureId, _text: &str, _speak_type: u8) {}
```
Call both from `broadcast_creature_say_viewport` and the new channel/private/whisper/yell paths —
mirrors C++ "send to client" + "event method" two-pass loop (`game.cpp:3529-3544`). Lua body deferred
(§1 non-goals); `NullEventDispatcher` no-op is sufficient for this plan to land cleanly.

---

## 3. Phased plan

### CH-0 — Missing outgoing wire (prerequisite, small, unblocks everything else)
1. Add the 5 functions from §0.2's table to `outgoing_extra.rs`, byte-exact per the cited
   `protocolgame.cpp` line ranges.
2. Wire them through `codec::wire.rs` / `v772.rs` / `v1098.rs` if era divergence exists (check 1098
   `src/protocolgame.cpp` equivalents for `sendChannelsDialog`/`sendChannel`/etc. — expect identical
   shape; 1098 may add `channel->isClosable()` or level requirements not in 772 — **verify against
   repo-root `src/protocolgame.cpp` before assuming parity**).
3. Unit tests: byte-for-byte wire snapshot tests, following the existing pattern in
   `crates/tfs-rust-net/tests/protocol_compat.rs`.

### CH-1 — `player_say` dispatch skeleton + plain SAY
1. Add `chat.rs` module (empty `ChatRegistry` ok for this phase — SAY doesn't need channels).
2. Add `game_world_chat.rs::player_say` matching `Game::playerSay`'s switch (`game.cpp:3235-3280`),
   initially handling only `TALKTYPE_SAY` (call `broadcast_creature_say_viewport`), with pass-through
   stubs (`todo!`/`warn!`) for the other arms — filled in by CH-2/CH-3/CH-4.
3. Add the `playerSaySpell` call-site stub (`game.cpp:3219`: check spell/talkaction first, return
   early if handled) — call into `spell.rs` only if/when word-based casting exists; until then, a
   `false`-returning stub with a `// TODO(chat): wire playerSaySpell once talkactions land` comment
   is correct and matches current behavior (no spells triggered via say text today).
4. Wire `GamePacket::Say` in `handle_game_packet` (§2.4).
5. Text length validation: 772 `parseSay` already drops texts > 255 chars at the wire layer
   (`protocolgame.cpp:945-947`) — confirm this exists in the parser (`game_parse.rs`), not just here.

### CH-2 — Whisper + Yell
1. `player_whisper` — `game.cpp:3400-3422`. Needs per-viewer distance check
   (`Position::areInRange<1,1>`) to decide real text vs `"pspsps"` — this is **not** the same as
   `broadcast_creature_say_viewport`'s uniform text-to-all; write a dedicated loop over
   `spectator_conns_via_grid`.
2. `player_yell` — `game.cpp:3424-3453`. Needs:
   - `CONDITION_YELLTICKS` 30s exhaust gate (`ConditionType::YellTicks`, already in the enum, §0.4)
     — GM/access-player bypass (`player->getAccountType() < ACCOUNT_TYPE_GAMEMASTER`).
   - Level gate (`YELL_MINIMUM_LEVEL` config) with premium bypass (`YELL_ALLOW_PREMIUM` config) —
     add both to `config.lua` following the existing config key conventions (check
     `crates/tfs-rust-core`'s config-loading module for the pattern used by other numeric/bool keys).
   - Uppercase transform (`asUpperCaseString`) — plain Rust `.to_uppercase()` is **not** byte-identical
     for non-ASCII; TFS 772 client charset is Latin-1-ish — use ASCII-only uppercase to match
     observable C++ behavior unless a Unicode name/text case is explicitly tested.
   - **Wide viewport** — extend `broadcast_creature_say_viewport` (or add a sibling
     `broadcast_creature_say_range(speaker, speak_type, text, range_x, range_y, chebyshev: bool)`)
     to support the yell range `(18, 18, 14, 14, chebyshev=true)` vs default
     `(Map::maxClientViewportX, Map::maxClientViewportX, Map::maxClientViewportY, Map::maxClientViewportY, chebyshev=false)`
     — confirm `Map::maxClientViewportX/Y` numeric values against `map.h`/`Map` in the 772 reference
     before hardcoding.

### CH-3 — Private message (tell) + Broadcast
1. `player_speak_to` — `game.cpp:3455-3479`: resolve target by name (`player_by_name` map, already
   game-thread-only per `TFS-threading.md`), `TALKTYPE_PRIVATE_RED` requires
   `PlayerFlag_CanTalkRedPrivate` else downgrades to `TALKTYPE_PRIVATE`, ghost-mode visibility check
   (`canSeeGhostMode`) before confirming send. Sends `MESSAGE_STATUS_SMALL` confirmation/failure via
   `send_text_message_simple`.
2. `player_broadcast_message` — `game.cpp:2005-2018`: `PlayerFlag_CanBroadcast` gate, iterate all
   online players (`world.creatures` filtered to `CreatureKind::Player`, or a `players_online`
   index if one exists — check before adding a new full scan).
3. Player flags (`PlayerFlag_CanBroadcast`, `PlayerFlag_CanTalkRedPrivate`, `PlayerFlag_CannotBeMuted`)
   don't exist on `creature/player.rs::Player` yet — add a `flags: PlayerFlags` bitset (or reuse an
   existing GM/account-type check if the codebase already gates broadcast via `account_type` instead
   of a flags system — **check `player.rs` for an existing `AccountType`/flags field before adding a
   parallel one**).

### CH-4 — Channels (guild / party / help / trade / game-chat / private)
1. **`tfs-rust-lua` pilot infra** (§2.1): `userdata/channel.rs::ChannelHandle` +
   `chat_channels.rs::{ChatChannelDef, load_chat_channel_scripts}`. This is new plumbing, not a small
   XML-loader tweak — budget real time here (§0.5).
2. **Rewrite each script** under `data/scripts/chatchannels/` to the `Channel(id, name)` /
   `:public(bool)` / table-field-hook / `:register()` convention (§2.1 examples): `gamechat.lua` (4,
   public), `realchat.lua` (5, public), `trade.lua` (6, public), `help.lua` (7, public), `tutor.lua`
   (3), `gamemaster.lua` (8), `advertising.lua`/`advertising-rook.lua` (confirm these two are meant to
   be alternate `canJoin` policies for the *same* channel id or genuinely separate channels — read
   both fully before assigning ids; not yet resolved, see §4). Skip `ruleviolations.lua` (§1 non-goal)
   and the two currently-unreferenced extras (`worldchat.lua`, `englishchat.lua`) unless the user wants
   them enabled as additional channels — flag, don't silently add.
3. `ChatRegistry` full implementation: `add_user_to_channel`, `remove_user_from_channel`,
   `remove_user_from_all_channels` (call on logout — hook into existing logout path), `get_channel`,
   `get_channel_list` (per-player visibility: guild channel only if `GuildRegistry` has membership,
   party channel only if in a `Party`, private channel only if owner/invited; static Lua-defined
   channels visible per their `canJoin` result).
4. `player_request_channels` → `send_channels_dialog` with the per-player-visible list.
5. `player_open_channel` → run `canJoin` (deny + no-op if it returns `false`) → `add_user_to_channel`
   → run `onJoin` if present → `send_channel` ack. Guild MOTD send is optional (§1 non-goal note) —
   land without the 150ms scheduler delay first, revisit if parity-critical.
6. `player_close_channel` → run `onLeave` if present → `remove_user_from_channel`.
7. `player_talk_to_channel` — `Chat::talkToChannel`/`ChatChannel::talk` (`chat.cpp:107-117`):
   membership check, then run `onSpeak` if present (may rewrite `type` or reject with `false` —
   propagate the cancel message the script sends via `sendCancelMessage`, don't add a second one),
   then per-user `send_to_channel` fan-out (not viewport-based — channel membership is the whole
   audience, unlike say/yell).
8. `player_create_private_channel` — premium-only gate (`player->isPremium()`, `game.cpp:2023`) —
   check `Player::premium_ends_at` (already exists, `player.rs:104`) against current time. Private
   channels are dynamic/Rust-native — never touch the Lua loader.
9. `player_channel_invite` / `player_channel_exclude` — `PrivateChatChannel::invitePlayer`/
   `excludePlayer` (`chat.cpp:29-52`) — sends `MESSAGE_INFO_DESCR` info text to both parties; exclude
   also sends `send_close_private` to the excluded player.
10. `player_open_private_channel` — name validation (`IOLoginData::formatPlayerName` equivalent —
    check if a name-normalization helper already exists in `tfs-rust-db`/`tfs-rust-core` before writing
    a new one) + self-channel rejection, then `send_open_private_channel`.
11. Wire all 8 `GamePacket::*` channel variants in `handle_game_packet`.

### CH-5 — Flood protection (mute-on-spam)
1. Add `message_buffer_count: i32` to `Player` (or `PlayerSocial` if that's the right home — check
   existing struct groupings in `creature/player.rs` before picking a field owner).
2. Add a persistent (process-lifetime is fine; DB persistence optional) `mute_count_map: HashMap<u32, u32>`
   (player guid → escalation count) on `GameWorld` — `player.cpp:1366-1373`'s `muteCountMap` is a
   static/global in C++; a `GameWorld` field is the idiomatic equivalent here.
3. `player_remove_message_buffer` — call at the top of every successful `player_say` dispatch
   (`game.cpp:3233`, **after** the spell/mute early-outs, **before** the type switch) — increments the
   buffer count, applies `ConditionType::Muted` with `5 * muteCount²` seconds when it exceeds
   `MAX_MESSAGEBUFFER` (config key, add if missing), sends the "You are muted for N seconds." message.
4. `player_add_message_buffer` — tick hook (`Player::onThink`, `player.cpp:1314-1318`, every 1500ms)
   decrements the buffer count. Wire into whatever periodic per-player think tick already exists
   (`idle_stimulus.rs` or a player-tick equivalent — **find the existing `onThink`-equivalent cadence
   before adding a new timer**).
5. `player_say`'s mute check (`game.cpp:3223-3227`) queries the active `ConditionType::Muted` ticks
   remaining and short-circuits with the "still muted for N seconds" message — this is a **read**, not
   a new condition system; reuse `condition.rs`'s existing active-condition query surface (check for
   a `has_condition`/`condition_ticks_remaining`-style method before adding one).
6. `CannotBeMuted` flag bypass — ties into the same flags question raised in CH-3 step 3.

### CH-6 — Talkactions/spell integration seam (stub only, per §1 non-goals)
1. Leave the `player_say` call-site added in CH-1 step 3 as the integration point.
2. Document in `chat.rs`'s module doc comment exactly which C++ function
   (`TalkActions::playerSaySpell`, `talkaction.cpp`) a future Lua phase must replace the stub with,
   and the `TALKACTION_CONTINUE`/`BREAK`/`FAILED` tri-state contract (`talkaction.h:13-17`) so the
   Rust replacement signature is `Result`/enum shaped from day one instead of a `bool`.

### CH-7 — Tests
1. Wire-format snapshot tests (CH-0) in `tfs-rust-net/tests/protocol_compat.rs`.
2. `game_world_chat.rs` unit tests: say viewport fan-out (reuse existing
   `game_world_spectators.rs` spectator-set test helpers), yell range vs say range spectator set
   difference, whisper distance-based garbling, private-channel invite/exclude state transitions,
   flood mute escalation (`5 * n²` sequence), channel membership visibility per guild/party.
3. End-to-end `sim_harness.rs`-style test (if that harness supports multi-connection scenarios) for:
   player A says something → player B (in range) receives `sendCreatureSay`, player C (out of range)
   does not.

---

## 4. Open questions (resolve before/while implementing, not after)

1. **Private channel id allocation** — `CHANNEL_PRIVATE = 0xFFFF` is a sentinel/base in the decompile,
   not a real id ceiling for concurrent private channels. Read `Chat::createChannel`'s actual id
   allocation (not yet excerpted in this plan) before implementing CH-4 step 1's dynamic ids.
2. ~~`chatchannels.xml` → Rust data format~~ **Fully resolved, locked in.** No XML at all —
   `chatchannels.xml` deleted (§0.6). Static channels live at `data/scripts/chatchannels/*.lua`,
   self-registering via a new `Channel(id, name)` Lua object (§2.1), executed for real through a new
   `tfs-rust-lua` pilot of the `Action`/`TalkAction` self-registration pattern (CH-4 step 1). No
   further decision needed here — remaining open item is §4.7 below (advertising script ids).
3. **Player flags system** — does one already exist (account-type based) that broadcast/red-private/
   cannot-be-muted should hook into, or is this the first flags bitset in the codebase? Audit
   `creature/player.rs` and `tfs-rust-db::player` fully before adding a parallel mechanism.
4. **Existing per-player tick cadence** — where does `Player::onThink`'s 1500ms `MessageBufferTicks`
   accumulator map to in the current game loop (a global tick counter divided by interval, or a
   per-player scheduled event)? Reuse it; don't add a second timer wheel for chat alone.
5. **1098 wire divergence** — this plan cites 772 exclusively for byte layouts. Confirm none of the
   5 new outgoing functions (CH-0) differ in 1098's `src/protocolgame.cpp` before assuming
   `codec::wire.rs` can share one encoder for both eras.
6. **RVR and talkactions/spell scope** — confirmed non-goals in §1; get explicit user sign-off before
   any future phase touches `TALKTYPE_RVR_*` or word-based spell casting under this plan's umbrella.
7. **`advertising.lua` / `advertising-rook.lua` channel identity** — not yet read in full this pass.
   Likely a Rook (newbie island) vs. mainland split of the same conceptual "Advertising" channel (TFS
   has an Advertising channel in some configs) — need their ids/`canJoin` bodies read before CH-4 step
   2 assigns them `Channel(id, name)` calls. Could be one channel with a level/location-based `canJoin`
   instead of two separate ids — don't guess, read both files fully first.

---

## 5. Naming compliance (TFS-Core.md)

No `cip`/`Cip`/`CipSoft` identifiers introduced. Decompile **file** citations (`chat.cpp`,
`talkaction.h`, `player.cpp`) appear only in `//!`/`//` doc comments, never in Rust symbol names.
`ChatChannel`/`ChatRegistry`/`PrivateChatChannel` mirror TFS class names (allowed — these are TFS
structure names, not CipSoft-specific formula/system names requiring the rename table in
`docs/CIP_CIPSOFT_NAMING_AUDIT.md`).

---

## 6. Phased implementation checklist

Trackable form of §3 (`tasks/todo.md` convention: `- [ ]` per landable unit, check off as merged).
Resolve §4's open questions inline in the phase that hits them — do not defer silently.

### CH-0 — Missing outgoing wire (prerequisite)
- [x] `send_channels_dialog` (`0xAB` out) — `Codec::encode_channels_dialog` (era-identical;
      free fn `send_channels_dialog_count` retained for the empty-list special case)
- [x] `send_channel` open-channel ack (`0xAC` out) — `Codec::encode_channel_open` (era-divergent:
      1098 appends user/invited name lists, 772 omits; old `send_channel_open` free fn `#[deprecated]`)
- [x] `send_create_private_channel` (`0xB2`) — `Codec::encode_create_private_channel` (era-divergent:
      1098 appends owner + invited name lists, 772 omits; old free fn `#[deprecated]`)
- [x] `send_close_private` (`0xB3`) — `send_close_private` free fn (era-identical, opcode constant)
- [x] `send_open_private_channel` (`0xAD`) — `send_open_private_channel` free fn (era-identical, opcode constant)
- [x] Confirm 1098 `src/protocolgame.cpp` parity for all 5 (§4.5) — **resolved:** `sendChannelsDialog`/
      `sendClosePrivate`/`sendOpenPrivateChannel` era-identical; `sendChannel`/`sendCreatePrivateChannel`
      diverge (1098 appends user/invited lists). Divergence isolated to `codec::v772`/`codec::v1098`.
- [x] Wire-snapshot tests in `tfs-rust-net/tests/protocol_compat.rs` — 6 new (3 per era), 74 total pass

### CH-1 — `player_say` dispatch skeleton + plain SAY
- [x] `crates/tfs-rust-core/src/chat.rs` — module skeleton (`ChatChannel`/`PrivateChatChannel`/
      `ChatRegistry`, empty registry ok for this phase)
- [x] `GameWorld::chat: ChatRegistry` field + startup init in `run_server.rs`
      (init in `GameWorld::new` — `run_server.rs` calls `GameWorld::new`, no separate init needed)
- [x] `game_world_chat.rs::player_say` — `TALKTYPE_SAY` arm only, calls
      `broadcast_creature_say_viewport`; other arms stubbed (`warn!`-logged)
- [x] `playerSaySpell` call-site stub (returns `false` = "not handled", documented TODO per CH-6)
- [x] Wire `GamePacket::Say` in `handle_game_packet` (`game_loop.rs`), remove from `_ => trace!`
- [x] Confirm 255-char text length is already enforced in the wire parser (`game_parse.rs`), not just
      here — **added:** `parse_say` now returns `Err` for texts > 255 bytes, matching
      `protocolgame.cpp:945-947`'s silent drop (caller logs + continues)
- [x] `EventDispatcher::on_creature_say` / `on_hear` no-op default methods added; called from
      `broadcast_creature_say_viewport` (two-pass loop mirroring `game.cpp:3529-3544`)

### CH-2 — Whisper + Yell
- [x] `player_whisper` — `broadcast_creature_whisper` with per-viewer 1-tile Chebyshev distance
      check (real text ≤1 tile, `"pspsps"` beyond); two-pass send+event loop matching
      `game.cpp:3400-3422`
- [x] `ConditionType::YellTicks` 30s exhaust condition application (`ConditionData::Generic {
      ticks: 30_000 }`) + GM/access bypass via `player_is_access_player` (maps C++
      `getAccountType() < ACCOUNT_TYPE_GAMEMASTER` to `Group::access`)
- [x] `YELL_MINIMUM_LEVEL` / `YELL_ALLOW_PREMIUM` config keys added to `config.lua` +
      `ChatConfig` struct in `config.rs` (defaults: 2 / false, matching `configmanager.cpp:264,196`)
- [x] ASCII-safe uppercase transform — `ascii_uppercase` helper mirrors C++ `asUpperCaseString`
      (`tools.cpp:257`, byte-level `toupper`), not Unicode `.to_uppercase()`
- [x] Wide-viewport spectator variant — `spectator_players_in_box(pos, range_x, range_y,
      multifloor)` + `broadcast_creature_yell` for yell's `(18,18,14,14,multifloor=true)` range
      with **no `canSee` filtering** (matching C++ `getSpectators` yell path). `Map::maxClientViewportX/Y`
      confirmed as 8/6 (`map.h:183-184`); whisper reuses the same ±8/±6 range
- [x] `YellTicks` condition ticking in `process_skills` — decrements `ticks` by 1000 ms per
      `ProcessSkills` tick (~1s), removes at 0 (mirrors `ConditionGeneric::executeCondition`)
- [x] `player_yell` + `player_whisper` wired into `player_say`'s `TALKTYPE_YELL`/`WHISPER` arms

### CH-3 — Private message (tell) + Broadcast
- [ ] Resolve player-flags open question (§4.3) — reuse existing account-type gate or add
      `PlayerFlags` bitset
- [ ] `player_speak_to` — name resolution, `PRIVATE_RED` downgrade rule, ghost-mode visibility check,
      confirmation/failure `MESSAGE_STATUS_SMALL` text
- [ ] `player_broadcast_message` — `CanBroadcast` gate + all-online-players fan-out
- [ ] Wired into `player_say`'s `PRIVATE`/`PRIVATE_RED`/`BROADCAST` arms

### CH-4 — Channels (`tfs-rust-lua` pilot + registry + dispatch)
- [ ] Resolve `advertising.lua`/`advertising-rook.lua` identity (§4.7) before assigning ids
- [ ] `crates/tfs-rust-lua/src/userdata/channel.rs::ChannelHandle` (`Channel(id,name)`, `:public()`,
      `:register()`)
- [ ] `crates/tfs-rust-lua/src/chat_channels.rs` — directory-scan loader +
      `ChatChannelDef`/`load_chat_channel_scripts`
- [ ] Rewrite `data/scripts/chatchannels/{gamechat,realchat,trade,help,tutor,gamemaster}.lua` to the
      `Channel(...)`/`:register()` convention (§2.1); resolve/rewrite `advertising*.lua` per the item
      above; leave `ruleviolations.lua` un-rewritten and unloaded (§1); decide on
      `worldchat.lua`/`englishchat.lua` (currently unreferenced) with the user before enabling
- [ ] `ChatRegistry` methods: `add_user_to_channel`, `remove_user_from_channel`,
      `remove_user_from_all_channels` (hook logout path), `get_channel`, `get_channel_list`
      (per-player visibility incl. guild/party membership)
- [ ] Resolve `CHANNEL_PRIVATE` dynamic id allocation scheme (§4.1) against `Chat::createChannel`
- [ ] `player_request_channels` → `send_channels_dialog`
- [ ] `player_open_channel` → `canJoin` → `add_user_to_channel` → `onJoin` → `send_channel`
- [ ] `player_close_channel` → `onLeave` → `remove_user_from_channel`
- [ ] `player_talk_to_channel` → membership check → `onSpeak` → `send_to_channel` fan-out
- [ ] `player_create_private_channel` — premium gate via `premium_ends_at`
- [ ] `player_channel_invite` / `player_channel_exclude` (+ `send_close_private` on exclude)
- [ ] `player_open_private_channel` — name validation + self-channel rejection
- [ ] Wire all 8 `GamePacket::*` channel variants in `handle_game_packet`

### CH-5 — Flood protection (mute-on-spam)
- [ ] Resolve per-player tick cadence question (§4.4) — reuse existing `onThink`-equivalent, don't add
      a second timer
- [ ] `message_buffer_count` field on `Player`/`PlayerSocial`
- [ ] `mute_count_map: HashMap<u32, u32>` on `GameWorld`
- [ ] `player_remove_message_buffer` — escalating `5 * n²`s `ConditionType::Muted` application, called
      from `player_say`'s top (after spell check, before type switch)
- [ ] `player_add_message_buffer` — 1500ms decrement tick hook
- [ ] Mute-check read at `player_say` entry using existing `condition.rs` active-condition query
      surface (add one only if it doesn't already exist)
- [ ] `CannotBeMuted` bypass wired to the CH-3 flags decision

### CH-6 — Talkactions/spell integration seam (stub only)
- [ ] Confirm CH-1's `playerSaySpell` stub is the single integration point (no duplicate call sites)
- [ ] Module doc comment in `chat.rs` documenting the `TALKACTION_CONTINUE`/`BREAK`/`FAILED` contract
      for the future Lua replacement

### CH-7 — Tests
- [ ] `game_world_chat.rs` unit tests: say viewport fan-out, yell range vs say range spectator-set
      diff, whisper distance garbling, private-channel invite/exclude transitions, flood mute
      escalation sequence, channel membership visibility (guild/party/Lua `canJoin`)
- [ ] End-to-end multi-connection test (say in range / out of range) if `sim_harness.rs` supports it
