//! Classic 8.0 wire encoder. Framing and most packets match 7.72; only the
//! bytes below differ.
//!
//! C++ reference: `800src/protocolgame.cpp`.
//! - `AddOutfit` ~2386: addons byte, no mount.
//! - `AddPlayerStats` ~2351: trailing `u16` stamina minutes.
//! - `sendCreatureSay` / `sendToChannel` / `sendPrivateMessage` / `sendChannelMessage`
//!   ~1684, ~1714, ~1745, ~1317: `u16` level after a speaker name.
//! - `sendTextWindow` ~2144: date string when client version >= 790.
//! - `tools.cpp` `getLiquidColor` ~189 and `const.h` `reverseFluidMap`: the 8.0
//!   palette. Blood is red (5) and mana/wine are purple (2). The 7.72 table
//!   paints those as purple and brown on this client.
//!
//! Omitted from this codec (OT-only in `800src`, not classic 8.0): mount `u16`,
//! mount list, wings/aura/shader, duplicate login account, market, quests.
//!
//! Game loop, monster AI, and combat stay on the shared corpus. This type only
//! chooses bytes.

use tfs_rust_common::protocol_opcodes::server;
use tfs_rust_common::{Position, ProtocolCaps, ProtocolVersion};

use crate::NetworkMessage;
use crate::creature_encode::{AddCreatureWire, OutfitWire};

use super::wire::{
    AnimatedTextWire, ChannelMessageWire, ChannelOpenWire, ChannelsDialogWire,
    CombatDamageNotifyWire, ContainerOpenWire, CreatePrivateChannelWire, CreatureHealthWire,
    CreatureSayWire, CreatureSpeedWire, CreatureSquareWire, DistanceShootWire, ItemTemplateArgs,
    MagicEffectWire, PlayerSkillsWire, PlayerStatsWire, PrivateMessageWire, TextWindowWire,
    ToChannelWire,
};
use super::{Codec772, ProtocolCodec};

/// Corpus `TALKTYPE_CHANNEL_R2` (`game_world_chat.rs`). Classic 8.0 wire is 13
/// (`800src/const.h:116`).
const CORPUS_CHANNEL_R2: u8 = 14;
const WIRE_CHANNEL_R2: u8 = 13;

/// Zero-sized 8.0 codec. Identical packets forward to [`Codec772`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Codec800;

fn speak_type_on_wire(speak_type: u8) -> u8 {
    if speak_type == CORPUS_CHANNEL_R2 {
        WIRE_CHANNEL_R2
    } else {
        speak_type
    }
}

/// 8.0 `getLiquidColor` (`800src/tools.cpp` ~189). Server fluid ids stay the
/// sequential `FluidTypes_t` (`800src/const.h` ~138). The byte is `FluidColor_t`.
pub fn liquid_color_800(fluid_type: u8) -> u8 {
    match fluid_type {
        1 => 1,                        // water → blue
        0 => 0,                        // none
        6 => 6,                        // slime → green
        3 | 4 | 7 | 13 | 16 | 17 => 3, // beer, mud, oil, rum, mead, tea → brown
        9 | 14 => 9,                   // milk, coconut milk → white
        2 | 10 => 2,                   // wine, mana → purple
        5 | 11 => 5,                   // blood, life → red
        8 | 12 | 15 => 8,              // urine, lemonade, fruit juice → yellow
        _ => 0,
    }
}

/// Shop / hotkey color index → server fluid (`800src/const.h` `reverseFluidMap`).
/// Indexes past the table are not a fluid.
pub fn client_color_to_fluid_800(color: u8) -> u8 {
    const REVERSE: [u8; 10] = [
        0,  // none
        1,  // water
        10, // mana
        3,  // beer
        0,  // none
        11, // life
        6,  // slime
        0,  // none
        12, // lemonade
        9,  // milk
    ];
    REVERSE.get(usize::from(color)).copied().unwrap_or(0)
}

impl Codec800 {
    pub fn caps(&self) -> ProtocolCaps {
        ProtocolVersion::V800.caps()
    }

    /// 8.0 `NetworkMessage::addItem`: same fields as 7.72, `getLiquidColor` from
    /// `800src/tools.cpp`.
    #[allow(clippy::too_many_arguments)]
    pub fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        _is_animation: bool,
        _with_description: bool,
    ) {
        msg.write_u16(client_id);
        if stackable {
            msg.write_u8(count);
        } else if is_splash_or_fluid {
            msg.write_u8(liquid_color_800(count));
        }
    }

    fn write_item_template_args(&self, msg: &mut NetworkMessage, args: ItemTemplateArgs) {
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

    /// 7.72 framing (`v772.rs`) with this codec's liquid color.
    pub fn encode_add_tile_item(
        &self,
        pos: Position,
        _stack_pos: u8,
        args: ItemTemplateArgs,
        _otclient_stackpos: bool,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6A);
        m.write_position(&pos);
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

    pub fn encode_add_container_item(
        &self,
        cid: u8,
        _slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x70);
        m.write_u8(cid);
        self.write_item_template_args(&mut m, args);
        m
    }

    pub fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        if slot >= 36 {
            return NetworkMessage::new();
        }
        let mut m = NetworkMessage::new();
        m.write_u8(0x71);
        m.write_u8(cid);
        m.write_u8(slot as u8);
        self.write_item_template_args(&mut m, args);
        m
    }

    pub fn encode_container_open(&self, c: &super::wire::ContainerOpenWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6E);
        m.write_u8(c.cid);
        self.write_item_template_args(&mut m, c.header_item);
        m.write_string(&c.name);
        m.write_u8(c.capacity);
        m.write_u8(u8::from(c.has_parent));
        let n = c
            .items
            .len()
            .min(c.capacity as usize)
            .min(36)
            .min(u8::MAX as usize) as u8;
        m.write_u8(n);
        for args in c.items.iter().take(n as usize) {
            self.write_item_template_args(&mut m, *args);
        }
        m
    }

    pub fn encode_trade_item_request(
        &self,
        trader_name: &str,
        own_offer: bool,
        items: &[ItemTemplateArgs],
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(if own_offer {
            server::TRADE_OFFER_OWN
        } else {
            server::TRADE_OFFER_PARTNER
        });
        m.write_string(trader_name);
        m.write_u8(items.len().min(255) as u8);
        for args in items.iter().take(255) {
            self.write_item_template_args(&mut m, *args);
        }
        m
    }

    /// `800src/protocolgame.cpp` `AddOutfit` ~2386. Addons when `lookType != 0`.
    /// No `lookMount`.
    pub fn write_outfit(&self, msg: &mut NetworkMessage, o: &OutfitWire) {
        msg.write_u16(o.look_type);
        if o.look_type != 0 {
            msg.write_u8(o.look_head);
            msg.write_u8(o.look_body);
            msg.write_u8(o.look_legs);
            msg.write_u8(o.look_feet);
            msg.write_u8(o.look_addons);
        } else {
            msg.write_u16(o.look_type_ex);
        }
    }

    fn outfit_wire_len(&self, o: &OutfitWire) -> usize {
        // lookType + (colors + addons) or lookTypeEx.
        2 + if o.look_type != 0 { 5 } else { 2 }
    }

    /// `800src/protocolgame.cpp` `AddCreature` ~2311. Known is always `0x62`
    /// plus the full block. `0x63` is turn-only (`sendCreatureTurn`).
    ///
    /// The 8.0 client (`Tibia.exe` `0x40d3b0`) does not create a creature from
    /// `0x63`. A missed lookup consumes the direction byte and leaves the tile
    /// empty; the next `0x6D` asserts `bug0000017`.
    pub fn write_add_creature(&self, msg: &mut NetworkMessage, c: &AddCreatureWire) {
        if c.known {
            msg.write_u16(0x62);
            msg.write_u32(c.id);
        } else {
            msg.write_u16(0x61);
            msg.write_u32(c.remove_known);
            msg.write_u32(c.id);
            msg.write_string(&c.name);
        }

        msg.write_u8(c.health_percent);
        msg.write_u8(c.direction);
        self.write_outfit(msg, &c.outfit);
        msg.write_u8(c.light_level);
        msg.write_u8(c.light_color);
        msg.write_u16(c.step_speed);
        msg.write_u8(c.skull);
        msg.write_u8(c.party_shield);
    }

    pub fn add_creature_wire_len(&self, c: &AddCreatureWire) -> usize {
        let head = if c.known {
            2 + 4
        } else {
            2 + 4 + 4 + 2 + c.name.len()
        };
        head + 1 + 1 + self.outfit_wire_len(&c.outfit) + 2 + 2 + 1 + 1
    }

    /// 772 stats body plus `u16` stamina (`800src/protocolgame.cpp` ~2373).
    /// The value is the stored minutes; this codec does not add a drain system.
    pub fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xA0);
        m.write_u16(s.health);
        m.write_u16(s.max_health);
        m.write_u16((s.free_capacity / 100).min(u16::MAX as u32) as u16);
        if s.experience >= u32::MAX as u64 - 1 {
            m.write_u32(0);
        } else {
            m.write_u32(s.experience as u32);
        }
        m.write_u16(s.level);
        m.write_u8(s.level_percent);
        m.write_u16(s.mana);
        m.write_u16(s.max_mana);
        m.write_u8(s.magic_level);
        m.write_u8(s.magic_level_percent);
        m.write_u8(s.soul);
        m.write_u16(s.stamina_minutes);
        m
    }

    pub fn encode_add_tile_creature(
        &self,
        pos: Position,
        wire: &AddCreatureWire,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6A);
        m.write_position(&pos);
        self.write_add_creature(&mut m, wire);
        m
    }

    pub fn encode_creature_outfit(&self, creature_id: u32, outfit: &OutfitWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::CREATURE_OUTFIT);
        m.write_u32(creature_id);
        self.write_outfit(&mut m, outfit);
        m
    }

    pub fn encode_creature_say(&self, statement_id: u32, w: &CreatureSayWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(statement_id);
        m.write_string(&w.speaker_name);
        m.write_u16(w.level);
        m.write_u8(speak_type_on_wire(w.speak_type));
        m.write_position(&w.pos);
        m.write_string(&w.text);
        m
    }

    pub fn encode_to_channel(&self, statement_id: u32, w: &ToChannelWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(statement_id);
        match &w.speaker_name {
            Some(name) => {
                m.write_string(name);
                m.write_u16(w.level);
            }
            None => m.write_u32(0),
        }
        m.write_u8(speak_type_on_wire(w.speak_type));
        m.write_u16(w.channel_id);
        m.write_string(&w.text);
        m
    }

    pub fn encode_private_message(
        &self,
        statement_id: u32,
        w: &PrivateMessageWire,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(statement_id);
        match &w.speaker_name {
            Some(name) => {
                m.write_string(name);
                m.write_u16(w.level);
            }
            None => m.write_u32(0),
        }
        m.write_u8(speak_type_on_wire(w.speak_type));
        m.write_string(&w.text);
        m
    }

    pub fn encode_channel_message(&self, w: &ChannelMessageWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(0);
        m.write_string(&w.author);
        m.write_u16(0);
        m.write_u8(speak_type_on_wire(w.speak_type));
        m.write_u16(w.channel_id);
        m.write_string(&w.text);
        m
    }

    /// 772 text window plus the date string (`800src/protocolgame.cpp` ~2144).
    pub fn encode_text_window(&self, w: &TextWindowWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::TEXT_WINDOW);
        m.write_u32(w.window_text_id);
        self.write_item_template(
            &mut m,
            w.item.client_id,
            w.item.count,
            w.item.stackable,
            w.item.is_splash_or_fluid,
            w.item.is_animation,
            w.item.with_description,
        );
        let maxlen = if w.can_write {
            w.max_text_len
        } else {
            w.text.len() as u16
        };
        m.write_u16(maxlen);
        m.write_string(&w.text);
        m.write_string(&w.writer);
        match w.written_date.as_deref() {
            Some(date) if !date.is_empty() => m.write_string(date),
            _ => m.write_u16(0),
        }
        m
    }
}

impl ProtocolCodec for Codec800 {
    fn caps(&self) -> ProtocolCaps {
        Codec800::caps(self)
    }

    fn write_tile_environment_prefix(&self, msg: &mut NetworkMessage) {
        ProtocolCodec::write_tile_environment_prefix(&Codec772, msg);
    }

    fn tile_environment_prefix_len(&self) -> usize {
        ProtocolCodec::tile_environment_prefix_len(&Codec772)
    }

    fn tile_description_caps_creatures(&self) -> bool {
        ProtocolCodec::tile_description_caps_creatures(&Codec772)
    }

    fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) {
        Codec800::write_item_template(
            self,
            msg,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        );
    }

    fn item_template_wire_len(
        &self,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) -> usize {
        ProtocolCodec::item_template_wire_len(
            &Codec772,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        )
    }

    fn write_add_creature(&self, msg: &mut NetworkMessage, c: &AddCreatureWire) {
        Codec800::write_add_creature(self, msg, c);
    }

    fn add_creature_wire_len(&self, c: &AddCreatureWire) -> usize {
        Codec800::add_creature_wire_len(self, c)
    }

    fn write_outfit(&self, msg: &mut NetworkMessage, o: &OutfitWire) {
        Codec800::write_outfit(self, msg, o);
    }

    fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage {
        Codec800::encode_player_stats(self, s)
    }

    fn encode_player_skills(&self, s: &PlayerSkillsWire) -> NetworkMessage {
        ProtocolCodec::encode_player_skills(&Codec772, s)
    }

    fn encode_basic_data(
        &self,
        is_premium: bool,
        premium_ends_at: u32,
        vocation_client_id: u8,
    ) -> NetworkMessage {
        ProtocolCodec::encode_basic_data(&Codec772, is_premium, premium_ends_at, vocation_client_id)
    }

    fn encode_self_appear_login(&self, player_id: u32, server_beat: u16) -> NetworkMessage {
        ProtocolCodec::encode_self_appear_login(&Codec772, player_id, server_beat)
    }

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
        otclient_stackpos: bool,
    ) -> NetworkMessage {
        Codec800::encode_add_tile_item(self, pos, stack_pos, args, otclient_stackpos)
    }

    fn encode_update_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec800::encode_update_tile_item(self, pos, stack_pos, args)
    }

    fn encode_inventory_item(&self, slot: u8, args: ItemTemplateArgs) -> NetworkMessage {
        Codec800::encode_inventory_item(self, slot, args)
    }

    fn encode_add_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec800::encode_add_container_item(self, cid, slot, args)
    }

    fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec800::encode_update_container_item(self, cid, slot, args)
    }

    fn encode_remove_container_item(&self, cid: u8, slot: u16) -> NetworkMessage {
        ProtocolCodec::encode_remove_container_item(&Codec772, cid, slot)
    }

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &AddCreatureWire,
        otclient_stackpos: bool,
    ) -> NetworkMessage {
        let _ = (stack_pos, otclient_stackpos);
        Codec800::encode_add_tile_creature(self, pos, wire)
    }

    fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage {
        ProtocolCodec::encode_remove_tile_thing(&Codec772, pos, stackpos)
    }

    fn encode_remove_tile_creature_by_id(&self, creature_id: u32) -> NetworkMessage {
        ProtocolCodec::encode_remove_tile_creature_by_id(&Codec772, creature_id)
    }

    fn encode_creature_light(
        &self,
        creature_id: u32,
        level: u8,
        color: u8,
        access_player: bool,
    ) -> NetworkMessage {
        ProtocolCodec::encode_creature_light(&Codec772, creature_id, level, color, access_player)
    }

    fn encode_creature_turn(
        &self,
        creature_id: u32,
        stack_pos: u8,
        tile_pos: Position,
        direction: u8,
        can_walkthrough: bool,
    ) -> NetworkMessage {
        ProtocolCodec::encode_creature_turn(
            &Codec772,
            creature_id,
            stack_pos,
            tile_pos,
            direction,
            can_walkthrough,
        )
    }

    fn encode_cancel_walk(&self, direction: u8) -> NetworkMessage {
        ProtocolCodec::encode_cancel_walk(&Codec772, direction)
    }

    fn encode_clear_target(&self) -> NetworkMessage {
        ProtocolCodec::encode_clear_target(&Codec772)
    }

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage {
        Codec800::encode_container_open(self, c)
    }

    fn encode_animated_text(&self, w: &AnimatedTextWire) -> NetworkMessage {
        ProtocolCodec::encode_animated_text(&Codec772, w)
    }

    fn encode_magic_effect(&self, w: &MagicEffectWire) -> NetworkMessage {
        ProtocolCodec::encode_magic_effect(&Codec772, w)
    }

    fn encode_distance_shoot(&self, w: &DistanceShootWire) -> NetworkMessage {
        ProtocolCodec::encode_distance_shoot(&Codec772, w)
    }

    fn encode_creature_health(&self, w: &CreatureHealthWire) -> NetworkMessage {
        ProtocolCodec::encode_creature_health(&Codec772, w)
    }

    fn encode_creature_square(&self, w: &CreatureSquareWire) -> NetworkMessage {
        ProtocolCodec::encode_creature_square(&Codec772, w)
    }

    fn encode_creature_speed(&self, w: &CreatureSpeedWire) -> NetworkMessage {
        ProtocolCodec::encode_creature_speed(&Codec772, w)
    }

    fn encode_creature_outfit(&self, creature_id: u32, outfit: &OutfitWire) -> NetworkMessage {
        Codec800::encode_creature_outfit(self, creature_id, outfit)
    }

    fn encode_combat_damage_text_message(&self, w: &CombatDamageNotifyWire) -> NetworkMessage {
        ProtocolCodec::encode_combat_damage_text_message(&Codec772, w)
    }

    fn encode_creature_say(&self, statement_id: u32, w: &CreatureSayWire) -> NetworkMessage {
        Codec800::encode_creature_say(self, statement_id, w)
    }

    fn encode_to_channel(&self, statement_id: u32, w: &ToChannelWire) -> NetworkMessage {
        Codec800::encode_to_channel(self, statement_id, w)
    }

    fn encode_private_message(&self, statement_id: u32, w: &PrivateMessageWire) -> NetworkMessage {
        Codec800::encode_private_message(self, statement_id, w)
    }

    fn encode_channel_message(&self, w: &ChannelMessageWire) -> NetworkMessage {
        Codec800::encode_channel_message(self, w)
    }

    fn encode_channels_dialog(&self, w: &ChannelsDialogWire) -> NetworkMessage {
        ProtocolCodec::encode_channels_dialog(&Codec772, w)
    }

    fn encode_channel_open(&self, w: &ChannelOpenWire) -> NetworkMessage {
        ProtocolCodec::encode_channel_open(&Codec772, w)
    }

    fn encode_create_private_channel(&self, w: &CreatePrivateChannelWire) -> NetworkMessage {
        ProtocolCodec::encode_create_private_channel(&Codec772, w)
    }

    fn encode_text_window(&self, w: &TextWindowWire) -> NetworkMessage {
        Codec800::encode_text_window(self, w)
    }

    fn encode_house_window(&self, window_text_id: u32, text: &str) -> NetworkMessage {
        ProtocolCodec::encode_house_window(&Codec772, window_text_id, text)
    }

    fn encode_trade_item_request(
        &self,
        trader_name: &str,
        own_offer: bool,
        items: &[ItemTemplateArgs],
    ) -> NetworkMessage {
        Codec800::encode_trade_item_request(self, trader_name, own_offer, items)
    }

    fn encode_close_trade(&self) -> NetworkMessage {
        ProtocolCodec::encode_close_trade(&Codec772)
    }

    fn encode_vip_entry(
        &self,
        guid: u32,
        name: &str,
        description: &str,
        icon: u32,
        notify: bool,
        status: u8,
    ) -> NetworkMessage {
        ProtocolCodec::encode_vip_entry(&Codec772, guid, name, description, icon, notify, status)
    }

    fn encode_vip_status(&self, guid: u32, status: u8) -> NetworkMessage {
        ProtocolCodec::encode_vip_status(&Codec772, guid, status)
    }

    fn failure_message_type(&self) -> u8 {
        ProtocolCodec::failure_message_type(&Codec772)
    }

    fn status_message_type(&self) -> u8 {
        ProtocolCodec::status_message_type(&Codec772)
    }

    fn periodic_ping_packet(&self, is_otclient: bool) -> NetworkMessage {
        ProtocolCodec::periodic_ping_packet(&Codec772, is_otclient)
    }
}
