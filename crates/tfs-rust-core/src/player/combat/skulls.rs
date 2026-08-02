//! 772 player-killing marks — AttackedPlayers / Aggressor / skulls.
//!
//! Domain: TFS-shaped player combat marks (secure mode, skull wire).
//! Outcomes: `crplayer.cc` `IsAttackJustified` / `RecordAttack` / `ClearPlayerkillingMarks`
//! / `GetPlayerkillingMark`; timer `crmain.cc:1102`; wire `sending.cc:1045` `SendCreatureSkull`.
//!
//! P2 owns `RecordMurder` / `CheckPlayerkilling` / assigning `PlayerkillerEnd`.

use tfs_rust_common::enums::SkullType;
use tfs_rust_common::WorldType;
use tfs_rust_net::outgoing_extra::send_creature_skull;

use crate::creature::CreatureKind;
use crate::creature::Player;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::login_out::creature_wire_id;

/// 772 former-mark justification window (`crplayer.cc:1421`, `1434`).
const FORMER_MARK_ROUNDS: u32 = 5;

impl Player {
    /// 772 `TPlayer::IsAttacker` — `crplayer.cc:1414–1430`.
    pub(crate) fn is_attacker(&self, victim: CreatureId, check_former: bool, round_nr: u32) -> bool {
        if self.attacked_players.contains(&victim) {
            return true;
        }
        if check_former
            && self
                .former_logout_round
                .saturating_add(FORMER_MARK_ROUNDS)
                >= round_nr
            && self.former_attacked_players.contains(&victim)
        {
            return true;
        }
        false
    }

    /// 772 `TPlayer::IsAggressor` — `crplayer.cc:1432–1436`.
    pub(crate) fn is_aggressor(&self, check_former: bool, round_nr: u32) -> bool {
        self.aggressor
            || (check_former
                && self.former_aggressor
                && self
                    .former_logout_round
                    .saturating_add(FORMER_MARK_ROUNDS)
                    >= round_nr)
    }

    /// 772 `TPlayer::InPartyWith` live arm — `crplayer.cc:1686–1694` (no FormerParty yet).
    pub(crate) fn in_party_with(&self, other: &Player) -> bool {
        match (self.social.party_id, other.social.party_id) {
            (Some(a), Some(b)) if a != 0 && a == b => true,
            _ => false,
        }
    }
}

impl GameWorld {
    /// 772 `TPlayer::IsAttackJustified` — `crplayer.cc:1438–1460`.
    ///
    /// Always justified when world is `PvpEnforced` or victim has `PlayerkillerEnd != 0`.
    /// Otherwise: victim aggressor (incl. former), same party, or victim lists attacker.
    pub(crate) fn player_is_attack_justified(
        &self,
        attacker: CreatureId,
        victim: CreatureId,
    ) -> bool {
        let Some(CreatureKind::Player(vic)) = self.creatures.get(victim) else {
            // Missing victim → fail-open (`crplayer.cc:1440–1442`).
            return true;
        };
        if self.pvp_config.world_type == WorldType::PvpEnforced || vic.playerkiller_end != 0 {
            return true;
        }
        let round_nr = self.round_nr;
        if vic.is_aggressor(true, round_nr) {
            return true;
        }
        let Some(CreatureKind::Player(atk)) = self.creatures.get(attacker) else {
            return false;
        };
        if vic.in_party_with(atk) {
            return true;
        }
        vic.is_attacker(attacker, true, round_nr)
    }

    /// 772 `TPlayer::GetPlayerkillingMark` — `crplayer.cc:1650–1676`.
    ///
    /// Observer-relative; open PvP (`WorldType::Pvp`) only. Priority:
    /// Red → White → Green → Yellow → None.
    pub(crate) fn player_get_killing_mark(
        &self,
        subject: CreatureId,
        observer: CreatureId,
    ) -> SkullType {
        if self.pvp_config.world_type != WorldType::Pvp {
            return SkullType::None;
        }
        let Some(CreatureKind::Player(subj)) = self.creatures.get(subject) else {
            return SkullType::None;
        };
        let Some(CreatureKind::Player(obs)) = self.creatures.get(observer) else {
            return SkullType::None;
        };
        if subj.playerkiller_end != 0 {
            return SkullType::Red;
        }
        if subj.aggressor {
            return SkullType::White;
        }
        if subj.in_party_with(obs) {
            return SkullType::Green;
        }
        if subj.is_attacker(observer, false, self.round_nr) {
            return SkullType::Yellow;
        }
        SkullType::None
    }

    /// 772 `TPlayer::RecordAttack` — `crplayer.cc:1462–1490`.
    ///
    /// Open PvP only. Appends victim to AttackedPlayers (yellow for victim) and/or sets
    /// Aggressor (white for everyone) when the attack is unjustified.
    pub(crate) fn player_record_attack(&mut self, attacker: CreatureId, victim: CreatureId) {
        if self.pvp_config.world_type != WorldType::Pvp || attacker == victim {
            return;
        }
        if !matches!(self.creatures.get(victim), Some(CreatureKind::Player(_))) {
            return;
        }
        if !matches!(self.creatures.get(attacker), Some(CreatureKind::Player(_))) {
            return;
        }

        let round_nr = self.round_nr;
        let skip_yellow = {
            let Some(CreatureKind::Player(vic)) = self.creatures.get(victim) else {
                return;
            };
            let Some(CreatureKind::Player(atk)) = self.creatures.get(attacker) else {
                return;
            };
            vic.in_party_with(atk)
                || vic.is_attacker(attacker, true, round_nr)
                || atk.is_attacker(victim, false, round_nr)
        };

        let mut send_yellow_to_victim = false;
        if !skip_yellow {
            if let Some(CreatureKind::Player(atk)) = self.creatures.get_mut(attacker) {
                if !atk.attacked_players.contains(&victim) {
                    atk.attacked_players.push(victim);
                    send_yellow_to_victim = true;
                }
            }
        }

        let unjustified = !self.player_is_attack_justified(attacker, victim);
        let mut became_aggressor = false;
        if unjustified {
            if let Some(CreatureKind::Player(atk)) = self.creatures.get_mut(attacker) {
                if !atk.aggressor {
                    atk.aggressor = true;
                    became_aggressor = true;
                }
            }
        }

        if send_yellow_to_victim {
            self.send_creature_skull_to_conn(attacker, victim);
        }
        if became_aggressor {
            self.announce_creature_skull(attacker);
        }
    }

    /// 772 `TPlayer::ClearAttacker` — `crplayer.cc:1586–1609`.
    ///
    /// Removes `cleared` from `subject`'s AttackedPlayers and refreshes skull to that victim.
    pub(crate) fn player_clear_attacker(&mut self, subject: CreatureId, cleared: CreatureId) {
        let removed = if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(subject) {
            if let Some(idx) = p.attacked_players.iter().position(|&id| id == cleared) {
                p.attacked_players.swap_remove(idx);
                true
            } else {
                false
            }
        } else {
            false
        };
        if removed {
            // Send skull of `subject` to the cleared victim (yellow→none for that observer).
            self.send_creature_skull_to_conn(subject, cleared);
        }
    }

    /// 772 `TPlayer::ClearPlayerkillingMarks` — `crplayer.cc:1611–1648`.
    pub(crate) fn clear_playerkilling_marks(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return;
        };
        let was_aggressor = p.aggressor;
        let former_victims: Vec<CreatureId> = p.attacked_players.clone();

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.former_attacked_players = former_victims.clone();
            p.attacked_players.clear();
            p.former_aggressor = was_aggressor;
            p.former_logout_round = self.round_nr;
            p.aggressor = false;
        }

        if was_aggressor {
            self.announce_creature_skull(cid);
        } else {
            for victim in &former_victims {
                self.send_creature_skull_to_conn(cid, *victim);
            }
        }

        // Every online player: ClearAttacker(this->ID).
        let others: Vec<CreatureId> = self
            .creatures
            .iter()
            .filter_map(|(id, k)| {
                if id != cid && matches!(k, CreatureKind::Player(_)) {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();
        for other in others {
            self.player_clear_attacker(other, cid);
        }
    }

    /// 772 `SendCreatureSkull` to one observer connection — `sending.cc:1045–1060`.
    ///
    /// Sends only when the observer knows the subject creature (UPTODATE gate).
    pub(crate) fn send_creature_skull_to_conn(
        &mut self,
        subject: CreatureId,
        observer: CreatureId,
    ) {
        let Some(obs_conn) = self.creature_to_conn.get(&observer).copied() else {
            return;
        };
        let wire_id = match self.creatures.get(subject) {
            Some(k) => creature_wire_id(subject, k),
            None => return,
        };
        let known = self
            .known_creatures_by_conn
            .get(&obs_conn)
            .is_some_and(|set| set.contains(&wire_id));
        if !known {
            return;
        }
        let mark = self.player_get_killing_mark(subject, observer) as u8;
        self.enqueue_outgoing(obs_conn, send_creature_skull(wire_id, mark).into_bytes());
    }

    /// 772 `AnnounceChangedCreature(CREATURE_SKULL_CHANGED)` — per-observer mark.
    pub(crate) fn announce_creature_skull(&mut self, cid: CreatureId) {
        let Some(pos) = self.creatures.get(cid).map(|k| k.position()) else {
            return;
        };
        let wire_id = match self.creatures.get(cid) {
            Some(k) => creature_wire_id(cid, k),
            None => return,
        };
        let conns = self.spectator_conns(pos);
        let mut packets: Vec<(tfs_rust_common::ConnId, Vec<u8>)> = Vec::new();
        for conn in conns {
            let Some(observer) = self.conn_to_creature.get(&conn).copied() else {
                continue;
            };
            let known = self
                .known_creatures_by_conn
                .get(&conn)
                .is_some_and(|set| set.contains(&wire_id));
            if !known {
                continue;
            }
            let mark = self.player_get_killing_mark(cid, observer) as u8;
            packets.push((conn, send_creature_skull(wire_id, mark).into_bytes()));
        }
        for (conn, pkt) in packets {
            self.enqueue_outgoing(conn, pkt);
        }
    }

    /// Resolve responsible player for RecordAttack on the damage path (summon → master).
    pub(crate) fn player_responsible_for_attack(
        &self,
        attacker: CreatureId,
    ) -> Option<CreatureId> {
        match self.creatures.get(attacker) {
            Some(CreatureKind::Player(_)) => Some(attacker),
            Some(k) => k.base().master.filter(|&m| {
                matches!(self.creatures.get(m), Some(CreatureKind::Player(_)))
            }),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PvpConfig;
    use crate::creature::CreatureKind;
    use tfs_rust_common::WorldType;

    fn make_pvp_world(world_type: WorldType) -> GameWorld {
        let mut world = crate::sim_harness::minimal_world();
        world.pvp_config = PvpConfig {
            world_type,
            ..PvpConfig::defaults()
        };
        world
    }

    fn insert_player(world: &mut GameWorld, name: &str) -> CreatureId {
        let pos = tfs_rust_common::Position::new(0, 0, 7);
        let mut player = crate::sim_harness::test_player(name, pos);
        player.guid = (name.as_bytes().iter().map(|&b| b as u32).sum::<u32>())
            .wrapping_add(name.len() as u32);
        world.creatures.insert(CreatureKind::Player(player))
    }

    #[test]
    fn unjustified_attack_sets_aggressor_and_attacked_list() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_record_attack(a, b);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing alice");
        };
        assert!(pa.aggressor);
        assert!(pa.attacked_players.contains(&b));
        assert_eq!(
            world.player_get_killing_mark(a, b),
            SkullType::White,
            "aggressor shows white to victim (white > yellow)"
        );
        assert_eq!(world.player_get_killing_mark(a, a), SkullType::White);
    }

    #[test]
    fn secure_mode_allows_retaliation_after_record() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        // Alice attacks Bob → Alice is aggressor / listed attacker for Bob.
        world.player_record_attack(a, b);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.secure_mode = true;
        }
        // Bob may retaliate (Alice is aggressor → justified).
        assert!(!world.player_secure_mode_blocks_attack(b, a));
        // Alice attacking unmarked Charlie is blocked under secure.
        let c = insert_player(&mut world, "carol");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(world.player_secure_mode_blocks_attack(a, c));
    }

    #[test]
    fn red_playerkiller_always_justified_and_red_mark() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.playerkiller_end = i64::MAX;
        }
        assert!(world.player_is_attack_justified(a, b));
        assert_eq!(world.player_get_killing_mark(b, a), SkullType::Red);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(!world.player_secure_mode_blocks_attack(a, b));
    }

    #[test]
    fn party_mates_justified_and_green() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.social.party_id = Some(7);
        }
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(b) {
            p.social.party_id = Some(7);
        }
        assert!(world.player_is_attack_justified(a, b));
        assert_eq!(world.player_get_killing_mark(b, a), SkullType::Green);
        // RecordAttack should not add yellow for party mates.
        world.player_record_attack(a, b);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing");
        };
        assert!(!pa.aggressor);
        assert!(pa.attacked_players.is_empty());
    }

    #[test]
    fn clear_marks_copies_former_and_clears_display() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_record_attack(a, b);
        world.clear_playerkilling_marks(a);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing");
        };
        assert!(!pa.aggressor);
        assert!(pa.former_aggressor);
        assert!(pa.attacked_players.is_empty());
        assert!(pa.former_attacked_players.contains(&b));
        assert_eq!(pa.former_logout_round, 100);
        // Display gone (CheckFormer=false).
        assert_eq!(world.player_get_killing_mark(a, b), SkullType::None);
        // Still justified within +5 rounds via former.
        world.round_nr = 104;
        assert!(world.player_is_attack_justified(b, a));
        world.round_nr = 106;
        assert!(!world.player_is_attack_justified(b, a));
    }

    #[test]
    fn clear_marks_removes_self_from_others_attacked_lists() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        // Bob lists Alice (Bob is aggressor vs Alice).
        world.player_record_attack(b, a);
        assert!(
            world
                .creatures
                .get(b)
                .and_then(|k| match k {
                    CreatureKind::Player(p) => Some(p.attacked_players.contains(&a)),
                    _ => None,
                })
                .unwrap_or(false)
        );
        // Clearing Alice's marks also ClearAttacker(Alice) on every player → drop from Bob.
        world.clear_playerkilling_marks(a);
        let Some(CreatureKind::Player(pb)) = world.creatures.get(b) else {
            panic!("missing");
        };
        assert!(
            !pb.attacked_players.contains(&a),
            "ClearAttacker should drop alice from bob's list"
        );
    }

    #[test]
    fn record_attack_noop_outside_open_pvp() {
        let mut world = make_pvp_world(WorldType::PvpEnforced);
        let a = insert_player(&mut world, "alice");
        let b = insert_player(&mut world, "bob");
        world.player_record_attack(a, b);
        let Some(CreatureKind::Player(pa)) = world.creatures.get(a) else {
            panic!("missing");
        };
        assert!(!pa.aggressor);
        assert!(pa.attacked_players.is_empty());
        assert_eq!(world.player_get_killing_mark(a, b), SkullType::None);
    }
}
