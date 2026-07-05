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
/// `underground_sees_surface` selects the era's underground Z-rule (K3 profile knob):
/// - **false (1098 / TFS `canSee`):** underground viewers cannot see surface (`tz < 8` rejects).
/// - **true (772 `TConnection::IsVisible`, `connections.cc:357-378`):** underground viewers CAN see
///   surface within 2 floors — only `abs(dz) > 2` rejects (audit M5).
pub fn creature_can_see(
    viewer_pos: Position,
    target: Position,
    view_range_x: i32,
    view_range_y: i32,
    underground_sees_surface: bool,
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
        if !underground_sees_surface && tz < 8 {
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

    /// Register a bidirectional `ConnId ↔ CreatureId` mapping (audit #4).
    /// Maintains both [`Self::conn_to_creature`] and the reverse [`Self::creature_to_conn`]
    /// index so spatial fan-out can resolve a creature's connection in O(1).
    /// Call this instead of `conn_to_creature.insert` directly.
    pub fn register_conn_mapping(&mut self, conn: ConnId, cid: CreatureId) {
        self.conn_to_creature.insert(conn, cid);
        self.creature_to_conn.insert(cid, conn);
    }

    /// Remove a `ConnId ↔ CreatureId` mapping (audit #4). Companion to
    /// [`Self::register_conn_mapping`]. Call this instead of `conn_to_creature.remove`
    /// directly so the reverse index stays in sync.
    pub fn unregister_conn_mapping(&mut self, conn: ConnId) {
        if let Some(cid) = self.conn_to_creature.remove(&conn) {
            self.creature_to_conn.remove(&cid);
        }
    }

    /// Collect all `ConnId`s whose creature can see `pos`. Used by every broadcast.
    ///
    /// Resolves spectators through the chunk grid spatial index (audit #4) —
    /// O(local crowd) instead of O(all online players). The grid over-collects
    /// at chunk granularity; `protocol_can_see` filters precisely.
    // C++ reference: `Map::getSpectators` (`map.cpp` ~386–474) + `ProtocolGame::canSee`
    // (`protocolgame.cpp` ~796–823).
    fn spectator_conns(&self, pos: Position) -> Vec<ConnId> {
        self.spectator_conns_via_grid(pos)
    }

    /// Grid-based spectator connection resolution (audit #4).
    ///
    /// Walks the chunk spatial index over the multi-floor Z span
    /// ([`Self::spectator_z_range`], shared with the monster fan-out path), collects
    /// all creatures in the overlapping chunks, then keeps only those that (a) hold a
    /// `ConnId` (i.e. are online players) and (b) pass `ProtocolGame::canSee` for `pos`.
    /// Sorted + deduped by SlotMap key for deterministic fan-out order.
    pub(crate) fn spectator_conns_via_grid(&self, pos: Position) -> Vec<ConnId> {
        let mut creature_ids: Vec<CreatureId> = Vec::new();
        for z in Self::spectator_z_range(pos.z, true) {
            self.map.grid.collect_spectators(
                pos.x,
                pos.y,
                z,
                MAX_CLIENT_VIEWPORT_X as u16,
                MAX_CLIENT_VIEWPORT_Y as u16,
                &mut creature_ids,
            );
        }
        creature_ids.sort_by_key(|id| id.data().as_ffi());
        creature_ids.dedup();

        let mut conns: Vec<ConnId> = Vec::with_capacity(creature_ids.len());
        for cid in creature_ids {
            let Some(&viewer_conn) = self.creature_to_conn.get(&cid) else {
                continue; // monster / NPC — not a player spectator
            };
            if self.can_see_position(cid, pos) {
                conns.push(viewer_conn);
            }
        }
        conns
    }

    /// Wide-range spectator connection resolution for yell — `Game::internalCreatureSay`
    /// yell path (`gameserver/src/game.cpp:3522-3523`): `map.getSpectators(pos, true,
    /// false, 18, 18, 14, 14)`. Unlike [`Self::spectator_conns_via_grid`], this does
    /// **not** apply `ProtocolGame::canSee` filtering — the C++ yell path sends to every
    /// creature in the wider `(±18 X, ±14 Y, multifloor)` box, with only `ghostMode`
    /// gating at the send site. Returns `(ConnId, CreatureId, viewer_pos)` tuples so the
    /// caller can do per-viewer distance checks (whisper) or ghost-mode filtering (yell).
    pub(crate) fn spectator_players_in_box(
        &self,
        pos: Position,
        range_x: u16,
        range_y: u16,
        multifloor: bool,
    ) -> Vec<(ConnId, CreatureId, Position)> {
        let mut creature_ids: Vec<CreatureId> = Vec::new();
        for z in Self::spectator_z_range(pos.z, multifloor) {
            self.map.grid.collect_spectators(
                pos.x,
                pos.y,
                z,
                range_x,
                range_y,
                &mut creature_ids,
            );
        }
        creature_ids.sort_by_key(|id| id.data().as_ffi());
        creature_ids.dedup();

        let mut out: Vec<(ConnId, CreatureId, Position)> = Vec::with_capacity(creature_ids.len());
        for cid in creature_ids {
            let Some(&viewer_conn) = self.creature_to_conn.get(&cid) else {
                continue;
            };
            let Some(viewer_pos) = self.creatures.get(cid).map(|k| k.position()) else {
                continue;
            };
            out.push((viewer_conn, cid, viewer_pos));
        }
        out
    }

    /// Enqueue the same packet bytes for every connection that can see `pos` (clone per viewer).
    // C++ ref: repeated `ProtocolGame` fan-out in `game.cpp` / `protocolgame.cpp`.
    pub(crate) fn broadcast_to_spectators(&mut self, pos: Position, packet: Vec<u8>) {
        let conns = self.spectator_conns(pos);
        for conn in conns {
            self.enqueue_outgoing(conn, packet.clone());
        }
    }

    /// C++ `AnnounceChangedCreature(CREATURE_SPEED_CHANGED)` → `SendCreatureSpeed`
    /// (`operate.cc:82`, `sending.cc:1028`). Broadcasts the creature's current `GetSpeed()`
    /// to all spectators who can see the creature's tile.
    pub(crate) fn announce_creature_speed(&mut self, cid: CreatureId) {
        use tfs_rust_net::codec::wire::CreatureSpeedWire;
        let (pos, wire_speed, base_speed) = match self.creatures.get(cid) {
            Some(k) => {
                let base = k.base();
                let role = match k {
                    CreatureKind::Player(_) => crate::walk::WalkSpeedRole::Player,
                    _ => crate::walk::WalkSpeedRole::MonsterOrNpc,
                };
                let wire = crate::walk::wire_step_speed(role, base, &self.mechanics);
                (base.position, wire, base.base_speed.max(0) as u16)
            }
            None => return,
        };
        let creature_id = cid.data().as_ffi() as u32;
        let packet = self
            .codec
            .encode_creature_speed(&CreatureSpeedWire {
                creature_id,
                speed: wire_speed,
                base_speed,
            })
            .into_bytes();
        self.broadcast_to_spectators(pos, packet);
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
        // `spectator_conns_via_grid` already filters by `can_see_position(viewer, pos)`,
        // so every conn here can see the speaker's tile.
        let viewers: Vec<(ConnId, CreatureId)> = self
            .spectator_conns_via_grid(pos)
            .into_iter()
            .filter_map(|conn| {
                self.conn_to_creature
                    .get(&conn)
                    .copied()
                    .map(|viewer| (conn, viewer))
            })
            .collect();
        // C++ `internalCreatureSay` two-pass loop — "send to client" then "event method"
        // (`gameserver/src/game.cpp:3529-3544`). Pass 1: per-viewer `sendCreatureSay`.
        for (conn, _viewer) in &viewers {
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
            self.enqueue_outgoing(*conn, pkt.into_bytes());
        }
        // Pass 2: `Creature::onCreatureSay` (all spectators incl. speaker) +
        // `Events::eventCreatureOnHear` (excludes speaker). Both default to no-ops
        // until the NPC/creaturescript Lua runtime lands (chat plan §2.5); the call
        // sites are wired now so the trait dispatch doesn't need revisiting later.
        for (_conn, viewer) in &viewers {
            self.events.on_creature_say(*viewer, speaker, speak_type, text);
            if *viewer != speaker {
                self.events
                    .on_hear(*viewer, speaker, text, speak_type);
            }
        }
    }

    /// TFS `Game::playerWhisper` — `gameserver/src/game.cpp:3400-3422`.
    ///
    /// Per-viewer distance check: spectators within 1 tile (Chebyshev ≤1 in X **and** Y,
    /// `Position::areInRange<1,1>`) receive the real text; beyond that they receive
    /// `"pspsps"`. Uses the same viewport range as SAY (`Map::maxClientViewportX/Y`).
    /// Two-pass loop: send-to-client then event-method, matching `internalCreatureSay`.
    pub fn broadcast_creature_whisper(
        &mut self,
        speaker: CreatureId,
        speak_type: u8,
        text: &str,
    ) {
        use tfs_rust_net::codec::wire::CreatureSayWire;
        let (pos, name, level) = match self.creatures.get(speaker) {
            Some(CreatureKind::Player(p)) => (p.base.position, p.base.name.clone(), p.level as u16),
            _ => return, // whisper is player-only
        };
        // C++ `map.getSpectators(spectators, pos, false, false, maxClientViewportX,
        // maxClientViewportX, maxClientViewportY, maxClientViewportY)` — same-floor,
        // ±8 X / ±6 Y box. `spectator_players_in_box` collects without `canSee` filter,
        // matching the C++ whisper path (no per-viewer `canSee` check).
        let viewers = self.spectator_players_in_box(
            pos,
            MAX_CLIENT_VIEWPORT_X as u16,
            MAX_CLIENT_VIEWPORT_Y as u16,
            false,
        );
        // Pass 1: per-viewer `sendCreatureSay` with distance-based text selection.
        for (conn, _viewer, viewer_pos) in &viewers {
            let within_one = pos.z == viewer_pos.z
                && (pos.x as i32 - viewer_pos.x as i32).unsigned_abs() <= 1
                && (pos.y as i32 - viewer_pos.y as i32).unsigned_abs() <= 1;
            let viewer_text = if within_one { text } else { "pspsps" };
            let sid = self.alloc_statement_id();
            let pkt = self.codec.encode_creature_say(
                sid,
                &CreatureSayWire {
                    speaker_name: name.clone(),
                    level,
                    speak_type,
                    pos,
                    text: viewer_text.into(),
                },
            );
            self.enqueue_outgoing(*conn, pkt.into_bytes());
        }
        // Pass 2: event-method loop — `onCreatureSay` + `on_hear` (excludes speaker).
        // C++ fires `onCreatureSay` with the **real** text for all spectators
        // (`game.cpp:3420`), not the garbled "pspsps".
        for (_conn, viewer, _viewer_pos) in &viewers {
            self.events.on_creature_say(*viewer, speaker, speak_type, text);
            if *viewer != speaker {
                self.events.on_hear(*viewer, speaker, text, speak_type);
            }
        }
    }

    /// TFS `Game::internalCreatureSay` yell path — `gameserver/src/game.cpp:3518-3544`.
    ///
    /// Wide-range fan-out: `map.getSpectators(pos, true, false, 18, 18, 14, 14)` —
    /// multifloor, ±18 X / ±14 Y box, **no `canSee` filtering** (the wider range IS the
    /// filter). Ghost-mode gating is handled by the caller (C++ checks `!ghostMode ||
    /// tmpPlayer->canSeeCreature(creature)` at the send site). Two-pass loop matching
    /// `internalCreatureSay`.
    pub fn broadcast_creature_yell(
        &mut self,
        speaker: CreatureId,
        speak_type: u8,
        text: &str,
    ) {
        use tfs_rust_net::codec::wire::CreatureSayWire;
        let (pos, name, level) = match self.creatures.get(speaker) {
            Some(CreatureKind::Player(p)) => (p.base.position, p.base.name.clone(), p.level as u16),
            _ => return,
        };
        // C++ yell range: `(18, 18, 14, 14)` with `multifloor=true`.
        let viewers = self.spectator_players_in_box(pos, 18, 14, true);
        // Pass 1: per-viewer `sendCreatureSay`. C++ ghost-mode check:
        // `if (!ghostMode || tmpPlayer->canSeeCreature(creature))` — for non-ghost
        // speakers (the common case) all viewers receive the packet.
        for (conn, _viewer, _viewer_pos) in &viewers {
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
            self.enqueue_outgoing(*conn, pkt.into_bytes());
        }
        // Pass 2: event-method loop.
        for (_conn, viewer, _viewer_pos) in &viewers {
            self.events.on_creature_say(*viewer, speaker, speak_type, text);
            if *viewer != speaker {
                self.events.on_hear(*viewer, speaker, text, speak_type);
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

        // 772 emits the (race-keyed) hit effect + splash via `apply_physical_hit_blood` in the
        // combat apply path (`crmain.cc:762-775`), so no duplicate draw-blood here. Phase 3: both
        // eras use the 772 blood path now (1098 monster AI deleted).

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
            // K10: damage text format — 772 attributes attacker, 1098 uses simple loss text.
            let text = match self.mechanics.profile.damage_text_format {
                crate::formulas::DamageTextFormat::AttackerAttribution => {
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
                }
                crate::formulas::DamageTextFormat::SimpleLoss => {
                    if dmg == 1 {
                        "You lose 1 hitpoint.".to_string()
                    } else {
                        format!("You lose {dmg} hitpoints.")
                    }
                }
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
        let msg = rv.description();
        self.enqueue_outgoing(
            conn_id,
            send_text_message_simple(self.codec.failure_message_type(), msg).into_bytes(),
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

/// Audit #4 / Phase 3 — equivalence test: the grid-based spectator connection set
/// (`spectator_conns_via_grid`) must equal the old O(all players) full-scan set for a
/// range of positions and floors, including the surface/underground boundary and
/// underground ±2.
#[cfg(test)]
mod spectator_fanout_grid_tests {
    use super::*;
    use crate::sim_harness::{insert_spectator_player, minimal_world, test_player};
    use slotmap::Key;
    use std::collections::HashSet;
    use tfs_rust_common::{ConnId, Position};

    /// The pre-Phase-3 full-scan implementation, replicated locally for equivalence
    /// comparison. Iterates every online connection and applies `can_see_position`.
    fn spectator_conns_full_scan(world: &GameWorld, pos: Position) -> HashSet<ConnId> {
        world
            .conn_to_creature
            .iter()
            .filter(|(_, &vid)| world.can_see_position(vid, pos))
            .map(|(&c, _)| c)
            .collect()
    }

    fn grid_set(world: &GameWorld, pos: Position) -> HashSet<ConnId> {
        world.spectator_conns_via_grid(pos).into_iter().collect()
    }

    /// Seed a world with players on surface (z=7), underground (z=8, 9, 10), and far away.
    /// Returns the world. Each player gets a unique ConnId.
    fn seeded_world() -> GameWorld {
        let mut world = minimal_world();
        // Surface cluster around (100, 100, 7).
        insert_spectator_player(
            &mut world,
            ConnId(1),
            test_player("A", Position::new(100, 100, 7)),
        );
        insert_spectator_player(
            &mut world,
            ConnId(2),
            test_player("B", Position::new(105, 103, 7)),
        );
        insert_spectator_player(
            &mut world,
            ConnId(3),
            test_player("C", Position::new(108, 106, 7)),
        );
        // Just outside the 8×6+1 viewport of (100,100,7): x=110 is at the edge.
        insert_spectator_player(
            &mut world,
            ConnId(4),
            test_player("D", Position::new(120, 120, 7)),
        );
        // Underground, directly below the surface cluster.
        insert_spectator_player(
            &mut world,
            ConnId(5),
            test_player("E", Position::new(100, 100, 8)),
        );
        insert_spectator_player(
            &mut world,
            ConnId(6),
            test_player("F", Position::new(100, 100, 9)),
        );
        // Underground too deep to see z=8 from z=10 is fine (|dz|=2), but z=11 vs z=8 is |dz|=3.
        insert_spectator_player(
            &mut world,
            ConnId(7),
            test_player("G", Position::new(100, 100, 11)),
        );
        // Far away on surface — no spatial overlap with the (100,100) cluster.
        insert_spectator_player(
            &mut world,
            ConnId(8),
            test_player("H", Position::new(200, 200, 7)),
        );
        world
    }

    #[test]
    fn grid_equals_full_scan_surface_center() {
        let world = seeded_world();
        let pos = Position::new(100, 100, 7);
        assert_eq!(grid_set(&world, pos), spectator_conns_full_scan(&world, pos));
    }

    #[test]
    fn grid_equals_full_scan_surface_edge() {
        let world = seeded_world();
        // A position near the viewport edge of player D.
        let pos = Position::new(119, 119, 7);
        assert_eq!(grid_set(&world, pos), spectator_conns_full_scan(&world, pos));
    }

    #[test]
    fn grid_equals_full_scan_underground() {
        let world = seeded_world();
        // Underground at z=8 — surface players (z<=7) cannot see this; underground
        // players within ±2 can.
        let pos = Position::new(100, 100, 8);
        assert_eq!(grid_set(&world, pos), spectator_conns_full_scan(&world, pos));
    }

    #[test]
    fn grid_equals_full_scan_underground_deep() {
        let world = seeded_world();
        // z=9: visible from z=8 (|dz|=1) and z=10..11 (|dz|<=2), not from z=7 surface.
        let pos = Position::new(100, 100, 9);
        assert_eq!(grid_set(&world, pos), spectator_conns_full_scan(&world, pos));
    }

    #[test]
    fn grid_equals_full_scan_surface_underground_boundary() {
        let world = seeded_world();
        // Querying from surface (z=7) — underground players are never visible
        // (protocol_can_see: my_z<=7 && z>7 → false).
        let pos = Position::new(105, 103, 7);
        assert_eq!(grid_set(&world, pos), spectator_conns_full_scan(&world, pos));
    }

    #[test]
    fn grid_equals_full_scan_far_away_isolated() {
        let world = seeded_world();
        // A position near the far-away player H — only H should see it.
        let pos = Position::new(200, 200, 7);
        let grid = grid_set(&world, pos);
        let full = spectator_conns_full_scan(&world, pos);
        assert_eq!(grid, full);
        // Sanity: the far-away player is the only spectator.
        assert!(grid.contains(&ConnId(8)), "far-away player H must be a spectator of its own tile");
        assert_eq!(grid.len(), 1, "only H should see (200,200,7)");
    }

    #[test]
    fn grid_equals_full_scan_void_position_no_spectators() {
        let world = seeded_world();
        // A position with no tile and no nearby players in the chunk.
        let pos = Position::new(50, 50, 7);
        let grid = grid_set(&world, pos);
        let full = spectator_conns_full_scan(&world, pos);
        assert_eq!(grid, full);
        assert!(grid.is_empty(), "no player should see an isolated void position");
    }

    /// The grid path must not return duplicate conns even when a creature's chunk
    /// is collected from multiple Z-floors (it can't be on two floors, but the
    /// dedup must hold for the general case).
    #[test]
    fn grid_no_duplicates() {
        let world = seeded_world();
        let pos = Position::new(100, 100, 8);
        let conns = world.spectator_conns_via_grid(pos);
        let unique: HashSet<ConnId> = conns.iter().copied().collect();
        assert_eq!(conns.len(), unique.len(), "spectator_conns_via_grid must not produce duplicates");
    }

    /// `conn_for_creature` must agree with the reverse index for every online player.
    #[test]
    fn conn_for_creature_uses_reverse_index() {
        let world = seeded_world();
        for (&conn, &cid) in &world.conn_to_creature {
            assert_eq!(
                world.conn_for_creature(cid),
                Some(conn),
                "conn_for_creature must agree with conn_to_creature for creature {:?}",
                cid.data().as_ffi()
            );
        }
    }

    /// `register_conn_mapping` / `unregister_conn_mapping` keep both maps in sync.
    #[test]
    fn register_unregister_conn_mapping_keeps_reverse_index_in_sync() {
        let mut world = seeded_world();
        let conn = ConnId(99);
        let cid = insert_spectator_player(
            &mut world,
            conn,
            test_player("Z", Position::new(100, 100, 7)),
        );
        assert_eq!(world.conn_for_creature(cid), Some(conn));
        assert_eq!(world.conn_to_creature.get(&conn), Some(&cid));
        assert_eq!(world.creature_to_conn.get(&cid), Some(&conn));

        world.unregister_conn_mapping(conn);
        assert!(world.conn_for_creature(cid).is_none());
        assert!(!world.conn_to_creature.contains_key(&conn));
        assert!(!world.creature_to_conn.contains_key(&cid));
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
    /// surface within 2 floors (no `tz < 8` rejection). `underground_sees_surface = true`.
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
