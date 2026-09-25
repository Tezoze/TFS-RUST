//! Protocol known-creature table + reverse conn index for `AnnounceChangedCreature`.
//!
//! Corpus: `FirstKnowingConnection` then `IsVisible` — `operate.cc` `AnnounceChangedCreature`.
//! Forward map stays `known_creatures_by_conn`; reverse is `conns_by_known_wire`.

use std::collections::HashSet;

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;
use tfs_rust_net::creature_known::KnownCreatureTable;

use crate::game_world::GameWorld;
use crate::ids::CreatureId;

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
