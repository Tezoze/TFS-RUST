//! Minimal 772 inbound: self id, own pos, move/remove, combat effects, ping.
//!
//! C++ reference: `codec/v772.rs` self-appear `0x0A`; `map_description.rs` `0x64` header
//! and `0x6D` move; `v772.rs` `0x6C` remove (no named server consts). Combat counters
//! (`0x83`/`0x84`/`0x85`/`0x8C`) feed the Tier 4 content-equivalence gate.
//!
//! Map bodies (`0x64` / `0x65`–`0x68` / `0xBE`/`0xBF`) are skipped with the client
//! skip-counter so a later `0x83` in the same decrypted payload is counted. Login
//! trailers (`0x78`/`0x79`/`0xA0`/`0xA1`/`0x82`/`0x8D`/`0xA2`/`0xB4`) are
//! length-skipped for the same reason. `0xB5` cancel-walk is parsed (direction
//! byte) and emitted so latency can retire a rejected walk. TVP also emits
//! `0x86` square, `0x8E` outfit, `0x8F` speed, `0x90`/`0x91` skull/shield,
//! `0xA3` cancel-target, and `0xAA` creature-say in the same payload as later
//! `0x6D`/`0x83` — those must be length-skipped too. Fight modes `0xA7` (three
//! body bytes, TVP `sendFightModes`) and VIP `0xD3`/`0xD4` (`u32` guid) are
//! skipped the same way. Unknown opcodes still discard the rest and are
//! counted (`unknown_opcodes` / `unknown_opcode_first`).

use std::collections::HashSet;
use std::sync::Arc;

use tfs_rust_common::Position;
use tfs_rust_common::protocol_constants::{client_viewport_height, client_viewport_width};
use tfs_rust_net::map_skip::{
    skip_772_map_description_body, skip_772_move_down_floor_body, skip_772_move_up_floor_body,
    skip_772_thing, skip_772_tile_description,
};

use crate::item_extra::ItemExtraBits;

/// Server → client self-appear (772).
const OP_SELF_APPEAR: u8 = 0x0A;
/// Server → client map description (not login char-list `0x64`).
const OP_MAP_DESCRIPTION: u8 = 0x64;
/// Server → client remove tile thing (raw; no `protocol_opcodes::server` const).
const OP_REMOVE: u8 = 0x6C;
/// Server → client creature move (raw).
const OP_MOVE: u8 = 0x6D;
/// `MAGIC_EFFECT`.
const OP_MAGIC_EFFECT: u8 = 0x83;
/// `ANIMATED_TEXT` — `v772.rs` `encode_animated_text` (pos + color + string).
const OP_ANIMATED_TEXT: u8 = 0x84;
/// `DISTANCE_SHOOT` — `v772.rs` `encode_distance_shoot`.
const OP_DISTANCE_SHOOT: u8 = 0x85;
/// `CREATURE_HEALTH` — `v772.rs` `encode_creature_health`.
const OP_CREATURE_HEALTH: u8 = 0x8C;
const OP_PING: u8 = 0x1D;
const OP_PING_BACK: u8 = 0x1E;
const OP_MAP_NORTH: u8 = 0x65;
const OP_MAP_EAST: u8 = 0x66;
const OP_MAP_SOUTH: u8 = 0x67;
const OP_MAP_WEST: u8 = 0x68;
const OP_UPDATE_TILE: u8 = 0x69;
const OP_ADD_TILE_THING: u8 = 0x6A;
const OP_UPDATE_TILE_THING: u8 = 0x6B;
const OP_FLOOR_UP: u8 = 0xBE;
const OP_FLOOR_DOWN: u8 = 0xBF;
const OP_INVENTORY_ITEM: u8 = 0x78;
const OP_INVENTORY_EMPTY: u8 = 0x79;
const OP_WORLD_LIGHT: u8 = 0x82;
const OP_CREATURE_LIGHT: u8 = 0x8D;
const OP_PLAYER_STATS: u8 = 0xA0;
const OP_PLAYER_SKILLS: u8 = 0xA1;
const OP_PLAYER_ICONS: u8 = 0xA2;
const OP_TEXT_MESSAGE: u8 = 0xB4;
const OP_CANCEL_WALK: u8 = 0xB5;
const OP_VIP_ENTRY: u8 = 0xD2;
/// TVP `sendCreatureSquare` (`protocolgame.cpp` ~1180): creature id + color.
const OP_CREATURE_SQUARE: u8 = 0x86;
/// TVP `sendCreatureOutfit` (`protocolgame.cpp` ~1119): id + `AddOutfit`.
const OP_CREATURE_OUTFIT: u8 = 0x8E;
/// TVP `sendChangeSpeed` (`protocolgame.cpp` ~1492): id + u16 speed.
const OP_CHANGE_SPEED: u8 = 0x8F;
/// TVP `sendCreatureSkull` (`protocolgame.cpp` ~1163): id + skull byte.
const OP_CREATURE_SKULL: u8 = 0x90;
/// TVP `sendCreatureShield` (`protocolgame.cpp` ~1150): id + shield byte.
const OP_CREATURE_SHIELD: u8 = 0x91;
/// TVP `sendCancelTarget` (`protocolgame.cpp` ~1485): no payload.
const OP_CANCEL_TARGET: u8 = 0xA3;
/// TVP `sendCreatureSay` / `sendToChannel` / `sendPrivateMessage` (`0xAA`).
const OP_CREATURE_SAY: u8 = 0xAA;
/// TVP `sendFightModes` (`protocolgame.cpp` ~1684): fight + chase + secure.
/// 772 body is 3 bytes; do not skip Rust's extra pvp byte (own frame).
const OP_FIGHT_MODES: u8 = 0xA7;
/// TVP `sendUpdatedVIPStatus` online (`protocolgame.cpp` ~2010): `u32` guid.
const OP_VIP_STATUS: u8 = 0xD3;
/// TVP `sendUpdatedVIPStatus` logout (`protocolgame.cpp` ~2016): `u32` guid.
const OP_VIP_LOGOUT: u8 = 0xD4;
/// 772 fight-mode body after `0xA7` (not the 1098/Rust extra pvp byte).
const FIGHT_MODES_772_LEN: usize = 3;
/// `TALKTYPE_RVR_CHANNEL` — `sendToChannel` writes `u32` time, not `u16` id.
const SPEAK_RVR_CHANNEL: u8 = 6;
/// 772 `AddPlayerStats` after opcode (`v772.rs` `encode_player_stats`).
const PLAYER_STATS_LEN: usize = 20;
/// 7 skills × (level + percent).
const PLAYER_SKILLS_LEN: usize = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboundEvent {
    WalkAck,
    /// Server rejected the current walk (`0xB5`); not a latency sample.
    CancelWalk,
    MagicEffect {
        pos: Position,
    },
    Ping(u8),
    OtherCreature {
        id: u32,
        pos: Position,
    },
}

#[derive(Debug, Clone)]
pub struct InboundState {
    pub self_id: Option<u32>,
    pub pos: Option<Position>,
    pub last_other_creature_id: Option<u32>,
    pub last_other_creature_pos: Option<Position>,
    pub bytes_in: u64,
    pub bytes_discarded: u64,
    pub magic_effects: u64,
    pub animated_texts: u64,
    pub damage_sum: u64,
    pub damage_samples: u64,
    pub distance_shoots: u64,
    pub creature_health: u64,
    pub other_creature_moves: u64,
    pub seen_creatures: HashSet<u32>,
    item_extra: Arc<ItemExtraBits>,
    pub skip_failures: u64,
    pub unknown_opcodes: u64,
    /// First unknown server opcode byte this session, if any.
    pub unknown_opcode_first: Option<u8>,
}

impl Default for InboundState {
    fn default() -> Self {
        Self::new(Arc::new(ItemExtraBits::default()))
    }
}

impl InboundState {
    pub fn new(item_extra: Arc<ItemExtraBits>) -> Self {
        Self {
            self_id: None,
            pos: None,
            last_other_creature_id: None,
            last_other_creature_pos: None,
            bytes_in: 0,
            bytes_discarded: 0,
            magic_effects: 0,
            animated_texts: 0,
            damage_sum: 0,
            damage_samples: 0,
            distance_shoots: 0,
            creature_health: 0,
            other_creature_moves: 0,
            seen_creatures: HashSet::new(),
            item_extra,
            skip_failures: 0,
            unknown_opcodes: 0,
            unknown_opcode_first: None,
        }
    }

    pub fn unique_creatures(&self) -> u64 {
        self.seen_creatures.len() as u64
    }

    pub fn add_counters(&mut self, other: &InboundState) {
        self.bytes_in += other.bytes_in;
        self.bytes_discarded += other.bytes_discarded;
        self.magic_effects += other.magic_effects;
        self.animated_texts += other.animated_texts;
        self.damage_sum += other.damage_sum;
        self.damage_samples += other.damage_samples;
        self.distance_shoots += other.distance_shoots;
        self.creature_health += other.creature_health;
        self.other_creature_moves += other.other_creature_moves;
        self.skip_failures += other.skip_failures;
        self.unknown_opcodes += other.unknown_opcodes;
        if self.unknown_opcode_first.is_none() {
            self.unknown_opcode_first = other.unknown_opcode_first;
        }
        self.seen_creatures.extend(&other.seen_creatures);
    }

    fn note_unknown(&mut self, op: u8) {
        self.unknown_opcodes += 1;
        if self.unknown_opcode_first.is_none() {
            self.unknown_opcode_first = Some(op);
        }
    }

    fn note_creature(&mut self, id: u32, pos: Option<Position>) {
        if self.self_id == Some(id) {
            return;
        }
        self.seen_creatures.insert(id);
        self.last_other_creature_id = Some(id);
        if let Some(p) = pos {
            self.last_other_creature_pos = Some(p);
        }
    }

    /// Client look id for a server item id (`items.otb`), or `None` if unknown.
    pub fn client_id_for(&self, server_id: u16) -> Option<u16> {
        self.item_extra.client_id(server_id)
    }

    /// Parse one decrypted inner payload. Returns events in order.
    pub fn feed(&mut self, payload: &[u8]) -> Vec<InboundEvent> {
        self.bytes_in += payload.len() as u64;
        let mut events = Vec::new();
        let mut i = 0usize;
        while i < payload.len() {
            let op = payload[i];
            i += 1;
            match op {
                OP_SELF_APPEAR => {
                    if payload.len().saturating_sub(i) < 7 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    let id = u32::from_le_bytes(payload[i..i + 4].try_into().unwrap_or([0; 4]));
                    i += 4;
                    i += 2; // server beat
                    i += 1; // canReportBugs
                    self.self_id = Some(id);
                }
                OP_MAP_DESCRIPTION => {
                    if payload.len().saturating_sub(i) < 5 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    self.pos = Some(read_pos(payload, i));
                    i += 5;
                    if !self.skip_map_body(
                        payload,
                        &mut i,
                        client_viewport_width(),
                        client_viewport_height(),
                    ) {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_MAP_NORTH | OP_MAP_SOUTH => {
                    if !self.skip_map_body(payload, &mut i, client_viewport_width(), 1) {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_MAP_EAST | OP_MAP_WEST => {
                    if !self.skip_map_body(payload, &mut i, 1, client_viewport_height()) {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_FLOOR_UP => {
                    if !self.skip_floor_up(payload, &mut i) {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_FLOOR_DOWN => {
                    if !self.skip_floor_down(payload, &mut i) {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_UPDATE_TILE => {
                    if payload.len().saturating_sub(i) < 5 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    i += 5;
                    if skip_772_tile_description(payload, &mut i, |id| self.item_extra.has(id))
                        .is_none()
                    {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_ADD_TILE_THING => {
                    if payload.len().saturating_sub(i) < 5 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    i += 5;
                    if !skip_772_thing(payload, &mut i, |id| self.item_extra.has(id)) {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_UPDATE_TILE_THING => {
                    // 772 `0x6B` is update-item (pos+stackpos+item) or creature-turn (pos+stackpos+0x63…).
                    if payload.len().saturating_sub(i) < 6 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    i += 6;
                    if !skip_772_thing(payload, &mut i, |id| self.item_extra.has(id)) {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_MOVE => match parse_move(payload, &mut i) {
                    None => {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    Some((id, old_pos, new_pos)) => {
                        if self.is_self_move(id, old_pos) {
                            self.pos = Some(new_pos);
                            events.push(InboundEvent::WalkAck);
                        } else if let Some(cid) = id {
                            self.note_creature(cid, Some(new_pos));
                            self.other_creature_moves += 1;
                            events.push(InboundEvent::OtherCreature {
                                id: cid,
                                pos: new_pos,
                            });
                        }
                    }
                },
                OP_REMOVE => {
                    if parse_remove(payload, &mut i).is_none() {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_MAGIC_EFFECT => {
                    if payload.len().saturating_sub(i) < 6 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    let pos = read_pos(payload, i);
                    i += 5;
                    i += 1; // effect id
                    self.magic_effects += 1;
                    events.push(InboundEvent::MagicEffect { pos });
                }
                OP_ANIMATED_TEXT => match parse_animated_text(payload, &mut i) {
                    None => {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    Some(text) => {
                        self.animated_texts += 1;
                        if let Some(n) = parse_damage_text(&text) {
                            self.damage_sum += n;
                            self.damage_samples += 1;
                        }
                    }
                },
                OP_DISTANCE_SHOOT => {
                    if payload.len().saturating_sub(i) < 11 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    i += 11;
                    self.distance_shoots += 1;
                }
                OP_CREATURE_HEALTH => {
                    if payload.len().saturating_sub(i) < 5 {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    let id = u32::from_le_bytes(payload[i..i + 4].try_into().unwrap_or([0; 4]));
                    i += 5; // id + health percent
                    self.creature_health += 1;
                    self.note_creature(id, None);
                }
                OP_PING | OP_PING_BACK => {
                    events.push(InboundEvent::Ping(op));
                }
                OP_INVENTORY_EMPTY => {
                    if !take(payload, &mut i, 1) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_INVENTORY_ITEM => {
                    if !take(payload, &mut i, 1)
                        || !skip_772_thing(payload, &mut i, |id| self.item_extra.has(id))
                    {
                        self.skip_failed(payload, i);
                        break;
                    }
                }
                OP_PLAYER_STATS => {
                    if !take(payload, &mut i, PLAYER_STATS_LEN) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_PLAYER_SKILLS => {
                    if !take(payload, &mut i, PLAYER_SKILLS_LEN) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_WORLD_LIGHT => {
                    if !take(payload, &mut i, 2) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_CREATURE_LIGHT => {
                    if !take(payload, &mut i, 6) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_PLAYER_ICONS => {
                    if !take(payload, &mut i, 1) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_VIP_ENTRY => {
                    if !take(payload, &mut i, 4)
                        || !skip_len_string(payload, &mut i)
                        || !take(payload, &mut i, 1)
                    {
                        self.discard_rest(payload, i);
                        break;
                    }
                }
                OP_TEXT_MESSAGE => {
                    // TVP `sendTextMessage` (`protocolgame.cpp` ~1246): type + string.
                    if !take(payload, &mut i, 1) || !skip_len_string(payload, &mut i) {
                        self.discard_rest(payload, i);
                        break;
                    }
                }
                OP_CANCEL_WALK => {
                    // TVP `sendCancelWalk` (`protocolgame.cpp` ~1501): direction byte.
                    if !take(payload, &mut i, 1) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    events.push(InboundEvent::CancelWalk);
                }
                OP_CREATURE_SQUARE => {
                    if !take(payload, &mut i, 5) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_CHANGE_SPEED => {
                    if !take(payload, &mut i, 6) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_CREATURE_SKULL | OP_CREATURE_SHIELD => {
                    if !take(payload, &mut i, 5) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_CANCEL_TARGET => {}
                OP_CREATURE_OUTFIT => {
                    if !take(payload, &mut i, 4) || !skip_772_outfit(payload, &mut i) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_CREATURE_SAY => {
                    if !skip_creature_say(payload, &mut i) {
                        self.discard_rest(payload, i);
                        break;
                    }
                }
                OP_FIGHT_MODES => {
                    if !take(payload, &mut i, FIGHT_MODES_772_LEN) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_VIP_STATUS | OP_VIP_LOGOUT => {
                    if !take(payload, &mut i, 4) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                _ => {
                    self.note_unknown(op);
                    self.discard_rest(payload, i.saturating_sub(1));
                    break;
                }
            }
        }
        events
    }

    fn player_z(&self) -> u8 {
        self.pos.map(|p| p.z).unwrap_or(7)
    }

    fn skip_map_body(&self, payload: &[u8], i: &mut usize, width: i32, height: i32) -> bool {
        skip_772_map_description_body(payload, i, self.player_z(), width, height, |id| {
            self.item_extra.has(id)
        })
    }

    fn skip_floor_up(&self, payload: &[u8], i: &mut usize) -> bool {
        skip_772_move_up_floor_body(payload, i, self.player_z(), |id| self.item_extra.has(id))
    }

    fn skip_floor_down(&self, payload: &[u8], i: &mut usize) -> bool {
        skip_772_move_down_floor_body(payload, i, self.player_z(), |id| self.item_extra.has(id))
    }

    fn is_self_move(&self, id: Option<u32>, old_pos: Option<Position>) -> bool {
        if let (Some(sid), Some(cid)) = (self.self_id, id) {
            return sid == cid;
        }
        match (self.pos, old_pos) {
            (Some(here), Some(old)) => here == old,
            _ => false,
        }
    }

    fn skip_failed(&mut self, payload: &[u8], start: usize) {
        self.skip_failures += 1;
        self.discard_rest(payload, start);
    }

    fn discard_rest(&mut self, payload: &[u8], start: usize) {
        self.bytes_discarded += payload.len().saturating_sub(start) as u64;
    }
}

fn take(buf: &[u8], i: &mut usize, n: usize) -> bool {
    if buf.len().saturating_sub(*i) < n {
        return false;
    }
    *i += n;
    true
}

fn skip_len_string(buf: &[u8], i: &mut usize) -> bool {
    if buf.len().saturating_sub(*i) < 2 {
        return false;
    }
    let len = u16::from_le_bytes([buf[*i], buf[*i + 1]]) as usize;
    *i += 2;
    take(buf, i, len)
}

/// 772 `AddOutfit` (`protocolgame.cpp` ~2128): `u16` lookType, then 4 color bytes or item id.
fn skip_772_outfit(buf: &[u8], i: &mut usize) -> bool {
    if buf.len().saturating_sub(*i) < 2 {
        return false;
    }
    let look_type = u16::from_le_bytes([buf[*i], buf[*i + 1]]);
    *i += 2;
    if look_type != 0 {
        take(buf, i, 4)
    } else {
        take(buf, i, 2)
    }
}

/// 772 `0xAA` (`protocolgame.cpp` `sendCreatureSay` ~1422 / `sendToChannel` ~1442 /
/// `sendPrivateMessage` ~1465). No author-level field.
fn skip_creature_say(buf: &[u8], i: &mut usize) -> bool {
    if !take(buf, i, 4) {
        return false;
    }
    // Speaker: `u32 0` (anonymous channel), `u16 0` (anonymous private), or a string.
    if buf.len().saturating_sub(*i) >= 4
        && u32::from_le_bytes([buf[*i], buf[*i + 1], buf[*i + 2], buf[*i + 3]]) == 0
    {
        *i += 4;
    } else if buf.len().saturating_sub(*i) >= 2 && u16::from_le_bytes([buf[*i], buf[*i + 1]]) == 0 {
        *i += 2;
    } else if !skip_len_string(buf, i) {
        return false;
    }
    if !take(buf, i, 1) {
        return false;
    }
    let speak = buf[*i - 1];
    let extra_ok = match speak {
        1 | 2 | 3 | 0x10 | 0x11 => take(buf, i, 5),
        5 | 10 | 12 | 14 => take(buf, i, 2),
        SPEAK_RVR_CHANNEL => take(buf, i, 4),
        _ => true,
    };
    extra_ok && skip_len_string(buf, i)
}

fn read_pos(buf: &[u8], i: usize) -> Position {
    let x = u16::from_le_bytes([buf[i], buf[i + 1]]);
    let y = u16::from_le_bytes([buf[i + 2], buf[i + 3]]);
    Position::new(x, y, buf[i + 4])
}

/// `0x6D`: stackpos&lt;10 → old pos + stack; else `u16 0xFFFF` + creature id; then new pos.
fn parse_move(buf: &[u8], i: &mut usize) -> Option<(Option<u32>, Option<Position>, Position)> {
    if buf.len().saturating_sub(*i) < 2 {
        return None;
    }
    let tag = u16::from_le_bytes([buf[*i], buf[*i + 1]]);
    let (id, old_pos) = if tag == 0xFFFF {
        if buf.len().saturating_sub(*i) < 6 {
            return None;
        }
        *i += 2;
        let id = u32::from_le_bytes(buf[*i..*i + 4].try_into().ok()?);
        *i += 4;
        (Some(id), None)
    } else {
        if buf.len().saturating_sub(*i) < 6 {
            return None;
        }
        let old = read_pos(buf, *i);
        *i += 5;
        *i += 1; // stackpos
        (None, Some(old))
    };
    if buf.len().saturating_sub(*i) < 5 {
        return None;
    }
    let new_pos = read_pos(buf, *i);
    *i += 5;
    Some((id, old_pos, new_pos))
}

/// `0x84`: pos + color + u16-prefixed string (`v772.rs` `encode_animated_text`).
fn parse_animated_text(buf: &[u8], i: &mut usize) -> Option<String> {
    if buf.len().saturating_sub(*i) < 8 {
        return None;
    }
    *i += 5; // pos
    *i += 1; // color
    let len = u16::from_le_bytes([buf[*i], buf[*i + 1]]) as usize;
    *i += 2;
    if buf.len().saturating_sub(*i) < len {
        return None;
    }
    let text = String::from_utf8_lossy(&buf[*i..*i + len]).into_owned();
    *i += len;
    Some(text)
}

fn parse_damage_text(text: &str) -> Option<u64> {
    let t = text.trim();
    let digits = t
        .strip_prefix('+')
        .or_else(|| t.strip_prefix('-'))
        .unwrap_or(t);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn parse_remove(buf: &[u8], i: &mut usize) -> Option<()> {
    // Both 772 forms are 6 bytes: pos+stackpos, or `0xFFFF` + creature id.
    if buf.len().saturating_sub(*i) < 6 {
        return None;
    }
    *i += 6;
    Some(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tfs_rust_common::ProtocolVersion;
    use tfs_rust_net::map_description::{
        TileContent, send_map_description_packet, write_map_description_body,
    };
    use tfs_rust_net::{Codec, NetworkMessage};

    use super::*;

    fn codec_772() -> Codec {
        Codec::from_version(ProtocolVersion::V772).expect("772 codec")
    }

    fn magic_effect_bytes(pos: Position, effect: u8) -> Vec<u8> {
        let mut p = vec![OP_MAGIC_EFFECT];
        p.extend_from_slice(&pos.x.to_le_bytes());
        p.extend_from_slice(&pos.y.to_le_bytes());
        p.push(pos.z);
        p.push(effect);
        p
    }

    #[test]
    fn self_appear_then_map_body_then_magic_effect() {
        let mut s = InboundState::default();
        let player = Position::new(32369, 32241, 7);
        let mut p = vec![OP_SELF_APPEAR];
        p.extend_from_slice(&42u32.to_le_bytes());
        p.extend_from_slice(&50u16.to_le_bytes());
        p.push(0);

        let mut known = HashSet::new();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let map = send_map_description_packet(
            &codec_772(),
            player,
            player,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        );
        p.extend_from_slice(map.as_bytes());
        p.extend_from_slice(&magic_effect_bytes(player, 11));

        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::MagicEffect { pos: player }]);
        assert_eq!(s.self_id, Some(42));
        assert_eq!(s.pos, Some(player));
        assert_eq!(s.magic_effects, 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn north_strip_then_magic_effect() {
        let mut s = InboundState {
            pos: Some(Position::new(100, 200, 7)),
            ..InboundState::default()
        };
        let mut known = HashSet::new();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut body = NetworkMessage::new();
        write_map_description_body(
            &codec_772(),
            &mut body,
            100,
            200,
            7,
            client_viewport_width(),
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        );
        let mut p = vec![OP_MAP_NORTH];
        p.extend_from_slice(body.as_bytes());
        p.extend_from_slice(&magic_effect_bytes(Position::new(100, 199, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn walk_ack_stack_form_matches_self_pos() {
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(Position::new(10, 10, 7)),
            ..InboundState::default()
        };
        let mut p = vec![OP_MOVE];
        p.extend_from_slice(&10u16.to_le_bytes());
        p.extend_from_slice(&10u16.to_le_bytes());
        p.push(7);
        p.push(1); // stackpos
        p.extend_from_slice(&10u16.to_le_bytes());
        p.extend_from_slice(&9u16.to_le_bytes());
        p.push(7);
        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::WalkAck]);
        assert_eq!(s.pos, Some(Position::new(10, 9, 7)));
    }

    #[test]
    fn walk_ack_ffff_self_id() {
        let mut s = InboundState {
            self_id: Some(99),
            pos: Some(Position::new(1, 1, 7)),
            ..InboundState::default()
        };
        let mut p = vec![OP_MOVE];
        p.extend_from_slice(&0xFFFFu16.to_le_bytes());
        p.extend_from_slice(&99u32.to_le_bytes());
        p.extend_from_slice(&2u16.to_le_bytes());
        p.extend_from_slice(&1u16.to_le_bytes());
        p.push(7);
        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::WalkAck]);
        assert_eq!(s.pos, Some(Position::new(2, 1, 7)));
    }

    #[test]
    fn other_creature_ffff_recorded() {
        let mut s = InboundState {
            self_id: Some(1),
            ..InboundState::default()
        };
        let mut p = vec![OP_MOVE];
        p.extend_from_slice(&0xFFFFu16.to_le_bytes());
        p.extend_from_slice(&77u32.to_le_bytes());
        p.extend_from_slice(&5u16.to_le_bytes());
        p.extend_from_slice(&5u16.to_le_bytes());
        p.push(7);
        let ev = s.feed(&p);
        assert_eq!(
            ev,
            vec![InboundEvent::OtherCreature {
                id: 77,
                pos: Position::new(5, 5, 7)
            }]
        );
        assert_eq!(s.last_other_creature_id, Some(77));
        assert_eq!(s.last_other_creature_pos, Some(Position::new(5, 5, 7)));
        assert_eq!(s.other_creature_moves, 1);
        assert_eq!(s.unique_creatures(), 1);
    }

    #[test]
    fn magic_effect_and_ping() {
        let mut s = InboundState::default();
        let mut p = vec![OP_MAGIC_EFFECT];
        p.extend_from_slice(&1u16.to_le_bytes());
        p.extend_from_slice(&2u16.to_le_bytes());
        p.push(7);
        p.push(11);
        p.push(OP_PING_BACK);
        let ev = s.feed(&p);
        assert_eq!(
            ev,
            vec![
                InboundEvent::MagicEffect {
                    pos: Position::new(1, 2, 7)
                },
                InboundEvent::Ping(OP_PING_BACK),
            ]
        );
        assert_eq!(s.magic_effects, 1);
    }

    #[test]
    fn combat_bundle_counts_damage_and_health() {
        let mut s = InboundState {
            self_id: Some(1),
            ..InboundState::default()
        };
        let mut p = vec![OP_MAGIC_EFFECT];
        p.extend_from_slice(&10u16.to_le_bytes());
        p.extend_from_slice(&10u16.to_le_bytes());
        p.push(7);
        p.push(2);
        p.push(OP_ANIMATED_TEXT);
        p.extend_from_slice(&10u16.to_le_bytes());
        p.extend_from_slice(&10u16.to_le_bytes());
        p.push(7);
        p.push(180);
        p.extend_from_slice(&2u16.to_le_bytes());
        p.extend_from_slice(b"42");
        p.push(OP_DISTANCE_SHOOT);
        p.extend_from_slice(&[0u8; 11]);
        p.push(OP_CREATURE_HEALTH);
        p.extend_from_slice(&77u32.to_le_bytes());
        p.push(50);
        let ev = s.feed(&p);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(s.animated_texts, 1);
        assert_eq!(s.damage_sum, 42);
        assert_eq!(s.damage_samples, 1);
        assert_eq!(s.distance_shoots, 1);
        assert_eq!(s.creature_health, 1);
        assert_eq!(s.unique_creatures(), 1);
    }

    #[test]
    fn parse_damage_text_strips_sign() {
        assert_eq!(parse_damage_text("42"), Some(42));
        assert_eq!(parse_damage_text("-7"), Some(7));
        assert_eq!(parse_damage_text("You see"), None);
    }

    #[test]
    fn remove_skips_six_bytes() {
        let mut s = InboundState::default();
        let mut p = vec![OP_REMOVE];
        p.extend_from_slice(&1u16.to_le_bytes());
        p.extend_from_slice(&2u16.to_le_bytes());
        p.push(7);
        p.push(3);
        p.push(OP_PING);
        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn text_message_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_TEXT_MESSAGE, 0x17];
        p.extend_from_slice(&3u16.to_le_bytes());
        p.extend_from_slice(b"hi!");
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn cancel_walk_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_CANCEL_WALK, 2];
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(
            ev,
            vec![
                InboundEvent::CancelWalk,
                InboundEvent::MagicEffect {
                    pos: Position::new(1, 2, 7)
                },
            ]
        );
        assert_eq!(s.bytes_discarded, 0);
    }

    fn creature_say_say(name: &str, pos: Position, text: &str) -> Vec<u8> {
        let mut p = vec![OP_CREATURE_SAY];
        p.extend_from_slice(&1u32.to_le_bytes());
        p.extend_from_slice(&(name.len() as u16).to_le_bytes());
        p.extend_from_slice(name.as_bytes());
        p.push(1); // TALKTYPE_SAY
        p.extend_from_slice(&pos.x.to_le_bytes());
        p.extend_from_slice(&pos.y.to_le_bytes());
        p.push(pos.z);
        p.extend_from_slice(&(text.len() as u16).to_le_bytes());
        p.extend_from_slice(text.as_bytes());
        p
    }

    #[test]
    fn creature_square_then_walk_ack() {
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(Position::new(10, 10, 7)),
            ..InboundState::default()
        };
        let mut p = vec![OP_CREATURE_SQUARE];
        p.extend_from_slice(&77u32.to_le_bytes());
        p.push(1);
        p.push(OP_MOVE);
        p.extend_from_slice(&0xFFFFu16.to_le_bytes());
        p.extend_from_slice(&1u32.to_le_bytes());
        p.extend_from_slice(&11u16.to_le_bytes());
        p.extend_from_slice(&10u16.to_le_bytes());
        p.push(7);
        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::WalkAck]);
        assert_eq!(s.pos, Some(Position::new(11, 10, 7)));
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn cancel_target_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_CANCEL_TARGET];
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn creature_say_then_magic_effect() {
        let mut s = InboundState::default();
        let pos = Position::new(3, 4, 7);
        let mut p = creature_say_say("Test", pos, "exori vis");
        p.extend_from_slice(&magic_effect_bytes(pos, 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn creature_say_channel_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_CREATURE_SAY];
        p.extend_from_slice(&2u32.to_le_bytes());
        p.extend_from_slice(&4u16.to_le_bytes());
        p.extend_from_slice(b"Test");
        p.push(5); // TALKTYPE_CHANNEL_Y
        p.extend_from_slice(&8u16.to_le_bytes());
        p.extend_from_slice(&2u16.to_le_bytes());
        p.extend_from_slice(b"hi");
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn fight_modes_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_FIGHT_MODES, 1, 0, 0];
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
    }

    #[test]
    fn vip_status_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_VIP_STATUS];
        p.extend_from_slice(&42u32.to_le_bytes());
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn unknown_opcode_discards_rest_and_counts() {
        let mut s = InboundState::default();
        let mut p = vec![0x15, 1, 2, 3];
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert!(ev.is_empty());
        assert_eq!(s.magic_effects, 0);
        assert_eq!(s.unknown_opcodes, 1);
        assert_eq!(s.unknown_opcode_first, Some(0x15));
        assert!(s.bytes_discarded > 0);
    }
}
