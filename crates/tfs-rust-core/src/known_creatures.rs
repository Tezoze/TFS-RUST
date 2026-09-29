//! Protocol known-creature table + reverse conn index for `AnnounceChangedCreature`.
//!
//! Corpus: `FirstKnowingConnection` then `IsVisible` — `operate.cc` `AnnounceChangedCreature`.
//! Forward map stays `known_creatures_by_conn`; reverse is `conns_by_known_wire`.

use std::collections::HashSet;

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_common::protocol_constants::{MAX_CLIENT_VIEWPORT_X, MAX_CLIENT_VIEWPORT_Y};
use tfs_rust_net::creature_known::KnownCreatureTable;

use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::creature_wire_id;
use crate::tile::{client_creature_stack_pos, client_creature_stack_pos_cip};

impl GameWorld {
    /// Drop reverse index entries for `conn` (call before removing the forward table).
    pub(crate) fn unindex_known_conn(&mut self, conn: ConnId) {
        if let Some(table) = self.known_creatures_by_conn.get(&conn) {
            for wire in table.iter() {
                if let Some(conns) = self.conns_by_known_wire.get_mut(&wire) {
                    conns.remove(&conn);
                    if conns.is_empty() {
                        self.conns_by_known_wire.remove(&wire);
                    }
                }
            }
        }
    }

    pub(crate) fn index_known_conn(&mut self, conn: ConnId, known: &KnownCreatureTable) {
        for wire in known.iter() {
            self.conns_by_known_wire
                .entry(wire)
                .or_default()
                .insert(conn);
        }
    }

    /// Take the known table for a map/move encode. Reverse index is cleared until
    /// [`Self::commit_known_creatures_after_send`].
    pub(crate) fn take_known_creatures_for_send(&mut self, conn: ConnId) -> KnownCreatureTable {
        self.unindex_known_conn(conn);
        self.known_creatures_by_conn
            .remove(&conn)
            .unwrap_or_default()
    }

    /// Drop known + fully-sent + reverse index for a disconnect / takeover.
    pub(crate) fn forget_known_creatures_for_conn(&mut self, conn: ConnId) {
        self.unindex_known_conn(conn);
        self.known_creatures_by_conn.remove(&conn);
        self.creature_fully_sent_by_conn.remove(&conn);
        self.creatures_on_client.remove(&conn);
    }

    pub(crate) fn creature_is_on_client(&self, conn: ConnId, wire_id: u32) -> bool {
        self.creatures_on_client
            .get(&conn)
            .is_some_and(|s| s.contains(&wire_id))
    }

    pub(crate) fn note_creature_on_client(&mut self, conn: ConnId, wire_id: u32) {
        self.creatures_on_client
            .entry(conn)
            .or_default()
            .insert(wire_id);
    }

    pub(crate) fn forget_creature_on_client(&mut self, conn: ConnId, wire_id: u32) {
        if let Some(set) = self.creatures_on_client.get_mut(&conn) {
            set.remove(&wire_id);
        }
    }

    /// Map shift drops a column without `0x6C`. Those ids must leave the drawn set
    /// or the next step is a `0x6D` onto empty pavement.
    pub(crate) fn forget_client_creatures_outside_view(
        &mut self,
        conn: ConnId,
        viewer: CreatureId,
    ) {
        let Some(set) = self.creatures_on_client.get(&conn).cloned() else {
            return;
        };
        let drop: Vec<u32> = set
            .into_iter()
            .filter(|wire| !self.client_still_shows(viewer, *wire))
            .collect();
        for wire in drop {
            self.forget_creature_on_client(conn, wire);
        }
    }

    /// Full `0x64` replaces the client map. Keep only bodies that description drew.
    pub(crate) fn replace_client_bodies_with_viewport(&mut self, conn: ConnId, viewer: CreatureId) {
        self.creatures_on_client.insert(conn, HashSet::new());
        self.note_client_bodies_where(conn, viewer, |_| true);
    }

    /// After an adjacent viewer step: drop the discarded strip, then mark the new one.
    pub(crate) fn sync_client_bodies_after_viewer_step(
        &mut self,
        conn: ConnId,
        viewer: CreatureId,
        old_pos: Position,
        new_pos: Position,
        replaced_map: bool,
    ) {
        if replaced_map {
            self.replace_client_bodies_with_viewport(conn, viewer);
            return;
        }
        self.forget_client_creatures_outside_view(conn, viewer);
        let px = i32::from(new_pos.x);
        let py = i32::from(new_pos.y);
        let pz = i32::from(new_pos.z);
        let min_x = px - MAX_CLIENT_VIEWPORT_X;
        let min_y = py - MAX_CLIENT_VIEWPORT_Y;
        let width = MAX_CLIENT_VIEWPORT_X * 2 + 2;
        let height = MAX_CLIENT_VIEWPORT_Y * 2 + 2;
        if old_pos.y > new_pos.y {
            self.note_client_bodies_in_rect(conn, viewer, min_x, min_y, width, 1, pz);
        } else if old_pos.y < new_pos.y {
            self.note_client_bodies_in_rect(conn, viewer, min_x, min_y + height - 1, width, 1, pz);
        }
        if old_pos.x < new_pos.x {
            self.note_client_bodies_in_rect(conn, viewer, min_x + width - 1, min_y, 1, height, pz);
        } else if old_pos.x > new_pos.x {
            self.note_client_bodies_in_rect(conn, viewer, min_x, min_y, 1, height, pz);
        }
    }

    fn client_still_shows(&self, viewer: CreatureId, wire_id: u32) -> bool {
        let Some(cid) = self.creature_by_wire_id(wire_id) else {
            return false;
        };
        if !self.viewer_has_creature_on_client(viewer, cid) {
            return false;
        }
        let Some(pos) = self.creatures.get(cid).map(|k| k.position()) else {
            return false;
        };
        self.can_see_position(viewer, pos)
            && (0..10).contains(&self.client_stack_index(viewer, pos, cid))
    }

    fn note_client_bodies_where(
        &mut self,
        conn: ConnId,
        viewer: CreatureId,
        include: impl Fn(Position) -> bool,
    ) {
        let bodies: Vec<(CreatureId, u32, Position)> = self
            .creatures
            .iter()
            .map(|(cid, kind)| (cid, creature_wire_id(cid, kind), kind.position()))
            .collect();
        let wires: Vec<u32> = bodies
            .into_iter()
            .filter_map(|(cid, wire, pos)| {
                if !include(pos) || !self.can_see_position(viewer, pos) {
                    return None;
                }
                if !self.viewer_has_creature_on_client(viewer, cid) {
                    return None;
                }
                if !(0..10).contains(&self.client_stack_index(viewer, pos, cid)) {
                    return None;
                }
                Some(wire)
            })
            .collect();
        for wire in wires {
            self.note_creature_on_client(conn, wire);
        }
    }

    fn note_client_bodies_in_rect(
        &mut self,
        conn: ConnId,
        viewer: CreatureId,
        x0: i32,
        y0: i32,
        width: i32,
        height: i32,
        z: i32,
    ) {
        self.note_client_bodies_where(conn, viewer, |pos| {
            let x = i32::from(pos.x);
            let y = i32::from(pos.y);
            i32::from(pos.z) == z && (x0..x0 + width).contains(&x) && (y0..y0 + height).contains(&y)
        });
    }

    fn client_stack_index(&self, viewer: CreatureId, pos: Position, cid: CreatureId) -> i32 {
        let Some(tile) = self.map.get_tile(pos) else {
            return -1;
        };
        let body = tile.body();
        if self.uses_cip_map_order(viewer) {
            let bottoms = body
                .down_items()
                .iter()
                .filter(|id| self.item_is_cip_priority_bottom(**id))
                .count();
            client_creature_stack_pos_cip(body, cid, bottoms)
        } else {
            client_creature_stack_pos(body, cid)
        }
    }

    /// Knowers of `wire_id` who `canSee` `pos`, plus the creature's own conn.
    /// C++ `AnnounceChangedCreature` / `FirstKnowingConnection` — `operate.cc`.
    pub(crate) fn knower_conns_who_can_see(
        &self,
        cid: CreatureId,
        pos: Position,
        wire_id: u32,
    ) -> Vec<ConnId> {
        let mut out: Vec<ConnId> = Vec::new();
        let mut seen: HashSet<ConnId> = HashSet::new();
        if let Some(&own) = self.creature_to_conn.get(&cid) {
            out.push(own);
            seen.insert(own);
        }
        if let Some(conns) = self.conns_by_known_wire.get(&wire_id) {
            for &conn in conns {
                if !seen.insert(conn) {
                    continue;
                }
                let Some(&viewer) = self.conn_to_creature.get(&conn) else {
                    continue;
                };
                if self.can_see_position(viewer, pos)
                    && self.is_creature_fully_sent_to_conn(conn, wire_id)
                {
                    out.push(conn);
                }
            }
        }
        out
    }

    pub(crate) fn broadcast_to_knowers(
        &mut self,
        cid: CreatureId,
        pos: Position,
        wire_id: u32,
        packet: Vec<u8>,
    ) {
        let conns = self.knower_conns_who_can_see(cid, pos, wire_id);
        for conn in conns {
            self.enqueue_outgoing(conn, packet.clone());
        }
    }
}
