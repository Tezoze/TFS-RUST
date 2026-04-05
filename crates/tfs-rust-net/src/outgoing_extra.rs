//! Additional `ProtocolGame::send*` builders (this repo `src/protocolgame.cpp`).
// Pair with `outgoing.rs` — together cover the full server → client game opcode set.

use tfs_rust_common::Position;

use crate::creature_encode::OutfitWire;
use crate::NetworkMessage;

pub use crate::map_description::send_update_tile;

/// `ProtocolGame::sendAddCreature` self branch — `0x17` (`src/protocolgame.cpp` ~2771–2793).
/// Matches OTClient `parseLogin` / `GameNewSpeedLaw` + 1054/1058 + `GameIngameStore` tail.
pub fn send_self_appear_login(player_id: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x17);
    m.write_u32(player_id);
    m.write_u16(0x32); // server beat (50 × 0.1 s), same as TFS
    m.write_double_tfs(857.36, 3);
    m.write_double_tfs(261.29, 3);
    m.write_double_tfs(-4795.01, 3);
    m.write_u8(0); // can report bugs
    m.write_u8(0); // protocol ≥ 1054: PVP framing option
    m.write_u8(0); // protocol ≥ 1058: expert PVP
    // `GameIngameStore`: `getString()` URL + `getU16()` coin pack size (TFS: empty URL, 25)
    m.write_string("");
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

/// `ProtocolGame::AddPlayerStats` opcode `0xA0` (1098 layout).
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::AddPlayerStats` (~3246)
#[derive(Debug, Clone)]
pub struct PlayerStats1098 {
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
    /// `getBaseSpeed() / 2` (C++).
    pub base_speed_half: u16,
    pub regeneration_ticks_sec: u16,
    pub offline_training_time: u16,
}

pub fn send_player_stats_1098(s: &PlayerStats1098) -> NetworkMessage {
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

/// `GameServerPlayerSkills` opcode `0xA1` — layout for **OTClient v8** with `GameAdditionalSkills` (≥1094).
///
/// `parsePlayerSkills`: one loop `skill = 0 .. lastSkill-1` with `lastSkill = 13` when `GameAdditionalSkills` is on.
/// Core skills 0–6 (Fist…Fishing): `u16` level, `u16` base, `u8` percent. Skills 7–12 (critical / leech): `u16`, `u16` only.
///
/// **Deviation from C++** `AddPlayerSkills` in this repo (~3289): TFS sends 7 core + **7** trailing `(u16,u16)` special rows.
/// OTC 1098 expects **6** extra skills **without** percent in the **same** loop — see `docs/OTCLIENT_INFO.md` §2.
// C++ reference (TFS classic shape): `src/protocolgame.cpp` `ProtocolGame::AddPlayerSkills`
pub fn send_player_skills_1098(
    levels: &[u16; 7],
    bases: &[u16; 7],
    percents: &[u8; 7],
    additional_levels: &[u16; 6],
    additional_bases: &[u16; 6],
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA1);
    for i in 0..7 {
        m.write_u16(levels[i]);
        m.write_u16(bases[i]);
        m.write_u8(percents[i]);
    }
    for i in 0..6 {
        m.write_u16(additional_levels[i]);
        m.write_u16(additional_bases[i]);
    }
    m
}

/// `ProtocolGame::sendBasicData` opcode `0x9F` — `0xFF` “known spells” + 255 bytes `0x00..=0xFE`.
// C++ reference: `src/protocolgame.cpp` `ProtocolGame::sendBasicData` (~1564)
pub fn send_basic_data_1098(is_premium: bool, premium_ends_at: u32, vocation_client_id: u8) -> NetworkMessage {
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

/// C++ `ProtocolGame::sendCancelTarget` (`src/protocolgame.cpp` ~2497).
pub fn send_cancel_target() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA3);
    m.write_u32(0);
    m
}

/// C++ `ProtocolGame::sendChangeSpeed` (`src/protocolgame.cpp` ~2505).
pub fn send_change_speed(creature_id: u32, base_speed: u32, speed: u32) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x8F);
    m.write_u32(creature_id);
    m.write_u16((base_speed / 2) as u16);
    m.write_u16((speed / 2) as u16);
    m
}

/// C++ `ProtocolGame::sendCancelWalk` (`src/protocolgame.cpp` ~2515).
pub fn send_cancel_walk(direction: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB5);
    m.write_u8(direction);
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

pub fn send_close_container(cid: u8) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6F);
    m.write_u8(cid);
    m
}

/// `ProtocolGame::sendContainer` — opcode `0x6E` (`src/protocolgame.cpp` ~1751).
/// `write_container_header` must write `addItem` + `addString(name)` (or browse-field bag + name).
/// `write_item` is called once per item in the window (`addItem` each).
pub fn send_container_open(
    cid: u8,
    capacity: u8,
    has_parent: bool,
    unlocked: bool,
    pagination: bool,
    container_size: u16,
    first_index: u16,
    items_to_send: u8,
    write_container_header: impl FnOnce(&mut NetworkMessage),
    mut write_item: impl FnMut(&mut NetworkMessage),
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6E);
    m.write_u8(cid);
    write_container_header(&mut m);
    m.write_u8(capacity);
    m.write_u8(u8::from(has_parent));
    m.write_u8(u8::from(unlocked));
    m.write_u8(u8::from(pagination));
    m.write_u16(container_size);
    m.write_u16(first_index);
    if u32::from(first_index) < u32::from(container_size) {
        m.write_u8(items_to_send);
        for _ in 0..items_to_send {
            write_item(&mut m);
        }
    } else {
        m.write_u8(0);
    }
    m
}

pub fn send_quest_log() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF0);
    m
}

/// C++ `ProtocolGame::sendQuestLine` (`src/protocolgame.cpp` ~2333) — started missions only.
pub fn send_quest_line(
    quest_id: u16,
    missions_count: u8,
    started_missions: &[(String, String)],
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF1);
    m.write_u16(quest_id);
    m.write_u8(missions_count);
    for (name, desc) in started_missions {
        m.write_string(name);
        m.write_string(desc);
    }
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

/// `ProtocolGame::sendMarketEnter` (`src/protocolgame.cpp` ~1894).
pub fn send_market_enter_full(
    bank_balance: u64,
    player_offer_count: u8,
    depot_wares: &[(u16, u16)],
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF6);
    m.write_u64(bank_balance);
    m.write_u8(player_offer_count);
    let n = depot_wares.len().min(u16::MAX as usize) as u16;
    m.write_u16(n);
    for &(ware_id, cnt) in depot_wares.iter().take(n as usize) {
        m.write_u16(ware_id);
        m.write_u16(cnt);
    }
    m
}

/// One row on the NPC shop list (`AddShopItem`, `src/protocolgame.cpp` ~3448).
#[derive(Debug, Clone)]
pub struct ShopItemWire {
    pub client_id: u16,
    pub fluid_subtype: u8,
    pub is_fluid: bool,
    pub real_name: String,
    pub weight: u32,
    pub buy_price: u32,
    pub sell_price: u32,
}

/// `ProtocolGame::sendShop` — opcode `0x7A`.
pub fn send_shop(npc_name: &str, items: &[ShopItemWire]) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x7A);
    m.write_string(npc_name);
    let n = items.len().min(u16::MAX as usize) as u16;
    m.write_u16(n);
    for it in items.iter().take(n as usize) {
        m.write_u16(it.client_id);
        if it.is_fluid {
            m.write_u8(crate::item_encode::server_fluid_to_client(it.fluid_subtype));
        } else {
            m.write_u8(0x00);
        }
        m.write_string(&it.real_name);
        m.write_u32(it.weight);
        m.write_u32(it.buy_price);
        m.write_u32(it.sell_price);
    }
    m
}

/// `ProtocolGame::sendSaleItemList` — opcode `0x7B`. Pairs are `(client_item_id, count)` per `addItemId`.
pub fn send_sale_item_list(coins_total: u64, sale_counts: &[(u16, u8)]) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x7B);
    m.write_u64(coins_total);
    let n = sale_counts.len().min(u8::MAX as usize) as u8;
    m.write_u8(n);
    for &(client_id, cnt) in sale_counts.iter().take(n as usize) {
        m.write_u16(client_id);
        m.write_u8(cnt);
    }
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

/// `ProtocolGame::sendOutfitWindow` — opcode `0xC8` (`src/protocolgame.cpp` ~3022).
pub fn send_outfit_window(
    current: &OutfitWire,
    outfits: &[(u16, &str, u8)],
    mounts: &[(u16, &str)],
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xC8);
    crate::creature_encode::write_outfit(&mut m, current);
    let noc = outfits.len().min(u8::MAX as usize) as u8;
    m.write_u8(noc);
    for &(look_type, name, addons) in outfits.iter().take(noc as usize) {
        m.write_u16(look_type);
        m.write_string(name);
        m.write_u8(addons);
    }
    let nm = mounts.len().min(u8::MAX as usize) as u8;
    m.write_u8(nm);
    for &(mount_cid, name) in mounts.iter().take(nm as usize) {
        m.write_u16(mount_cid);
        m.write_string(name);
    }
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

/// `ProtocolGame::sendTextWindow` with a real item (`src/protocolgame.cpp` ~2966).
pub fn send_text_window_item(
    window_text_id: u32,
    can_write: bool,
    maxlen: u16,
    item_writer: impl FnOnce(&mut NetworkMessage),
    text: &str,
    writer: &str,
    written_date: Option<&str>,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x96);
    m.write_u32(window_text_id);
    item_writer(&mut m);
    if can_write {
        m.write_u16(maxlen);
        m.write_string(text);
    } else {
        m.write_u16(text.len() as u16);
        m.write_string(text);
    }
    if !writer.is_empty() {
        m.write_string(writer);
    } else {
        m.write_u16(0);
    }
    if let Some(d) = written_date {
        m.write_string(d);
    } else {
        m.write_u16(0);
    }
    m
}

/// Second `sendTextWindow` overload — template item id (`src/protocolgame.cpp` ~2999).
pub fn send_text_window_simple_item(
    window_text_id: u32,
    client_id: u16,
    count: u8,
    stackable: bool,
    is_splash_or_fluid: bool,
    is_animation: bool,
    with_description: bool,
    text: &str,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x96);
    m.write_u32(window_text_id);
    crate::item_encode::write_item_template(
        &mut m,
        client_id,
        count,
        stackable,
        is_splash_or_fluid,
        is_animation,
        with_description,
    );
    m.write_u16(text.len() as u16);
    m.write_string(text);
    m.write_u16(0);
    m.write_u16(0);
    m
}

/// One VIP line (`sendVIP`, `src/protocolgame.cpp` ~3097). Login sends one packet per entry.
pub fn send_vip_entry(
    guid: u32,
    name: &str,
    description: &str,
    icon: u32,
    notify: bool,
    status: u8,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xD2);
    m.write_u32(guid);
    m.write_string(name);
    m.write_string(description);
    m.write_u32(icon.min(10));
    m.write_u8(u8::from(notify));
    m.write_u8(status);
    m
}

#[derive(Debug, Clone)]
pub struct MarketOfferWire {
    pub timestamp: u32,
    pub counter: u16,
    pub amount: u16,
    pub price: u32,
    pub player_name: String,
}

/// `sendMarketBrowseItem` — opcode `0xF9` (`src/protocolgame.cpp` ~1971).
pub fn send_market_browse_item_offers(
    item_client_id: u16,
    buy_offers: &[MarketOfferWire],
    sell_offers: &[MarketOfferWire],
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xF9);
    m.write_u16(item_client_id);
    m.write_u32(buy_offers.len() as u32);
    for o in buy_offers {
        m.write_u32(o.timestamp);
        m.write_u16(o.counter);
        m.write_u16(o.amount);
        m.write_u32(o.price);
        m.write_string(&o.player_name);
    }
    m.write_u32(sell_offers.len() as u32);
    for o in sell_offers {
        m.write_u32(o.timestamp);
        m.write_u16(o.counter);
        m.write_u16(o.amount);
        m.write_u32(o.price);
        m.write_string(&o.player_name);
    }
    m
}

/// `ProtocolGame::sendModalWindow` — opcode `0xFA` (`src/protocolgame.cpp` ~3145).
pub fn send_modal_window(
    id: u32,
    title: &str,
    message: &str,
    buttons: &[(&str, u8)],
    choices: &[(&str, u8)],
    default_escape: u8,
    default_enter: u8,
    priority: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xFA);
    m.write_u32(id);
    m.write_string(title);
    m.write_string(message);
    m.write_u8(buttons.len().min(u8::MAX as usize) as u8);
    for (s, bid) in buttons {
        m.write_string(s);
        m.write_u8(*bid);
    }
    m.write_u8(choices.len().min(u8::MAX as usize) as u8);
    for (s, cid) in choices {
        m.write_string(s);
        m.write_u8(*cid);
    }
    m.write_u8(default_escape);
    m.write_u8(default_enter);
    m.write_u8(u8::from(priority));
    m
}

/// C++ `ProtocolGame::sendCreatureSay` (`src/protocolgame.cpp` ~2427).
pub fn send_creature_say(
    statement_id: u32,
    name: &str,
    level: u16,
    speak_type: u8,
    pos: Position,
    text: &str,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAA);
    m.write_u32(statement_id);
    m.write_string(name);
    m.write_u16(level);
    m.write_u8(speak_type);
    m.write_position(&pos);
    m.write_string(text);
    m
}

/// C++ `ProtocolGame::sendToChannel` (`src/protocolgame.cpp` ~2455).
pub fn send_to_channel(
    statement_id: u32,
    speaker_name: Option<&str>,
    level: u16,
    speak_type: u8,
    channel_id: u16,
    text: &str,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAA);
    m.write_u32(statement_id);
    if let Some(n) = speaker_name {
        m.write_string(n);
        m.write_u16(level);
    } else {
        m.write_u32(0);
    }
    m.write_u8(speak_type);
    m.write_u16(channel_id);
    m.write_string(text);
    m
}

/// C++ `ProtocolGame::sendPrivateMessage` (`src/protocolgame.cpp` ~2480).
pub fn send_private_message_speech(
    statement_id: u32,
    speaker_name: Option<&str>,
    level: u16,
    speak_type: u8,
    text: &str,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAA);
    m.write_u32(statement_id);
    if let Some(n) = speaker_name {
        m.write_string(n);
        m.write_u16(level);
    } else {
        m.write_u32(0);
    }
    m.write_u8(speak_type);
    m.write_string(text);
    m
}

/// C++ `ProtocolGame::sendChannelMessage` (`src/protocolgame.cpp` ~1730).
pub fn send_channel_message(author: &str, text: &str, speak: u8, channel: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAA);
    m.write_u32(0);
    m.write_string(author);
    m.write_u16(0);
    m.write_u8(speak);
    m.write_u16(channel);
    m.write_string(text);
    m
}

pub fn send_icons(icons: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xA2);
    m.write_u16(icons);
    m
}

/// C++ `ProtocolGame::sendCreatureTurn` (`src/protocolgame.cpp` ~2404).
pub fn send_creature_turn(
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

/// `GameServerUnjustifiedStats` (`0xB7`) — OTClient `parseUnjustifiedStats`: **7× `u8`** after opcode.
// C++ bundled in this repo (`protocolgame.cpp`) matches; older stub used 8 bytes and drifted — see `docs/OTCLIENT_INFO.md`.
pub fn send_unjustified_stats_stub() -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB7);
    for _ in 0..7 {
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

/// C++ `sendAddTileItem` with template `addItem` (`src/protocolgame.cpp` ~2605).
pub fn send_add_tile_item_template(
    pos: Position,
    stack_pos: u8,
    client_id: u16,
    count: u8,
    stackable: bool,
    is_splash_or_fluid: bool,
    is_animation: bool,
    with_description: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6A);
    m.write_position(&pos);
    m.write_u8(stack_pos);
    crate::item_encode::write_item_template(
        &mut m,
        client_id,
        count,
        stackable,
        is_splash_or_fluid,
        is_animation,
        with_description,
    );
    m
}

/// C++ `sendUpdateTileItem` template path (`src/protocolgame.cpp` ~2619).
pub fn send_update_tile_item_template(
    pos: Position,
    stack_pos: u8,
    client_id: u16,
    count: u8,
    stackable: bool,
    is_splash_or_fluid: bool,
    is_animation: bool,
    with_description: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x6B);
    m.write_position(&pos);
    m.write_u8(stack_pos);
    crate::item_encode::write_item_template(
        &mut m,
        client_id,
        count,
        stackable,
        is_splash_or_fluid,
        is_animation,
        with_description,
    );
    m
}

/// C++ `sendRemoveContainerItem` when `lastItem == nullptr` (`src/protocolgame.cpp` ~2952).
pub fn send_remove_container_item_empty(cid: u8, slot: u16) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x72);
    m.write_u8(cid);
    m.write_u16(slot);
    m.write_u16(0);
    m
}

/// C++ `sendInventoryItem` with item (`src/protocolgame.cpp` ~2896).
pub fn send_inventory_item_template(
    slot: u8,
    client_id: u16,
    count: u8,
    stackable: bool,
    is_splash_or_fluid: bool,
    is_animation: bool,
    with_description: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x78);
    m.write_u8(slot);
    crate::item_encode::write_item_template(
        &mut m,
        client_id,
        count,
        stackable,
        is_splash_or_fluid,
        is_animation,
        with_description,
    );
    m
}

/// `sendInventoryItem` with live `addItem(const Item*)` (`src/networkmessage.cpp` L117+).
pub fn send_inventory_item_live(
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
    crate::item_encode::write_item_live(
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

/// C++ `sendChannel` with no user lists (`src/protocolgame.cpp` ~1702).
pub fn send_channel_open(channel_id: u16, channel_name: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xAC);
    m.write_u16(channel_id);
    m.write_string(channel_name);
    m.write_u16(0);
    m.write_u16(0);
    m
}

/// `sendTextMessage` default branch — type + text only (`src/protocolgame.cpp` ~1583).
pub fn send_text_message_simple(message_type: u8, text: &str) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB4);
    m.write_u8(message_type);
    m.write_string(text);
    m
}

/// Damage-style `sendTextMessage` (`MESSAGE_DAMAGE_DEALT` etc.).
pub fn send_text_message_damage(
    message_type: u8,
    pos: Position,
    primary_value: u32,
    primary_color: u8,
    secondary_value: u32,
    secondary_color: u8,
    text: &str,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0xB4);
    m.write_u8(message_type);
    m.write_position(&pos);
    m.write_u32(primary_value);
    m.write_u8(primary_color);
    m.write_u32(secondary_value);
    m.write_u8(secondary_color);
    m.write_string(text);
    m
}

pub fn send_add_container_item_template(
    cid: u8,
    slot: u16,
    client_id: u16,
    count: u8,
    stackable: bool,
    with_description: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x70);
    m.write_u8(cid);
    m.write_u16(slot);
    crate::item_encode::write_item_template(
        &mut m,
        client_id,
        count,
        stackable,
        false,
        false,
        with_description,
    );
    m
}

pub fn send_update_container_item_template(
    cid: u8,
    slot: u16,
    client_id: u16,
    count: u8,
    stackable: bool,
    with_description: bool,
) -> NetworkMessage {
    let mut m = NetworkMessage::new();
    m.write_u8(0x71);
    m.write_u8(cid);
    m.write_u16(slot);
    crate::item_encode::write_item_template(
        &mut m,
        client_id,
        count,
        stackable,
        false,
        false,
        with_description,
    );
    m
}
