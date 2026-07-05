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

use tfs_rust_common::enums::ConditionType;
use tfs_rust_common::ConnId;

use crate::combat::apply_condition;
use crate::condition::{ActiveCondition, ConditionData};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::return_value::ReturnValue;

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
                // C++ `playerWhisper(player, text)` — `game.cpp:3240-3241, 3400-3422`.
                self.player_whisper(cid, text);
            }
            TALKTYPE_YELL => {
                // C++ `playerYell(player, text)` — `game.cpp:3244-3245, 3424-3453`.
                self.player_yell(cid, text);
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

    /// TFS `Game::playerWhisper` — `gameserver/src/game.cpp:3400-3422`.
    ///
    /// Spectators within 1 tile (Chebyshev ≤1 in X **and** Y) receive the real text;
    /// beyond that they receive `"pspsps"`. The fan-out + per-viewer distance garbling
    /// is delegated to [`Self::broadcast_creature_whisper`].
    fn player_whisper(&mut self, cid: CreatureId, text: &str) {
        if text.is_empty() {
            return;
        }
        self.broadcast_creature_whisper(cid, TALKTYPE_WHISPER, text);
    }

    /// TFS `Game::playerYell` — `gameserver/src/game.cpp:3424-3453`.
    ///
    /// Gates (matching the reference):
    /// 1. `CONDITION_YELLTICKS` active → `RETURNVALUE_YOUAREEXHAUSTED` cancel, return.
    /// 2. Level < `yellMinimumLevel`:
    ///    - If `yellAlwaysAllowPremium` && player is premium → allow (uppercase + broadcast).
    ///    - Else → `MESSAGE_STATUS_SMALL` "You may not yell..." text, return.
    /// 3. Non-GM players get `CONDITION_YELLTICKS` 30s applied after a successful yell.
    /// 4. Text is ASCII-uppercased (`asUpperCaseString`, `tools.cpp:257`) then broadcast
    ///    via the wide-range yell viewport (`broadcast_creature_yell`).
    fn player_yell(&mut self, cid: CreatureId, text: &str) {
        if text.is_empty() {
            return;
        }

        // C++ `if (player->hasCondition(CONDITION_YELLTICKS))` — `game.cpp:3426-3429`.
        let has_yell_ticks = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p
                .base
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::YellTicks),
            _ => return,
        };
        if has_yell_ticks {
            if let Some(conn) = self.conn_for_creature(cid) {
                self.send_cancel_message(conn, ReturnValue::YouAreExhausted);
            }
            return;
        }

        // C++ level gate — `game.cpp:3431-3444`.
        let (level, is_premium, is_access) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => {
                let free_premium = self.config.get_bool("freePremium").unwrap_or(false);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as u32;
                let premium = free_premium || p.premium_ends_at > now;
                (p.level, premium, self.player_is_access_player(cid))
            }
            _ => return,
        };

        let min_level = self.chat_config.yell_minimum_level as i32;
        if level < min_level {
            if self.chat_config.yell_allow_premium && is_premium {
                // C++ premium bypass — `game.cpp:3433-3436`.
                let upper = ascii_uppercase(text);
                self.broadcast_creature_yell(cid, TALKTYPE_YELL, &upper);
                return;
            }
            if let Some(conn) = self.conn_for_creature(cid) {
                use tfs_rust_net::outgoing_extra::send_text_message_simple;
                let msg = if self.chat_config.yell_allow_premium {
                    format!(
                        "You may not yell unless you have reached level {min_level} or have a premium account."
                    )
                } else {
                    format!("You may not yell unless you have reached level {min_level}.")
                };
                self.enqueue_outgoing(
                    conn,
                    send_text_message_simple(self.codec.failure_message_type(), &msg).into_bytes(),
                );
            }
            return;
        }

        // C++ `if (player->getAccountType() < ACCOUNT_TYPE_GAMEMASTER)` — `game.cpp:3446-3449`.
        // GM/access players bypass the 30s exhaust. `player_is_access_player` mirrors
        // `Group::access` which maps to the same GM bypass semantics.
        if !is_access {
            apply_condition(
                &mut self.creatures,
                cid,
                ActiveCondition::new(
                    0,
                    0,
                    ConditionType::YellTicks,
                    ConditionData::Generic { ticks: 30_000 },
                    None,
                ),
            );
        }

        // C++ `internalCreatureSay(player, TALKTYPE_YELL, asUpperCaseString(text), false)`
        // — `game.cpp:3451`.
        let upper = ascii_uppercase(text);
        self.broadcast_creature_yell(cid, TALKTYPE_YELL, &upper);
    }
}

/// C++ `asUpperCaseString` — `gameserver/src/tools.cpp:257-261`.
///
/// Uses `std::transform(..., toupper)` which is ASCII-only for the 772 Latin-1 client
/// charset. Rust `.to_uppercase()` is Unicode-aware and would produce different bytes
/// for non-ASCII characters (e.g. accented letters), so this helper mirrors the C++
/// byte-level `toupper` behavior exactly.
fn ascii_uppercase(s: &str) -> String {
    s.bytes().map(|b| b.to_ascii_uppercase() as char).collect()
}
