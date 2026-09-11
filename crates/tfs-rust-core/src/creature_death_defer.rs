//! Deferred death/despawn finalize — 772 ProcessCreatures destructor.
//! C++ reference: crmain.cc Death / ProcessCreatures 878-881, 1108-1125;
//! crplayer.cc TPlayer::Death; crnonpl.cc IdleStimulus 2346-2415, ~TMonster 2113;
//! cract.cc Execute 785; crmain.cc Damage IsDead 488.

use slotmap::Key;
use tfs_rust_common::Position;
use tfs_rust_db::player::PlayerStore;

use crate::creature::{CreatureKind, MonsterInventory, MonsterState};
use crate::game_world::GameWorld;
use crate::game_world_lifecycle::LogoutPossible;
use crate::ids::CreatureId;

/// TFS `CONST_ME_POFF` / 772 `EFFECT_POFF` — despawn puff (`crmain.cc` `~TMonster` !IsDead).
const MAGIC_EFFECT_POFF: u8 = 3;

impl GameWorld {
    /// True when the creature is gone or `IsDead` (`crcombat.cc:643` `Target->IsDead`).
    pub(crate) fn creature_is_dead(&self, cid: CreatureId) -> bool {
        self.creatures.get(cid).is_none_or(|k| k.base().is_dead)
    }

    /// 772 `TCreature::Death` + `TPlayer::Death` + Damage lethal tail — flags only.
    ///
    /// Body stays on the map until [`Self::finalize_pending`]. Idempotent if already `is_dead`.
    pub fn mark_dead(&mut self, cid: CreatureId) {
        let Some(kind) = self.creatures.get_mut(cid) else {
            return;
        };
        if kind.base().is_dead {
            return;
        }
        kind.base_mut().is_dead = true;
        kind.base_mut().logging_out = true;

        self.announce_health_zero(cid);
        self.record_lethal_outcome(cid);

        let is_player = matches!(self.creatures.get(cid), Some(CreatureKind::Player(_)));
        if is_player {
            self.consume_amulet_of_loss_on_death(cid);
            self.player_on_pvp_death_marks(cid);
            if matches!(
                self.creatures.get(cid),
                Some(CreatureKind::Player(p)) if p.base.skill_loss
            ) {
                self.apply_player_death_skill_loss(cid);
            }
            // Player exp loss + PvP-enforced XP (`crplayer.cc:339-340`) — not monster XP.
            self.run_handle_creature_death(cid, false);
            self.send_player_skills(cid);
            // 772 `TPlayer::Death` — `crplayer.cc:331-334`. Trailing newline is required.
            self.send_player_advance_message(cid, "You are dead.\n");
            if let Some(conn) = self.conn_for_creature(cid) {
                self.dead_connections.insert(conn);
                let is_otclient = matches!(
                    self.creatures.get(cid),
                    Some(CreatureKind::Player(p)) if p.is_otclient()
                );
                self.dead_conn_state.insert(
                    conn,
                    crate::connections::DeadConnState {
                        last_command_round: self.round_nr,
                        is_otclient,
                    },
                );
            }
        }
    }

    /// 772 `StartLogout(true, true)` + monster `State = SLEEPING` (`crnonpl.cc:2352-2415`).
    pub(crate) fn start_logout_despawn(&mut self, cid: CreatureId) {
        self.creature_begin_logout(cid, true, true);
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.state = MonsterState::Sleeping;
        }
    }

    /// 772 `Kill()` — `HP Set(0); Death()` (`cr.hh:567-570`) + idle `SLEEPING`.
    pub(crate) fn kill_for_despawn(&mut self, cid: CreatureId) {
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().health = 0;
        }
        self.mark_dead(cid);
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.state = MonsterState::Sleeping;
        }
    }

    /// 772 `LoggingOut && LogoutPossible == 0` → destructor / logout remove (`crmain.cc:1113-1125`).
    pub(crate) fn finalize_pending(&mut self) {
        self.scratch_creature_ids.clear();
        for (cid, k) in self.creatures.iter() {
            if k.base().logging_out {
                self.scratch_creature_ids.push(cid);
            }
        }
        for cid in std::mem::take(&mut self.scratch_creature_ids) {
            if self.player_logout_possible(cid) != LogoutPossible::Ok {
                continue;
            }
            let is_dead = self.creatures.get(cid).is_some_and(|k| k.base().is_dead);
            if is_dead {
                self.finalize_creature_death(cid);
            } else if matches!(self.creatures.get(cid), Some(CreatureKind::Player(_))) {
                let _ = self.player_try_finalize_logout(cid);
            } else {
                if let Some(pos) = self.creatures.get(cid).map(|k| k.position()) {
                    self.broadcast_magic_effect(pos, MAGIC_EFFECT_POFF);
                }
                self.remove_creature(cid);
            }
        }
    }

    /// `~TCreature` / `~TMonster` — corpse, loot, monster XP, death-save, remove.
    pub(crate) fn finalize_creature_death(&mut self, victim: CreatureId) {
        if self.creatures.get(victim).is_none() {
            return;
        }
        let is_player = matches!(self.creatures.get(victim), Some(CreatureKind::Player(_)));
        let is_summon = self
            .creatures
            .get(victim)
            .is_some_and(|k| k.base().master.is_some());

        if is_player {
            self.player_death_drop_inventory(victim);
        }

        let corpse_snapshot = self.creatures.get(victim).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            // ~TCreature always places pool+corpse when IsDead (`crmain.cc:207-250`).
            // `drop_loot` / LoseInventory only gates inventory transfer, not the corpse.
            let inventory = if m.base.drop_loot {
                m.inventory.clone()
            } else {
                MonsterInventory::default()
            };
            Some((m.base.position, m.corpse_id, m.blood, inventory))
        });
        if let Some((pos, corpse_id, blood, inventory)) = corpse_snapshot {
            self.drop_monster_corpse(pos, corpse_id, blood, &inventory);
        }

        if crate::chase_debug::chase_path_debug_enabled()
            && let Some(CreatureKind::Monster(m)) = self.creatures.get(victim)
        {
            let killer_id = m
                .base
                .damage_map
                .most_dangerous(self.round_nr, self.mechanics.profile.exp_attribution_rounds)
                .map(|id| id.data().as_ffi())
                .unwrap_or(0);
            crate::chase_debug::log_creature_death(
                self.chase_trace_tick(),
                victim,
                &m.base.name,
                killer_id,
                m.experience,
                m.corpse_id,
            );
        }

        // Monster XP in `~TMonster` when Master==0 (`crnonpl.cc:2113-2126`).
        if !is_player && !is_summon {
            self.run_handle_creature_death(victim, true);
        }

        if is_player {
            self.prepare_player_death_save(victim);
            let db = self.db.clone();
            match self.build_player_save_data(victim) {
                Ok(mut data) => {
                    let temple = self
                        .player_temple_position(victim)
                        .unwrap_or(Position::new(0, 0, 0));
                    data.player.posx = i32::from(temple.x);
                    data.player.posy = i32::from(temple.y);
                    data.player.posz = i32::from(temple.z);
                    let guid = data.player.id;
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        handle.spawn(async move {
                            if let Err(e) = PlayerStore::new(&db).save_player(&data).await {
                                tracing::error!(?e, guid, "player save on death failed");
                            }
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        ?e,
                        ?victim,
                        "build_player_save_data failed on death — body still removed"
                    );
                }
            }
        }

        let dead_conn = is_player.then(|| self.conn_for_creature(victim)).flatten();
        self.remove_creature(victim);
        if let Some(conn) = dead_conn {
            self.unregister_conn_mapping(conn);
        }
    }

    fn announce_health_zero(&mut self, cid: CreatureId) {
        if let Some(snap) = self.combat_notify_snapshot(cid) {
            self.notify_creature_healed(cid, snap);
        }
    }

    fn run_handle_creature_death(&mut self, victim: CreatureId, schedule_generic_corpse: bool) {
        let decay_now = self.now_ms();
        let victim_active_promotion = self.player_active_promotion(victim);
        let (leveled, xp_grants) = crate::lua_scope::with_lua_script_scope(self, |world| {
            crate::death::handle_creature_death(
                &mut world.creatures,
                &mut world.items,
                &mut world.decay,
                world.events.as_ref(),
                victim,
                decay_now,
                None,
                world.mechanics.profile.step_speed,
                world.config.as_ref(),
                schedule_generic_corpse,
                world.mechanics.profile.corpse_decay_offset_ms,
                world.pvp_config.world_type,
                &world.mechanics.profile,
                world.round_nr,
                victim_active_promotion,
            )
        });
        for cid in leveled {
            self.announce_creature_speed(cid);
            self.player_check_combat_values(cid);
        }
        for grant in xp_grants {
            let is_player = matches!(self.creatures.get(grant.cid), Some(CreatureKind::Player(_)));
            if is_player {
                self.send_player_stats(grant.cid);
            }
            if grant.amount > 0
                && let Some(pos) = self.creatures.get(grant.cid).map(|k| k.position())
            {
                self.broadcast_experience_popup(pos, grant.amount);
            }
            if is_player && grant.new_level > grant.old_level {
                self.send_player_advance_message(
                    grant.cid,
                    &format!(
                        "You advanced from Level {} to Level {}.",
                        grant.old_level, grant.new_level
                    ),
                );
            } else if is_player && grant.new_level < grant.old_level {
                self.send_player_advance_message(
                    grant.cid,
                    &format!(
                        "You were downgraded from Level {} to Level {}.",
                        grant.old_level, grant.new_level
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::Position;
    use tfs_rust_common::enums::CombatType;

    use crate::combat::{CombatDamage, CombatParams};
    use crate::creature::CreatureKind;
    use crate::test_world::support::{
        TEST_SYNTHETIC_GROUND_WP, beat_driven_test_world, ensure_walkable_tile, insert_monster,
        insert_player, test_player,
    };

    fn tile_has_item(world: &crate::game_world::GameWorld, pos: Position) -> bool {
        world
            .map
            .get_tile(pos)
            .is_some_and(|t| !t.body().down_items.is_empty())
    }

    #[test]
    fn death_finalizes_on_next_process_creatures() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.base.health = 1;
            m.base.max_health = 1;
            m.base.drop_loot = true;
        }
        let player = insert_player(&mut world, test_player("Hero", pos));
        world.map.register_creature_at(pos, player);

        let applied = world.combat_execute_with_stimulus(
            Some(player),
            monster,
            &CombatDamage {
                primary: (CombatType::Physical, -50),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(applied > 0);
        assert!(
            world.creatures.contains_key(monster),
            "body stays on the map until ProcessCreatures"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().is_dead && k.base().logging_out)
        );

        world.process_creatures();
        assert!(
            !world.creatures.contains_key(monster),
            "ProcessCreatures destructor removes the body"
        );
        assert!(
            tile_has_item(&world, pos),
            "corpse/pool lands on the death tile at finalize"
        );
    }

    #[test]
    fn dead_body_takes_no_damage() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);
        world.mark_dead(monster);
        let applied = world.combat_execute_with_stimulus(
            None,
            monster,
            &CombatDamage {
                primary: (CombatType::Physical, -10),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert_eq!(applied, 0, "Damage returns 0 when IsDead (`crmain.cc:488`)");
        assert!(world.creatures.contains_key(monster));
    }

    #[test]
    fn dead_body_runs_no_todo() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().health = 50;
            k.base_mut().next_wakeup = Some(0);
        }
        world.mark_dead(monster);
        world.process_creature_todo(monster);
        assert!(
            world.creatures.contains_key(monster),
            "Execute is a no-op on a dead body"
        );
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().next_wakeup.is_some()),
            "wakeup must not be consumed"
        );
    }

    #[test]
    fn summon_despawns_via_idle_after_master_removed() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let master = insert_monster(&mut world, "Master", mpos, 200);
        let summon = insert_monster(&mut world, "Summon", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.bind_master(master, false);
        }
        world.remove_creature(master);
        assert!(
            world.creatures.contains_key(summon),
            "remove_creature must not cascade to summons"
        );

        world.monster_idle_stimulus(summon);
        assert!(
            world.creatures.contains_key(summon),
            "Kill() leaves the summon until ProcessCreatures"
        );
        assert!(
            world
                .creatures
                .get(summon)
                .is_some_and(|k| k.base().is_dead && k.base().logging_out)
        );

        world.process_creatures();
        assert!(
            !world.creatures.contains_key(summon),
            "ProcessCreatures removes the logging-out summon"
        );
        assert!(
            tile_has_item(&world, mpos),
            "monster-master gone uses Kill → corpse (`crmain.cc:207-250`)"
        );
    }

    #[test]
    fn summon_player_master_gone_start_logout() {
        let mut world = beat_driven_test_world();
        let mpos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, TEST_SYNTHETIC_GROUND_WP);
        let master = insert_player(&mut world, test_player("Hero", mpos));
        world.map.register_creature_at(mpos, master);
        let summon = insert_monster(&mut world, "Summon", mpos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.bind_master(master, true);
        }
        world.remove_creature(master);
        world.monster_idle_stimulus(summon);
        assert!(
            world
                .creatures
                .get(summon)
                .is_some_and(|k| k.base().logging_out && !k.base().is_dead),
            "player-master gone → StartLogout (`crnonpl.cc:2388`)"
        );
        world.process_creatures();
        assert!(!world.creatures.contains_key(summon));
    }

    #[test]
    fn combat_killed_summon_leaves_empty_corpse() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let master = insert_player(&mut world, test_player("Hero", pos));
        world.map.register_creature_at(pos, master);
        let summon = insert_monster(&mut world, "Rat", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.bind_master(master, true);
            m.base.drop_loot = false;
            m.base.health = 1;
            m.base.max_health = 1;
        }
        let applied = world.combat_execute_with_stimulus(
            Some(master),
            summon,
            &CombatDamage {
                primary: (CombatType::Physical, -50),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(applied > 0);
        assert!(world.creatures.contains_key(summon));
        world.process_creatures();
        assert!(!world.creatures.contains_key(summon));
        assert!(
            tile_has_item(&world, pos),
            "IsDead summon still gets ~TCreature corpse (`crmain.cc:207-250`)"
        );
    }

    #[test]
    fn combat_kill_grants_monster_skill_level_exp() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let kicker = insert_monster(&mut world, "Dragon", pos, 200);
        let victim = insert_monster(&mut world, "Rat", pos, 200);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(victim) {
            m.base.health = 1;
            m.base.max_health = 1;
            m.experience = 5;
        }
        let applied = world.combat_execute_with_stimulus(
            Some(kicker),
            victim,
            &CombatDamage {
                primary: (CombatType::Physical, -50),
                secondary: (CombatType::Physical, 0),
            },
            &CombatParams::default(),
        );
        assert!(applied > 0);
        world.process_creatures();
        assert!(!world.creatures.contains_key(victim));
        let got = match world.creatures.get(kicker) {
            Some(CreatureKind::Monster(m)) => m.skill_level_exp,
            _ => panic!("kicker"),
        };
        assert_eq!(
            got, 5,
            "AoE/melee Damage combat-list credit grants SKILL_LEVEL exp to the monster"
        );
    }

    #[test]
    fn logging_out_living_still_runs_todo() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, TEST_SYNTHETIC_GROUND_WP);
        let monster = insert_monster(&mut world, "Rat", pos, 200);
        if let Some(k) = world.creatures.get_mut(monster) {
            k.base_mut().health = 50;
            k.base_mut().next_wakeup = Some(0);
        }
        world.start_logout_despawn(monster);
        assert!(
            world
                .creatures
                .get(monster)
                .is_some_and(|k| k.base().logging_out && !k.base().is_dead && k.base().health > 0)
        );
        world.process_creature_todo(monster);
        let wakeup = world
            .creatures
            .get(monster)
            .and_then(|k| k.base().next_wakeup);
        assert_ne!(
            wakeup,
            Some(0),
            "Execute must run for living LoggingOut (`cract.cc:785`); skip would leave wakeup=0"
        );
        assert!(world.creatures.contains_key(monster));
    }
}
