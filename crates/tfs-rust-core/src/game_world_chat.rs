//! Player chat dispatch — `Game::playerSay` and the per-talk-type handlers.
//!
//! Mirrors `game_world_spectators.rs`'s `impl GameWorld` extension-file pattern.
//! Houses the chat-related `Game::player*` methods (`playerSay`, `playerWhisper`,
//! `playerYell`, `playerSpeakTo`, `playerBroadcastMessage`, channel lifecycle, and
//! the flood/mute tick hooks) per `tasks/chat-system-plan.md` §2.3.
//!
//! CH-1 lands only `player_say`'s `TALKTYPE_SAY` arm + the `playerSaySpell` stub;
//! the other arms are `warn!`-logged stubs filled in by CH-2/CH-3/CH-4/CH-5.
// C++ reference: `Game::playerSay` — `gameserver/src/game.cpp:3208-3281`;
// `Game::playerSaySpell` — `game.cpp:3375-3398`; `Player::resetIdleTime` /
// `isMuted` / `removeMessageBuffer` — `player.cpp:1314-1380`.

use std::time::Instant;

use tfs_rust_common::ConnId;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// `SpeakClasses` byte values — `gameserver/src/const.h:61-77`.
///
/// These are the **server-side** speak classes that `Game::playerSay` switches on.
/// The incoming client byte in `SayPayload::speak_class` is the same enum (the 772
/// client sends these values directly; see `protocolgame.cpp:924` `parseSay`).
const TALKTYPE_SAY: u8 = 1;
const TALKTYPE_WHISPER: u8 = 2;
const TALKTYPE_YELL: u8 = 3;
const TALKTYPE_PRIVATE: u8 = 4;
const TALKTYPE_CHANNEL_Y: u8 = 5;
const TALKTYPE_RVR_CHANNEL: u8 = 6;
const TALKTYPE_RVR_ANSWER: u8 = 7;
const TALKTYPE_RVR_CONTINUE: u8 = 8;
const TALKTYPE_BROADCAST: u8 = 9;
const TALKTYPE_CHANNEL_R1: u8 = 10;
const TALKTYPE_PRIVATE_RED: u8 = 11;
const TALKTYPE_CHANNEL_O: u8 = 12;
const TALKTYPE_CHANNEL_R2: u8 = 14;

impl GameWorld {
    /// TFS `Game::playerSay` — `gameserver/src/game.cpp:3208-3281`.
    ///
    /// Top-level chat dispatch: idle reset → spell/talkaction check → mute check →
    /// GM `/`-prefix check → flood buffer tick → per-type switch. CH-1 implements
    /// only the `TALKTYPE_SAY` arm (viewport broadcast via `broadcast_creature_say_viewport`);
    /// the remaining arms are stubs landed by CH-2 (whisper/yell), CH-3 (private/broadcast),
    /// CH-4 (channels), and CH-5 (flood/mute).
    pub fn player_say(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        speak_class: u8,
        channel_id: u16,
        receiver: &str,
        text: &str,
        now: Instant,
    ) {
        // C++ `Player* player = getPlayerByID(playerId); if (!player) return;`
        let is_player = matches!(self.creatures.get(cid), Some(CreatureKind::Player(_)));
        if !is_player {
            return;
        }

        // C++ `player->resetIdleTime();` — `player.cpp`. Mirrors `walk/mod.rs` inline reset.
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_activity = now;
        }

        // C++ `if (playerSaySpell(player, type, text)) return;` — `game.cpp:3219`.
        // CH-6 seam: word-based spell/talkaction dispatch is not wired yet (no Lua
        // talkactions runtime, no spell-words table). Stub returns `false` = "not
        // handled", matching current behavior (no spells triggered via say text today).
        if self.player_say_spell(cid, speak_class, text) {
            return;
        }

        // C++ `uint32_t muteTime = player->isMuted();` — `game.cpp:3223-3227`.
        // TODO(chat CH-5): wire `ConditionType::Muted` active-condition query + the
        // "You are still muted for N seconds." `MESSAGE_STATUS_SMALL` send. No
        // mute/flood system exists yet (`ConditionType::Muted` is in the enum but
        // has zero call sites, §0.4). Until CH-5 lands, mute is never active.

        // C++ `if (!text.empty() && text.front() == '/' && player->isAccessPlayer()) return;`
        // — `game.cpp:3229-3231`. GM `/`-prefix commands are handled by the talkaction
        // layer (CH-6); for access players the line is consumed and never broadcast.
        if !text.is_empty() && text.as_bytes()[0] == b'/' && self.player_is_access_player(cid) {
            return;
        }

        // C++ `player->removeMessageBuffer();` — `game.cpp:3233`, `player.cpp:1350-1380`.
        // TODO(chat CH-5): increment `message_buffer_count`, apply `ConditionType::Muted`
        // with `5 * muteCount²`s when it exceeds `MAX_MESSAGEBUFFER`. No-op until CH-5.

        // C++ `switch (type)` — `game.cpp:3235-3280`.
        match speak_class {
            TALKTYPE_SAY => {
                // C++ `internalCreatureSay(player, TALKTYPE_SAY, text, false, nullptr, &pos);`
                // — `game.cpp:3236-3238`. Reuses the existing viewport fan-out
                // (`broadcast_creature_say_viewport`) which already mirrors
                // `internalCreatureSay`'s normal-range spectator lookup + per-viewer
                // `sendCreatureSay` + (CH-1) `on_creature_say`/`on_hear` event hooks.
                self.broadcast_creature_say_viewport(cid, TALKTYPE_SAY, text);
            }
            TALKTYPE_WHISPER => {
                // CH-2: `player_whisper` — `game.cpp:3400-3422` (per-viewer 1-tile distance
                // check, `"pspsps"` garbling beyond range).
                tracing::warn!(conn_id = conn_id.0, cid = ?cid, "player_say TALKTYPE_WHISPER — CH-2 stub");
            }
            TALKTYPE_YELL => {
                // CH-2: `player_yell` — `game.cpp:3424-3453` (level gate, `CONDITION_YELLTICKS`
                // 30s exhaust, ASCII uppercase, wide `(18,18,14,14,chebyshev=true)` viewport).
                tracing::warn!(conn_id = conn_id.0, cid = ?cid, "player_say TALKTYPE_YELL — CH-2 stub");
            }
            TALKTYPE_PRIVATE | TALKTYPE_PRIVATE_RED | TALKTYPE_RVR_ANSWER => {
                // CH-3: `player_speak_to` — `game.cpp:3455-3479` (name resolution,
                // `PRIVATE_RED` downgrade rule, ghost-mode visibility, confirmation text).
                // `TALKTYPE_RVR_ANSWER` is the RVR tell path — non-goal per §1, but the
                // C++ switch folds it into `playerSpeakTo`; leave the arm stubbed until
                // the RVR sign-off decision (§4.6).
                tracing::warn!(conn_id = conn_id.0, cid = ?cid, ?speak_class, "player_say PRIVATE/RED/RVR_ANSWER — CH-3 stub");
            }
            TALKTYPE_CHANNEL_O | TALKTYPE_CHANNEL_Y | TALKTYPE_CHANNEL_R1 | TALKTYPE_CHANNEL_R2 => {
                // CH-4: `g_chat->talkToChannel(*player, type, text, channelId)` —
                // `game.cpp:3261`, `chat.cpp:107-117` (membership check → `onSpeak` →
                // `send_to_channel` fan-out). `CHANNEL_RULE_REP` special-case
                // (→ `internalCreatureSay`) is an RVR non-goal (§1).
                let _ = channel_id;
                tracing::warn!(conn_id = conn_id.0, cid = ?cid, ?speak_class, channel_id, "player_say CHANNEL_* — CH-4 stub");
            }
            TALKTYPE_BROADCAST => {
                // CH-3: `player_broadcast_message` — `game.cpp:2005-2018` (`PlayerFlag_CanBroadcast`
                // gate + all-online-players fan-out).
                tracing::warn!(conn_id = conn_id.0, cid = ?cid, "player_say TALKTYPE_BROADCAST — CH-3 stub");
            }
            TALKTYPE_RVR_CHANNEL | TALKTYPE_RVR_CONTINUE => {
                // RVR (Rule Violation Report) GM system — explicit non-goal (§1).
                // `playerReportRuleViolationReport` / `playerContinueRuleViolationReport`.
                // No-op until RVR is greenlit (§4.6).
            }
            other => {
                tracing::warn!(conn_id = conn_id.0, cid = ?cid, speak_class = other, "player_say unknown speak class");
            }
        }
        let _ = receiver; // consumed by CH-3 `player_speak_to` / CH-4 private channels.
    }

    /// TFS `Game::playerSaySpell` — `gameserver/src/game.cpp:3375-3398`.
    ///
    /// Word-based spell / talkaction dispatch. Returns `true` when the text was
    /// consumed (spell cast or talkaction fired) and the caller must **not** proceed
    /// to the talk-type switch; `false` when the text is plain chat.
    ///
    /// CH-1 stub: no `g_talkActions` / `g_spells->playerSaySpell` word-table runtime
    /// exists in `tfs-rust-lua` yet (§0.5, `TFS-lua-boundaries.md` step 3). Always
    /// returns `false` — matches current behavior (no spells triggered via say text).
    /// This is the **single integration point** for CH-6; do not add duplicate
    /// spell-words call sites elsewhere.
    // TODO(chat CH-6): wire `g_talkActions->playerSaySpell` + `g_spells->playerSaySpell`
    // once the Lua talkactions runtime lands. Contract mirrors C++ `TalkActionResult_t`:
    //   `TALKACTION_CONTINUE` (false, plain chat) / `TALKACTION_BREAK` (true, consumed +
    //   re-broadcast as `TALKTYPE_SAY`/`MONSTER_SAY` unless `EMOTE_SPELLS`) /
    //   `TALKACTION_FAILED` (true, consumed silently).
    fn player_say_spell(&self, _cid: CreatureId, _speak_class: u8, _text: &str) -> bool {
        false
    }
}
