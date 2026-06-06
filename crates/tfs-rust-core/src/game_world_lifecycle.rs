//! Creature release, removal, logout, and death.
//!
//! - `Game::removeCreature`, `Game::ReleaseCreature`, `Game::cleanup` — `game.cpp`.
//! - `ProtocolGame::logout` — `protocolgame.cpp`.

use std::time::Instant;

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
        let now = Instant::now();
        self.on_creature_removed_for_spawn(id, now);

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
        self.remove_creature_think_check(id);
        self.creatures.remove(id);
    }

    /// TFS `ProtocolGame::logout` (`protocolgame.cpp:336-372`).
    /// Handles player logout with validation, effects, and cleanup.
    // C++ ref: src/protocolgame.cpp:336-372
    pub fn player_logout(&mut self, conn_id: ConnId, cid: CreatureId, display_effect: bool, forced: bool) {
        // Verify player exists
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };

        // Check logout conditions if not forced
        if !forced {
            // Check if player has access (gamemaster/canAlwaysLogin flag equivalent)
            // C++: player->isAccessPlayer() checks group access
            // Using ghost_mode as proxy for GM access until proper groups are implemented
            let has_access = player.ghost_mode;

            if !has_access {
                // Check no-logout zone (TILESTATE_NOLOGOUT)
                let pos = player.base.position;
                if let Some(tile) = self.map.get_tile(pos) {
                    if tile.body().zone == ZoneType::NoLogout {
                        self.send_cancel_message(conn_id, ReturnValue::YouCannotLogoutHere);
                        return;
                    }

                    // Check infight condition outside protection zone
                    let in_protection_zone = tile.body().zone == ZoneType::Protection;
                    let has_infight = player
                        .base
                        .active_conditions
                        .iter()
                        .any(|c| c.ctype == ConditionType::Infight);
                    if !in_protection_zone && has_infight {
                        self.send_cancel_message(conn_id, ReturnValue::YouMayNotLogoutDuringAFight);
                        return;
                    }
                }
            }

            // Scripting event - onLogout
            // C++ ref: src/protocolgame.cpp:357 (`g_creatureEvents->playerLogout(player)`).
            self.events.on_logout(cid, self);
        }

        // Get player data for effect
        let health = player.base.health;
        let ghost_mode = player.ghost_mode;
        let pos = player.base.position;

        // Show logout effect if requested and player is alive and not in ghost mode
        // C++: if (displayEffect && player->getHealth() > 0 && !player->isInGhostMode())
        if display_effect && health > 0 && !ghost_mode {
            // Magic effect CONST_ME_POFF (value 4)
            self.broadcast_magic_effect(pos, 4);
        }

        // Remove connection mapping
        self.conn_to_creature.remove(&conn_id);
        self.known_creatures_by_conn.remove(&conn_id);
        self.creature_fully_sent_by_conn.remove(&conn_id);

        // Remove creature from world (C++: g_game.removeCreature(player))
        self.remove_creature(cid);

        tracing::info!(guid = self.player_guid(cid).unwrap_or(0), "player logged out");
    }

    /// Run death XP / events / corpse scheduling, then remove the creature (and summons).
    pub fn apply_creature_death(&mut self, victim: CreatureId) {
        crate::death::handle_creature_death(
            &mut self.creatures,
            &mut self.items,
            &mut self.decay,
            self.events.as_ref(),
            victim,
            self.tick_counter,
            None,
            self.mechanics.profile.step_speed,
            self.config.as_ref(),
        );
        self.remove_creature(victim);
    }
}
