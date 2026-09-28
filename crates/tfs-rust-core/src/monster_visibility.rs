//! Monster invisibility from protocol 780 upward (8.0 and 10.98).
//!
//! `800src/player.h` `sendCreatureChangeVisible`: a player gets an empty or real
//! outfit (`0x8E`). A monster the viewer cannot see is removed from the tile
//! (`sendRemoveTileThing`) and added back (`sendAddCreature`) when the condition
//! ends. `800src/map.cpp` `moveCreature` sends no step when `canSeeCreature` is
//! false, and `GetTileDescription` skips the body.
//!
//! 772 keeps the outfit path (`crmain.cc:636-641` restore `OrgOutfit`).

use tfs_rust_common::enums::ConditionType;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::walk::{capture_creature_stack_snapshot, stack_for_viewer, stack_indexes_for_snapshot};

impl GameWorld {
    /// True when this creature's invisibility is a tile remove, not an outfit packet.
    pub(crate) fn monster_invisibility_hides_from_tile(&self, cid: CreatureId) -> bool {
        self.codec.caps().monster_invis_removes_from_tile
            && matches!(self.creatures.get(cid), Some(CreatureKind::Monster(_)))
    }

    /// Hide or show a monster for viewers who cannot see invisibility.
    ///
    /// `group.access` matches `800src/player.h` `canSeeInvisibility` (access keeps
    /// the real outfit and is not removed). A second hide is a no-op: another
    /// `0x6C` would delete whatever now sits at that stack index.
    pub(crate) fn sync_monster_invisibility_on_tile(&mut self, cid: CreatureId, visible: bool) {
        let hidden = matches!(
            self.creatures.get(cid),
            Some(CreatureKind::Monster(m)) if m.invis_removed_from_clients
        );
        if !visible {
            if hidden {
                return;
            }
            self.remove_invisible_monster_from_tiles(cid);
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.invis_removed_from_clients = true;
            }
            return;
        }
        // `process_skills` announces when the illusion cycle hits 0, one tick
        // before the condition is erased. The map still omits the body until
        // `on_condition_ended`, so the add waits for that erase.
        let still_listed = self.creatures.get(cid).is_some_and(|k| {
            k.base()
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Invisible)
        });
        if still_listed || !hidden {
            return;
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.invis_removed_from_clients = false;
        }
        self.add_visible_monster_to_tiles(cid);
    }

    fn remove_invisible_monster_from_tiles(&mut self, cid: CreatureId) {
        let Some(pos) = self.creatures.get(cid).map(|k| k.position()) else {
            return;
        };
        let snap = capture_creature_stack_snapshot(self, pos);
        let (stack_772, stack_otc) = stack_indexes_for_snapshot(self, &snap, cid);
        let viewers: Vec<_> = self
            .spectator_conns_via_grid(pos)
            .into_iter()
            .filter_map(|conn| {
                let viewer = *self.conn_to_creature.get(&conn)?;
                if viewer == cid || self.player_is_access_player(viewer) {
                    return None;
                }
                let stack = stack_for_viewer(self, &stack_772, &stack_otc, viewer);
                Some((conn, stack))
            })
            .collect();
        for (conn, stack) in viewers {
            self.send_creature_remove_to_conn(conn, cid, pos, stack);
        }
    }

    fn add_visible_monster_to_tiles(&mut self, cid: CreatureId) {
        let Some(pos) = self.creatures.get(cid).map(|k| k.position()) else {
            return;
        };
        let viewers: Vec<_> = self
            .spectator_conns_via_grid(pos)
            .into_iter()
            .filter_map(|conn| {
                let viewer = *self.conn_to_creature.get(&conn)?;
                if viewer == cid || self.player_is_access_player(viewer) {
                    return None;
                }
                Some((conn, viewer))
            })
            .collect();
        for (conn, viewer) in viewers {
            self.send_creature_appear_to_conn(conn, viewer, cid, pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::ConnId;
    use tfs_rust_common::Position;
    use tfs_rust_common::enums::{CombatType, ConditionType};

    use crate::combat::{CombatDamage, CombatParams};
    use crate::condition::{ActiveCondition, ConditionData, add_condition_merge};
    use crate::creature::CreatureKind;
    use crate::test_world::support::{
        TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_monster,
        insert_player, insert_spectator_player, test_player,
    };

    fn arm_invisible(world: &mut crate::game_world::GameWorld, cid: crate::ids::CreatureId) {
        let Some(kind) = world.creatures.get_mut(cid) else {
            return;
        };
        add_condition_merge(
            &mut kind.base_mut().active_conditions,
            ActiveCondition::new(
                0,
                0,
                ConditionType::Invisible,
                ConditionData::Generic { ticks: 20_000 },
                Some(20),
            ),
        );
    }

    fn has_invisible(world: &crate::game_world::GameWorld, cid: crate::ids::CreatureId) -> bool {
        world.creatures.get(cid).is_some_and(|k| {
            k.base()
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Invisible)
        })
    }

    fn opcodes(world: &crate::game_world::GameWorld, conn: ConnId) -> Vec<u8> {
        world
            .pending_outgoing
            .get(&conn)
            .map(|pkts| pkts.iter().filter_map(|p| p.first().copied()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn energy_hit_clears_monster_invisibility() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Warlock", pos, 75);
        arm_invisible(&mut world, monster);
        world.combat_execute_with_stimulus(
            None,
            monster,
            &CombatDamage {
                primary: (CombatType::Energy, -20),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams {
                primary_type: CombatType::Energy,
                ..CombatParams::default()
            },
        );
        assert!(
            !has_invisible(&world, monster),
            "any non-periodic hit clears monster invisibility"
        );
    }

    #[test]
    fn periodic_damage_keeps_monster_invisibility() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Warlock", pos, 75);
        arm_invisible(&mut world, monster);
        world.combat_execute_with_stimulus(
            None,
            monster,
            &CombatDamage {
                primary: (CombatType::PoisonPeriodic, -20),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams {
                primary_type: CombatType::PoisonPeriodic,
                ..CombatParams::default()
            },
        );
        assert!(
            has_invisible(&world, monster),
            "periodic damage returns before the invisibility clear"
        );
    }

    #[test]
    fn player_keeps_invisibility_through_damage() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let player = insert_player(&mut world, test_player("Mage", pos));
        arm_invisible(&mut world, player);
        world.combat_execute_with_stimulus(
            None,
            player,
            &CombatDamage {
                primary: (CombatType::Physical, -20),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(has_invisible(&world, player));
    }

    #[test]
    fn protocol_800_removes_monster_and_adds_them_back_on_hit() {
        let mut world = beat_driven_test_world();
        world.codec =
            tfs_rust_net::Codec::from_version(tfs_rust_common::ProtocolVersion::V800).expect("800");
        let player_pos = Position::new(100, 100, 7);
        let monster_pos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, player_pos, TEST_SYNTHETIC_GROUND_WP);
        ensure_walkable_tile(&mut world.map, monster_pos, TEST_SYNTHETIC_GROUND_WP);
        let conn = ConnId(7);
        insert_spectator_player(&mut world, conn, test_player("Knight", player_pos));
        let monster = insert_monster(&mut world, "Warlock", monster_pos, 75);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.outfit.look_type = 130;
        }

        arm_invisible(&mut world, monster);
        world.pending_outgoing.clear();
        world.on_condition_started(monster, ConditionType::Invisible);
        let hide = opcodes(&world, conn);
        assert!(
            hide.contains(&0x6C),
            "8.0 hide removes the monster from the tile, got {hide:?}"
        );
        assert!(
            !hide.contains(&0x8E),
            "8.0 hide does not send an outfit packet, got {hide:?}"
        );

        world.pending_outgoing.clear();
        world.on_condition_started(monster, ConditionType::Invisible);
        assert!(
            opcodes(&world, conn).is_empty(),
            "recasting invisibility must not remove the next stack object"
        );

        world.pending_outgoing.clear();
        world.combat_execute_with_stimulus(
            None,
            monster,
            &CombatDamage {
                primary: (CombatType::Energy, -20),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams {
                primary_type: CombatType::Energy,
                ..CombatParams::default()
            },
        );
        let show = opcodes(&world, conn);
        assert!(
            show.contains(&0x6A),
            "a hit adds the monster back, got {show:?}"
        );
        assert!(!has_invisible(&world, monster));
    }
}
