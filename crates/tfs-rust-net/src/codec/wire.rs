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

#[deprecated(note = "use PlayerStatsWire")]
pub type PlayerStats1098 = PlayerStatsWire;
