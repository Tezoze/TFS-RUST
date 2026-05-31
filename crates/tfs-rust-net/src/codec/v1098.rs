//! 10.98 / OTCv8 wire encoder — moved from `outgoing_extra`, `item_encode`, `creature_encode`.
//!
//! C++ reference: repo-root `src/protocolgame.cpp`, `src/networkmessage.cpp`.

use tfs_rust_common::{Position, ProtocolCaps, ProtocolVersion};

use crate::creature_encode::{write_add_creature, write_outfit, AddCreatureWire, OutfitWire};
use crate::item_encode::{item_template_wire_len, write_item_live, write_item_template};
use crate::NetworkMessage;

use super::wire::{ItemTemplateArgs, PlayerSkillsWire, PlayerStatsWire};

/// Zero-sized 10.98 codec (stateless; caps from `ProtocolVersion::V1098`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Codec1098;

impl Codec1098 {
    pub fn caps(&self) -> ProtocolCaps {
        ProtocolVersion::V1098.caps()
    }

    /// Mirrors C++ `NetworkMessage::addItem(uint16_t, uint8_t)` template field list (parity); the
    /// arg-struct form is `ItemTemplateArgs`, used at the higher-level call sites.
    #[allow(clippy::too_many_arguments)]
    pub fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) {
        write_item_template(
            msg,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        );
    }

    pub fn write_item_template_args(&self, msg: &mut NetworkMessage, args: ItemTemplateArgs) {
        self.write_item_template(
            msg,
            args.client_id,
            args.count,
            args.stackable,
            args.is_splash_or_fluid,
            args.is_animation,
            args.with_description,
        );
    }

    pub fn item_template_wire_len(
        &self,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) -> usize {
        item_template_wire_len(
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        )
    }

    pub fn write_add_creature(&self, msg: &mut NetworkMessage, c: &AddCreatureWire) {
        write_add_creature(msg, c);
    }

    pub fn add_creature_wire_len(&self, c: &AddCreatureWire) -> usize {
        crate::creature_encode::add_creature_wire_len(c)
    }

    pub fn write_outfit(&self, msg: &mut NetworkMessage, o: &OutfitWire) {
        write_outfit(msg, o);
    }

    /// `ProtocolGame::AddPlayerStats` opcode `0xA0` (1098 layout).
    pub fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xA0);
        m.write_u16(s.health);
        m.write_u16(s.max_health);
        m.write_u32(s.free_capacity);
        m.write_u32(s.total_capacity);
        m.write_u64(s.experience);
        m.write_u16(s.level);
        m.write_u8(s.level_percent);
        m.write_u16(100);
        m.write_u16(0);
        m.write_u16(0);
        m.write_u16(0);
        m.write_u16(100);
        m.write_u16(s.mana);
        m.write_u16(s.max_mana);
        m.write_u8(s.magic_level);
        m.write_u8(s.base_magic_level);
        m.write_u8(s.magic_level_percent);
        m.write_u8(s.soul);
        m.write_u16(s.stamina_minutes);
        m.write_u16(s.base_speed_half);
        m.write_u16(s.regeneration_ticks_sec);
        m.write_u16(s.offline_training_time);
        m.write_u16(0);
        m.write_u8(0);
        m
    }

    /// `GameServerPlayerSkills` opcode `0xA1` — OTClient v8 thirteen-skill layout.
    pub fn encode_player_skills(&self, s: &PlayerSkillsWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xA1);
        for i in 0..7 {
            m.write_u16(s.levels[i]);
            m.write_u16(s.bases[i]);
            m.write_u8(s.percents[i]);
        }
        for i in 0..6 {
            m.write_u16(s.additional_levels[i]);
            m.write_u16(s.additional_bases[i]);
        }
        m
    }

    /// `ProtocolGame::sendBasicData` opcode `0x9F`.
    pub fn encode_basic_data(
        &self,
        is_premium: bool,
        premium_ends_at: u32,
        vocation_client_id: u8,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x9F);
        if is_premium {
            m.write_u8(1);
            m.write_u32(premium_ends_at);
        } else {
            m.write_u8(0);
            m.write_u32(0);
        }
        m.write_u8(vocation_client_id);
        m.write_u16(0xFF);
        for spell_id in 0u16..=254 {
            m.write_u8(spell_id as u8);
        }
        m
    }

    /// `ProtocolGame::sendAddCreature` self branch — opcode `0x17` (1098).
    /// Opcode is version-keyed via `protocol_opcodes::server::self_appear` (Phase A2).
    pub fn encode_self_appear_login(&self, player_id: u32) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(tfs_rust_common::protocol_opcodes::server::self_appear(
            ProtocolVersion::V1098,
        ));
        m.write_u32(player_id);
        m.write_u16(0x32);
        m.write_double_tfs(857.36, 3);
        m.write_double_tfs(261.29, 3);
        m.write_double_tfs(-4795.01, 3);
        m.write_u8(0);
        m.write_u8(0);
        m.write_u8(0);
        m.write_string("");
        m.write_u16(25);
        m
    }

    pub fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6A);
        m.write_position(&pos);
        m.write_u8(stack_pos);
        self.write_item_template_args(&mut m, args);
        m
    }

    pub fn encode_update_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6B);
        m.write_position(&pos);
        m.write_u8(stack_pos);
        self.write_item_template_args(&mut m, args);
        m
    }

    pub fn encode_inventory_item(&self, slot: u8, args: ItemTemplateArgs) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x78);
        m.write_u8(slot);
        self.write_item_template_args(&mut m, args);
        m
    }

    /// Mirrors C++ live `NetworkMessage::addItem(const Item*, bool)` field list (parity).
    #[allow(clippy::too_many_arguments)]
    pub fn encode_inventory_item_live(
        &self,
        slot: u8,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
        description: &str,
        duration_pickup: Option<(u32, u8)>,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x78);
        m.write_u8(slot);
        write_item_live(
            &mut m,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
            description,
            duration_pickup,
        );
        m
    }

    pub fn encode_add_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x70);
        m.write_u8(cid);
        m.write_u16(slot);
        self.write_item_template_args(&mut m, args);
        m
    }

    pub fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x71);
        m.write_u8(cid);
        m.write_u16(slot);
        self.write_item_template_args(&mut m, args);
        m
    }

    pub fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &AddCreatureWire,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6A);
        m.write_position(&pos);
        m.write_u8(stack_pos);
        self.write_add_creature(&mut m, wire);
        m
    }

    pub fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6C);
        m.write_position(&pos);
        m.write_u8(stackpos);
        m
    }

    pub fn encode_remove_tile_creature_by_id(&self, creature_id: u32) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6C);
        m.write_u16(0xFFFF);
        m.write_u32(creature_id);
        m
    }

    pub fn encode_creature_light(
        &self,
        creature_id: u32,
        level: u8,
        color: u8,
        access_player: bool,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x8D);
        m.write_u32(creature_id);
        m.write_u8(if access_player { 0xFF } else { level });
        m.write_u8(color);
        m
    }

    /// C++ `ProtocolGame::sendCreatureTurn` (`src/protocolgame.cpp` ~2404).
    pub fn encode_creature_turn(
        &self,
        creature_id: u32,
        stack_pos: u8,
        tile_pos: Position,
        direction: u8,
        can_walkthrough: bool,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6B);
        if stack_pos >= 10 {
            m.write_u16(0xFFFF);
            m.write_u32(creature_id);
        } else {
            m.write_position(&tile_pos);
            m.write_u8(stack_pos);
        }
        m.write_u16(0x63);
        m.write_u32(creature_id);
        m.write_u8(direction);
        m.write_u8(if can_walkthrough { 0x00 } else { 0x01 });
        m
    }

    pub fn encode_cancel_walk(&self, direction: u8) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xB5);
        m.write_u8(direction);
        m
    }

    /// `ProtocolGame::sendContainer` opcode `0x6E` (`src/protocolgame.cpp` ~1751): cid + container
    /// item + name + `u8` capacity + `u8` hasParent + `u8` unlocked + `u8` pagination + `u16` size +
    /// `u16` firstIndex + (`u8` count + items) when `firstIndex < size`.
    pub fn encode_container_open(&self, c: &super::wire::ContainerOpenWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6E);
        m.write_u8(c.cid);
        self.write_item_template_args(&mut m, c.header_item);
        m.write_string(&c.name);
        m.write_u8(c.capacity);
        m.write_u8(u8::from(c.has_parent));
        m.write_u8(u8::from(c.unlocked));
        m.write_u8(u8::from(c.pagination));
        m.write_u16(c.total_size);
        m.write_u16(c.first_index);
        if u32::from(c.first_index) < u32::from(c.total_size) {
            let n = c.items.len().min(u8::MAX as usize) as u8;
            m.write_u8(n);
            for args in c.items.iter().take(n as usize) {
                self.write_item_template_args(&mut m, *args);
            }
        } else {
            m.write_u8(0);
        }
        m
    }
}
