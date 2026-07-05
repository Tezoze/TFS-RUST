//! Version-neutral wire input structs (widest field widths).
//!
//! C++ reference: 10.98 `src/protocolgame.cpp`; 7.72 `gameserver/src/protocolgame.cpp` (Phase A5).
// PROTOCOL: neutral wire shape — encoders narrow per `ProtocolCodec` impl.

use tfs_rust_common::Position;

pub use crate::creature_encode::{AddCreatureWire, OutfitWire};
pub use crate::map_description::ItemStack;

/// Alias for map/tile item template encoding.
pub type ItemWire = ItemStack;

/// `ProtocolGame::AddPlayerStats` fields at max width (`src/protocolgame.cpp` ~3246).
#[derive(Debug, Clone)]
pub struct PlayerStatsWire {
    pub health: u16,
    pub max_health: u16,
    pub free_capacity: u32,
    pub total_capacity: u32,
    pub experience: u64,
    pub level: u16,
    pub level_percent: u8,
    pub mana: u16,
    pub max_mana: u16,
    pub magic_level: u8,
    pub base_magic_level: u8,
    pub magic_level_percent: u8,
    pub soul: u8,
    pub stamina_minutes: u16,
    /// `getBaseSpeed() / 2` when `ProtocolCaps::speed_halved` (C++ 10.98).
    pub base_speed_half: u16,
    pub regeneration_ticks_sec: u16,
    pub offline_training_time: u16,
}

/// `GameServerPlayerSkills` — OTCv8 / `GameAdditionalSkills` layout (`docs/OTCLIENT_INFO.md` §2).
#[derive(Debug, Clone)]
pub struct PlayerSkillsWire {
    pub levels: [u16; 7],
    pub bases: [u16; 7],
    pub percents: [u8; 7],
    pub additional_levels: [u16; 6],
    pub additional_bases: [u16; 6],
}

/// Parameters for template `addItem` on the wire (inventory / tile / container).
#[derive(Debug, Clone, Copy)]
pub struct ItemTemplateArgs {
    pub client_id: u16,
    pub count: u8,
    pub stackable: bool,
    pub is_splash_or_fluid: bool,
    pub is_animation: bool,
    pub with_description: bool,
}

/// `ProtocolGame::sendContainer` (`0x6E`) at max width. Core fills every field; each codec narrows:
/// 10.98 writes `unlocked` / `pagination` / `total_size` / `first_index`; 7.72 omits them
/// (`gameserver/src/protocolgame.cpp` `sendContainer` ~L1326). `items` is the already-windowed list
/// (core applies `first_index` + capacity); 7.72 never paginates so it is the leading slice.
#[derive(Debug, Clone)]
pub struct ContainerOpenWire {
    pub cid: u8,
    /// Container item itself (template `addItem`) + its name.
    pub header_item: ItemTemplateArgs,
    pub name: String,
    pub capacity: u8,
    pub has_parent: bool,
    /// 10.98 only.
    pub unlocked: bool,
    /// 10.98 only.
    pub pagination: bool,
    /// 10.98 only — total item count (for pagination).
    pub total_size: u16,
    /// 10.98 only — index of the first item in `items`.
    pub first_index: u16,
    pub items: Vec<ItemTemplateArgs>,
}

/// `ProtocolGame::sendAnimatedText` — 7.72 only (`gameserver/src/protocolgame.cpp` ~1255).
#[derive(Debug, Clone)]
pub struct AnimatedTextWire {
    pub pos: Position,
    pub color: u8,
    pub text: String,
}

/// `ProtocolGame::sendMagicEffect` — `src/protocolgame.cpp` / `gameserver/src/protocolgame.cpp`.
#[derive(Debug, Clone, Copy)]
pub struct MagicEffectWire {
    pub pos: Position,
    pub effect_id: u8,
}

/// `ProtocolGame::sendDistanceShoot` — `src/protocolgame.cpp` / `gameserver/src/protocolgame.cpp` ~1535.
#[derive(Debug, Clone, Copy)]
pub struct DistanceShootWire {
    pub from: Position,
    pub to: Position,
    pub shoot_type: u8,
}

/// `ProtocolGame::sendCreatureHealth`.
#[derive(Debug, Clone, Copy)]
pub struct CreatureHealthWire {
    pub creature_id: u32,
    pub health_percent: u8,
}

/// `SendCreatureSpeed` (772 `sending.cc:1028`) / `sendChangeSpeed` (1098 `src/protocolgame.cpp`).
///
/// - **772**: `0x8F + u32 creature_id + u16 speed` (single full `GetSpeed()`).
/// - **1098**: `0x8F + u32 creature_id + u16 base_speed/2 + u16 speed/2` (two halved values).
#[derive(Debug, Clone, Copy)]
pub struct CreatureSpeedWire {
    pub creature_id: u32,
    /// Full `GetSpeed()` value (`2*go+80` for 772). 1098 halves on the wire.
    pub speed: u16,
    /// 1098-only second field (`baseSpeed/2`); ignored by 772 encoder.
    pub base_speed: u16,
}

/// `Game::combatChangeHealth` player damage caption — layout differs by era.
/// 772: simple `sendTextMessage` (`gameserver/src/game.cpp` ~3918).
/// 1098: damage branch with position + color block (`src/game.cpp` ~4340).
#[derive(Debug, Clone)]
pub struct CombatDamageNotifyWire {
    pub pos: Position,
    pub damage: u32,
    pub damage_color: u8,
    pub text: String,
}

/// `ProtocolGame::sendCreatureSay` — `0xAA` speech packet.
/// 1098 (`src/protocolgame.cpp` ~2427): `name + u16 level + speak_type + pos + text`.
/// 772 (`gameserver/src/protocolgame.cpp` ~1422): `name + speak_type + pos + text` (no level).
/// `speak_type` is the era-native `SpeakClasses` byte (e.g. 772 `TALKTYPE_MONSTER_SAY=0x11`,
/// 1098 `TALKTYPE_MONSTER_SAY=36`).
#[derive(Debug, Clone)]
pub struct CreatureSayWire {
    pub speaker_name: String,
    /// 1098 only — 772 omits this field.
    pub level: u16,
    pub speak_type: u8,
    pub pos: Position,
    pub text: String,
}

/// `ProtocolGame::sendToChannel` — `0xAA` channel speech.
/// 1098 (`src/protocolgame.cpp` ~1730): `u32 stmt + name + u16 level + u8 speak_type + u16 channelId + text`.
/// 772 (`gameserver/src/protocolgame.cpp:1442`): `u32 stmt + name + u8 speak_type + u16 channelId + text`
/// — **no `level` field** (10.98 adds it, which desyncs a 772 client's message-mode read if written).
/// `speaker_name = None` → anonymous branch (C++ `!creature`): a `u32 0` is written in place of the name.
#[derive(Debug, Clone)]
pub struct ToChannelWire {
    pub speaker_name: Option<String>,
    /// 1098 only — 772 omits this field.
    pub level: u16,
    pub speak_type: u8,
    pub channel_id: u16,
    pub text: String,
}

/// `ProtocolGame::sendPrivateMessage` — `0xAA` private message (tell / broadcast).
/// 1098 (`src/protocolgame.cpp` ~2480): `u32 stmt + name + u16 level + u8 speak_type + text`.
/// 772 (`gameserver/src/protocolgame.cpp:1465`): `u32 stmt + name + u8 speak_type + text` — **no `level`**.
/// `speaker_name = None` → anonymous branch (`u32 0` in place of the name).
#[derive(Debug, Clone)]
pub struct PrivateMessageWire {
    pub speaker_name: Option<String>,
    /// 1098 only — 772 omits this field.
    pub level: u16,
    pub speak_type: u8,
    pub text: String,
}

/// `ProtocolGame::sendChannelMessage` — `0xAA` anonymous channel message (statement id + author level
/// always zero). Used for server-originated channel text (e.g. Help `!mute` broadcast).
/// 1098 (`src/protocolgame.cpp:1730`): `u32 0 + author + u16 0 + u8 speak_type + u16 channel + text`.
/// 772 (`gameserver/src/protocolgame.cpp:1306`): `u32 0 + author + u8 speak_type + u16 channel + text`
/// — **no `level` field**.
#[derive(Debug, Clone)]
pub struct ChannelMessageWire {
    pub author: String,
    pub speak_type: u8,
    pub channel_id: u16,
    pub text: String,
}

/// `ProtocolGame::sendChannelsDialog` — `0xAB` channel list dialog.
/// Era-identical layout (`gameserver/src/protocolgame.cpp:1282` == `src/protocolgame.cpp:1687`):
/// `byte + u8 count + [u16 id + string name]*`. Both codecs emit the same bytes; the struct is shared
/// for uniform call-site shape across the `ProtocolCodec` seam.
#[derive(Debug, Clone, Default)]
pub struct ChannelsDialogWire {
    /// `(channel_id, channel_name)` pairs — the per-player visible channel list from `Chat::getChannelList`.
    pub channels: Vec<(u16, String)>,
}

/// `ProtocolGame::sendChannel` — `0xAC` open-channel ack.
///
/// - **1098** (`src/protocolgame.cpp:1702`): `byte + u16 id + string name + u16 usersCount +
///   `[string userName]*` + `u16 invitedCount` + `[string invitedName]*`.
/// - **772** (`gameserver/src/protocolgame.cpp:1297`): `byte + u16 id + string name` — no user lists.
///
/// The 772 codec ignores `users` / `invited`; the 1098 codec emits them.
#[derive(Debug, Clone, Default)]
pub struct ChannelOpenWire {
    pub channel_id: u16,
    pub name: String,
    /// Member names currently in the channel (1098 only).
    pub users: Vec<String>,
    /// Invited-but-not-yet-joined names (1098 only; private channels).
    pub invited: Vec<String>,
}

/// `ProtocolGame::sendCreatePrivateChannel` — `0xB2` ack for a newly-created private channel.
///
/// - **1098** (`src/protocolgame.cpp:1675`): `byte + u16 id + string name + `u16(1)` +
///   `string ownerName` + `u16 invitedCount` + `[string invitedName]*`.
/// - **772** (`gameserver/src/protocolgame.cpp:1273`): `byte + u16 id + string name` — no owner/invited lists.
///
/// The 772 codec ignores `owner_name` / `invited`; the 1098 codec emits them.
#[derive(Debug, Clone, Default)]
pub struct CreatePrivateChannelWire {
    pub channel_id: u16,
    pub name: String,
    /// Creator's player name (1098 only — C++ writes `player->getName()`).
    pub owner_name: String,
    /// Invited player names (1098 only).
    pub invited: Vec<String>,
}

#[deprecated(note = "use PlayerStatsWire")]
pub type PlayerStats1098 = PlayerStatsWire;
