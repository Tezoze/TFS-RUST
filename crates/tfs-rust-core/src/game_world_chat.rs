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
use crate::player::flags::{
    PLAYER_FLAG_CANNOT_BE_MUTED, PLAYER_FLAG_CAN_BROADCAST, PLAYER_FLAG_CAN_TALK_RED_PRIVATE,
};
use crate::return_value::ReturnValue;
use tfs_rust_net::outgoing_extra;
use tfs_rust_net::ChannelOpenWire;
use tfs_rust_net::CreatePrivateChannelWire;
use tfs_rust_net::{PrivateMessageWire, ToChannelWire};

/// `SpeakClasses` byte values — `gameserver/src/const.h:61-77`.
///
/// These are the **server-side** speak classes that `Game::playerSay` switches on.
/// The incoming client byte in `SayPayload::speak_class` is the same enum (the 772
/// client sends these values directly; see `protocolgame.cpp:924` `parseSay`).
const TALKTYPE_SAY: u8 = 1;
const TALKTYPE_WHISPER: u8 = 2;
const TALKTYPE_YELL: u8 = 3;
const TALKTYPE_PRIVATE: u8 = 4;
const TALKTYPE_PRIVATE_FROM: u8 = 4; // Outgoing private message (to receiver)
const TALKTYPE_CHANNEL_Y: u8 = 5;
const TALKTYPE_RVR_CHANNEL: u8 = 6;
const TALKTYPE_RVR_ANSWER: u8 = 7;
const TALKTYPE_RVR_CONTINUE: u8 = 8;
const TALKTYPE_BROADCAST: u8 = 9;
const TALKTYPE_CHANNEL_R1: u8 = 10;
const TALKTYPE_PRIVATE_RED: u8 = 11;
const TALKTYPE_PRIVATE_RED_TO: u8 = 11; // Incoming red private
const TALKTYPE_PRIVATE_RED_FROM: u8 = 11; // Outgoing red private (to receiver)
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
    ) {
        // C++ `Player* player = getPlayerByID(playerId); if (!player) return;`
        let is_player = matches!(self.creatures.get(cid), Some(CreatureKind::Player(_)));
        if !is_player {
            return;
        }

        // C++ `player->resetIdleTime();` — `player.cpp`. Mirrors `walk/mod.rs` inline reset.
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_activity = Instant::now();
        }

        // C++ `if (playerSaySpell(player, type, text)) return;` — `game.cpp:3219`.
        // CH-6 seam: word-based spell/talkaction dispatch is not wired yet (no Lua
        // talkactions runtime, no spell-words table). Stub returns `false` = "not
        // handled", matching current behavior (no spells triggered via say text today).
        if self.player_say_spell(cid, speak_class, text) {
            return;
        }

        // C++ `uint32_t muteTime = player->isMuted();` — `game.cpp:3223-3227`.
        let mute_seconds = self.player_is_muted(cid);
        if mute_seconds > 0 {
            if let Some(conn) = self.conn_for_creature(cid) {
                use tfs_rust_net::outgoing_extra::send_text_message_simple;
                self.enqueue_outgoing(
                    conn,
                    send_text_message_simple(
                        self.codec.failure_message_type(),
                        &format!("You are still muted for {} seconds.", mute_seconds),
                    )
                    .into_bytes(),
                );
            }
            return;
        }

        // C++ `player->removeMessageBuffer();` — `game.cpp:3233`.
        // Called after mute check, before type switch. Increments buffer count and
        // applies escalating mute when exceeding `maxMessageBuffer`.
        self.player_remove_message_buffer(cid);

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
                // C++ `playerSpeakTo(player, type, receiver, text)` — `game.cpp:3557`.
                // `TALKTYPE_RVR_ANSWER` is the RVR tell path — non-goal per §1, but the
                // C++ switch folds it into `playerSpeakTo`; leave it stubbed until
                // the RVR sign-off decision (§4.6).
                if speak_class == TALKTYPE_RVR_ANSWER {
                    tracing::warn!(conn_id = conn_id.0, cid = ?cid, "player_say RVR_ANSWER — non-goal (§1)");
                    return;
                }
                self.player_speak_to(cid, speak_class, receiver, text);
            }
            TALKTYPE_CHANNEL_O | TALKTYPE_CHANNEL_Y | TALKTYPE_CHANNEL_R1 | TALKTYPE_CHANNEL_R2 => {
                // C++ `g_chat->talkToChannel(*player, type, text, channelId)` —
                // `game.cpp:3261`, `chat.cpp:107-117` (membership check → `onSpeak` →
                // `send_to_channel` fan-out). `CHANNEL_RULE_REP` special-case
                // (→ `internalCreatureSay`) is an RVR non-goal (§1).
                self.player_talk_to_channel(cid, speak_class, channel_id, text);
            }
            TALKTYPE_BROADCAST => {
                // C++ `playerBroadcastMessage(player, text)` — `game.cpp:3567`.
                self.player_broadcast_message(cid, text);
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
    }

    /// TFS `Game::playerSaySpell` — `gameserver/src/game.cpp:3375-3398`.
    ///
    /// Word-based spell / talkaction dispatch. Returns `true` when the text was
    /// consumed (spell cast or talkaction fired) and the caller must **not** proceed
    /// to the talk-type switch; `false` when the text is plain chat.
    ///
    /// PC-2b: spellword dispatch seam. Looks up `text` in the `SpellRegistry` (instant
    /// spells only), checks vocation/level/mana gates, and on success deducts mana/soul
    /// and calls the `onCastSpell` Lua callback. Full spell mechanics (cooldowns, group
    /// cooldowns, PZ lock, aggressive-target validation) land in PC-3a.
    ///
    /// C++ reference: `game.cpp:3584` `Spells::playerSaySpell` → `spells.cpp:30` word
    /// matching → `InstantSpell::playerCastInstant` → `CombatSpell::castSpell` → Lua
    /// `onCastSpell(creature, variant)`.
    fn player_say_spell(&mut self, cid: CreatureId, _speak_class: u8, text: &str) -> bool {
        // CH-6: Talkaction dispatch — `talkaction.cpp:84-134`
        // `TalkActions::playerSaySpell`. Checked BEFORE spell words, matching
        // C++ `Game::playerSaySpell` order (`game.cpp:3379` talkactions first,
        // then `g_spells->playerSaySpell` at `:3385`).
        //
        // `TALKACTION_BREAK` (onSay returned false) → consumed, return true.
        // `TALKACTION_CONTINUE` (onSay returned true) → fall through to spell
        // check, matching C++ behavior.
        let talkaction_result = crate::lua_scope::fire_talkaction(self, cid, text);
        tracing::info!(?cid, text, ?talkaction_result, "CH-6 talkaction dispatch");
        if matches!(
            talkaction_result,
            crate::event_dispatcher::TalkActionResult::Break
        ) {
            return true;
        }

        // Look up the spellwords in the registry (case-insensitive). Clone the spell
        // definition to release the immutable borrow on `self.spells` before any
        // mutable calls (send_player_status_message, send_player_stats, etc.).
        let Some(spell) = self.spells.get_instant_by_words(text).cloned() else {
            // If a talkaction matched but returned `Continue`, the text is not a
            // spell — return `false` so the caller proceeds to the `/`-prefix
            // access-player drop (line 3229 in C++).
            return false; // Not a known spell — plain chat.
        };

        // Gate: vocation check — the player's vocation must be in the spell's allowed list.
        // Empty vocations list = all vocations allowed (TFS default).
        if !spell.vocations.is_empty() {
            let player_voc_id = self.creatures.get(cid).and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.vocation_id),
                _ => None,
            });
            let Some(voc_id) = player_voc_id else {
                return false; // Not a player — can't cast spells.
            };
            // Look up the vocation name from the registry for string comparison.
            let voc_name = self
                .vocations
                .vocations
                .get(&(voc_id as u16))
                .map(|v| v.name.clone())
                .unwrap_or_default();
            if !spell
                .vocations
                .iter()
                .any(|v| v.eq_ignore_ascii_case(&voc_name))
            {
                // Vocation not allowed — send "You cannot cast this spell." and consume.
                self.send_player_status_message(cid, "You cannot cast this spell.");
                return true;
            }
        }

        // Gate: level check.
        let player_level = self
            .creatures
            .get(cid)
            .map(|k| match k {
                CreatureKind::Player(p) => p.level,
                _ => 0,
            })
            .unwrap_or(0);
        if player_level < spell.level as i32 {
            self.send_player_status_message(cid, "You do not have enough level.");
            return true;
        }

        // Gate: mana check. `mana_percent` takes priority over `mana` when set.
        let (player_mana, player_max_mana, player_soul) = self
            .creatures
            .get(cid)
            .map(|k| match k {
                CreatureKind::Player(p) => (p.mana, p.max_mana, p.economy.soul),
                _ => (0, 0, 0),
            })
            .unwrap_or((0, 0, 0));
        let mana_cost = if spell.mana_percent > 0 {
            (player_max_mana as u32 * spell.mana_percent) / 100
        } else {
            spell.mana
        };
        if player_mana < mana_cost as i32 {
            self.send_player_status_message(cid, "You do not have enough mana.");
            return true;
        }
        if player_soul < spell.soul as i32 {
            self.send_player_status_message(cid, "You do not have enough soul.");
            return true;
        }

        // Deduct mana + soul.
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.mana -= mana_cost as i32;
            p.economy.soul -= spell.soul as i32;
        }
        self.send_player_stats(cid);

        // TODO(PC-3a): call the `onCastSpell` Lua callback via the event dispatcher.
        // For now, the spell is "cast" (mana deducted, text consumed) but no damage/
        // effect is applied. The callback dispatch requires the Lua runtime to be
        // wired into the game thread (fire_on_cast_spell helper).
        tracing::debug!(
            "Spellword '{}' cast by {:?}: mana={} soul={} (callback dispatch pending PC-3a)",
            text,
            cid,
            mana_cost,
            spell.soul
        );

        true // Text consumed — do not proceed to chat broadcast.
    }

    /// TFS `Game::playerRequestChannels` — `game.cpp:3490-3502`.
    ///
    /// Sends the channel list dialog to the player. Includes guild/party channels
    /// if the player has membership, and private channels where they are owner/invited.
    ///
    /// C++ reference: `src/game.cpp` `Game::playerRequestChannels`.
    pub fn player_request_channels(&mut self, conn_id: ConnId, cid: CreatureId) {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };

        let player_guid = player.guid;
        let guild_id = self.guilds.player_guild.get(&cid).copied();
        let party_leader = self
            .parties
            .values()
            .find(|p| p.leader == cid || p.members.contains(&cid))
            .map(|p| p.leader);

        let channel_list =
            self.chat
                .get_channel_list(cid, Some(player_guid), guild_id, party_leader);
        let msg = outgoing_extra::send_channels_dialog_full(&channel_list);

        self.pending_outgoing
            .entry(conn_id)
            .or_default()
            .push(msg.into_bytes());
    }

    /// TFS `Game::playerOpenChannel` — `game.cpp:3490-3502`.
    ///
    /// Opens a channel: runs `canJoin` hook (if present), adds user to channel,
    /// runs `onJoin` hook (if present), and sends the channel ack.
    ///
    /// C++ reference: `src/game.cpp` `Game::playerOpenChannel`; `chat.cpp` `ChatChannel::executeCanJoinEvent`.
    pub fn player_open_channel(&mut self, conn_id: ConnId, cid: CreatureId, channel_id: u16) {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };

        // Check if channel exists
        let Some(channel) = self.chat.get_channel(channel_id) else {
            return;
        };

        let channel_name = channel.name.clone();
        let is_public = channel.public_channel;

        // TODO(chat CH-4): Run Lua `canJoin` hook if present
        // For now, all public channels are joinable, private channels require ownership/invitation
        if !is_public {
            // Private channel: check if player is owner or invited
            let is_owner = self
                .chat
                .get_private_channel(channel_id)
                .map(|pc| pc.owner == cid)
                .unwrap_or(false);
            let is_invited = self
                .chat
                .get_private_channel(channel_id)
                .map(|pc| pc.invited.contains(&player.guid))
                .unwrap_or(false);

            if !is_owner && !is_invited {
                return;
            }
        }

        // Add user to channel
        self.chat.add_user_to_channel(channel_id, cid);

        // TODO(chat CH-4): Run Lua `onJoin` hook if present

        // Send channel ack (use codec version for era correctness)
        let wire = ChannelOpenWire {
            channel_id,
            name: channel_name,
            users: Vec::new(),   // 772 ignores this
            invited: Vec::new(), // 772 ignores this
        };
        let msg = self.codec.encode_channel_open(&wire);
        self.pending_outgoing
            .entry(conn_id)
            .or_default()
            .push(msg.into_bytes());
    }

    /// TFS `Game::playerCloseChannel` — `game.cpp:3490-3502`.
    ///
    /// Closes a channel: runs `onLeave` hook (if present) and removes user from channel.
    ///
    /// C++ reference: `src/game.cpp` `Game::playerCloseChannel`; `chat.cpp` `ChatChannel::executeOnLeaveEvent`.
    pub fn player_close_channel(&mut self, cid: CreatureId, channel_id: u16) {
        // TODO(chat CH-4): Run Lua `onLeave` hook if present

        // Remove user from channel
        self.chat.remove_user_from_channel(channel_id, cid);
    }

    /// TFS `Chat::talkToChannel` — `chat.cpp:107-117`.
    ///
    /// Talks to a channel: checks membership, runs `onSpeak` hook (if present),
    /// and fans out the message to all channel members.
    ///
    /// C++ reference: `src/chat.cpp` `Chat::talkToChannel`; `chat.cpp` `ChatChannel::executeOnSpeakEvent`.
    pub fn player_talk_to_channel(
        &mut self,
        cid: CreatureId,
        speak_class: u8,
        channel_id: u16,
        text: &str,
    ) {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };

        let speaker_guid = player.guid;
        let speaker_level = player.level;
        let speaker_name = player.base.name.clone();

        // Check if channel exists and player is a member
        let Some(channel) = self.chat.get_channel(channel_id) else {
            return;
        };

        if !self.chat.is_user_in_channel(channel_id, cid) {
            return;
        }

        // TODO(chat CH-4): Run Lua `onSpeak` hook if present, may modify speak_class or reject
        let _ = speaker_guid;

        // Collect recipient connections first so the `self.chat` borrow (from `channel`) is
        // released before we touch `self.codec` / `alloc_statement_id` / the outgoing queue.
        let recipients: Vec<ConnId> = channel
            .users
            .iter()
            .filter_map(|m| self.creature_to_conn.get(m).copied())
            .collect();

        // Fan out to all channel members through the era-aware codec. `sendToChannel` differs by
        // era: 1098 writes `name + u16 level + type + …`, 772 omits the `level` field
        // (`gameserver/src/protocolgame.cpp:1442`). Writing the 1098 layout to a 772 client shifts
        // the speak-type byte, which the client reads as an invalid message mode.
        for conn_id in recipients {
            let statement_id = self.alloc_statement_id();
            let wire = ToChannelWire {
                speaker_name: Some(speaker_name.clone()),
                level: speaker_level as u16,
                speak_type: speak_class,
                channel_id,
                text: text.to_string(),
            };
            let msg = self.codec.encode_to_channel(statement_id, &wire);
            self.enqueue_outgoing(conn_id, msg.into_bytes());
        }
    }

    /// TFS `Game::playerCreatePrivateChannel` — `game.cpp:2023`.
    ///
    /// Creates a new private channel (premium-only gate). The channel name is
    /// server-generated as "Private Channel <id>".
    ///
    /// C++ reference: `src/game.cpp` `Game::playerCreatePrivateChannel`; `chat.cpp` `Chat::createChannel`.
    pub fn player_create_private_channel(&mut self, conn_id: ConnId, cid: CreatureId) {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };

        // Premium-only gate
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        let free_premium = self.config.get_bool("freePremium").unwrap_or(false);
        let has_premium = free_premium || player.premium_ends_at > now;
        if !has_premium {
            // TODO(chat CH-4): Send "You need a premium account to create private channels." message
            return;
        }

        // Generate channel name (server-generated)
        let channel_id = self.chat.next_private_channel_id;
        let channel_name = format!("Private Channel {}", channel_id);

        // Create private channel
        let channel_id = self.chat.create_private_channel(channel_name.clone(), cid);

        // Add owner to channel
        self.chat.add_user_to_channel(channel_id, cid);

        // Send creation ack (use codec version - 772 omits owner/invited)
        let wire = CreatePrivateChannelWire {
            channel_id,
            name: channel_name.clone(),
            owner_name: String::new(), // 772 ignores this
            invited: Vec::new(),       // 772 ignores this
        };
        let msg = self.codec.encode_create_private_channel(&wire);
        self.pending_outgoing
            .entry(conn_id)
            .or_default()
            .push(msg.into_bytes());
    }

    /// TFS `PrivateChatChannel::invitePlayer` — `chat.cpp:29-52`.
    ///
    /// Invites a player to a private channel. Sends info text to both parties.
    ///
    /// C++ reference: `src/chat.cpp` `PrivateChatChannel::invitePlayer`.
    pub fn player_channel_invite(&mut self, cid: CreatureId, target_name: &str) {
        let Some(CreatureKind::Player(_player)) = self.creatures.get(cid) else {
            return;
        };

        // Find a private channel owned by this player
        let (channel_id, _private_channel) = match self
            .chat
            .private_channels
            .iter()
            .find(|(_, pc)| pc.owner == cid)
        {
            Some((id, pc)) => (*id, pc),
            None => {
                // TODO(chat CH-4): Send "You do not own a private channel." message
                return;
            }
        };

        // Resolve target player by name
        let Some(target_id) = self.player_by_name.get(target_name) else {
            // TODO(chat CH-4): Send "Player not found." message
            return;
        };

        let Some(CreatureKind::Player(target_player)) = self.creatures.get(*target_id) else {
            return;
        };

        // Add to invited list
        self.chat
            .invite_to_private_channel(channel_id, target_player.guid);

        // TODO(chat CH-4): Send info text to both parties
    }

    /// TFS `PrivateChatChannel::excludePlayer` — `chat.cpp:29-52`.
    ///
    /// Excludes a player from a private channel. Sends info text to both parties
    /// and sends `send_close_private` to the excluded player.
    ///
    /// C++ reference: `src/chat.cpp` `PrivateChatChannel::excludePlayer`.
    pub fn player_channel_exclude(&mut self, cid: CreatureId, target_name: &str) {
        let Some(CreatureKind::Player(_player)) = self.creatures.get(cid) else {
            return;
        };

        // Find a private channel owned by this player
        let (channel_id, _private_channel) = match self
            .chat
            .private_channels
            .iter()
            .find(|(_, pc)| pc.owner == cid)
        {
            Some((id, pc)) => (*id, pc),
            None => {
                // TODO(chat CH-4): Send "You do not own a private channel." message
                return;
            }
        };

        // Resolve target player by name
        let Some(target_id) = self.player_by_name.get(target_name) else {
            // TODO(chat CH-4): Send "Player not found." message
            return;
        };

        let Some(CreatureKind::Player(target_player)) = self.creatures.get(*target_id) else {
            return;
        };

        // Remove from invited list
        if self
            .chat
            .exclude_from_private_channel(channel_id, target_player.guid)
        {
            // Remove from channel if they were in it
            self.chat.remove_user_from_channel(channel_id, *target_id);

            // Send close private to excluded player (use existing function)
            if let Some(conn_id) = self.creature_to_conn.get(target_id) {
                let msg = outgoing_extra::send_close_private(channel_id);
                self.pending_outgoing
                    .entry(*conn_id)
                    .or_default()
                    .push(msg.into_bytes());
            }
        }

        // TODO(chat CH-4): Send info text to both parties
    }

    /// TFS `Game::playerOpenPrivateChannel` — `game.cpp:3490-3502`.
    ///
    /// Opens a private channel dialog by name. Validates the name and rejects
    /// self-channel.
    ///
    /// C++ reference: `src/game.cpp` `Game::playerOpenPrivateChannel`.
    pub fn player_open_private_channel(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        receiver_name: &str,
    ) {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };

        // Reject self-channel
        if receiver_name == player.base.name {
            // TODO(chat CH-4): Send "You cannot create a private channel with yourself." message
            return;
        }

        // TODO(chat CH-4): Validate name format (IOLoginData::formatPlayerName equivalent)

        // Find private channel by name (owned by this player)
        let private_channel = self
            .chat
            .private_channels
            .values()
            .find(|pc| pc.owner == cid && pc.base.name == receiver_name);

        if let Some(_pc) = private_channel {
            // Use existing send_open_private_channel (takes receiver name only)
            let msg = outgoing_extra::send_open_private_channel(receiver_name);
            self.pending_outgoing
                .entry(conn_id)
                .or_default()
                .push(msg.into_bytes());
        } else {
            // TODO(chat CH-4): Send "Private channel not found." message
        }
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

    /// TFS `Game::playerSpeakTo` — `src/game.cpp:3654-3678`.
    ///
    /// Private message (tell) to another player by name. Resolves target via
    /// `player_by_name`, downgrades `TALKTYPE_PRIVATE_RED_TO` to `TALKTYPE_PRIVATE_FROM`
    /// unless the sender has `PlayerFlag_CanTalkRedPrivate` or is a GM, checks ghost-mode
    /// visibility, and sends confirmation/failure via `MESSAGE_STATUS_SMALL`.
    fn player_speak_to(&mut self, cid: CreatureId, speak_class: u8, receiver: &str, text: &str) {
        let Some(speaker_conn) = self.conn_for_creature(cid) else {
            return;
        };

        // C++ `Player* toPlayer = getPlayerByName(receiver);` — `game.cpp:3657-3661`.
        let Some(target_cid) = self.player_by_name.get(receiver).copied() else {
            use tfs_rust_net::outgoing_extra::send_text_message_simple;
            self.enqueue_outgoing(
                speaker_conn,
                send_text_message_simple(
                    self.codec.failure_message_type(),
                    "A player with this name is not online.",
                )
                .into_bytes(),
            );
            return;
        };

        let (target_ghost_mode, speaker_name, speaker_level) =
            match (self.creatures.get(target_cid), self.creatures.get(cid)) {
                (Some(CreatureKind::Player(target)), Some(CreatureKind::Player(speaker))) => {
                    (target.ghost_mode, speaker.base.name.clone(), speaker.level)
                }
                _ => return,
            };

        // C++ `if (type == TALKTYPE_PRIVATE_RED_TO && (player->hasFlag(PlayerFlag_CanTalkRedPrivate) || player->getAccountType() >= ACCOUNT_TYPE_GAMEMASTER))`
        // — `game.cpp:3663-3667`. Downgrade to normal private unless sender has flag or is GM.
        let actual_speak_class = if speak_class == TALKTYPE_PRIVATE_RED_TO
            && (self.player_has_flag(cid, PLAYER_FLAG_CAN_TALK_RED_PRIVATE)
                || self.player_is_access_player(cid))
        {
            TALKTYPE_PRIVATE_RED_FROM
        } else {
            TALKTYPE_PRIVATE_FROM
        };

        // C++ `toPlayer->sendPrivateMessage(player, type, text);` — `game.cpp:3669`.
        // Era-aware codec: 772 `sendPrivateMessage` omits the `u16 level` field that 1098 writes
        // after the name (`gameserver/src/protocolgame.cpp:1465`).
        if let Some(target_conn) = self.conn_for_creature(target_cid) {
            let statement_id = self.alloc_statement_id();
            let wire = PrivateMessageWire {
                speaker_name: Some(speaker_name.clone()),
                level: speaker_level as u16,
                speak_type: actual_speak_class,
                text: text.to_string(),
            };
            let msg = self.codec.encode_private_message(statement_id, &wire);
            self.enqueue_outgoing(target_conn, msg.into_bytes());

            // C++ `toPlayer->onCreatureSay(player, type, text);` — `game.cpp:3670`.
            // Event hook for future talkactions/creaturescripts (CH-6).
            self.events
                .on_creature_say(cid, target_cid, actual_speak_class, text);
        }

        // C++ ghost-mode visibility check — `game.cpp:3672-3676`.
        // If target is in ghost mode and sender cannot see ghosts, report "not online".
        if target_ghost_mode && !self.player_can_see_ghost_mode(cid, target_cid) {
            use tfs_rust_net::outgoing_extra::send_text_message_simple;
            self.enqueue_outgoing(
                speaker_conn,
                send_text_message_simple(
                    self.codec.failure_message_type(),
                    "A player with this name is not online.",
                )
                .into_bytes(),
            );
        } else {
            use tfs_rust_net::outgoing_extra::send_text_message_simple;
            self.enqueue_outgoing(
                speaker_conn,
                send_text_message_simple(
                    self.codec.failure_message_type(),
                    &format!("Message sent to {}.", receiver),
                )
                .into_bytes(),
            );
        }
    }

    /// TFS `Game::playerBroadcastMessage` — `src/game.cpp:1898-1910`.
    ///
    /// GM broadcast to all online players. Requires `PlayerFlag_CanBroadcast`. Sends
    /// `TALKTYPE_BROADCAST` to every online player via `sendPrivateMessage`.
    fn player_broadcast_message(&mut self, cid: CreatureId, text: &str) {
        // C++ `if (!player->hasFlag(PlayerFlag_CanBroadcast))` — `game.cpp:1900-1902`.
        if !self.player_has_flag(cid, PLAYER_FLAG_CAN_BROADCAST) {
            if let Some(conn) = self.conn_for_creature(cid) {
                use tfs_rust_net::outgoing_extra::send_text_message_simple;
                self.enqueue_outgoing(
                    conn,
                    send_text_message_simple(
                        self.codec.failure_message_type(),
                        "You are not allowed to broadcast.",
                    )
                    .into_bytes(),
                );
            }
            return;
        }

        let (speaker_name, speaker_level) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(speaker)) => (speaker.base.name.clone(), speaker.level),
            _ => return,
        };

        // C++ `std::cout << "> " << player->getName() << " broadcasted: \"" << text << "\"." << std::endl;`
        // — `game.cpp:1904`. Console log for audit trail.
        tracing::info!(player = %speaker_name, "broadcasted: \"{}\"", text);

        // C++ `for (const auto& it : players) { it.second->sendPrivateMessage(player, TALKTYPE_BROADCAST, text); }`
        // — `game.cpp:1906-1908`. Fan-out to all online players via the era-aware codec (772 omits
        // the `u16 level` field, `gameserver/src/protocolgame.cpp:1465`).
        let target_conns: Vec<ConnId> = self.conn_to_creature.keys().copied().collect();

        for target_conn in target_conns {
            let statement_id = self.alloc_statement_id();
            let wire = PrivateMessageWire {
                speaker_name: Some(speaker_name.clone()),
                level: speaker_level as u16,
                speak_type: TALKTYPE_BROADCAST,
                text: text.to_string(),
            };
            let msg = self.codec.encode_private_message(statement_id, &wire);
            self.enqueue_outgoing(target_conn, msg.into_bytes());
        }
    }

    /// C++ `Player::canSeeGhostMode` — `src/player.cpp:729-732`.
    ///
    /// Returns `true` if the viewer can see the target in ghost mode. Access players
    /// (GMs) can always see ghosts; regular players cannot.
    fn player_can_see_ghost_mode(&self, viewer_cid: CreatureId, _target_cid: CreatureId) -> bool {
        self.player_is_access_player(viewer_cid)
    }

    /// C++ `Player::removeMessageBuffer` — `player.cpp:1357-1380`.
    ///
    /// Called at the top of every successful `player_say` dispatch. Increments the
    /// message buffer count and applies escalating mute when exceeding `maxMessageBuffer`.
    /// Mute duration follows the `5 * n²` formula where n is the escalation count.
    fn player_remove_message_buffer(&mut self, cid: CreatureId) {
        let (guid, has_cannot_be_muted) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => (
                p.guid,
                self.player_has_flag(cid, PLAYER_FLAG_CANNOT_BE_MUTED),
            ),
            _ => return,
        };

        // C++ `if (hasFlag(PlayerFlag_CannotBeMuted)) return;` — `player.cpp:1359-1361`.
        if has_cannot_be_muted {
            return;
        }

        let max_buffer = self.chat_config.max_message_buffer as i32;
        if max_buffer == 0 {
            return;
        }

        let (buffer_count, player_name) = match self.creatures.get_mut(cid) {
            Some(CreatureKind::Player(p)) => {
                p.message_buffer_count += 1;
                (p.message_buffer_count, p.base.name.clone())
            }
            _ => return,
        };

        // C++ `if (++MessageBufferCount > maxMessageBuffer)` — `player.cpp:1364-1378`.
        if buffer_count > max_buffer {
            let mute_count = self.mute_count_map.get(&guid).copied().unwrap_or(1);
            let mute_time = 5 * mute_count * mute_count;
            self.mute_count_map.insert(guid, mute_count + 1);

            // C++ `Condition* condition = Condition::createCondition(CONDITIONID_DEFAULT, CONDITION_MUTED, muteTime * 1000, 0);`
            // — `player.cpp:1374`. Add `ConditionType::Muted` with the calculated duration.
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                use tfs_rust_common::enums::ConditionType;
                p.base.active_conditions.push(ActiveCondition {
                    id: 0,
                    sub_id: 0,
                    ctype: ConditionType::Muted,
                    data: ConditionData::Generic {
                        ticks: (mute_time * 1000) as i32,
                    },
                    timer_rounds_left: None,
                });
            }

            // C++ `sendTextMessage(MESSAGE_STATUS_SMALL, fmt::format("You are muted for {:d} seconds.", muteTime));`
            // — `player.cpp:1377`.
            if let Some(conn) = self.conn_for_creature(cid) {
                use tfs_rust_net::outgoing_extra::send_text_message_simple;
                self.enqueue_outgoing(
                    conn,
                    send_text_message_simple(
                        self.codec.failure_message_type(),
                        &format!("You are muted for {} seconds.", mute_time),
                    )
                    .into_bytes(),
                );
            }

            tracing::warn!(player = %player_name, mute_time, "flood mute applied");
        }
    }

    /// C++ `Player::isMuted` — `player.cpp:1335-1348`.
    ///
    /// Returns the remaining mute time in seconds, or 0 if not muted.
    /// Checks all active `ConditionType::Muted` conditions and returns the maximum
    /// remaining ticks (converted to seconds).
    fn player_is_muted(&self, cid: CreatureId) -> u32 {
        // C++ `if (hasFlag(PlayerFlag_CannotBeMuted)) return 0;` — `player.cpp:1337-1339`.
        if self.player_has_flag(cid, PLAYER_FLAG_CANNOT_BE_MUTED) {
            return 0;
        }

        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return 0;
        };

        use tfs_rust_common::enums::ConditionType;
        let mut max_ticks = 0i32;
        for cond in &p.base.active_conditions {
            if cond.ctype == ConditionType::Muted {
                if let ConditionData::Generic { ticks } = cond.data {
                    if ticks > max_ticks {
                        max_ticks = ticks;
                    }
                }
            }
        }
        (max_ticks / 1000) as u32
    }
    // ---- LUA-3 / LUA-4: Lua mutation applier helpers ----

    /// Send a status-style text message to a single player creature
    /// (`SendMessage(conn, TALK_STATUS_MESSAGE, ...)` — `crcombat.cc:674-676`,
    /// `enums.hh:672`). Used by the melee poison-on-hit path (M10) to notify a
    /// player target: "You are poisoned.". No-op if the creature has no
    /// connection (NPC/monster target) or is no longer online.
    pub(crate) fn send_player_status_message(&mut self, cid: CreatureId, text: &str) {
        if let Some(conn) = self.conn_for_creature(cid) {
            let msg =
                outgoing_extra::send_text_message_simple(self.codec.status_message_type(), text);
            self.enqueue_outgoing(conn, msg.into_bytes());
        }
    }

    /// `player:sendCancelMessage(text)` — LUA-3. Enqueues a
    /// `MESSAGE_STATUS_SMALL` text message to the player's connection.
    /// C++ reference: `protocolgame.cpp` `sendTextMessage(MESSAGE_STATUS_SMALL, text)`.
    pub fn lua_script_player_send_cancel_message(
        &mut self,
        creature_u64: u64,
        text: String,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        if let Some(conn) = self.conn_for_creature(cid) {
            let msg =
                outgoing_extra::send_text_message_simple(self.codec.failure_message_type(), &text);
            self.enqueue_outgoing(conn, msg.into_bytes());
        }
        Ok(())
    }

    /// `player:addCondition(condition)` — LUA-4. Immediate-apply condition add.
    /// C++ reference: `luascript.cpp:2117` `Creature::addCondition`;
    /// `condition.cpp` `Condition::addCondition` merge rules.
    ///
    /// `ctype` is the Lua-facing 772 bit-flag value (e.g.
    /// `CONDITION_CHANNELMUTEDTICKS = 1<<15 = 32768`); mapped to the Rust
    /// `ConditionType` enum via `condition_type_from_lua`.
    pub fn lua_script_player_add_condition(
        &mut self,
        creature_u64: u64,
        ctype: i32,
        _cond_id: i32,
        sub_id: u32,
        ticks: i32,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let rust_ctype = condition_type_from_lua(ctype);
        let cond = ActiveCondition {
            id: 0,
            sub_id,
            ctype: rust_ctype,
            data: ConditionData::Generic { ticks },
            timer_rounds_left: None,
        };
        apply_condition(&mut self.creatures, cid, cond);
        Ok(())
    }

    /// `player:removeCondition(type, id, subId)` — LUA-4. Immediate-apply
    /// condition removal. C++ reference: `luascript.cpp:2118`
    /// `Creature::removeCondition`; `condition.cpp` `Condition::endCondition`.
    pub fn lua_script_player_remove_condition(
        &mut self,
        creature_u64: u64,
        ctype: i32,
        _cond_id: i32,
        sub_id: u32,
    ) -> Result<(), String> {
        let cid = self
            .resolve_creature_u64(creature_u64)
            .ok_or_else(|| "creature not found".to_string())?;
        let rust_ctype = condition_type_from_lua(ctype);
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.base
                .active_conditions
                .retain(|c| !(c.ctype == rust_ctype && c.sub_id == sub_id));
        }
        Ok(())
    }

    /// `sendChannelMessage(channelId, type, message)` — LUA-4 §1.7.
    /// Server-originated channel broadcast (anonymous speaker). Fans out to all
    /// channel members via the era-aware codec's `encode_channel_message`.
    /// C++ reference: `chat.cpp` `sendChannelMessage` / `protocolgame.cpp`
    /// `sendChannelMessage` (`0xAA` anonymous branch).
    pub fn lua_script_send_channel_message(
        &mut self,
        channel_id: u16,
        speak_type: u8,
        text: String,
    ) -> Result<(), String> {
        use tfs_rust_net::ChannelMessageWire;
        let Some(channel) = self.chat.get_channel(channel_id) else {
            return Err("channel not found".into());
        };
        let recipients: Vec<ConnId> = channel
            .users
            .iter()
            .filter_map(|m| self.creature_to_conn.get(m).copied())
            .collect();
        let wire = ChannelMessageWire {
            author: String::new(),
            speak_type,
            channel_id,
            text,
        };
        let msg_bytes = self.codec.encode_channel_message(&wire).into_bytes();
        for conn_id in recipients {
            self.enqueue_outgoing(conn_id, msg_bytes.clone());
        }
        Ok(())
    }
}

/// Map a Lua-facing 772 bit-flag `ConditionType_t` value to the Rust
/// `ConditionType` enum (TFS 1.4.2 sequential numbering).
///
/// The 772 enum (`enums.h:249-269`) uses `1 << N` bit flags; the Rust enum
/// (`tfs_rust_common::enums::ConditionType`) uses sequential `0..=24`. Only the
/// variants used by the channel scripts are mapped here; unknown values log a
/// warning and return `ConditionType::None` (no-op).
///
/// C++ reference: `enums.h:249-269` `ConditionType_t` (772 bit flags).
pub(crate) fn condition_type_from_lua(ctype: i32) -> ConditionType {
    match ctype {
        0 => ConditionType::None,
        1 => ConditionType::Poison,                // 1 << 0
        2 => ConditionType::Fire,                  // 1 << 1
        4 => ConditionType::Energy,                // 1 << 2
        8 => ConditionType::Bleeding,              // 1 << 3
        16 => ConditionType::Haste,                // 1 << 4
        32 => ConditionType::Paralyze,             // 1 << 5
        64 => ConditionType::Outfit,               // 1 << 6
        128 => ConditionType::Invisible,           // 1 << 7
        256 => ConditionType::Light,               // 1 << 8
        512 => ConditionType::ManaShield,          // 1 << 9
        1024 => ConditionType::Infight,            // 1 << 10
        2048 => ConditionType::Drunk,              // 1 << 11
        16384 => ConditionType::Muted,             // 1 << 14
        32768 => ConditionType::ChannelMutedTicks, // 1 << 15
        65536 => ConditionType::YellTicks,         // 1 << 16
        131072 => ConditionType::Attributes,       // 1 << 17
        other => {
            tracing::warn!(
                ctype = other,
                "unknown 772 ConditionType bit-flag; mapping to None"
            );
            ConditionType::None
        }
    }
}
///
/// Uses `std::transform(..., toupper)` which is ASCII-only for the 772 Latin-1 client
/// charset. Rust `.to_uppercase()` is Unicode-aware and would produce different bytes
/// for non-ASCII characters (e.g. accented letters), so this helper mirrors the C++
/// byte-level `toupper` behavior exactly.
fn ascii_uppercase(s: &str) -> String {
    s.bytes().map(|b| b.to_ascii_uppercase() as char).collect()
}
