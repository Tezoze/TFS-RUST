//! Additional `ProtocolGame::send*` builders (this repo `src/protocolgame.cpp`).
// Pair with `outgoing.rs` — together cover the full server → client game opcode set.

use tfs_rust_common::Position;

use crate::creature_encode::OutfitWire;
use crate::NetworkMessage;

/// `ProtocolGame::sendAddCreature` self branch — `0x17` + id + beat + speed doubles (`src/protocolgame.cpp`).
pub fn send_self_appear_login(player_id: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x17);
    m.write_u32(player_id);
    m.write_u16(0x32);
    m.write_double_tfs(857.36, 3);
    m.write_double_tfs(261.29, 3);
    m.write_double_tfs(-4795.01, 3);
    m.write_u8(0);
    m.write_u8(0);
    m.write_u8(0);
    m.write_u16(0);
    m.write_u16(25);
    m
}

#[inline]
pub fn send_pending_state_entered() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x0A);
    m
}

#[inline]
pub fn send_enter_world() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x0F);
    m
}

pub fn send_fight_modes(fight: u8, chase: u8, secure: u8, pvp: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA7);
    m.write_u8(fight);
    m.write_u8(chase);
    m.write_u8(secure);
    m.write_u8(pvp);
    m
}

pub fn send_world_light(level: u8, color: u8, access_player: bool) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x82);
    m.write_u8(if access_player { 0xFF } else { level });
    m.write_u8(color);
    m
}

pub fn send_creature_light(
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

pub fn send_open_private_channel(receiver: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAD);
    m.write_string(receiver);
    m
}

pub fn send_channel_event(channel_id: u16, player_name: &str, event: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF3);
    m.write_u16(channel_id);
    m.write_string(player_name);
    m.write_u8(event);
    m
}

pub fn send_creature_outfit(creature_id: u32, outfit: &OutfitWire) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x8E);
    m.write_u32(creature_id);
    crate::creature_encode::write_outfit(&mut m, outfit);
    m
}

pub fn send_creature_walkthrough(creature_id: u32, walkthrough: bool) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x92);
    m.write_u32(creature_id);
    m.write_u8(if walkthrough { 0x00 } else { 0x01 });
    m
}

pub fn send_creature_shield(creature_id: u32, shield: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x91);
    m.write_u32(creature_id);
    m.write_u8(shield);
    m
}

pub fn send_creature_skull(creature_id: u32, skull: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x90);
    m.write_u32(creature_id);
    m.write_u8(skull);
    m
}

pub fn send_creature_type(creature_id: u32, ty: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x95);
    m.write_u32(creature_id);
    m.write_u8(ty);
    m
}

pub fn send_creature_helpers(creature_id: u32, helpers: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x94);
    m.write_u32(creature_id);
    m.write_u16(helpers);
    m
}

pub fn send_creature_square(creature_id: u32, color: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x93);
    m.write_u32(creature_id);
    m.write_u8(0x01);
    m.write_u8(color);
    m
}

pub fn send_tutorial(tutorial_id: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xDC);
    m.write_u8(tutorial_id);
    m
}

pub fn send_add_marker(pos: Position, mark_type: u8, desc: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xDD);
    m.write_position(&pos);
    m.write_u8(mark_type);
    m.write_string(desc);
    m
}

pub fn send_relogin_window(unfair_fight_reduction: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x28);
    m.write_u8(0x00);
    m.write_u8(unfair_fight_reduction);
    m
}

pub fn send_close_private(channel_id: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB3);
    m.write_u16(channel_id);
    m
}

pub fn send_create_private_channel(channel_id: u16, name: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB2);
    m.write_u16(channel_id);
    m.write_string(name);
    m.write_u16(0x01);
    m.write_string("self");
    m.write_u16(0x00);
    m
}

pub fn send_channels_dialog_count() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAB);
    m.write_u8(0);
    m
}

pub fn send_cancel_target() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA3);
    m
}

pub fn send_change_speed(creature_id: u32, speed: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x8F);
    m.write_u32(creature_id);
    m.write_u32(speed);
    m
}

pub fn send_cancel_walk() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB5);
    m.write_u8(0); // direction placeholder
    m
}

pub fn send_distance_shoot(from: Position, to: Position, shoot_type: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x85);
    m.write_position(&from);
    m.write_position(&to);
    m.write_u8(shoot_type);
    m
}

pub fn send_fyi_box(message: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x15);
    m.write_string(message);
    m
}

pub fn send_remove_tile_thing(pos: Position, stackpos: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6C);
    m.write_position(&pos);
    m.write_u8(stackpos);
    m
}

pub fn send_remove_tile_creature_by_id(creature_id: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6C);
    m.write_u16(0xFFFF);
    m.write_u32(creature_id);
    m
}

pub fn send_update_tile_end(pos: Position, empty: bool) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x69);
    m.write_position(&pos);
    if empty {
        m.write_u8(0x01);
        m.write_u8(0xFF);
    } else {
        m.write_u8(0x00);
        m.write_u8(0xFF);
    }
    m
}

pub fn send_close_container(cid: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6F);
    m.write_u8(cid);
    m
}

pub fn send_quest_log() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF0);
    m
}

pub fn send_quest_line(quest_id: u16, name: &str, completed: bool) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF1);
    m.write_u16(quest_id);
    m.write_u8(u8::from(completed));
    m.write_string(name);
    m
}

pub fn send_close_trade() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x7F);
    m
}

pub fn send_market_leave() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF7);
    m
}

pub fn send_spell_cooldown(spell_id: u8, time_ms: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA4);
    m.write_u8(spell_id);
    m.write_u32(time_ms);
    m
}

pub fn send_spell_group_cooldown(group: u8, time_ms: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA5);
    m.write_u8(group);
    m.write_u32(time_ms);
    m
}

pub fn send_vip_status(guid: u32, status: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xD3);
    m.write_u32(guid);
    m.write_u8(status);
    m
}

pub fn send_outfit_window_open() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xC8);
    m
}

pub fn send_house_window(window_text_id: u32, text: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x97);
    m.write_u32(window_text_id);
    m.write_string(text);
    m
}

pub fn send_supply_used(client_id: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xCE);
    m.write_u16(client_id);
    m
}

pub fn send_kill_tracker_header(monster_name: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xD1);
    m.write_string(monster_name);
    m
}

pub fn send_market_enter(depot_id: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF6);
    m.write_u32(depot_id);
    m
}

pub fn send_channel_message(author: &str, text: &str, speak: u8, channel: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAA);
    m.write_u32(0); // creature id 0 for anonymous
    m.write_u8(speak);
    m.write_u16(channel);
    m.write_string(author);
    m.write_string(text);
    m
}

pub fn send_icons(icons: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA2);
    m.write_u16(icons);
    m
}

pub fn send_creature_turn(creature_id: u32, stack_pos: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6B);
    m.write_u32(creature_id);
    m.write_u32(stack_pos);
    m
}

pub fn send_trade_request(trader: &str, ack: bool) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x7D);
    m.write_string(trader);
    m.write_u8(u8::from(ack));
    m
}

pub fn send_wait_list() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x65);
    m
}

pub fn send_floor_change_up() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xBE);
    m
}

pub fn send_unjustified_stats_stub() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB7);
    for _ in 0..8 {
        m.write_u8(0);
    }
    m
}

pub fn send_otc_features_raw() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x43);
    m.write_u16(1);
    m.write_u8(68);
    m.write_u8(1);
    m
}

pub fn send_basic_data_stub() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x9F);
    m.write_u8(0);
    m.write_u32(0);
    m.write_u8(0);
    m.write_u16(0);
    m
}

pub fn send_player_data_appear_stub(player_id: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x17);
    m.write_u32(player_id);
    m.write_u16(0x32);
    m
}

pub fn send_modal_window_stub(window_id: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xFA);
    m.write_u32(window_id);
    m.write_u32(0);
    m.write_u8(0);
    m
}

pub fn send_vip_entries_empty() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xD2);
    m.write_u16(0);
    m
}

pub fn send_market_detail_stub(item_id: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF8);
    m.write_u16(item_id);
    m
}

pub fn send_loot_stats_stub(creature_name: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xCF);
    m.write_u16(100);
    m.write_u8(0xFF);
    m.write_u8(0);
    m.write_string(creature_name);
    m
}

pub fn send_combat_analyzer_stub() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB6);
    m.write_u8(0);
    m.write_u32(0);
    m.write_u8(0);
    m.write_string("");
    m
}

/// Empty inventory slot (`sendInventoryItem` with `item == nullptr`).
pub fn send_inventory_slot_empty(slot: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x79);
    m.write_u8(slot);
    m
}

/// Minimal `sendItems` body (11 sentinel slots only).
pub fn send_items_inventory_stub() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF5);
    m.write_u16(11);
    for i in 1u16..=11 {
        m.write_u16(i);
        m.write_u8(0);
        m.write_u16(1);
    }
    m
}

pub fn send_add_container_item_template(
    cid: u8,
    slot: u16,
    client_id: u16,
    count: u8,
    stackable: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x70);
    m.write_u8(cid);
    m.write_u16(slot);
    crate::item_encode::write_item_template(&mut m, client_id, count, stackable);
    m
}

pub fn send_update_container_item_template(
    cid: u8,
    slot: u16,
    client_id: u16,
    count: u8,
    stackable: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x71);
    m.write_u8(cid);
    m.write_u16(slot);
    crate::item_encode::write_item_template(&mut m, client_id, count, stackable);
    m
}
