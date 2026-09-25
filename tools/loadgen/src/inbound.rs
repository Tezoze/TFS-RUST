//! Minimal 772 inbound: self id, own pos, move/remove, combat effects, ping.
//!
//! C++ reference: `codec/v772.rs` self-appear `0x0A`; `map_description.rs` `0x64` header
//! and `0x6D` move; `v772.rs` `0x6C` remove (no named server consts). Combat counters
//! (`0x83`/`0x84`/`0x85`/`0x8C`) feed the Tier 4 content-equivalence gate.
//!
//! Map bodies (`0x64` / `0x65`–`0x68` / `0xBE`/`0xBF`) are skipped with the client
//! skip-counter so a later `0x83` in the same decrypted payload is counted.
//! Surface→underground NotifyGo is `0x6C` (no dest) + `0xBF`; inbound must bump z
//! before skipping 3-floor `0xBF` at the new z. Underground `0x6D` already has dest;
//! do not bump again on `0xBF` (dest z=9 skips 1). Climb up is always `0x6D` + dest then
//! `0xBE`; if `0x6D` did not apply (not self), `0xBE` at z=8 would skip 1 floor while
//! the encoder wrote 6 (new z==7). Bump z-1 on `0xBE` only when `player_z==8` and no
//! self `0x6D` dest z>7 is awaiting (9→8 dest z=8 must still skip 1, including when
//! `0x6D`/`0xBE` split frames). Encoder omits floor bytes when up `z<7` or down has no
//! range (`append_send_floors_body` early return); the next opcode is `SendRow`
//! (`0x65`–`0x68`) or a tile-update (`0x69`/`0x6A`/`0x6B`, `20260920T010424Z` peek
//! `6bfa7d…`). If skip-floors fails and the body starts with that opcode, treat
//! as 0 floors — do not eat `0xFF` (`20260919T223936Z` peek `67…`). After a counted
//! map/floor skip, extra skip-stream tiles may remain (`0x68` leftover `2f1171…`);
//! drain them until a known opcode. A trailing skip pair `[n, 0xFF]` with `n` equal
//! to a map opcode (`20260920T011720Z` `0x68` peek `eof`) is leftover empty tiles,
//! not a new `SendRow` — eat it when the next byte is a known opcode. Unread skip
//! tiles after `0xA0` (`0400ff…`) are drained on the following `feed` loop, not
//! treated as opcodes. Leftover `0xFF` before skip-stream (`ff2a11…` after `0xA0`)
//! is skip-tiled, not left as unknown 255. First-opcode `ff98…` stays unknown.
//! An omitted `SendRow` may be followed by a speak-shaped body (`00000600Test22…`).
//! Orphan skip-count before a leftover `[n, 0xFF]` (`20260920T014718Z` peek
//! `0400ff68…` at z=8) is 0-floor residue, not a 5-floor row. Orphan then a
//! leftover skip-tile (`20260920T021501Z` peek `00711100ff68…`) is the same
//! class: nibble one non-`0xFF` byte, skip-tile, next known opcode. Several
//! leftover skip-tiles after the orphan (`20260920T022857Z` peek `043d0400ff36…`)
//! drain until a known opcode, not one tile. Eof after those tiles
//! (`20260920T023807Z` peek `043d0428ff`) is 0-floor success. Leftover skip-stream
//! whose first byte is a known opcode (`20260920T025706Z` peek `6e02b21200ff65…`,
//! client id `0x026E`) is skip-tiled before 0-floor — do not parse it as
//! `OP_CONTAINER_OPEN`. Omitted-row speak may have a non-zero statement id
//! (`20260920T031649Z` peek `4b000700Test742…`). Skip-tile from a known opcode
//! only when the tile ends with a skip pair (`[n, 0xFF]`); a 10-thing cap on
//! `0xAA` say (`20260920T032906Z` peek `are a bot swarm` after `0x72`) is 0-floor.
//! A leftover known opcode with no body at eof (`20260920T034532Z` discarded=1,
//! unknown=0) is skip-stream residue — do not discard. Keep empty-body ping /
//! cancel-target.
//! Login
//! trailers (`0x78`/`0x79`/`0xA0`/`0xA1`/`0x82`/`0x8D`/`0xA2`/`0xB4`) are
//! length-skipped for the same reason. `0xB5` cancel-walk is parsed (direction
//! byte) and emitted so latency can retire a rejected walk. TVP also emits
//! `0x86` square, `0x8E` outfit, `0x8F` speed, `0x90`/`0x91` skull/shield,
//! `0xA3` cancel-target, and `0xAA` creature-say in the same payload as later
//! `0x6D`/`0x83` — those must be length-skipped too. Fight modes `0xA7` (three
//! body bytes, TVP `sendFightModes`) and VIP `0xD3`/`0xD4` (`u32` guid) are
//! skipped the same way. Unknown opcodes still discard the rest and are
//! counted (`unknown_opcodes` / `unknown_opcode_first` plus per-opcode counts).
//! First unknown also records peek / previous opcode / `player_z` so a lone `0xFF`
//! leftover skip high byte is attributable. A top-level `0xFF` is skipped when the
//! next byte is a known opcode (`20260920T005241Z` peek `ff66…`); skip-stream
//! `ff98…` stays unknown.
//! Map/thing skip failures record the opcode that was being skipped
//! (`skip_failure_opcodes`) so a Rust-only desync is attributable.
//! Spectator `0x6D` pos+stack from our tile is not self unless NotifyGo map
//! opcodes follow or dest is adjacent (`20260923T111647Z` `player_z=1`).

use std::collections::{HashMap, HashSet};
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
/// 772 `sendContainer` (`v772.rs` `encode_container_open`).
const OP_CONTAINER_OPEN: u8 = 0x6E;
/// 772 `sendCloseContainer` (`outgoing_extra.rs`).
const OP_CONTAINER_CLOSE: u8 = 0x6F;
/// 772 `sendAddContainerItem` (`v772.rs`).
const OP_CONTAINER_ADD: u8 = 0x70;
/// 772 `sendUpdateContainerItem` (`v772.rs`).
const OP_CONTAINER_UPDATE: u8 = 0x71;
/// 772 `sendRemoveContainerItem` (`v772.rs`).
const OP_CONTAINER_REMOVE: u8 = 0x72;
/// 772 `sendChannelsDialog`.
const OP_CHANNELS_DIALOG: u8 = 0xAB;
/// 772 `sendChannel`.
const OP_CHANNEL_OPEN: u8 = 0xAC;
/// 772 `sendOpenPrivateChannel`.
const OP_OPEN_PRIVATE: u8 = 0xAD;
/// 772 `sendCreatePrivateChannel`.
const OP_CREATE_PRIVATE: u8 = 0xB2;
/// 772 `sendClosePrivate`.
const OP_CLOSE_PRIVATE: u8 = 0xB3;
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
    /// Walk bump: `0xB4` walk-bump text with a trailing `0xB5` in this payload
    /// (`on_walk_step_rejected`). Unpaired Sorry is histogram-only.
    WalkRejected,
    MagicEffect {
        pos: Position,
        effect: u8,
    },
    /// Server refused an action (`0xB4` cancel text); retires one outstanding
    /// spell/rune instead of recording a sample. Login MOTD / broadcasts never
    /// match the failure list below.
    SpellRejected,
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
    /// Decrypted server frames (= server `write`s; one XTEA frame each).
    pub frames_in: u64,
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
    /// `skip_failed` counts keyed by the opcode whose body could not be skipped.
    pub skip_failure_opcodes: HashMap<u8, u64>,
    /// Unknown top-level opcode counts (not first-only).
    pub unknown_opcode_counts: HashMap<u8, u64>,
    /// `0xB4` action-failure texts (walk + spell). Walk-bump strings emit
    /// [`InboundEvent::WalkRejected`] only when the next opcode is `0xB5`;
    /// unpaired Sorry stays histogram-only. Spell texts emit [`InboundEvent::SpellRejected`].
    pub text_reject_counts: HashMap<String, u64>,
    /// First 8 bytes at the skip-failure cursor (hex), if any.
    pub skip_failure_first_peek: Option<String>,
    /// `player_z` when the first skip failed (None if no skip fail).
    pub skip_failure_player_z: Option<u8>,
    /// First 32 bytes at the unknown-opcode cursor (hex, includes the opcode).
    pub unknown_opcode_peek: Option<String>,
    /// Opcode successfully parsed immediately before the first unknown, if any.
    pub unknown_opcode_prev: Option<u8>,
    /// `player_z` when the first unknown opcode was seen.
    pub unknown_opcode_player_z: Option<u8>,
    /// Self `0x6D` dest z>7 and dest<old (underground climb). Next `0xBE` skips 1
    /// floor at that z even if it arrives in a later frame — do not bump 8→7.
    awaiting_move_up_one_floor: bool,
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
            frames_in: 0,
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
            skip_failure_opcodes: HashMap::new(),
            unknown_opcode_counts: HashMap::new(),
            text_reject_counts: HashMap::new(),
            skip_failure_first_peek: None,
            skip_failure_player_z: None,
            unknown_opcode_peek: None,
            unknown_opcode_prev: None,
            unknown_opcode_player_z: None,
            awaiting_move_up_one_floor: false,
        }
    }

    pub fn unique_creatures(&self) -> u64 {
        self.seen_creatures.len() as u64
    }

    pub fn add_counters(&mut self, other: &InboundState) {
        self.bytes_in += other.bytes_in;
        self.frames_in += other.frames_in;
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
        merge_u8_counts(&mut self.skip_failure_opcodes, &other.skip_failure_opcodes);
        merge_u8_counts(
            &mut self.unknown_opcode_counts,
            &other.unknown_opcode_counts,
        );
        merge_str_counts(&mut self.text_reject_counts, &other.text_reject_counts);
        if self.skip_failure_first_peek.is_none() {
            self.skip_failure_first_peek = other.skip_failure_first_peek.clone();
        }
        if self.skip_failure_player_z.is_none() {
            self.skip_failure_player_z = other.skip_failure_player_z;
        }
        if self.unknown_opcode_peek.is_none() {
            self.unknown_opcode_peek = other.unknown_opcode_peek.clone();
        }
        if self.unknown_opcode_prev.is_none() {
            self.unknown_opcode_prev = other.unknown_opcode_prev;
        }
        if self.unknown_opcode_player_z.is_none() {
            self.unknown_opcode_player_z = other.unknown_opcode_player_z;
        }
        self.seen_creatures.extend(&other.seen_creatures);
    }

    fn note_unknown(&mut self, op: u8, payload: &[u8], start: usize, prev: Option<u8>) {
        self.unknown_opcodes += 1;
        *self.unknown_opcode_counts.entry(op).or_insert(0) += 1;
        if self.unknown_opcode_first.is_none() {
            self.unknown_opcode_first = Some(op);
            let n = payload.len().saturating_sub(start).min(32);
            self.unknown_opcode_peek = Some(hex_bytes(&payload[start..start + n]));
            self.unknown_opcode_prev = prev;
            self.unknown_opcode_player_z = Some(self.player_z());
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
        self.frames_in += 1;
        let mut events = Vec::new();
        let mut i = 0usize;
        let mut prev: Option<u8> = None;
        while i < payload.len() {
            let op = payload[i];
            // Orphan skip high byte (`[n, 0xFF]` pair) left after a finished
            // tile/container. Next byte is a real opcode (`20260920T005241Z`
            // peek `ff66…`). Do not eat when the next byte is still skip-stream
            // (`ff98…`, `20260919T221503Z`).
            if op == 0xFF
                && payload
                    .get(i + 1)
                    .copied()
                    .is_some_and(is_known_inbound_opcode)
            {
                i += 1;
                continue;
            }
            // Skip-stream left after a finished packet (`20260920T011720Z` `0x04`
            // after `0xA0`). Do not drain the first opcode of a payload (`0x15`
            // unknown test).
            if prev.is_some() && !is_known_inbound_opcode(op) {
                let before = i;
                self.drain_trailing_skip_stream(payload, &mut i);
                if i != before {
                    continue;
                }
            }
            // `20260920T034532Z`: leftover skip-stream byte that collides with a
            // known opcode, at eof (0-floor then `0x72` with no cid/slot). Do not
            // discard. Empty-body opcodes (`0x1D`/`0x1E`/`0xA3`) still parse.
            if i + 1 == payload.len() && is_known_inbound_opcode(op) && !opcode_has_empty_body(op) {
                i += 1;
                continue;
            }
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
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_MAP_NORTH | OP_MAP_SOUTH => {
                    if !self.skip_map_body(payload, &mut i, client_viewport_width(), 1) {
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_MAP_EAST | OP_MAP_WEST => {
                    if !self.skip_map_body(payload, &mut i, 1, client_viewport_height()) {
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_FLOOR_UP => {
                    // Encoder `SendFloors` up uses z after the NotifyGo up-step.
                    // 8→7 without dest still at z=8 would skip 1 floor (need 6).
                    // 9→8 dest z=8 skips 1; `awaiting_move_up_one_floor` survives
                    // a frame split so we do not bump (`20260919T222725Z`).
                    let dest_applied = self.awaiting_move_up_one_floor;
                    self.awaiting_move_up_one_floor = false;
                    if self.player_z() == 8 && !dest_applied {
                        events.push(InboundEvent::WalkAck);
                        self.apply_notify_go_z_up();
                    }
                    if !self.skip_floor_up(payload, &mut i) {
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_FLOOR_DOWN => {
                    // NotifyGo increments z then `SendFloors` (`map_description.rs`).
                    // Surface→underground is `0x6C` (no dest) so z is still 7 here.
                    // Underground downs already applied dest via `0x6D` — do not bump
                    // again (`player_z==14` would skip 0 floors instead of 1).
                    // Do not bump `0xBF` at z=8: dest-applied 7→8/`0x6D` skips 3
                    // (`20260919T223936Z` skip 10→17).
                    if self.player_z() == 7 {
                        events.push(InboundEvent::WalkAck);
                        self.apply_notify_go_z_down();
                    }
                    if !self.skip_floor_down(payload, &mut i) {
                        self.skip_failed(payload, i, op);
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
                        self.skip_failed(payload, i, op);
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
                        self.skip_failed(payload, i, op);
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
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_MOVE => match parse_move(payload, &mut i) {
                    None => {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    Some((id, old_pos, new_pos)) => {
                        if self.is_self_move(id, old_pos, new_pos, &payload[i..]) {
                            let old_z = self.player_z();
                            self.pos = Some(new_pos);
                            if new_pos.z > 7 && old_z > new_pos.z {
                                self.awaiting_move_up_one_floor = true;
                            }
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
                    let effect = payload[i];
                    i += 1;
                    self.magic_effects += 1;
                    events.push(InboundEvent::MagicEffect { pos, effect });
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
                        self.skip_failed(payload, i, op);
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
                    // Failure cancels (`send_cancel_message` / `SendResult`) share this
                    // opcode with MOTD / broadcasts, so only known action-failure texts
                    // retire an outstanding action — never match on the opcode alone.
                    // Walk-bump text retires a walk only when the next byte is `0xB5`.
                    if !take(payload, &mut i, 1) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                    match read_len_string(payload, &mut i) {
                        None => {
                            self.discard_rest(payload, i);
                            break;
                        }
                        Some(text) if is_action_failure_text(&text) => {
                            *self.text_reject_counts.entry(text.clone()).or_insert(0) += 1;
                            if is_walk_snapback_text(&text)
                                && payload.get(i).copied() == Some(OP_CANCEL_WALK)
                            {
                                events.push(InboundEvent::WalkRejected);
                            } else if is_spell_failure_text(&text) {
                                events.push(InboundEvent::SpellRejected);
                            }
                        }
                        Some(_) => {}
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
                OP_CONTAINER_CLOSE => {
                    if !take(payload, &mut i, 1) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_CONTAINER_REMOVE => {
                    if !take(payload, &mut i, 2) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                OP_CONTAINER_ADD => {
                    if !take(payload, &mut i, 1)
                        || !skip_772_thing(payload, &mut i, |id| self.item_extra.has(id))
                    {
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_CONTAINER_UPDATE => {
                    if !take(payload, &mut i, 2)
                        || !skip_772_thing(payload, &mut i, |id| self.item_extra.has(id))
                    {
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_CONTAINER_OPEN => {
                    if !self.skip_container_open(payload, &mut i) {
                        self.skip_failed(payload, i, op);
                        break;
                    }
                }
                OP_CHANNELS_DIALOG => {
                    if !skip_channels_dialog(payload, &mut i) {
                        self.discard_rest(payload, i);
                        break;
                    }
                }
                OP_CHANNEL_OPEN | OP_CREATE_PRIVATE => {
                    if !take(payload, &mut i, 2) || !skip_len_string(payload, &mut i) {
                        self.discard_rest(payload, i);
                        break;
                    }
                }
                OP_OPEN_PRIVATE => {
                    if !skip_len_string(payload, &mut i) {
                        self.discard_rest(payload, i);
                        break;
                    }
                }
                OP_CLOSE_PRIVATE => {
                    if !take(payload, &mut i, 2) {
                        self.discard_rest(payload, i.saturating_sub(1));
                        break;
                    }
                }
                _ => {
                    self.note_unknown(op, payload, i.saturating_sub(1), prev);
                    self.discard_rest(payload, i.saturating_sub(1));
                    break;
                }
            }
            prev = Some(op);
        }
        events
    }

    fn player_z(&self) -> u8 {
        self.pos.map(|p| p.z).unwrap_or(7)
    }

    /// Encoder `SendFloors` uses z after the NotifyGo down-step.
    fn apply_notify_go_z_down(&mut self) {
        if let Some(p) = &mut self.pos
            && p.z < 15
        {
            p.z = p.z.saturating_add(1);
        }
    }

    /// Encoder `SendFloors` uses z after the NotifyGo up-step.
    fn apply_notify_go_z_up(&mut self) {
        if let Some(p) = &mut self.pos
            && p.z > 0
        {
            p.z = p.z.saturating_sub(1);
        }
    }

    fn skip_map_body(&self, payload: &[u8], i: &mut usize, width: i32, height: i32) -> bool {
        let start = *i;
        if skip_772_map_description_body(payload, i, self.player_z(), width, height, |id| {
            self.item_extra.has(id)
        }) {
            self.drain_trailing_skip_stream(payload, i);
            return true;
        }
        *i = start;
        // Encoder omitted the row body, or this `0x65`–`0x68` was a leftover skip
        // pair consumed as an opcode (`20260920T011720Z` peek `eof`).
        if start >= payload.len() {
            return true;
        }
        // `20260920T013342Z` peek `00000600Test22…`: omitted row then speak.
        if skip_speak_shaped_body(payload, i) {
            return true;
        }
        *i = start;
        // `20260920T014718Z` peek `0400ff68…`: orphan skip-count then `[n, 0xFF]`
        // then the next opcode. Counted 5-floor skip treats `04 00` as item 4.
        if skip_orphan_skip_count_then_pair(payload, i) {
            self.drain_trailing_skip_stream(payload, i);
            return true;
        }
        *i = start;
        // `20260920T021501Z` peek `00711100ff68…` / `20260920T022857Z`
        // `043d0400ff36…`: orphan then leftover skip-stream (one or more tiles).
        // `20260920T025706Z` peek `6e02b21200ff65…`: no orphan — first byte is
        // leftover id `0x026E` (`OP_CONTAINER_OPEN`). Skip-tile before 0-floor.
        if self.skip_orphan_then_skip_stream(payload, i) {
            self.drain_trailing_skip_stream(payload, i);
            return true;
        }
        *i = start;
        if payload
            .get(start)
            .copied()
            .is_some_and(is_known_inbound_opcode)
        {
            return true;
        }
        false
    }

    fn skip_floor_up(&self, payload: &[u8], i: &mut usize) -> bool {
        self.skip_send_floors_body(payload, i, true)
    }

    fn skip_floor_down(&self, payload: &[u8], i: &mut usize) -> bool {
        self.skip_send_floors_body(payload, i, false)
    }

    /// Skip `SendFloors` body. Encoder may write **no** floor bytes (`z<7` up, or
    /// down with no range) and then `SendRow` (`0x65`–`0x68`) or a tile-update
    /// (`0x69`/`0x6A`/`0x6B`). If the counted skip fails and the body starts with
    /// that next opcode, rewind and succeed (0 floors).
    fn skip_send_floors_body(&self, payload: &[u8], i: &mut usize, up: bool) -> bool {
        let start = *i;
        let ok = if up {
            skip_772_move_up_floor_body(payload, i, self.player_z(), |id| self.item_extra.has(id))
        } else {
            skip_772_move_down_floor_body(payload, i, self.player_z(), |id| self.item_extra.has(id))
        };
        if ok {
            self.drain_trailing_skip_stream(payload, i);
            return true;
        }
        if send_floors_omitted_body(payload, start) {
            *i = start;
            return true;
        }
        *i = start;
        false
    }

    /// Encoder wrote more skip tiles than inbound z counted (`20260920T010424Z`
    /// `0x68` leftover `2f1171…`). Consume whole tiles until a known opcode.
    /// A leftover skip pair `[n, 0xFF]` is eaten even when `n` is a map opcode
    /// (`20260920T011720Z` `0x68` then eof) if the following byte is a known
    /// opcode. Do not eat `0x6D 0xFFFF` (self-move). Leftover `0xFF` before
    /// skip-stream (`20260920T013342Z` `ff2a11…`) is skip-tiled; first-opcode
    /// `ff98…` is not drained (`prev` is unset).
    fn drain_trailing_skip_stream(&self, payload: &[u8], i: &mut usize) {
        self.drain_skip_stream(payload, i, true);
    }

    fn drain_skip_stream(&self, payload: &[u8], i: &mut usize, use_extra: bool) {
        loop {
            if *i >= payload.len() {
                return;
            }
            let op = payload[*i];
            if op == 0xFF {
                // Orphan skip high byte. Next may be skip-stream (`ff2a11…`)
                // not a known opcode (`20260920T013342Z`).
                *i += 1;
                continue;
            }
            if is_leftover_skip_pair(payload, *i) {
                *i += 2;
                continue;
            }
            if is_known_inbound_opcode(op) {
                return;
            }
            let start = *i;
            if skip_772_tile_description(payload, i, |id| use_extra && self.item_extra.has(id))
                .is_some()
            {
                continue;
            }
            *i = start;
            return;
        }
    }

    /// Leftover skip-stream tiles, then a known opcode or eof.
    /// Use on `SendRow` skip failure (`20260920T021501Z` `00711100ff68`,
    /// `20260920T022857Z` `043d0400ff36` — several tiles, not one).
    /// `20260920T023807Z` peek `043d0428ff`: skip pair then eof is 0-floor.
    /// `20260920T025706Z` peek `6e02b21200ff65…`: no orphan nibble — first byte is
    /// leftover id `0x026E`. A known-opcode start must end the tile on a skip pair;
    /// 10-thing cap on a real `0xAA` say (`20260920T032906Z`) is not leftover
    /// skip-stream. Do not nibble `0xFF`. Speak-shaped `00000600Test22…` is
    /// handled before this. Skip-tile **before** 0-floor known opcode.
    fn skip_orphan_then_skip_stream(&self, payload: &[u8], i: &mut usize) -> bool {
        let start = *i;
        let Some(orphan) = payload.get(start).copied() else {
            return false;
        };
        if orphan == 0xFF {
            return false;
        }
        let started_at_known = is_known_inbound_opcode(orphan);
        if !started_at_known {
            *i = start + 1;
        }
        let mut consumed = false;
        loop {
            if consumed
                && (*i >= payload.len()
                    || payload
                        .get(*i)
                        .copied()
                        .is_some_and(is_known_inbound_opcode))
            {
                return true;
            }
            if *i >= payload.len() {
                *i = start;
                return false;
            }
            let tile_start = *i;
            let mut ok = false;
            for use_extra in [false, true] {
                *i = tile_start;
                if skip_772_tile_description(payload, i, |id| use_extra && self.item_extra.has(id))
                    .is_some()
                    && *i > tile_start
                {
                    ok = true;
                    break;
                }
            }
            if !ok {
                *i = start;
                return false;
            }
            // Known-opcode first byte: leftover id (`0x026E`) ends `[n, 0xFF]`.
            // 10-thing cap on `0xAA` / `0x72` is a real following packet.
            if started_at_known && payload.get(*i - 1).copied() != Some(0xFF) {
                *i = start;
                return false;
            }
            consumed = true;
        }
    }

    fn is_self_move(
        &self,
        id: Option<u32>,
        old_pos: Option<Position>,
        new_pos: Position,
        rest: &[u8],
    ) -> bool {
        if let (Some(sid), Some(cid)) = (self.self_id, id) {
            return sid == cid;
        }
        if id.is_some() {
            return false;
        }
        let Some(here) = self.pos else {
            return false;
        };
        let Some(old) = old_pos else {
            return false;
        };
        if here != old {
            return false;
        }
        // Spectator `0x6D` uses pos+stack when stackpos < 10 (`send_move_creature_spectator`).
        // On a shared tile that packet's old pos is ours, so treating every match as self
        // adopts the mover's dest — `20260923T111647Z` first skip was `player_z=1`.
        // Self NotifyGo keeps `0x6D` then `0xBE`/`0xBF`/`SendRow` in the same payload
        // (`send_notify_go_fill`). Split-frame self is adjacent (`|d|≤1`) with empty rest.
        notify_go_map_follows(rest) || (rest.is_empty() && chebyshev_le1(here, new_pos))
    }

    fn skip_container_open(&self, payload: &[u8], i: &mut usize) -> bool {
        // cid + container item + name + capacity + hasParent + count + items.
        if !take(payload, i, 1) {
            return false;
        }
        if !skip_772_thing(payload, i, |id| self.item_extra.has(id)) {
            return false;
        }
        if !skip_len_string(payload, i) || !take(payload, i, 2) {
            return false;
        }
        if !take(payload, i, 1) {
            return false;
        }
        let n = payload[*i - 1];
        for _ in 0..n {
            if !skip_772_thing(payload, i, |id| self.item_extra.has(id)) {
                return false;
            }
        }
        true
    }

    fn skip_failed(&mut self, payload: &[u8], start: usize, op: u8) {
        self.skip_failures += 1;
        *self.skip_failure_opcodes.entry(op).or_insert(0) += 1;
        if self.skip_failure_first_peek.is_none() {
            let n = payload.len().saturating_sub(start).min(32);
            self.skip_failure_first_peek = if n == 0 {
                Some("eof".into())
            } else {
                Some(hex_bytes(&payload[start..start + n]))
            };
            self.skip_failure_player_z = Some(self.player_z());
        }
        self.discard_rest(payload, start);
    }

    fn discard_rest(&mut self, payload: &[u8], start: usize) {
        self.bytes_discarded += payload.len().saturating_sub(start) as u64;
    }
}

fn notify_go_map_follows(rest: &[u8]) -> bool {
    matches!(
        rest.first(),
        Some(
            &OP_FLOOR_UP
                | &OP_FLOOR_DOWN
                | &OP_MAP_NORTH
                | &OP_MAP_EAST
                | &OP_MAP_SOUTH
                | &OP_MAP_WEST
                | &OP_MAP_DESCRIPTION
                | &OP_UPDATE_TILE
                | &OP_ADD_TILE_THING
                | &OP_UPDATE_TILE_THING
        )
    )
}

fn chebyshev_le1(a: Position, b: Position) -> bool {
    (i32::from(a.x) - i32::from(b.x)).unsigned_abs() <= 1
        && (i32::from(a.y) - i32::from(b.y)).unsigned_abs() <= 1
        && (i32::from(a.z) - i32::from(b.z)).unsigned_abs() <= 1
}

/// Leftover skip pair `[n, 0xFF]` after a counted skip-stream.
/// `n` may equal a map opcode (`0x68`); eat it only when the next byte is a
/// known opcode so a real `SendRow` with skip-count `0xFF` (`0x68 0xFF 0xFF…`)
/// and self-move `0x6D 0xFFFF` stay intact.
fn is_leftover_skip_pair(payload: &[u8], i: usize) -> bool {
    if payload.get(i + 1).copied() != Some(0xFF) {
        return false;
    }
    let op = payload[i];
    let after = payload.get(i + 2).copied();
    !is_known_inbound_opcode(op) || after.is_some_and(is_known_inbound_opcode)
}

/// Orphan skip-count byte before leftover `[n, 0xFF]`, then a known opcode.
/// Use on `SendRow` skip failure when the counted body is `XX nn FF <op>`
/// (`20260920T014718Z` peek `0400ff68`). Do not use on speak-shaped
/// `00000600Test22…` (byte 2 ≠ `0xFF`) or first-opcode `ff98…` (never here;
/// byte 2 is not `0xFF` / `[3]` not a known opcode).
fn skip_orphan_skip_count_then_pair(payload: &[u8], i: &mut usize) -> bool {
    let start = *i;
    if payload.len() < start + 4 {
        return false;
    }
    let orphan = payload[start];
    if is_known_inbound_opcode(orphan)
        || payload[start + 2] != 0xFF
        || !is_known_inbound_opcode(payload[start + 3])
    {
        return false;
    }
    *i = start + 3;
    true
}

/// Encoder `append_send_floors_body` returned with no floor bytes; next opcode is
/// `SendRow`, a tile-update, or another top-level packet (`20260919T223936Z` peek
/// `67…`, `20260920T010424Z` peek `6bfa7d…`). Never `0xFF`.
fn send_floors_omitted_body(payload: &[u8], start: usize) -> bool {
    matches!(
        payload.get(start).copied(),
        Some(
            OP_MAP_NORTH
                | OP_MAP_EAST
                | OP_MAP_SOUTH
                | OP_MAP_WEST
                | OP_MAP_DESCRIPTION
                | OP_UPDATE_TILE
                | OP_ADD_TILE_THING
                | OP_UPDATE_TILE_THING
                | OP_FLOOR_UP
                | OP_FLOOR_DOWN
                | OP_MOVE
                | OP_REMOVE
                | OP_MAGIC_EFFECT
        )
    )
}

/// Server packets with no body. A leftover known opcode at eof is skip-stream
/// residue (`20260920T034532Z`); these still emit.
fn opcode_has_empty_body(op: u8) -> bool {
    matches!(op, OP_PING | OP_PING_BACK | OP_CANCEL_TARGET)
}

/// Opcodes `feed` actually parses. A leftover `0xFF` is skipped only when the
/// following byte is one of these — not `0x98` skip-stream or another `0xFF`.
fn is_known_inbound_opcode(op: u8) -> bool {
    matches!(
        op,
        OP_SELF_APPEAR
            | OP_PING
            | OP_PING_BACK
            | OP_MAP_DESCRIPTION
            | OP_MAP_NORTH
            | OP_MAP_EAST
            | OP_MAP_SOUTH
            | OP_MAP_WEST
            | OP_UPDATE_TILE
            | OP_ADD_TILE_THING
            | OP_UPDATE_TILE_THING
            | OP_REMOVE
            | OP_MOVE
            | OP_CONTAINER_OPEN
            | OP_CONTAINER_CLOSE
            | OP_CONTAINER_ADD
            | OP_CONTAINER_UPDATE
            | OP_CONTAINER_REMOVE
            | OP_INVENTORY_ITEM
            | OP_INVENTORY_EMPTY
            | OP_FLOOR_UP
            | OP_FLOOR_DOWN
            | OP_WORLD_LIGHT
            | OP_MAGIC_EFFECT
            | OP_ANIMATED_TEXT
            | OP_DISTANCE_SHOOT
            | OP_CREATURE_SQUARE
            | OP_CREATURE_HEALTH
            | OP_CREATURE_LIGHT
            | OP_CREATURE_OUTFIT
            | OP_CHANGE_SPEED
            | OP_CREATURE_SKULL
            | OP_CREATURE_SHIELD
            | OP_PLAYER_STATS
            | OP_PLAYER_SKILLS
            | OP_PLAYER_ICONS
            | OP_CANCEL_TARGET
            | OP_FIGHT_MODES
            | OP_CREATURE_SAY
            | OP_CHANNELS_DIALOG
            | OP_CHANNEL_OPEN
            | OP_OPEN_PRIVATE
            | OP_TEXT_MESSAGE
            | OP_CANCEL_WALK
            | OP_CREATE_PRIVATE
            | OP_CLOSE_PRIVATE
            | OP_VIP_ENTRY
            | OP_VIP_STATUS
            | OP_VIP_LOGOUT
    )
}

fn skip_channels_dialog(buf: &[u8], i: &mut usize) -> bool {
    if !take(buf, i, 1) {
        return false;
    }
    let n = buf[*i - 1];
    for _ in 0..n {
        if !take(buf, i, 2) || !skip_len_string(buf, i) {
            return false;
        }
    }
    true
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn merge_u8_counts(dst: &mut HashMap<u8, u64>, src: &HashMap<u8, u64>) {
    for (&k, &v) in src {
        *dst.entry(k).or_insert(0) += v;
    }
}

fn merge_str_counts(dst: &mut HashMap<String, u64>, src: &HashMap<String, u64>) {
    for (k, &v) in src {
        *dst.entry(k.clone()).or_insert(0) += v;
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
    read_len_string(buf, i).is_some()
}

fn read_len_string(buf: &[u8], i: &mut usize) -> Option<String> {
    if buf.len().saturating_sub(*i) < 2 {
        return None;
    }
    let len = u16::from_le_bytes([buf[*i], buf[*i + 1]]) as usize;
    *i += 2;
    if buf.len().saturating_sub(*i) < len {
        return None;
    }
    let s = String::from_utf8_lossy(&buf[*i..*i + len]).into_owned();
    *i += len;
    Some(s)
}

/// `ReturnValue::description` texts (`crates/tfs-rust-core/src/return_value.rs`)
/// for actions the loadgen schedules (use / rune / say / attack). Anything else
/// on `0xB4` (MOTD, broadcasts, advance) is not a rejection.
fn is_action_failure_text(text: &str) -> bool {
    is_walk_failure_text(text) || is_spell_failure_text(text)
}

/// Cylinder / walk `SendResult` strings. Histogrammed always. Walk FIFO retire
/// is [`is_walk_snapback_text`] plus a trailing `0xB5` in the same payload.
fn is_walk_failure_text(text: &str) -> bool {
    is_walk_snapback_text(text)
        || matches!(
            text,
            "You cannot throw there." | "You cannot use this object."
        )
}

/// Walk bump / blocked step — `on_walk_step_rejected` sends this text then
/// `0xB5` in one coalesced frame. `"Sorry, not possible."` alone is also
/// unpaired `send_cancel_message(NotPossible)` (`20260919T225517Z`).
fn is_walk_snapback_text(text: &str) -> bool {
    matches!(
        text,
        "Sorry, not possible."
            | "Destination is out of range."
            | "You are too far away."
            | "First go downstairs."
            | "First go upstairs."
            | "There is no way."
            | "This is impossible."
    )
}

fn is_spell_failure_text(text: &str) -> bool {
    matches!(
        text,
        "Your magic level is too low."
            | "You do not have enough magic level."
            | "You are exhausted."
            | "You cannot use objects that fast."
            | "You can only use it on creatures."
            | "This action is not permitted in a protection zone."
            | "You do not have enough mana."
            | "You do not have enough soulpoints."
            | "You must learn this spell first."
            | "You have the wrong vocation to cast this spell."
            | "Turn secure mode off if you really want to attack unmarked players."
            | "You may not attack this person."
            | "You may not attack this creature."
    )
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

/// Omitted `SendRow` body that is actually a speak packet without `0xAA`
/// (`20260920T013342Z` peek `00000600Test22` + type SAY + pos + `bots bots bots`;
/// `20260920T031649Z` peek `4b000700Test742` + SAY + pos + `we hunt as one`).
/// Optional leading `u16` statement id (0 or non-zero), then name + speak-type + pos + text.
fn skip_speak_shaped_body(buf: &[u8], i: &mut usize) -> bool {
    let start = *i;
    if buf.len().saturating_sub(*i) >= 4 {
        let follow = u16::from_le_bytes([buf[*i + 2], buf[*i + 3]]);
        if (1..=32).contains(&follow) {
            *i += 2;
        }
    }
    let name_len = buf
        .get(*i..*i + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .unwrap_or(0);
    let name_off = *i + 2;
    let name_end = name_off + usize::from(name_len);
    let name_ok = (1..=32).contains(&name_len)
        && buf
            .get(name_off..name_end)
            .is_some_and(|n| !n.is_empty() && n.iter().all(u8::is_ascii_alphanumeric));
    if !name_ok || !skip_len_string(buf, i) || !take(buf, i, 1) {
        *i = start;
        return false;
    }
    let speak = buf[*i - 1];
    if !matches!(speak, 1 | 2 | 3 | 0x10 | 0x11) || !take(buf, i, 5) || !skip_len_string(buf, i) {
        *i = start;
        return false;
    }
    true
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
    use tfs_rust_common::ProtocolVersion;
    use tfs_rust_net::map_description::{
        KnownCreatureTable, TileContent, send_map_description_packet, send_notify_go,
        write_map_description_body,
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

        let mut known = KnownCreatureTable::default();
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
        assert_eq!(
            ev,
            vec![InboundEvent::MagicEffect {
                pos: player,
                effect: 11
            }]
        );
        assert_eq!(s.self_id, Some(42));
        assert_eq!(s.pos, Some(player));
        assert_eq!(s.magic_effects, 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn leftover_ff_before_magic_effect_is_skipped() {
        let mut s = InboundState::default();
        let player = Position::new(32369, 32241, 7);
        let mut known = KnownCreatureTable::default();
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
        let mut p = map.into_bytes();
        p.push(0xFF);
        p.extend_from_slice(&magic_effect_bytes(player, 11));

        let ev = s.feed(&p);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(s.skip_failures, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn leftover_ff_after_container_remove_then_ping() {
        let mut s = InboundState::default();
        let p = vec![OP_CONTAINER_REMOVE, 0, 0, 0xFF, OP_PING];
        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn ff_then_skip_stream_stays_unknown() {
        // `20260919T221503Z` peek `980100ff…` after eating 0xFF. Next byte is
        // not a known opcode — leave the 0xFF as unknown.
        let mut s = InboundState::default();
        let ev = s.feed(&[0xFF, 0x98, 0x01, 0x00, 0xFF]);
        assert!(ev.is_empty());
        assert_eq!(s.unknown_opcodes, 1);
        assert_eq!(s.unknown_opcode_first, Some(0xFF));
        assert!(s.bytes_discarded > 0);
    }

    #[test]
    fn north_strip_then_magic_effect() {
        let mut s = InboundState {
            pos: Some(Position::new(100, 200, 7)),
            ..InboundState::default()
        };
        let mut known = KnownCreatureTable::default();
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
    fn spectator_move_from_our_tile_to_z1_does_not_steal_pos() {
        // `20260923T111647Z`: other creature `0x6D` pos+stack, dest z=1.
        let here = Position::new(32776, 32240, 7);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(here),
            ..InboundState::default()
        };
        let mut p = vec![OP_MOVE];
        p.extend_from_slice(&here.x.to_le_bytes());
        p.extend_from_slice(&here.y.to_le_bytes());
        p.push(here.z);
        p.push(1);
        p.extend_from_slice(&here.x.to_le_bytes());
        p.extend_from_slice(&here.y.to_le_bytes());
        p.push(1);
        p.extend_from_slice(&magic_effect_bytes(here, 3));
        let ev = s.feed(&p);
        assert_eq!(ev.len(), 1, "{ev:?}");
        assert_eq!(s.pos, Some(here));
        assert_eq!(s.player_z(), 7);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(s.skip_failures, 0);
    }

    #[test]
    fn spectator_adjacent_walk_without_notify_go_rows_still_acks_split_self() {
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(Position::new(10, 10, 7)),
            ..InboundState::default()
        };
        let mut p = vec![OP_MOVE];
        p.extend_from_slice(&10u16.to_le_bytes());
        p.extend_from_slice(&10u16.to_le_bytes());
        p.push(7);
        p.push(1);
        p.extend_from_slice(&11u16.to_le_bytes());
        p.extend_from_slice(&10u16.to_le_bytes());
        p.push(7);
        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::WalkAck]);
        assert_eq!(s.pos, Some(Position::new(11, 10, 7)));
    }

    #[test]
    fn notify_go_hole_down_skips_and_acks() {
        let orig = Position::new(100, 100, 7);
        let dest = Position::new(100, 100, 8);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let ev = s.feed(&p);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.pos.map(|p| p.z), Some(8));
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn notify_go_same_z_south_pos_comes_from_6d() {
        let orig = Position::new(100, 100, 7);
        let dest = Position::new(100, 101, 7);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        let ev = s.feed(&p);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.pos, Some(dest));
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn notify_go_stairs_down_diag_skips() {
        let orig = Position::new(100, 100, 7);
        let dest = Position::new(100, 101, 8);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let ev = s.feed(&p);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.pos.map(|p| p.z), Some(8));
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn notify_go_underground_down_skips() {
        let orig = Position::new(100, 100, 8);
        let dest = Position::new(100, 100, 9);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(
            s.pos,
            Some(dest),
            "0x6D dest z must not be bumped again on 0xBF"
        );
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn notify_go_ladder_up_skips() {
        let orig = Position::new(100, 100, 8);
        let dest = Position::new(100, 100, 7);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.pos.map(|p| p.z), Some(7));
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn notify_go_underground_up_skips() {
        let orig = Position::new(100, 100, 9);
        let dest = Position::new(100, 100, 8);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(
            s.pos,
            Some(dest),
            "0x6D dest z must not be bumped again on 0xBE"
        );
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn notify_go_underground_up_split_frames_does_not_bump() {
        // `0x6D` dest z=8 in one frame, `0xBE` in the next — must still skip 1 floor.
        let orig = Position::new(100, 100, 9);
        let dest = Position::new(100, 100, 8);
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let full = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        let be = full
            .iter()
            .position(|&b| b == OP_FLOOR_UP)
            .expect("0xBE after 0x6D");
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let ev1 = s.feed(&full[..be]);
        assert!(ev1.contains(&InboundEvent::WalkAck));
        assert_eq!(s.pos, Some(dest));
        let mut rest = full[be..].to_vec();
        rest.extend_from_slice(&magic_effect_bytes(dest, 11));
        let ev2 = s.feed(&rest);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(
            s.pos,
            Some(dest),
            "split-frame 9→8 must not bump 8→7 on 0xBE"
        );
        assert_eq!(s.magic_effects, 1);
        assert!(!ev2.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn floor_up_at_z8_without_6d_skips_six_floors() {
        // Production leftover after 0xBE at inbound z=8: encoder 8→7 wrote 6 floors.
        let orig = Position::new(100, 100, 8);
        let dest = Position::new(100, 100, 7);
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let full = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        let be = full
            .iter()
            .position(|&b| b == OP_FLOOR_UP)
            .expect("0xBE after 0x6D");
        let mut p = full[be..].to_vec();
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.pos.map(|p| p.z), Some(7));
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn notify_go_underground_down_split_frames_does_not_bump() {
        // `0x6D` dest z=9 in one frame, `0xBF` in the next — must still skip 1 floor.
        let orig = Position::new(100, 100, 8);
        let dest = Position::new(100, 100, 9);
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let full = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        let bf = full
            .iter()
            .position(|&b| b == OP_FLOOR_DOWN)
            .expect("0xBF after 0x6D");
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let ev1 = s.feed(&full[..bf]);
        assert!(ev1.contains(&InboundEvent::WalkAck));
        assert_eq!(s.pos, Some(dest));
        let mut rest = full[bf..].to_vec();
        rest.extend_from_slice(&magic_effect_bytes(dest, 11));
        let ev2 = s.feed(&rest);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(
            s.pos,
            Some(dest),
            "split-frame 8→9 must not bump 9→10 on 0xBF"
        );
        assert_eq!(s.magic_effects, 1);
        assert!(!ev2.contains(&InboundEvent::WalkAck));
    }

    fn update_tile_thing_item(pos: Position, stackpos: u8, client_id: u16) -> Vec<u8> {
        let mut p = vec![OP_UPDATE_TILE_THING];
        p.extend_from_slice(&pos.x.to_le_bytes());
        p.extend_from_slice(&pos.y.to_le_bytes());
        p.push(pos.z);
        p.push(stackpos);
        p.extend_from_slice(&client_id.to_le_bytes());
        p
    }

    fn leftover_skip_tile_bytes() -> Vec<u8> {
        // `20260920T010424Z` peek `2f11711100ff` after `0x68`.
        let mut p = Vec::new();
        p.extend_from_slice(&0x112Fu16.to_le_bytes());
        p.extend_from_slice(&0x1171u16.to_le_bytes());
        p.extend_from_slice(&0xFF00u16.to_le_bytes());
        p
    }

    #[test]
    fn floor_down_zero_floors_then_update_tile_thing() {
        // `20260920T010424Z`: 0-floor `0xBF` then `0x6B` (peek `6bfa7d…`).
        let pos = Position::new(0x7DFA, 0x7D8A, 8);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(pos),
            ..InboundState::default()
        };
        let mut p = vec![OP_FLOOR_DOWN];
        p.extend_from_slice(&update_tile_thing_item(
            Position::new(0x7DFA, 0x7D8A, 7),
            2,
            100,
        ));
        p.push(OP_PING);
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.pos, Some(pos));
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_row_leftover_skip_tile_then_magic() {
        // `20260920T010424Z`: counted `0x68` skip then leftover client-id tile.
        let orig = Position::new(100, 100, 8);
        let dest = Position::new(99, 100, 8);
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        p.extend_from_slice(&leftover_skip_tile_bytes());
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.pos, Some(dest));
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    fn player_stats_bytes() -> Vec<u8> {
        let mut p = vec![OP_PLAYER_STATS];
        p.extend_from_slice(&[0u8; PLAYER_STATS_LEN]);
        p
    }

    #[test]
    fn west_row_leftover_skip_pair_then_magic() {
        // `20260920T011720Z`: trailing `[0x68, 0xFF]` is leftover empty tiles, not SendRow.
        let orig = Position::new(100, 100, 8);
        let dest = Position::new(99, 100, 8);
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let mut p = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        p.push(OP_MAP_WEST);
        p.push(0xFF);
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            ..InboundState::default()
        };
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.magic_effects, 1);
        assert!(ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn player_stats_then_leftover_skip_tile_then_ping() {
        // `20260920T011720Z`: unread skip-stream after `0xA0` (peek `0400ff…`).
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 8)),
            ..InboundState::default()
        };
        let mut p = player_stats_bytes();
        p.extend_from_slice(&leftover_skip_tile_bytes());
        p.push(OP_PING);
        let ev = s.feed(&p);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    fn test22_say_body() -> Vec<u8> {
        // `20260920T013342Z` peek `00000600Test22…01` + pos + `bots bots bots`.
        let pos = Position::new(0x7E6A, 0x7DF2, 7);
        let mut p = Vec::new();
        p.extend_from_slice(&0u16.to_le_bytes());
        p.extend_from_slice(&6u16.to_le_bytes());
        p.extend_from_slice(b"Test22");
        p.push(1); // TALKTYPE_SAY
        p.extend_from_slice(&pos.x.to_le_bytes());
        p.extend_from_slice(&pos.y.to_le_bytes());
        p.push(pos.z);
        p.extend_from_slice(&14u16.to_le_bytes());
        p.extend_from_slice(b"bots bots bots");
        p
    }

    #[test]
    fn player_stats_then_ff_skip_stream_then_ping() {
        // `20260920T013342Z`: leftover `0xFF` then skip-stream after `0xA0` (peek `ff2a11…`).
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 8)),
            ..InboundState::default()
        };
        let mut p = player_stats_bytes();
        p.push(0xFF);
        p.extend_from_slice(&leftover_skip_tile_bytes());
        p.push(OP_PING);
        let ev = s.feed(&p);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_test22_say_then_ping() {
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 7)),
            ..InboundState::default()
        };
        let mut p = vec![OP_MAP_WEST];
        p.extend_from_slice(&test22_say_body());
        p.push(OP_PING);
        let ev = s.feed(&p);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    fn test742_say_body() -> Vec<u8> {
        // `20260920T031649Z` peek `4b000700Test742…` at z=7.
        let pos = Position::new(0x7E71, 0x7DF1, 7);
        let mut p = Vec::new();
        p.extend_from_slice(&0x004Bu16.to_le_bytes());
        p.extend_from_slice(&7u16.to_le_bytes());
        p.extend_from_slice(b"Test742");
        p.push(1); // TALKTYPE_SAY
        p.extend_from_slice(&pos.x.to_le_bytes());
        p.extend_from_slice(&pos.y.to_le_bytes());
        p.push(pos.z);
        p.extend_from_slice(&14u16.to_le_bytes());
        p.extend_from_slice(b"we hunt as one");
        p
    }

    #[test]
    fn north_opcode_then_test742_say_then_ping() {
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 7)),
            ..InboundState::default()
        };
        let mut p = vec![OP_MAP_NORTH];
        p.extend_from_slice(&test742_say_body());
        p.push(OP_PING);
        let ev = s.feed(&p);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_orphan_skip_count_then_west_then_ping() {
        // `20260920T014718Z` peek `0400ff68…` at z=8: orphan `04` + `[00, FF]` + SendWest.
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 8)),
            ..InboundState::default()
        };
        let ev = s.feed(&[OP_MAP_WEST, 0x04, 0x00, 0xFF, OP_MAP_WEST, OP_PING]);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_orphan_skip_tile_then_west_then_ping() {
        // `20260920T021501Z` peek `00711100ff68…` at z=8: orphan `00` + id `0x1171`
        // + `[00, FF]` + SendWest.
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 8)),
            ..InboundState::default()
        };
        let ev = s.feed(&[
            OP_MAP_WEST,
            0x00,
            0x71,
            0x11,
            0x00,
            0xFF,
            OP_MAP_WEST,
            OP_PING,
        ]);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_orphan_multi_tile_skip_stream_then_west_then_ping() {
        // `20260920T022857Z` peek `043d0400ff36…` at z=8: orphan `04` then two
        // leftover skip-tiles, then SendWest (next after first tile is `0x1136`,
        // not a known opcode).
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 8)),
            ..InboundState::default()
        };
        let ev = s.feed(&[
            OP_MAP_WEST,
            0x04,
            0x3D,
            0x04,
            0x00,
            0xFF,
            0x36,
            0x11,
            0x53,
            0x04,
            0x00,
            0xFF,
            OP_MAP_WEST,
            OP_PING,
        ]);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_orphan_skip_tile_eof() {
        // `20260920T023807Z` peek `043d0428ff` at z=8: nibble `04` + id `0x043d`
        // + skip pair `[0x28, 0xFF]` then eof.
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 8)),
            ..InboundState::default()
        };
        let ev = s.feed(&[OP_MAP_WEST, 0x04, 0x3D, 0x04, 0x28, 0xFF]);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert!(ev.is_empty());
    }

    #[test]
    fn west_opcode_then_leftover_skip_tile_starting_with_container_open_byte() {
        // `20260920T025706Z` peek `6e02b21200ff65…` at z=1: leftover skip-tile
        // client id `0x026E` (low byte is `OP_CONTAINER_OPEN`) then `[00, FF]`
        // then SendNorth leftover tile then ping. Do not 0-floor into container.
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 1)),
            ..InboundState::default()
        };
        let ev = s.feed(&[
            OP_MAP_WEST,
            0x6E,
            0x02,
            0xB2,
            0x12,
            0x00,
            0xFF,
            OP_MAP_NORTH,
            0x01,
            0x00,
            0xFF,
            OP_PING,
        ]);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_real_container_open_then_ping() {
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 1)),
            ..InboundState::default()
        };
        let mut p = vec![OP_MAP_WEST, OP_CONTAINER_OPEN, 0];
        p.extend_from_slice(&0x0ABCu16.to_le_bytes());
        p.extend_from_slice(&3u16.to_le_bytes());
        p.extend_from_slice(b"bag");
        p.push(8);
        p.push(0);
        p.push(0);
        p.push(OP_PING);
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_creature_say_then_container_remove_then_ping() {
        // `20260920T032906Z`: 0-floor `0x68` then real `0xAA` `"we are a bot swarm"`
        // then `0x72`. Skip-tile must not 10-cap the say and stop on `'r'`/`0x72`.
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 7)),
            ..InboundState::default()
        };
        let pos = Position::new(0x7E71, 0x7DF1, 7);
        let mut p = vec![OP_MAP_WEST];
        p.extend_from_slice(&creature_say_say("Test742", pos, "we are a bot swarm"));
        p.extend_from_slice(&[OP_CONTAINER_REMOVE, 0, 0, OP_PING]);
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.unknown_opcode_peek, None);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn west_opcode_then_leftover_container_remove_opcode_at_eof() {
        // `20260920T034532Z`: skip=0 discarded=1 unknown=0. 0-floor `0x68` then
        // leftover `0x72` (skip-stream / truncated) with no cid/slot.
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 7)),
            ..InboundState::default()
        };
        let ev = s.feed(&[OP_MAP_WEST, OP_CONTAINER_REMOVE]);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert!(ev.is_empty());
    }

    #[test]
    fn lone_ping_is_not_leftover_eof_opcode() {
        let mut s = InboundState::default();
        let ev = s.feed(&[OP_PING]);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
    }

    #[test]
    fn west_opcode_then_ping_is_zero_floor_row() {
        let mut s = InboundState {
            pos: Some(Position::new(100, 100, 8)),
            ..InboundState::default()
        };
        let ev = s.feed(&[OP_MAP_WEST, OP_PING]);
        assert_eq!(s.skip_failures, 0, "peek={:?}", s.skip_failure_first_peek);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(ev, vec![InboundEvent::Ping(OP_PING)]);
    }

    #[test]
    fn floor_down_zero_floors_then_magic_skips() {
        // `20260919T223936Z`: 0-floor `0xBF` then a top-level opcode (not 3 floors at z=8).
        let pos = Position::new(100, 100, 8);
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(pos),
            ..InboundState::default()
        };
        let mut p = vec![OP_FLOOR_DOWN];
        p.extend_from_slice(&magic_effect_bytes(pos, 11));
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(s.pos, Some(pos));
        assert_eq!(s.magic_effects, 1);
        assert!(!ev.contains(&InboundEvent::WalkAck));
    }

    #[test]
    fn floor_up_zero_floors_then_map_south_skips() {
        // Encoder omitted floor bytes; next is `SendRow` `0x67` (`20260919T223936Z` peek).
        let orig = Position::new(100, 100, 8);
        let dest = Position::new(100, 101, 8);
        let mut known = KnownCreatureTable::default();
        let mut get_tile = |_x: i32, _y: i32, _z: i32| -> Option<TileContent> { None };
        let mut can_see = |_id: u32| true;
        let full = send_notify_go(
            &codec_772(),
            orig,
            dest,
            0,
            1,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        )
        .into_bytes();
        let south = full
            .iter()
            .position(|&b| b == OP_MAP_SOUTH)
            .expect("0x67 after same-z south");
        let mut p = vec![OP_FLOOR_UP];
        p.extend_from_slice(&full[south..]);
        p.extend_from_slice(&magic_effect_bytes(dest, 11));
        let mut s = InboundState {
            self_id: Some(1),
            pos: Some(orig),
            awaiting_move_up_one_floor: true,
            ..InboundState::default()
        };
        let ev = s.feed(&p);
        assert_eq!(
            s.skip_failures, 0,
            "peek={:?} z={:?}",
            s.skip_failure_first_peek, s.skip_failure_player_z
        );
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
        assert_eq!(
            s.pos,
            Some(orig),
            "0-floor 0xBE must not bump when dest already applied"
        );
        assert_eq!(s.magic_effects, 1);
        assert!(!ev.contains(&InboundEvent::WalkAck));
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
                    pos: Position::new(1, 2, 7),
                    effect: 11
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

    fn text_message_bytes(text: &str) -> Vec<u8> {
        let mut p = vec![OP_TEXT_MESSAGE, 0x17];
        p.extend_from_slice(&(text.len() as u16).to_le_bytes());
        p.extend_from_slice(text.as_bytes());
        p
    }

    #[test]
    fn failure_text_emits_spell_rejected() {
        let mut s = InboundState::default();
        let ev = s.feed(&text_message_bytes("You are exhausted."));
        assert_eq!(ev, vec![InboundEvent::SpellRejected]);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn walk_sorry_text_does_not_steal_spell_fifo() {
        let mut s = InboundState::default();
        let ev = s.feed(&text_message_bytes("Sorry, not possible."));
        assert!(ev.is_empty());
        assert_eq!(
            s.text_reject_counts.get("Sorry, not possible.").copied(),
            Some(1)
        );
    }

    #[test]
    fn walk_sorry_then_snapback_emits_rejected_then_cancel() {
        let mut s = InboundState::default();
        let mut p = text_message_bytes("Sorry, not possible.");
        p.push(OP_CANCEL_WALK);
        p.push(0);
        let ev = s.feed(&p);
        assert_eq!(
            ev,
            vec![InboundEvent::WalkRejected, InboundEvent::CancelWalk]
        );
    }

    #[test]
    fn throw_text_then_snapback_does_not_emit_walk_rejected() {
        let mut s = InboundState::default();
        let mut p = text_message_bytes("You cannot throw there.");
        p.push(OP_CANCEL_WALK);
        p.push(0);
        let ev = s.feed(&p);
        assert_eq!(ev, vec![InboundEvent::CancelWalk]);
        assert_eq!(
            s.text_reject_counts.get("You cannot throw there.").copied(),
            Some(1)
        );
    }

    #[test]
    fn broadcast_text_is_not_a_rejection() {
        let mut s = InboundState::default();
        let ev = s.feed(&text_message_bytes("Beware, beware the halloween hare."));
        assert!(ev.is_empty());
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn magic_effect_carries_effect_id() {
        let mut s = InboundState::default();
        let ev = s.feed(&magic_effect_bytes(Position::new(1, 2, 7), 7));
        assert_eq!(
            ev,
            vec![InboundEvent::MagicEffect {
                pos: Position::new(1, 2, 7),
                effect: 7,
            }]
        );
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
                    pos: Position::new(1, 2, 7),
                    effect: 11,
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
        assert_eq!(s.unknown_opcode_counts.get(&0x15).copied(), Some(1));
        assert_eq!(s.unknown_opcode_prev, None);
        assert_eq!(s.unknown_opcode_player_z, Some(7));
        let peek = s.unknown_opcode_peek.as_deref().unwrap_or("");
        assert!(peek.starts_with("15"), "peek={peek}");
        assert!(s.bytes_discarded > 0);
    }

    #[test]
    fn skip_failed_records_opcode_histogram() {
        let mut s = InboundState::default();
        // 0x6A + pos, then a truncated thing (one byte) so skip_772_thing fails.
        let mut p = vec![OP_ADD_TILE_THING];
        p.extend_from_slice(&1u16.to_le_bytes());
        p.extend_from_slice(&2u16.to_le_bytes());
        p.push(7);
        p.push(0x01);
        let ev = s.feed(&p);
        assert!(ev.is_empty());
        assert_eq!(s.skip_failures, 1);
        assert_eq!(
            s.skip_failure_opcodes.get(&OP_ADD_TILE_THING).copied(),
            Some(1)
        );
        assert!(s.bytes_discarded > 0);
        assert!(s.skip_failure_first_peek.is_some());
    }

    #[test]
    fn container_open_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_CONTAINER_OPEN, 0];
        p.extend_from_slice(&0x0ABCu16.to_le_bytes()); // header item, no extra
        p.extend_from_slice(&3u16.to_le_bytes());
        p.extend_from_slice(b"bag");
        p.push(8); // capacity
        p.push(0); // hasParent
        p.push(0); // count
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
        assert_eq!(s.unknown_opcodes, 0);
    }

    #[test]
    fn channels_dialog_then_magic_effect() {
        let mut s = InboundState::default();
        let mut p = vec![OP_CHANNELS_DIALOG, 1];
        p.extend_from_slice(&5u16.to_le_bytes());
        p.extend_from_slice(&5u16.to_le_bytes());
        p.extend_from_slice(b"Trade");
        p.extend_from_slice(&magic_effect_bytes(Position::new(1, 2, 7), 11));
        let ev = s.feed(&p);
        assert_eq!(s.magic_effects, 1);
        assert_eq!(ev.len(), 1);
        assert_eq!(s.bytes_discarded, 0);
    }

    #[test]
    fn failure_text_counts_histogram() {
        let mut s = InboundState::default();
        let ev = s.feed(&text_message_bytes("You are exhausted."));
        assert_eq!(ev, vec![InboundEvent::SpellRejected]);
        assert_eq!(
            s.text_reject_counts.get("You are exhausted.").copied(),
            Some(1)
        );
    }
}
