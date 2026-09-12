//! Minimal 772 inbound: self id, own pos, move/remove, combat effects, ping.
//!
//! C++ reference: `codec/v772.rs` self-appear `0x0A`; `map_description.rs` `0x64` header
//! and `0x6D` move; `v772.rs` `0x6C` remove (no named server consts). Combat counters
//! (`0x83`/`0x84`/`0x85`/`0x8C`) feed the Tier 4 content-equivalence gate. Unknown
//! opcodes discard the rest of the decrypted payload (length-framed at TCP, counted here).

use std::collections::HashSet;

use tfs_rust_common::Position;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboundEvent {
    WalkAck,
    MagicEffect { pos: Position },
    Ping(u8),
    OtherCreature { id: u32 },
}

#[derive(Debug, Default)]
pub struct InboundState {
    pub self_id: Option<u32>,
    pub pos: Option<Position>,
    pub last_other_creature_id: Option<u32>,
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
}

impl InboundState {
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
        self.seen_creatures.extend(&other.seen_creatures);
    }

    fn note_creature(&mut self, id: u32) {
        if self.self_id == Some(id) {
            return;
        }
        self.seen_creatures.insert(id);
        self.last_other_creature_id = Some(id);
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
                    // Map body is not decoded — skip remainder of this payload.
                    self.bytes_discarded += (payload.len() - i) as u64;
                    break;
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
                            self.note_creature(cid);
                            self.other_creature_moves += 1;
                            events.push(InboundEvent::OtherCreature { id: cid });
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
                    self.note_creature(id);
                }
                OP_PING | OP_PING_BACK => {
                    events.push(InboundEvent::Ping(op));
                }
                _ => {
                    self.discard_rest(payload, i.saturating_sub(1));
                    break;
                }
            }
        }
        events
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

    fn discard_rest(&mut self, payload: &[u8], start: usize) {
        self.bytes_discarded += payload.len().saturating_sub(start) as u64;
    }
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
    use super::*;

    #[test]
    fn self_appear_then_map_header() {
        let mut s = InboundState::default();
        let mut p = vec![OP_SELF_APPEAR];
        p.extend_from_slice(&42u32.to_le_bytes());
        p.extend_from_slice(&50u16.to_le_bytes());
        p.push(0);
        p.push(OP_MAP_DESCRIPTION);
        p.extend_from_slice(&32369u16.to_le_bytes());
        p.extend_from_slice(&32241u16.to_le_bytes());
        p.push(7);
        p.extend_from_slice(&[0xAA, 0xBB]); // map body discarded
        let ev = s.feed(&p);
        assert!(ev.is_empty());
        assert_eq!(s.self_id, Some(42));
        assert_eq!(s.pos, Some(Position::new(32369, 32241, 7)));
        assert_eq!(s.bytes_discarded, 2);
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
        assert_eq!(ev, vec![InboundEvent::OtherCreature { id: 77 }]);
        assert_eq!(s.last_other_creature_id, Some(77));
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
}
