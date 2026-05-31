//! Version-neutral wire input structs (widest field widths).
//!
//! C++ reference: 10.98 `src/protocolgame.cpp`; 7.72 `gameserver/src/protocolgame.cpp` (Phase A5).
// PROTOCOL: neutral wire shape — encoders narrow per `ProtocolCodec` impl.

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

#[deprecated(note = "use PlayerStatsWire")]
pub type PlayerStats1098 = PlayerStatsWire;
