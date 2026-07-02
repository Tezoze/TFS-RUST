//! Spectator visibility, protocol known-set, and outgoing packet fan-out.
//!
//! - `ProtocolGame::canSee` — `protocolgame.cpp`.
//! - `Creature::canSeeCreature` — `creature.cpp` / `player.cpp`.
//! - `Game::internalCreatureSay`, magic effect broadcasts — `game.cpp`.

use std::collections::{HashMap, HashSet};

use slotmap::Key;
use tfs_rust_common::enums::ConditionType;
use tfs_rust_common::protocol_constants::{MAX_CLIENT_VIEWPORT_X, MAX_CLIENT_VIEWPORT_Y};
use tfs_rust_common::{ConnId, Position};
use tfs_rust_net::codec::ItemTemplateArgs;
use tfs_rust_net::NetworkMessage;

use crate::condition::ActiveCondition;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;

/// C++ `ProtocolGame::canSee(int32_t x, int32_t y, int32_t z)` — `protocolgame.cpp` ~796–823.
pub fn protocol_can_see(viewer_pos: Position, target: Position) -> bool {
    let my_x = i32::from(viewer_pos.x);
    let my_y = i32::from(viewer_pos.y);
    let my_z = i32::from(viewer_pos.z);
    let x = i32::from(target.x);
    let y = i32::from(target.y);
    let z = i32::from(target.z);

    if my_z <= 7 {
        if z > 7 {
            return false;
        }
    } else if (my_z - z).abs() > 2 {
        return false;
    }

    let offsetz = my_z - z;
    let min_x = my_x - MAX_CLIENT_VIEWPORT_X + offsetz;
    let max_x = my_x + (MAX_CLIENT_VIEWPORT_X + 1) + offsetz;
    let min_y = my_y - MAX_CLIENT_VIEWPORT_Y + offsetz;
    let max_y = my_y + (MAX_CLIENT_VIEWPORT_Y + 1) + offsetz;

    (min_x..=max_x).contains(&x) && (min_y..=max_y).contains(&y)
}

/// C++ `Creature::canSee(myPos, pos, viewRangeX, viewRangeY)` — `creature.cpp` ~45–66.
/// Monster target list / follow use `Map::maxViewportX` / `maxViewportY` (11), not client viewport.
///
/// `beat_driven_loop` selects the era's underground Z-rule:
/// - **false (1098 / TFS `canSee`):** underground viewers cannot see surface (`tz < 8` rejects).
/// - **true (772 `TConnection::IsVisible`, `connections.cc:357-378`):** underground viewers CAN see
///   surface within 2 floors — only `abs(dz) > 2` rejects (audit M5).
pub fn creature_can_see(
    viewer_pos: Position,
    target: Position,
    view_range_x: i32,
    view_range_y: i32,
    beat_driven_loop: bool,
) -> bool {
    let my_z = i32::from(viewer_pos.z);
    let tz = i32::from(target.z);

    if my_z <= 7 {
        if tz > 7 {
            return false;
        }
    } else if my_z >= 8 {
        // 772 `IsVisible` (`connections.cc:363-366`) has no `tz < 8` rejection — underground CAN
        // see surface within ±2 floors. 1098/TFS `canSee` keeps the surface rejection.
        if !beat_driven_loop && tz < 8 {
            return false;
        }
        if (my_z - tz).abs() > 2 {
            return false;
        }
    }

    let offsetz = my_z - tz;
    let my_x = i32::from(viewer_pos.x);
    let my_y = i32::from(viewer_pos.y);
    let tx = i32::from(target.x);
    let ty = i32::from(target.y);

    tx >= my_x - view_range_x + offsetz
        && tx <= my_x + view_range_x + offsetz
        && ty >= my_y - view_range_y + offsetz
        && ty <= my_y + view_range_y + offsetz
}

impl GameWorld {
    /// TFS `ProtocolGame::canSee(Position)` — multi-floor viewport (`protocolgame.cpp` ~796–823).
    pub fn can_see_position(&self, viewer: CreatureId, pos: Position) -> bool {
        let Some(viewer_pos) = self.creatures.get(viewer).map(|k| k.position()) else {
            return false;
        };
        protocol_can_see(viewer_pos, pos)
    }

    /// Collect all `ConnId`s whose creature can see `pos`. Used by every broadcast.
    fn spectator_conns(&self, pos: Position) -> Vec<ConnId> {
        self.conn_to_creature
            .iter()
            .filter(|(_, &vid)| self.can_see_position(vid, pos))
            .map(|(&c, _)| c)
            .collect()
    }

    /// Enqueue the same packet bytes for every connection that can see `pos` (clone per viewer).
    // C++ ref: repeated `ProtocolGame` fan-out in `game.cpp` / `protocolgame.cpp`.
    pub(crate) fn broadcast_to_spectators(&mut self, pos: Position, packet: Vec<u8>) {
        let conns = self.spectator_conns(pos);
        for conn in conns {
            self.enqueue_outgoing(conn, packet.clone());
        }
    }

    /// C++ `++statementId` before each `sendCreatureSay` / related speech packet.
    pub fn alloc_statement_id(&mut self) -> u32 {
        self.next_statement_id = self.next_statement_id.wrapping_add(1);
        self.next_statement_id
    }

    /// TFS `Game::internalCreatureSay` — one `ProtocolGame::sendCreatureSay` per viewer in range (`game.cpp` ~3723–3758).
    pub fn broadcast_creature_say_viewport(
        &mut self,
        speaker: CreatureId,
        speak_type: u8,
        text: &str,
    ) {
        // C++ `Game::internalCreatureSay` — `game.cpp` / `gameserver/src/game.cpp`.
        // Era-aware wire: 1098 writes `name + u16 level + speakType + pos + text`;
        // 772 omits `level` (`gameserver/src/protocolgame.cpp:1422`).
        use tfs_rust_net::codec::wire::CreatureSayWire;
        let (pos, name, level) = match self.creatures.get(speaker) {
            Some(CreatureKind::Player(p)) => (p.base.position, p.base.name.clone(), p.level as u16),
            // Monster talk — 772 `crnonpl.cc:2458` `Talk(this->ID, Mode, NULL, Text, false)`.
            // Level is unused on 772 wire (codec omits it); 1098 monsters don't talk.
            Some(CreatureKind::Monster(m)) => (m.base.position, m.base.name.clone(), 0),
            _ => return,
        };
        let viewers: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .map(|(&conn, &viewer)| (conn, viewer))
            .collect();
        for (conn, viewer) in viewers {
            if self.can_see_position(viewer, pos) {
                let sid = self.alloc_statement_id();
                let pkt = self.codec.encode_creature_say(
                    sid,
                    &CreatureSayWire {
                        speaker_name: name.clone(),
                        level,
                        speak_type,
                        pos,
                        text: text.into(),
                    },
                );
                self.enqueue_outgoing(conn, pkt.into_bytes());
            }
        }
    }

    /// Queue raw packet bytes for a connection (built by `tfs-rust-net` outgoing helpers).
    pub fn enqueue_outgoing(&mut self, conn: ConnId, packet: Vec<u8>) {
        // A codec may produce an empty packet for an opcode with no equivalent in the active era
        // (e.g. 7.72 has no `sendBasicData` / by-id tile removal). Drop those so the framing layer
        // never emits a zero-length body. 10.98 never enqueues an empty packet, so this is a no-op
        // there.
        if packet.is_empty() {
            return;
        }
        self.pending_outgoing.entry(conn).or_default().push(packet);
    }

    pub fn enqueue_encoded(&mut self, conn: ConnId, msg: NetworkMessage) {
        self.enqueue_outgoing(conn, msg.into_bytes());
    }

    /// Drain all queued outgoing packets at end of tick; IO layer sends each blob in order per connection.
    pub fn flush_output_buffers(&mut self) -> HashMap<ConnId, Vec<Vec<u8>>> {
        std::mem::take(&mut self.pending_outgoing)
    }

    /// Broadcast a magic effect to all spectators at a position.
    // C++ ref: src/game.cpp:4816 Game::addMagicEffect
    pub fn broadcast_magic_effect(&mut self, pos: Position, effect_id: u8) {
        use tfs_rust_net::codec::wire::MagicEffectWire;
        let pkt = self
            .codec
            .encode_magic_effect(&MagicEffectWire { pos, effect_id })
            .into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }

    /// C++ `Game::addDistanceEffect` — projectile from caster to target tile.
    pub fn broadcast_distance_shoot(&mut self, from: Position, to: Position, shoot_type: u8) {
        use tfs_rust_net::codec::wire::DistanceShootWire;
        let pkt = self
            .codec
            .encode_distance_shoot(&DistanceShootWire {
                from,
                to,
                shoot_type,
            })
            .into_bytes();
        self.broadcast_to_spectators(from, pkt);
    }

    /// C++ `Game::combatChangeHealth` — stats + damage message + health bar fan-out.
    pub(crate) fn notify_player_combat_damage(
        &mut self,
        attacker_id: Option<CreatureId>,
        target_id: CreatureId,
        damage_done: i32,
    ) {
        if damage_done <= 0 {
            return;
        }
        let (pos, wire_id, hp_pct) = {
            let Some(CreatureKind::Player(p)) = self.creatures.get(target_id) else {
                return;
            };
            let max_hp = p.base.max_health.max(1);
            let pct = ((p.base.health.max(0) as u64 * 100) / max_hp as u64).min(100) as u8;
            (
                p.base.position,
                crate::login_out::creature_wire_id(
                    target_id,
                    self.creatures.get(target_id).unwrap(),
                ),
                pct,
            )
        };

        self.send_player_stats(target_id);

        let attacker_desc = attacker_id
            .and_then(|aid| self.creatures.get(aid))
            .map(|k| k.base().name.clone());

        const TEXTCOLOR_RED: u8 = 180;
        const CONST_ME_DRAWBLOOD: u8 = 1;

        // 772 emits the (race-keyed) hit effect + splash via `apply_physical_hit_blood_772` in the
        // combat apply path (`crmain.cc:762-775`), so avoid a duplicate draw-blood here. 1098 keeps
        // the effect on this player-notify path (`game.cpp` combatGetTypeInfo is not yet wired).
        if !self.beat_driven_loop {
            self.broadcast_magic_effect(pos, CONST_ME_DRAWBLOOD);
        }

        use tfs_rust_net::codec::wire::{
            AnimatedTextWire, CombatDamageNotifyWire, CreatureHealthWire,
        };

        let animated = self.codec.encode_animated_text(&AnimatedTextWire {
            pos,
            color: TEXTCOLOR_RED,
            text: damage_done.to_string(),
        });
        if !animated.as_bytes().is_empty() {
            self.broadcast_to_spectators(pos, animated.into_bytes());
        }

        if let Some(conn) = self.conn_for_creature(target_id) {
            let dmg = damage_done as u32;
            let text = if self.beat_driven_loop {
                let damage_string = if dmg == 1 {
                    "1 hitpoint".to_string()
                } else {
                    format!("{dmg} hitpoints")
                };
                if let Some(attacker) = attacker_desc {
                    format!("You lose {damage_string} due to an attack by {attacker}.")
                } else {
                    format!("You lose {damage_string}.")
                }
            } else if dmg == 1 {
                "You lose 1 hitpoint.".to_string()
            } else {
                format!("You lose {dmg} hitpoints.")
            };
            self.enqueue_encoded(
                conn,
                self.codec
                    .encode_combat_damage_text_message(&CombatDamageNotifyWire {
                        pos,
                        damage: dmg,
                        damage_color: TEXTCOLOR_RED,
                        text,
                    }),
            );
        }

        self.broadcast_to_spectators(
            pos,
            self.codec
                .encode_creature_health(&CreatureHealthWire {
                    creature_id: wire_id,
                    health_percent: hp_pct,
                })
                .into_bytes(),
        );
    }

    /// Strip wire ids from `known` that this conn never received as a full `AddCreature` block.
    /// C++ `ProtocolGame::knownCreatureSet` only marks known after the client got full data.
    pub fn reconcile_known_creatures_for_send(&self, conn_id: ConnId, known: &mut HashSet<u32>) {
        let Some(sent) = self.creature_fully_sent_by_conn.get(&conn_id) else {
            return;
        };
        known.retain(|id| sent.contains(id));
    }

    /// Persist post-packet known set and record all ids as fully sent to this conn.
    pub fn commit_known_creatures_after_send(&mut self, conn_id: ConnId, known: &HashSet<u32>) {
        self.known_creatures_by_conn.insert(conn_id, known.clone());
        self.creature_fully_sent_by_conn
            .insert(conn_id, known.clone());
    }

    /// Record one wire id as fully sent (e.g. after `0x6A` tile appear).
    pub fn mark_creature_fully_sent(&mut self, conn_id: ConnId, wire_id: u32) {
        self.creature_fully_sent_by_conn
            .entry(conn_id)
            .or_default()
            .insert(wire_id);
    }

    /// Whether `viewer` may treat `target_protocol_id` as “seen” for `knownCreatureSet` eviction.
    /// C++: `ProtocolGame::canSee` / `Player::canSeeCreature` (`protocolgame.cpp` ~778+).
    pub fn can_see_creature_for_known_set(
        &self,
        viewer: CreatureId,
        target_protocol_id: u32,
    ) -> bool {
        if self.player_guid(viewer) == Some(target_protocol_id) {
            return true;
        }
        for (cid, k) in self.creatures.iter() {
            let wire_id = match k {
                CreatureKind::Player(p) => p.guid,
                CreatureKind::Monster(_) | CreatureKind::Npc(_) => {
                    (cid.data().as_ffi() & 0xFFFF_FFFF) as u32
                }
            };
            if wire_id != target_protocol_id {
                continue;
            }
            return self.can_see_creature(viewer, cid);
        }
        true
    }

    /// C++ `Creature::canSeeCreature` / `Player::canSeeCreature` — ghost mode + invisibility.
    /// `creature.cpp` ~74, `player.cpp` ~715–726.
    pub fn can_see_creature(&self, viewer: CreatureId, target: CreatureId) -> bool {
        if viewer == target {
            return true;
        }
        let Some(target_kind) = self.creatures.get(target) else {
            return false;
        };
        if let CreatureKind::Player(tp) = target_kind {
            if tp.ghost_mode {
                let viewer_has_access = self
                    .creatures
                    .get(viewer)
                    .and_then(|k| match k {
                        CreatureKind::Player(p) => Some(p.ghost_mode),
                        _ => None,
                    })
                    .unwrap_or(false);
                if !viewer_has_access {
                    return false;
                }
            }
        }
        // C++ `Player::canSeeCreature` — invisibility only hides non-players from viewers without `canSeeInvisibility`.
        if !matches!(target_kind, CreatureKind::Player(_))
            && Self::has_invisible(&target_kind.base().active_conditions)
        {
            return false;
        }
        true
    }

    fn has_invisible(conditions: &[ActiveCondition]) -> bool {
        conditions
            .iter()
            .any(|c| c.ctype == ConditionType::Invisible)
    }

    pub(crate) fn player_guid(&self, cid: CreatureId) -> Option<u32> {
        self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.guid),
            _ => None,
        })
    }

    pub(crate) fn send_cancel_message(&mut self, conn_id: ConnId, rv: ReturnValue) {
        use tfs_rust_net::outgoing_extra::send_text_message_simple;
        const MESSAGE_STATUS_SMALL: u8 = 0x15;
        let msg = rv.description();
        self.enqueue_outgoing(
            conn_id,
            send_text_message_simple(MESSAGE_STATUS_SMALL, msg).into_bytes(),
        );
    }

    // === B.6: Tile item change broadcasts ===
    // C++ ref: src/protocolgame.cpp sendAddTileItem (~2605), sendUpdateTileItem (~2619),
    //          sendRemoveTileThing (~2633)

    /// Broadcast `sendAddTileItem` (0x6A) to all spectators.
    pub(crate) fn broadcast_tile_item_add(
        &mut self,
        pos: Position,
        item_id: ItemId,
        stack_pos: u8,
    ) {
        let (client_id, count, stackable, is_splash_or_fluid, is_animation) =
            match self.items.get(item_id) {
                Some(item) => {
                    let it = self.items_db.items.get(&item.item_type);
                    (
                        it.map(|t| t.client_id).unwrap_or(0),
                        item.client_count(),
                        it.map(|t| t.stackable()).unwrap_or(false),
                        it.map(|t| t.is_splash() || t.is_fluid_container())
                            .unwrap_or(false),
                        it.map(|t| t.is_animation()).unwrap_or(false),
                    )
                }
                None => return,
            };
        let args = ItemTemplateArgs {
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description: false,
        };
        for conn in self.spectator_conns(pos) {
            let pkt = self
                .codec
                .encode_add_tile_item(pos, stack_pos, args, false)
                .into_bytes();
            self.enqueue_outgoing(conn, pkt);
        }
    }

    /// Broadcast `sendUpdateTileItem` (0x6B) to all spectators.
    pub(crate) fn broadcast_tile_item_update(
        &mut self,
        pos: Position,
        item_id: ItemId,
        stack_pos: u8,
    ) {
        let (client_id, count, stackable, is_splash_or_fluid, is_animation) =
            match self.items.get(item_id) {
                Some(item) => {
                    let it = self.items_db.items.get(&item.item_type);
                    (
                        it.map(|t| t.client_id).unwrap_or(0),
                        item.client_count(),
                        it.map(|t| t.stackable()).unwrap_or(false),
                        it.map(|t| t.is_splash() || t.is_fluid_container())
                            .unwrap_or(false),
                        it.map(|t| t.is_animation()).unwrap_or(false),
                    )
                }
                None => return,
            };
        let args = ItemTemplateArgs {
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description: false,
        };
        let pkt = self
            .codec
            .encode_update_tile_item(pos, stack_pos, args)
            .into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }

    /// Broadcast `sendRemoveTileThing` (0x6C) to all spectators.
    pub(crate) fn broadcast_tile_item_remove(&mut self, pos: Position, stack_pos: u8) {
        let pkt = self
            .codec
            .encode_remove_tile_thing(pos, stack_pos)
            .into_bytes();
        self.broadcast_to_spectators(pos, pkt);
    }
}

#[cfg(test)]
mod protocol_can_see_tests {
    use super::*;
    use tfs_rust_common::Position;

    #[test]
    fn same_floor_in_viewport() {
        let viewer = Position::new(100, 100, 7);
        let target = Position::new(105, 103, 7);
        assert!(protocol_can_see(viewer, target));
    }

    #[test]
    fn same_floor_outside_viewport() {
        let viewer = Position::new(100, 100, 7);
        let target = Position::new(120, 100, 7);
        assert!(!protocol_can_see(viewer, target));
    }

    #[test]
    fn surface_look_one_floor_below_same_xy() {
        let viewer = Position::new(100, 100, 7);
        let target = Position::new(100, 100, 6);
        assert!(protocol_can_see(viewer, target));
    }

    #[test]
    fn surface_cannot_see_underground() {
        let viewer = Position::new(100, 100, 7);
        let target = Position::new(100, 100, 8);
        assert!(!protocol_can_see(viewer, target));
    }

    #[test]
    fn underground_within_two_floors() {
        let viewer = Position::new(100, 100, 10);
        let target = Position::new(100, 100, 8);
        assert!(protocol_can_see(viewer, target));
    }

    #[test]
    fn underground_beyond_two_floors() {
        let viewer = Position::new(100, 100, 10);
        let target = Position::new(100, 100, 7);
        assert!(!protocol_can_see(viewer, target));
    }
}

#[cfg(test)]
mod creature_can_see_tests {
    use super::*;

    #[test]
    fn within_map_viewport_range() {
        let viewer = Position::new(100, 100, 8);
        let target = Position::new(110, 100, 8);
        assert!(creature_can_see(viewer, target, 11, 11, false));
    }

    #[test]
    fn outside_map_viewport_range() {
        let viewer = Position::new(100, 100, 8);
        let target = Position::new(130, 100, 8);
        assert!(!creature_can_see(viewer, target, 11, 11, false));
    }

    /// M5: 772 `TConnection::IsVisible` (`connections.cc:357-378`) — underground viewers CAN see
    /// surface within 2 floors (no `tz < 8` rejection). `beat_driven_loop = true`.
    #[test]
    fn creature_can_see_underground_to_surface() {
        let viewer = Position::new(100, 100, 9);
        let target = Position::new(100, 100, 7);
        // 772: dz = 2 → visible (within ±2).
        assert!(
            creature_can_see(viewer, target, 11, 11, true),
            "772 underground viewer must see surface within 2 floors (IsVisible connections.cc:357-378)"
        );
        // 1098: underground viewer cannot see surface (`tz < 8` rejection).
        assert!(
            !creature_can_see(viewer, target, 11, 11, false),
            "1098 underground viewer must NOT see surface (TFS canSee)"
        );
        // Both eras reject |dz| > 2.
        let too_far = Position::new(100, 100, 6);
        assert!(!creature_can_see(viewer, too_far, 11, 11, true));
        assert!(!creature_can_see(viewer, too_far, 11, 11, false));
    }
}
