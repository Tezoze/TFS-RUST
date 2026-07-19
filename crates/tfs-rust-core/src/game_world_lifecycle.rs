//! Creature release, removal, logout, and death.
//!
//! - `Game::removeCreature`, `Game::ReleaseCreature`, `Game::cleanup` — `game.cpp`.
//! - `ProtocolGame::logout` — `protocolgame.cpp`.

use slotmap::Key;
use tfs_rust_common::enums::{ConditionType, ZoneType};
use tfs_rust_common::ConnId;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;

impl GameWorld {
    /// TFS `Game::ReleaseCreature` — deferred until [`Self::cleanup`] (`src/game.cpp` ~4766).
    pub fn release_creature(&mut self, id: CreatureId) {
        self.creatures_pending_release.push(id);
    }

    /// TFS `Game::ReleaseItem` — deferred until [`Self::cleanup`] (`src/game.cpp` ~4771).
    pub fn release_item(&mut self, id: ItemId) {
        self.items_pending_release.push(id);
    }

    /// TFS `Game::cleanup` (`src/game.cpp` ~4752) — after `Creature::onWalk` (`src/game.cpp` ~3778).
    pub fn cleanup(&mut self) {
        let creatures = std::mem::take(&mut self.creatures_pending_release);
        for id in creatures {
            if self.creatures.contains_key(id) {
                self.remove_creature(id);
            }
        }
        let items = std::mem::take(&mut self.items_pending_release);
        for id in items {
            self.decay.cancel(id);
            let _ = self.items.remove(id);
        }
    }

    /// Remove creature from map index, player lookups, guild online; remove summons if master dies.
    // C++ reference: `Game::removeCreature` — summon chain + spectator disappear.
    pub fn remove_creature(&mut self, id: CreatureId) {
        let now_ms = self.now_ms();
        self.on_creature_removed_for_spawn(id, now_ms);

        let mut summons: Vec<CreatureId> = Vec::new();
        for (cid, k) in self.creatures.iter() {
            if k.base().master == Some(id) {
                summons.push(cid);
            }
        }
        for s in summons {
            self.remove_creature(s);
        }

        let pos = self.creatures.get(id).map(|k| k.position());
        let player_cleanup = self.creatures.get(id).and_then(|k| {
            if let CreatureKind::Player(pl) = k {
                Some((pl.base.name.clone(), pl.guid, pl.social.guild_id.is_some()))
            } else {
                None
            }
        });

        if let Some(p) = pos {
            // Players only at INFO — monster/NPC death used to spam this on every kill and
            // made mass-death (UE) look like a logout storm while adding log I/O cost.
            if player_cleanup.is_some() {
                tracing::info!(?id, at = ?p, "LOGOUT: unregistering creature from map");
            } else {
                tracing::debug!(?id, at = ?p, "unregistering creature from map");
            }
            self.map.unregister_creature_at(p, id);
        }

        if let Some((name, guid, in_guild)) = player_cleanup {
            self.player_by_name.remove(&name);
            self.player_by_guid.remove(&guid);
            if in_guild {
                self.guilds.unregister_online(id);
            }
        }

        let _ = self.container_registry.close_all_for_player(id);

        self.deferred_turn_broadcast.remove(&id);
        self.stop_event_walk(id);
        self.creatures.remove(id);
    }

    /// TFS / 772 `ProtocolGame::logout` gates — returns `false` when logout is cancelled
    /// (no-logout tile, in-fight, or `onLogout` script). Caller then issues `PlayerDisconnect`.
    // C++ ref: `gameserver/src/protocolgame.cpp` `ProtocolGame::logout`; TFS `protocolgame.cpp:336-372`.
    pub fn player_logout_allowed(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        forced: bool,
    ) -> bool {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return false;
        };

        if !forced {
            let has_access = player.ghost_mode;
            if !has_access {
                let pos = player.base.position;
                if let Some(tile) = self.map.get_tile(pos) {
                    if tile.body().zone == ZoneType::NoLogout {
                        self.send_cancel_message(conn_id, ReturnValue::YouCannotLogoutHere);
                        return false;
                    }

                    let in_protection_zone = tile.body().zone == ZoneType::Protection;
                    let has_infight = player
                        .base
                        .active_conditions
                        .iter()
                        .any(|c| c.ctype == ConditionType::Infight);
                    if !in_protection_zone && has_infight {
                        self.send_cancel_message(conn_id, ReturnValue::YouMayNotLogoutDuringAFight);
                        return false;
                    }
                }
            }

            // Scripting event - onLogout
            // C++ ref: src/protocolgame.cpp:357 (`g_creatureEvents->playerLogout(player)`).
            if !self.events.on_logout(cid, self) {
                return false;
            }
        }

        true
    }

    /// TFS `ProtocolGame::logout` (`protocolgame.cpp:336-372`).
    /// Validates then removes the player; prefer game-loop `PlayerDisconnect` so TCP closes.
    // C++ ref: src/protocolgame.cpp:336-372
    pub fn player_logout(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        display_effect: bool,
        forced: bool,
    ) {
        if !self.player_logout_allowed(conn_id, cid, forced) {
            return;
        }

        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };
        let health = player.base.health;
        let ghost_mode = player.ghost_mode;
        let pos = player.base.position;
        let guid = player.guid;

        if display_effect && health > 0 && !ghost_mode {
            self.broadcast_magic_effect(pos, 4);
        }

        self.unregister_conn_mapping(conn_id);
        self.known_creatures_by_conn.remove(&conn_id);
        self.creature_fully_sent_by_conn.remove(&conn_id);
        self.remove_creature(cid);

        tracing::info!(guid, "player logged out");
    }

    /// Run death XP / events / corpse scheduling, then remove the creature (and summons).
    pub fn apply_creature_death(&mut self, victim: CreatureId) {
        if self.creatures.get(victim).is_none() {
            return;
        }

        let is_player = matches!(
            self.creatures.get(victim),
            Some(CreatureKind::Player(_))
        );

        // PC-5 M7 — player skill-try loss + inventory drop (AoL / SOME) before XP share.
        if is_player {
            self.apply_player_death_skill_loss(victim);
            self.player_death_drop_inventory(victim);
        }

        let corpse_snapshot = self.creatures.get(victim).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            Some((m.base.position, m.corpse_id, m.blood, m.inventory.clone()))
        });

        if let Some((pos, corpse_id, blood, inventory)) = corpse_snapshot {
            self.drop_monster_corpse(pos, corpse_id, blood, &inventory);
        }

        if crate::chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(victim) {
                let killer_id = m
                    .base
                    .damage_map
                    .iter()
                    .max_by_key(|(_, dmg)| *dmg)
                    .map(|(id, _)| id.data().as_ffi())
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
        }

        let decay_now = self.now_ms();
        // Players already placed corpse 3128 in `player_death_drop_inventory`; skip generic 3058.
        let schedule_generic_corpse = !is_player;
        let (leveled, xp_grants) = crate::death::handle_creature_death(
            &mut self.creatures,
            &mut self.items,
            &mut self.decay,
            self.events.as_ref(),
            victim,
            decay_now,
            None,
            self.mechanics.profile.step_speed,
            self.config.as_ref(),
            schedule_generic_corpse,
            self.mechanics.profile.corpse_decay_offset_ms,
        );
        // C++ `cract.cc:1637` `CREATURE_SPEED_CHANGED` — announce new speed to spectators
        // for any killer (or victim) whose level changed via experience gain/loss.
        for cid in leveled {
            self.announce_creature_speed(cid);
        }
        // TFS/772: `sendStats` + animated exp popup (`Creature::onGainExperience`) + level advance text.
        // Victim is always in `xp_grants` (even at zero exp loss) so blessing clear reaches the client.
        for grant in xp_grants {
            self.send_player_stats(grant.cid);
            if grant.amount > 0 {
                if let Some(pos) = self.creatures.get(grant.cid).map(|k| k.position()) {
                    self.broadcast_experience_popup(pos, grant.amount);
                }
            }
            if grant.new_level > grant.old_level {
                // 772 `Player::addExperience` — `player.cpp:1548`.
                self.send_player_advance_message(
                    grant.cid,
                    &format!(
                        "You advanced from Level {} to Level {}.",
                        grant.old_level, grant.new_level
                    ),
                );
            } else if grant.new_level < grant.old_level {
                self.send_player_advance_message(
                    grant.cid,
                    &format!(
                        "You were downgraded from Level {} to Level {}.",
                        grant.old_level, grant.new_level
                    ),
                );
            }
        }
        // TFS `Player::death` — `sendSkills()` after death penalties (`player.cpp:2154`).
        if is_player {
            self.send_player_skills(victim);
        }
        self.remove_creature(victim);
    }

    /// PC-5 M7 — skill / magic try loss at the bless-reduced death fraction.
    ///
    /// TFS `Player::death` skill loop (`player.cpp:2099-2108`); 772 `DecreasePercent`
    /// (`crplayer.cc:352-360`) produces the same demotion outcomes with per-level tries.
    fn apply_player_death_skill_loss(&mut self, victim: CreatureId) {
        let profile = self.mechanics.profile;
        let hooks = &self.mechanics.hooks;
        let frac = {
            let Some(CreatureKind::Player(v)) = self.creatures.get(victim) else {
                return;
            };
            crate::death::death_loss_fraction(
                self.config.as_ref(),
                v.level,
                v.experience,
                v.blessings,
            )
            .clamp(0.0, 1.0)
        };
        if let Some(CreatureKind::Player(v)) = self.creatures.get_mut(victim) {
            for skill in crate::player::combat::SkillNr::COMBAT_ALL {
                let total = v.skill_total_tries(skill, &profile, hooks);
                let lose_tries = ((total as f64) * frac).floor() as u64;
                let _ = v.skill_decrease(skill, lose_tries, &profile, hooks);
            }
            let mag_total = v.magic_total_tries(&profile, hooks);
            let mag_lose = ((mag_total as f64) * frac).floor() as u64;
            let _ = v.magic_decrease(mag_lose, &profile, hooks);
        }
    }

    /// PC-5 M7 — amulet of loss + inventory drop onto dead-human corpse.
    ///
    /// C++ `crmain.cc:790-815` (AoL → `LOSE_INVENTORY_NONE` + delete amulet);
    /// `crmain.cc:267-281` (`LOSE_INVENTORY_SOME`: containers always, else 10% chance).
    /// Corpse type `3128` (dead human). Default player mode is SOME (`crplayer.cc:30`).
    fn player_death_drop_inventory(&mut self, victim: CreatureId) {
        const AMULET_OF_LOSS: u16 = 2173;
        const DEAD_HUMAN_CORPSE: u16 = 3128;

        let pos = match self.creatures.get(victim) {
            Some(CreatureKind::Player(p)) => p.base.position,
            _ => return,
        };

        // Scan for amulet of loss in the necklace slot (or any clothes slot matching type).
        let mut lose_none = false;
        let necklace_slot = crate::inventory::InventorySlot::Necklace as u8;
        if let Some(iid) = self.get_player_inventory_item(victim, necklace_slot) {
            if self.items.get(iid).is_some_and(|i| i.item_type == AMULET_OF_LOSS) {
                lose_none = true;
                let _ = self.internal_remove_item_from_inventory_slot(victim, necklace_slot, iid);
                self.items.remove(iid);
                tracing::info!(?victim, "player died with amulet of loss");
            }
        }

        // Always create the corpse; items only move when lose mode is SOME.
        let corpse_id = self.items.insert(crate::item::Item::new(DEAD_HUMAN_CORPSE, 1));
        self.hydrate_container_if_needed(corpse_id);
        let decay_deadline = self
            .now_ms()
            .saturating_add(self.mechanics.profile.corpse_decay_offset_ms);
        self.decay.schedule(corpse_id, decay_deadline, None);
        if self
            .internal_add_item_to_tile(pos, corpse_id, crate::cylinder::CylinderFlags::NO_LIMIT)
            .is_err()
        {
            tracing::warn!(?pos, "player corpse could not be placed on tile");
        }

        if lose_none {
            return;
        }

        // LOSE_INVENTORY_SOME — drop containers always, other slots with 1/10 chance.
        for slot in 1u8..=10u8 {
            let Some(iid) = self.get_player_inventory_item(victim, slot) else {
                continue;
            };
            let is_container = self
                .items
                .get(iid)
                .is_some_and(|i| self.items_db.is_container(i.item_type));
            let drop = is_container || {
                #[cfg(any(test, feature = "sim"))]
                {
                    if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
                        crate::sim_glibc_rand::parity_rand_mod(10) == 0
                    } else {
                        use rand::Rng;
                        self.ai_rng.gen_range(0..10) == 0
                    }
                }
                #[cfg(not(any(test, feature = "sim")))]
                {
                    use rand::Rng;
                    self.ai_rng.gen_range(0..10) == 0
                }
            };
            if !drop {
                continue;
            }
            if self
                .internal_remove_item_from_inventory_slot(victim, slot, iid)
                .is_ok()
            {
                self.move_body_item_into_corpse(corpse_id, iid);
            }
        }
    }
}
