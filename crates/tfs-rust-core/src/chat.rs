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
// C++ reference: `chat.h` `ChatChannel`, `PrivateChatChannel`, `Chat`; `chat.cpp`.

use std::collections::{HashMap, HashSet};

use crate::ids::CreatureId;

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
/// Owns the static (`normal_channels`) and dynamic (`private_channels`) channel maps.
/// Guild/party channels are created on demand in CH-4 (C++ `guildChannels` /
/// `partyChannels`); they are not stored separately here yet — CH-4 will decide
/// whether to mirror the C++ split or fold guild/party into `normal_channels` keyed
/// by their reserved ids (`CHANNEL_GUILD` / `CHANNEL_PARTY`).
#[derive(Default)]
pub struct ChatRegistry {
    /// C++ `normalChannels` — static + Lua-defined public/GM/tutor channels.
    pub normal_channels: HashMap<u16, ChatChannel>,
    /// C++ `privateChannels` — dynamic player-created channels.
    pub private_channels: HashMap<u16, PrivateChatChannel>,
    /// Next dynamic private-channel id (allocation scheme finalized in CH-4 against
    /// `Chat::createChannel`; `CHANNEL_PRIVATE=0xFFFF` is the C++ sentinel base).
    pub next_private_channel_id: u16,
}

impl ChatRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}
