//! Chat channels — `Chat` / `ChatChannel` / `PrivateChatChannel` Rust shapes.
//!
//! TFS-structured (`chat.h` / `chat.cpp`), not a line-port: the C++ class hierarchy
//! (`ChatChannel` base + `PrivateChatChannel` final) is mirrored with a `base` field
//! instead of OOP inheritance, and the `UsersMap`/`InvitedMap` `std::map<uint32_t, Player*>`
//! become `HashSet<CreatureId>` / `HashSet<u32>` (SlotMap keys are stable + typed; player
//! guid is the C++ `InvitedMap` key, kept as `u32` to match).
//!
//! CH-1 lands only the type skeleton — `TALKTYPE_SAY` does not touch channels. The
//! membership/lookup methods (`add_user_to_channel`, `remove_user_from_channel`,
//! `get_channel`, `get_channel_list`, …) and the `tfs-rust-lua` self-registering
//! `Channel(id, name)` loader arrive in CH-4 (see `tasks/chat-system-plan.md` §2.1/§3).
//!
//! Static channel ids (C++ `const.h:302-305`): `CHANNEL_GUILD=0x00`, `CHANNEL_PARTY=0x01`,
//! `CHANNEL_RULE_REP=0x02` (excluded — RVR non-goal, §1), `CHANNEL_PRIVATE=0xFFFF`
//! (sentinel/base for dynamic private channels; exact allocation scheme confirmed
//! against `Chat::createChannel` in CH-4). Static public/GM/tutor channels
//! (Tutor=3, Game-Chat=4, RL-Chat=5, Trade=6, Help=7, Gamemaster=8) are seeded from
//! `data/scripts/chatchannels/*.lua` in CH-4.
//!
//! ## Talkactions/Spell Integration Contract (CH-6 stub)
//!
//! The future Lua talkactions runtime must replace the `player_say_spell` stub in
//! `game_world_chat.rs` with the C++ `TalkActionResult_t` contract from
//! `talkaction.h:14-18`:
//!
//! - `TALKACTION_CONTINUE` (false): Text is plain chat, proceed to normal talk-type
//!   dispatch (say/whisper/yell/channel/private/broadcast).
//! - `TALKACTION_BREAK` (true): Text was consumed by a spell/talkaction. Re-broadcast
//!   as `TALKTYPE_SAY` or `TALKTYPE_MONSTER_SAY` unless the spell has the
//!   `EMOTE_SPELLS` flag (silent consumption).
//! - `TALKACTION_FAILED` (true): Text was consumed but execution failed (e.g., cooldown,
//!   insufficient mana). Do not re-broadcast; send failure feedback to player.
//!
//! The stub is the **single integration point** — do not add duplicate spell-words
//! call sites elsewhere. See `game_world_chat.rs:172-190` for the stub and TODO.
// C++ reference: `chat.h` `ChatChannel`, `PrivateChatChannel`, `Chat`; `chat.cpp`.

use std::collections::{HashMap, HashSet};

use crate::ids::CreatureId;

/// Static channel ids (C++ `const.h:302-305`).
pub const CHANNEL_GUILD: u16 = 0x00;
pub const CHANNEL_PARTY: u16 = 0x01;
pub const CHANNEL_RULE_REP: u16 = 0x02; // Excluded — RVR non-goal
pub const CHANNEL_PRIVATE: u16 = 0xFFFF; // Sentinel/base for dynamic private channels

/// TFS `ChatChannel` — `chat.h:15-71`.
///
/// `users` is keyed by `CreatureId` (SlotMap key) instead of C++ `uint32_t playerId`;
/// both are stable per-session player identifiers. `public_channel` mirrors the C++
/// `publicChannel` flag set by `ChatChannel::setPublic` / the Lua `:public(bool)` setter.
pub struct ChatChannel {
    pub id: u16,
    pub name: String,
    pub public_channel: bool,
    /// C++ `UsersMap` — members currently joined to this channel.
    pub users: HashSet<CreatureId>,
}

impl ChatChannel {
    pub fn new(id: u16, name: String) -> Self {
        Self {
            id,
            name,
            public_channel: false,
            users: HashSet::new(),
        }
    }
}

/// TFS `PrivateChatChannel` — `chat.h:73-101`.
///
/// Player-created dynamic channel. `owner` is the creating player's `CreatureId`;
/// `invited` holds invited player guids (`u32`) matching C++ `InvitedMap`'s key type.
pub struct PrivateChatChannel {
    pub base: ChatChannel,
    pub owner: CreatureId,
    /// C++ `InvitedMap` — invited player guids (not `CreatureId`: guid is the
    /// persistent cross-session id used by `isInvited` / `invitePlayer`).
    pub invited: HashSet<u32>,
}

impl PrivateChatChannel {
    pub fn new(id: u16, name: String, owner: CreatureId) -> Self {
        Self {
            base: ChatChannel::new(id, name),
            owner,
            invited: HashSet::new(),
        }
    }
}

/// TFS `Chat` — `chat.h:105-144`.
///
/// Owns the static (`normal_channels`), dynamic (`private_channels`), and
/// guild/party channel maps. Guild/party channels are created on-demand
/// per the C++ `Chat::createChannel` pattern.
#[derive(Default)]
pub struct ChatRegistry {
    /// C++ `normalChannels` — static + Lua-defined public/GM/tutor channels.
    pub normal_channels: HashMap<u16, ChatChannel>,
    /// C++ `privateChannels` — dynamic player-created channels.
    pub private_channels: HashMap<u16, PrivateChatChannel>,
    /// C++ `guildChannels` — dynamic guild channels keyed by guild id.
    pub guild_channels: HashMap<u32, ChatChannel>,
    /// C++ `partyChannels` — dynamic party channels keyed by party leader id.
    pub party_channels: HashMap<CreatureId, ChatChannel>,
    /// Next dynamic private-channel id (allocation scheme finalized in CH-4 against
    /// `Chat::createChannel`; `CHANNEL_PRIVATE=0xFFFF` is the C++ sentinel base).
    pub next_private_channel_id: u16,
}

impl ChatRegistry {
    pub fn new() -> Self {
        Self {
            normal_channels: HashMap::new(),
            private_channels: HashMap::new(),
            guild_channels: HashMap::new(),
            party_channels: HashMap::new(),
            next_private_channel_id: CHANNEL_PRIVATE,
        }
    }

    /// Add a normal (static/Lua-defined) channel to the registry.
    ///
    /// C++ reference: `chat.cpp` `Chat::addChannel` — `normalChannels` insertion.
    pub fn add_normal_channel(&mut self, channel: ChatChannel) {
        self.normal_channels.insert(channel.id, channel);
    }

    /// Get a channel by id (normal, private, guild, or party).
    ///
    /// C++ reference: `chat.cpp` `Chat::getChannel` — all channel map lookups.
    pub fn get_channel(&self, id: u16) -> Option<&ChatChannel> {
        self.normal_channels
            .get(&id)
            .or_else(|| self.private_channels.get(&id).map(|p| &p.base))
            .or_else(|| {
                if id == CHANNEL_GUILD {
                    self.guild_channels.values().next()
                } else if id == CHANNEL_PARTY {
                    self.party_channels.values().next()
                } else {
                    None
                }
            })
    }

    /// Get a mutable channel by id (normal, private, guild, or party).
    pub fn get_channel_mut(&mut self, id: u16) -> Option<&mut ChatChannel> {
        if let Some(channel) = self.normal_channels.get_mut(&id) {
            Some(channel)
        } else if let Some(channel) = self.private_channels.get_mut(&id) {
            Some(&mut channel.base)
        } else if id == CHANNEL_GUILD {
            self.guild_channels.values_mut().next()
        } else if id == CHANNEL_PARTY {
            self.party_channels.values_mut().next()
        } else {
            None
        }
    }

    /// Get a private channel by id.
    pub fn get_private_channel(&self, id: u16) -> Option<&PrivateChatChannel> {
        self.private_channels.get(&id)
    }

    /// Get a mutable private channel by id.
    pub fn get_private_channel_mut(&mut self, id: u16) -> Option<&mut PrivateChatChannel> {
        self.private_channels.get_mut(&id)
    }

    /// Add a user to a channel.
    ///
    /// C++ reference: `chat.cpp` `ChatChannel::addUser` — `users` insertion.
    pub fn add_user_to_channel(&mut self, channel_id: u16, user_id: CreatureId) -> bool {
        if let Some(channel) = self.get_channel_mut(channel_id) {
            channel.users.insert(user_id);
            true
        } else {
            false
        }
    }

    /// Remove a user from a channel.
    ///
    /// C++ reference: `chat.cpp` `ChatChannel::removeUser` — `users` erasure.
    pub fn remove_user_from_channel(&mut self, channel_id: u16, user_id: CreatureId) -> bool {
        if let Some(channel) = self.get_channel_mut(channel_id) {
            channel.users.remove(&user_id)
        } else {
            false
        }
    }

    /// Remove a user from all channels (called on logout).
    ///
    /// C++ reference: `chat.cpp` `Chat::removeUserFromAllChannels` — iterates all channels.
    pub fn remove_user_from_all_channels(&mut self, user_id: CreatureId) {
        for channel in self.normal_channels.values_mut() {
            channel.users.remove(&user_id);
        }
        for channel in self.private_channels.values_mut() {
            channel.base.users.remove(&user_id);
        }
        for channel in self.guild_channels.values_mut() {
            channel.users.remove(&user_id);
        }
        for channel in self.party_channels.values_mut() {
            channel.users.remove(&user_id);
        }
    }

    /// Check if a user is in a channel.
    pub fn is_user_in_channel(&self, channel_id: u16, user_id: CreatureId) -> bool {
        self.get_channel(channel_id)
            .map(|channel| channel.users.contains(&user_id))
            .unwrap_or(false)
    }

    /// Get the list of channels visible to a player.
    ///
    /// C++ reference: `chat.cpp` `Chat::getChannelList` — per-player visibility logic.
    /// Creates guild/party channels on-demand if they don't exist.
    pub fn get_channel_list(
        &mut self,
        player_id: CreatureId,
        player_guid: Option<u32>,
        guild_id: Option<u32>,
        party_leader: Option<CreatureId>,
    ) -> Vec<(u16, String)> {
        let mut channels = Vec::new();

        // Add normal channels (public channels are always visible)
        for (id, channel) in &self.normal_channels {
            if channel.public_channel {
                channels.push((*id, channel.name.clone()));
            }
        }

        // Add guild channel if player has a guild (create on-demand per C++ behavior)
        if let Some(guild_id) = guild_id {
            if !self.guild_channels.contains_key(&guild_id) {
                // Create guild channel on-demand
                let channel = ChatChannel::new(CHANNEL_GUILD, "Guild Channel".to_string());
                self.guild_channels.insert(guild_id, channel);
            }
            if let Some(guild_channel) = self.guild_channels.get(&guild_id) {
                channels.push((CHANNEL_GUILD, guild_channel.name.clone()));
            }
        }

        // Add party channel if player is in a party (create on-demand per C++ behavior)
        if let Some(leader_id) = party_leader {
            if !self.party_channels.contains_key(&leader_id) {
                // Create party channel on-demand
                let channel = ChatChannel::new(CHANNEL_PARTY, "Party".to_string());
                self.party_channels.insert(leader_id, channel);
            }
            if let Some(party_channel) = self.party_channels.get(&leader_id) {
                channels.push((CHANNEL_PARTY, party_channel.name.clone()));
            }
        }

        // Add private channels where player is owner or invited
        if let Some(guid) = player_guid {
            for (id, channel) in &self.private_channels {
                if channel.owner == player_id || channel.invited.contains(&guid) {
                    channels.push((*id, channel.base.name.clone()));
                }
            }
        }

        channels.sort_by_key(|(id, _)| *id);
        channels
    }

    /// Create a new private channel.
    ///
    /// C++ reference: `chat.cpp` `Chat::createChannel` — dynamic private channel allocation.
    pub fn create_private_channel(&mut self, name: String, owner: CreatureId) -> u16 {
        let id = self.next_private_channel_id;
        self.next_private_channel_id += 1;

        let channel = PrivateChatChannel::new(id, name, owner);
        self.private_channels.insert(id, channel);

        id
    }

    /// Create a channel dynamically (guild/party/private).
    ///
    /// C++ reference: `chat.cpp` `Chat::createChannel` — handles CHANNEL_GUILD,
    /// CHANNEL_PARTY, and CHANNEL_PRIVATE allocation.
    pub fn create_channel(
        &mut self,
        channel_id: u16,
        player_id: CreatureId,
        guild_id: Option<u32>,
        party_leader: Option<CreatureId>,
    ) -> Option<u16> {
        match channel_id {
            CHANNEL_GUILD => {
                if let Some(guild_id) = guild_id {
                    if !self.guild_channels.contains_key(&guild_id) {
                        // Guild channel name would come from guild data; for now use placeholder
                        let channel = ChatChannel::new(CHANNEL_GUILD, "Guild Channel".to_string());
                        self.guild_channels.insert(guild_id, channel);
                    }
                    Some(CHANNEL_GUILD)
                } else {
                    None
                }
            }
            CHANNEL_PARTY => {
                if let Some(leader_id) = party_leader {
                    if !self.party_channels.contains_key(&leader_id) {
                        let channel = ChatChannel::new(CHANNEL_PARTY, "Party".to_string());
                        self.party_channels.insert(leader_id, channel);
                    }
                    Some(CHANNEL_PARTY)
                } else {
                    None
                }
            }
            CHANNEL_PRIVATE => {
                let id = self.next_private_channel_id;
                self.next_private_channel_id += 1;
                let channel =
                    PrivateChatChannel::new(id, format!("Private Channel {}", id), player_id);
                self.private_channels.insert(id, channel);
                Some(id)
            }
            _ => None,
        }
    }

    /// Invite a player to a private channel.
    ///
    /// C++ reference: `chat.cpp` `PrivateChatChannel::invitePlayer` — `invited` insertion.
    pub fn invite_to_private_channel(&mut self, channel_id: u16, player_guid: u32) -> bool {
        if let Some(channel) = self.private_channels.get_mut(&channel_id) {
            channel.invited.insert(player_guid);
            true
        } else {
            false
        }
    }

    /// Exclude a player from a private channel.
    ///
    /// C++ reference: `chat.cpp` `PrivateChatChannel::excludePlayer` — `invited` erasure.
    pub fn exclude_from_private_channel(&mut self, channel_id: u16, player_guid: u32) -> bool {
        if let Some(channel) = self.private_channels.get_mut(&channel_id) {
            channel.invited.remove(&player_guid)
        } else {
            false
        }
    }

    /// Check if a player is invited to a private channel.
    pub fn is_invited_to_private_channel(&self, channel_id: u16, player_guid: u32) -> bool {
        self.private_channels
            .get(&channel_id)
            .map(|channel| channel.invited.contains(&player_guid))
            .unwrap_or(false)
    }
}
