//! Creature release, removal, logout, death, and relog TakeOver.
//!
//! - `Game::removeCreature`, `Game::ReleaseCreature`, `Game::cleanup` — `game.cpp`.
//! - `ProtocolGame::logout` — `protocolgame.cpp`.
//! - 772 `TConnection` login TakeOver — `connections.cc:224-253`; `TPlayer::TakeOver` —
//!   `crplayer.cc:721-775`.

use slotmap::Key;
use tfs_rust_common::enums::{ConditionType, PlayerSex, SkullType, ZoneType};
use tfs_rust_common::{ConnId, Position};

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::return_value::ReturnValue;
use crate::tile::flags as tilestate;
use tfs_rust_db::player::PlayerStore;

/// 772 `TCreature::LogoutPossible` result (`crmain.cc:417-431`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoutPossible {
    Ok,
    Combat,
    NoLogoutField,
}

/// 772 player-corpse look text — `crmain.cc:253-264` (`Change(Corpse, TEXTSTRING, …)`).
/// Domain: TFS/TVP `Player::getCorpse` `setSpecialDescription` (`player.cpp:1943-1948`).
fn player_corpse_special_description(
    victim_name: &str,
    sex: PlayerSex,
    killer_name: Option<&str>,
) -> String {
    let pronoun = match sex {
        PlayerSex::Female => "She",
        PlayerSex::Male => "He",
    };
    match killer_name.filter(|n| !n.is_empty()) {
        Some(killer) => format!("You recognize {victim_name}. {pronoun} was killed by {killer}."),
        None => format!("You recognize {victim_name}."),
    }
}

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
        // NPC-7: fire onDisappear before teardown when registered.
        let disappear_cb = match self.creatures.get(id) {
            Some(CreatureKind::Npc(n)) => {
                self.npcs_db.get(n.definition).and_then(|d| d.on_disappear)
            }
            _ => None,
        };
        if let Some(cb) = disappear_cb {
            crate::lua_scope::fire_npc_disappear(self, id, cb);
        }

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
            // 772 `TNPC::CreatureMoveStimulus` with OBJECT_DELETED — prune queue / VANISH focus.
            self.npc_dispatch_creature_move(id, p, p, true);
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
            // 772 player teardown calls `ClearPlayerkillingMarks` (`crplayer.cc:315`).
            self.clear_playerkilling_marks(id);
            self.player_by_name.remove(&name);
            self.player_by_guid.remove(&guid);
            let db = self.db.clone();
            tokio::spawn(async move {
                if let Err(e) = tfs_rust_db::delete_player_online(&db, guid).await {
                    tracing::warn!(error = %e, guid, "players_online delete failed");
                }
            });
            if in_guild {
                self.guilds.unregister_online(id);
            }
        }

        let _ = self.container_registry.close_all_for_player(id);

        self.deferred_turn_broadcast.remove(&id);
        self.stop_event_walk(id);
        self.creatures.remove(id);
    }

    /// 772 `TCreature::LogoutPossible` — `crmain.cc:417-431`.
    ///
    /// On success, sets `LogoutAllowed = true` (sticky). Used by `CQuitGame` before
    /// `Logout`, and by `ProcessCreatures` to finalize `LoggingOut` bodies.
    pub fn player_logout_possible(&mut self, cid: CreatureId) -> LogoutPossible {
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            return LogoutPossible::Ok;
        };
        if p.logout_allowed || p.base.health <= 0 {
            return LogoutPossible::Ok;
        }
        let round_nr = self.round_nr;
        let earliest = p.earliest_logout_round;
        let pos = p.base.position;
        if earliest > round_nr {
            return LogoutPossible::Combat;
        }
        // NoLogout is an orthogonal tile flag — TFS `getZone()` never returns
        // `ZONE_NOLOGOUT`; OTBM sets `TILESTATE_NOLOGOUT` only (`iomap.cpp:270-280`).
        // 772: `IsNoLogoutField` (`map.cc`).
        if self
            .map
            .get_tile(pos)
            .is_some_and(|t| t.body().flags & tilestate::NOLOGOUT != 0)
        {
            return LogoutPossible::NoLogoutField;
        }
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.logout_allowed = true;
        }
        LogoutPossible::Ok
    }

    /// TFS / 772 logout gates — returns `false` when logout is cancelled
    /// (no-logout tile, in-fight, or `onLogout` script). Caller then issues `PlayerDisconnect`.
    // C++ ref: 772 `TCreature::LogoutPossible` (`crmain.cc:417-431`) + `CQuitGame`
    // (`receiving.cc:88-98`); TFS `ProtocolGame::logout` (`protocolgame.cpp:336-372`).
    pub fn player_logout_allowed(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        forced: bool,
    ) -> bool {
        // Body already removed after death, or CONNECTION_DEAD — OK must still close TCP.
        // 772 `CommandAllowed` + `CQuitGame` after `TConnection::Die` (`receiving.cc:17-21`).
        if self.dead_connections.contains(&conn_id) || self.creatures.get(cid).is_none() {
            return true;
        }

        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return true;
        };

        // 772 `LogoutPossible`: dead always OK — skip Infight / combat / nologout gates
        // (`crmain.cc:417-418` `!IsDead` guard).
        if player.base.health <= 0 {
            return true;
        }

        if !forced {
            let has_access = player.ghost_mode;
            if !has_access {
                // TFS Infight residual before sticky `LogoutAllowed` (`protocolgame.cpp:351`).
                if let Some(CreatureKind::Player(player)) = self.creatures.get(cid) {
                    let pos = player.base.position;
                    let has_infight = player
                        .base
                        .active_conditions
                        .iter()
                        .any(|c| c.ctype == ConditionType::Infight);
                    let in_pz = self
                        .map
                        .get_tile(pos)
                        .is_some_and(|t| t.body().zone == ZoneType::Protection);
                    if has_infight && !in_pz {
                        self.send_cancel_message(conn_id, ReturnValue::YouMayNotLogoutDuringAFight);
                        return false;
                    }
                }
                match self.player_logout_possible(cid) {
                    LogoutPossible::Ok => {}
                    LogoutPossible::Combat => {
                        self.send_cancel_message(conn_id, ReturnValue::YouMayNotLogoutDuringAFight);
                        return false;
                    }
                    LogoutPossible::NoLogoutField => {
                        self.send_cancel_message(conn_id, ReturnValue::YouCannotLogoutHere);
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

    /// 772 `TCreature::StartLogout` — `crmain.cc:404-415`.
    ///
    /// Sets `LoggingOut`, optionally forces `LogoutAllowed`, and schedules
    /// `StopAttack(0)` or `StopAttack(60)` from `stop_fight`. Does **not** remove the
    /// creature — `ProcessCreatures` finalizes when `LogoutPossible` succeeds.
    pub(crate) fn creature_begin_logout(&mut self, cid: CreatureId, force: bool, stop_fight: bool) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.logging_out = true;
            if force {
                p.logout_allowed = true;
            }
        }
        self.creature_start_logout_stop_fight(cid, stop_fight);
    }

    /// 772 login TakeOver gate + attach — `connections.cc:231-252`, `crplayer.cc:721-775`.
    ///
    /// When `player_by_guid` already has a live body:
    /// - dead → `Err` (dying — login failed)
    /// - `LoggingOut && LogoutPossible == Ok` → `Err` (about to despawn)
    /// - else → cancel logout, `StopAttack(0)`, leave channels, close containers, detach
    ///   old conn mapping (caller closes that TCP **without** `StartLogout`), return
    ///   `Some((cid, old_conn))`
    ///
    /// `None` = no existing body — caller should spawn from DB.
    pub(crate) fn player_try_takeover_for_login(
        &mut self,
        guid: u32,
        name: &str,
        operating_system: u16,
        otclient_v8: u16,
    ) -> Result<Option<(CreatureId, Option<ConnId>)>, tfs_rust_common::error::TfsRustError> {
        let Some(&cid) = self.player_by_guid.get(&guid) else {
            return Ok(None);
        };
        let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
            // Stale index — drop and allow a fresh spawn.
            self.player_by_guid.remove(&guid);
            if self.player_by_name.get(name).copied() == Some(cid) {
                self.player_by_name.remove(name);
            }
            return Ok(None);
        };

        if p.base.health <= 0 {
            return Err(tfs_rust_common::error::TfsRustError::Database(format!(
                "player `{name}` is dying — login failed"
            )));
        }

        let logging_out = p.logging_out;
        if logging_out && self.player_logout_possible(cid) == LogoutPossible::Ok {
            // `connections.cc:238-241` — body is finalize-ready; reject until removed.
            return Err(tfs_rust_common::error::TfsRustError::Database(format!(
                "player `{name}` is logging out — login failed"
            )));
        }

        let old_conn = self.player_takeover(cid, operating_system, otclient_v8);
        Ok(Some((cid, old_conn)))
    }

    /// 772 `TPlayer::TakeOver` — `crplayer.cc:721-775`.
    ///
    /// Clears `LoggingOut` / `LogoutAllowed`, stops attack, leaves channels, closes
    /// open containers. Detaches any prior connection mapping and returns that
    /// `ConnId` so the caller can close TCP without `StartLogout` (772 sets
    /// `OldConnection->CharacterID = 0` before `Logout`).
    pub(crate) fn player_takeover(
        &mut self,
        cid: CreatureId,
        operating_system: u16,
        otclient_v8: u16,
    ) -> Option<ConnId> {
        let old_conn = self.creature_to_conn.get(&cid).copied();
        if let Some(old) = old_conn {
            // `ClearConnection` — do not `StartLogout` (CharacterID already conceptually 0).
            self.unregister_conn_mapping(old);
            self.known_creatures_by_conn.remove(&old);
            self.creature_fully_sent_by_conn.remove(&old);
        }

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.logging_out = false;
            p.logout_allowed = false;
            p.operating_system = operating_system;
            p.otclient_v8 = otclient_v8;
        }

        // `Combat.StopAttack(0)` — TakeOver always clears dest (`crplayer.cc:757`).
        self.combat_stop_attack(cid, 0);
        self.chat.remove_user_from_all_channels(cid);
        let _ = self.container_registry.close_all_for_player(cid);
        // `RejectTrade` — trade not ported yet.

        tracing::info!(?cid, old_conn = ?old_conn.map(|c| c.0), "player takeover");
        old_conn
    }

    /// Finalize a `LoggingOut` player when `LogoutPossible` — `crmain.cc:1113-1124`.
    ///
    /// Returns `true` if the creature was removed.
    pub(crate) fn player_try_finalize_logout(&mut self, cid: CreatureId) -> bool {
        let logging_out = self
            .creatures
            .get(cid)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.logging_out),
                _ => None,
            })
            .unwrap_or(false);
        if !logging_out {
            return false;
        }
        if self.player_logout_possible(cid) != LogoutPossible::Ok {
            return false;
        }
        // Re-save: body may have taken damage / finished strikes while deferred.
        if let Ok(data) = self.build_player_save_data(cid) {
            let db = self.db.clone();
            let guid = data.player.id;
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    if let Err(e) = PlayerStore::new(&db).save_player(&data).await {
                        tracing::error!(?e, guid, "player save on deferred logout failed");
                    }
                });
            }
        }
        let guid = self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Player(p) => Some(p.guid),
            _ => None,
        });
        self.remove_creature(cid);
        if let Some(guid) = guid {
            tracing::info!(guid, "player deferred logout finalized");
        }
        true
    }

    /// TFS `CONST_ME_POFF` / 772 `EFFECT_POFF` — logout puff (`protocolgame.cpp` logout).
    /// Not `CONST_ME_BLOCKHIT` (4); that is the gray spark, not the smoke poff.
    const MAGIC_EFFECT_POFF: u8 = 3;

    /// Spectator logout puff — TFS `addMagicEffect(..., CONST_ME_POFF)` when
    /// `displayEffect && health > 0 && !ghost`. Used by protocol logout and TCP disconnect.
    pub(crate) fn broadcast_player_logout_poff(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Player(player)) = self.creatures.get(cid) else {
            return;
        };
        if player.base.health <= 0 || player.ghost_mode {
            return;
        }
        let pos = player.base.position;
        self.broadcast_magic_effect(pos, Self::MAGIC_EFFECT_POFF);
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
        let guid = player.guid;

        if display_effect {
            self.broadcast_player_logout_poff(cid);
        }

        // Intentional quit — `StopFight=true` (`receiving.cc:84,91`). Connection cleared by caller.
        self.creature_begin_logout(cid, forced, true);

        self.unregister_conn_mapping(conn_id);
        self.known_creatures_by_conn.remove(&conn_id);
        self.creature_fully_sent_by_conn.remove(&conn_id);

        // `LogoutPossible` already succeeded in `player_logout_allowed` → remove now.
        if self.player_logout_possible(cid) == LogoutPossible::Ok {
            self.remove_creature(cid);
            tracing::info!(guid, "player logged out");
        }
    }

    /// Run death XP / events / corpse scheduling, then remove the creature (and summons).
    pub fn apply_creature_death(&mut self, victim: CreatureId) {
        if self.creatures.get(victim).is_none() {
            return;
        }

        let is_player = matches!(self.creatures.get(victim), Some(CreatureKind::Player(_)));

        // 772 kill logout + RecordMurder before XP/remove (`crmain.cc:822–870`).
        if is_player {
            self.player_on_pvp_death_marks(victim);
        }

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

        let decay_now = self.now_ms();
        // Players already placed corpse 3128 in `player_death_drop_inventory`; skip generic 3058.
        let schedule_generic_corpse = !is_player;
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
            )
        });
        // C++ `cract.cc:1637` `CREATURE_SPEED_CHANGED` — announce new speed to spectators
        // for any killer (or victim) whose level changed via experience gain/loss.
        for cid in leveled {
            self.announce_creature_speed(cid);
            // 772 `TSkillLevel::Jump` → `Combat.CheckCombatValues()` (`crskill.cc:367`).
            self.player_check_combat_values(cid);
        }
        // TFS/772: `sendStats` + animated exp popup (`Creature::onGainExperience`) + level advance text.
        // Victim is always in `xp_grants` (even at zero exp loss) so blessing clear reaches the client.
        for grant in xp_grants {
            self.send_player_stats(grant.cid);
            if grant.amount > 0
                && let Some(pos) = self.creatures.get(grant.cid).map(|k| k.position())
            {
                self.broadcast_experience_popup(pos, grant.amount);
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
            // 772 `TPlayer::Death` — `crplayer.cc:331-334`: SendPlayerData (stats above),
            // `SendMessage(TALK_EVENT_MESSAGE, "You are dead.\n")`, `Connection->Die()`.
            // Trailing newline is required for the stock 772 death dialog.
            self.send_player_advance_message(victim, "You are dead.\n");
            if let Some(conn) = self.conn_for_creature(victim) {
                self.dead_connections.insert(conn);
            }
            // TFS `Player::death` (`player.cpp:2065` / `2157-2161` / TVP `1882-1897`):
            // set login position to temple, restore vitals, clear persistent conditions
            // *before* save — otherwise relog lands on the death tile at 1 HP and dies again.
            self.prepare_player_death_save(victim);
            // Persist before teardown — OK→Logout only closes TCP (`CONNECTION_DEAD`);
            // mapping is cleared so `handle_player_disconnect` cannot save afterwards.
            let db = self.db.clone();
            match self.build_player_save_data(victim) {
                Ok(mut data) => {
                    // TFS `loginPosition = town->getTemplePosition()` — write temple into the
                    // save row without moving the live body (remove still needs death tile).
                    let temple = self
                        .player_temple_position(victim)
                        .unwrap_or(Position::new(0, 0, 0));
                    data.player.posx = i32::from(temple.x);
                    data.player.posy = i32::from(temple.y);
                    data.player.posz = i32::from(temple.z);
                    let guid = data.player.id;
                    // Game loop always has a runtime; unit tests may not — skip spawn there.
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
        // Capture before remove — disappear broadcast still needs the mapping.
        let dead_conn = is_player.then(|| self.conn_for_creature(victim)).flatten();
        self.remove_creature(victim);
        // Drop ConnId↔CreatureId so a recycled SlotMap key cannot hijack the dead session.
        // TCP stays open (`CONNECTION_DEAD`) until OK→Logout / idle timeout.
        if let Some(conn) = dead_conn {
            self.unregister_conn_mapping(conn);
        }
    }

    /// TFS / TVP `Player::death` prep for the next login (`player.cpp:2065`, `2157-2161`).
    ///
    /// Restores HP/mana and clears conditions for the death-save. Does **not** move
    /// `base.position` — TFS keeps the corpse tile for remove fan-out and stores temple
    /// separately as `loginPosition` (patched onto the save row after build).
    pub(crate) fn prepare_player_death_save(&mut self, victim: CreatureId) {
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(victim) else {
            return;
        };
        // Black skull — TFS `player.cpp:2157-2161` (40 HP / 0 mana); else full vitals.
        if p.base.skull == SkullType::Black {
            p.base.health = 40.min(p.base.max_health.max(1));
            p.mana = 0;
        } else {
            p.base.health = p.base.max_health.max(1);
            p.mana = p.max_mana.max(0);
        }
        // Persistent combat/buff conditions must not survive death into the next login.
        p.base.active_conditions.clear();
        p.food_remaining = 0;
        p.food_level = 0;
    }

    /// Town temple used as TFS `loginPosition` after death (`Player::getTemplePosition`).
    pub(crate) fn player_temple_position(&self, victim: CreatureId) -> Option<Position> {
        let town_id = match self.creatures.get(victim) {
            Some(CreatureKind::Player(p)) => p.town_id,
            _ => return None,
        };
        self.map
            .towns
            .get(&(town_id as u32))
            .map(|t| t.temple_position)
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
            let promoted = v.vocation_profile.from_vocation != v.vocation_profile.id
                && v.vocation_profile.from_vocation != 0;
            crate::death::death_loss_fraction_for_profile(
                &profile,
                self.config.as_ref(),
                v.level,
                v.experience,
                v.blessings,
                promoted,
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
    /// `crmain.cc:267-281` drop logic: `LOSE_INVENTORY_ALL` drops everything,
    /// `LOSE_INVENTORY_SOME` drops containers always + 10% chance per slot.
    /// `crplayer.cc:292,296-300`: `LOSE_INVENTORY_ALL` when red skull
    /// (`PlayerkillerEnd != 0`); `LOSE_INVENTORY_NONE` under `KEEP_INVENTORY` right.
    /// Corpse type `3128` (dead human). Default player mode is SOME (`crplayer.cc:30`).
    ///
    /// AoL only when the killing blow was exact (`Damage == HitPoints`) — overkill skips it.
    /// Domain type id `2173` stands in for 772 `GetNewObjectType(77,12)` in the TFS pack.
    fn player_death_drop_inventory(&mut self, victim: CreatureId) {
        const AMULET_OF_LOSS: u16 = 2173;
        const DEAD_HUMAN_CORPSE: u16 = 3128;

        let last_hit = self.creatures.get(victim).and_then(|k| match k {
            CreatureKind::Player(p) => p.base.last_hit_by,
            _ => None,
        });
        let killer_name =
            last_hit.and_then(|kid| self.creatures.get(kid).map(|k| k.base().name.clone()));
        // Snapshot flags before borrowing the victim — `player_has_flag` also
        // reads `self.creatures`.
        let keep_inventory =
            self.player_has_flag(victim, crate::player_flags::PLAYER_FLAG_KEEP_INVENTORY);
        let (pos, exact_lethal, playerkiller_end, victim_name, sex) =
            match self.creatures.get(victim) {
                Some(CreatureKind::Player(p)) => (
                    p.base.position,
                    p.exact_lethal_blow,
                    p.playerkiller_end,
                    p.base.name.clone(),
                    p.sex,
                ),
                _ => return,
            };

        // M7 — Determine LoseInventory mode (`crplayer.cc:292,296-300`).
        // KEEP_INVENTORY right → NONE; red skull (PlayerkillerEnd != 0) → ALL; else SOME.
        let mut lose_none = keep_inventory;
        let lose_all = !lose_none && playerkiller_end != 0;

        if !lose_none && exact_lethal {
            // 772 loops all inventory slots requiring CLOTHES && BODYPOSITION == slot.
            for slot in crate::inventory::PLAYER_INVENTORY_SLOT_FIRST
                ..=crate::inventory::PLAYER_INVENTORY_SLOT_LAST
            {
                let Some(iid) = self.get_player_inventory_item(victim, slot) else {
                    continue;
                };
                let Some(item) = self.items.get(iid) else {
                    continue;
                };
                if item.item_type != AMULET_OF_LOSS {
                    continue;
                }
                let Some(it) = self.items_db.items.get(&item.item_type) else {
                    continue;
                };
                // TFS-domain: clothes slot mask must match the occupied slot (BODYPOSITION).
                if !crate::inventory::item_fits_equipment_slot(slot, it) {
                    continue;
                }
                lose_none = true;
                let _ = self.internal_remove_item_from_inventory_slot(victim, slot, iid);
                self.items.remove(iid);
                tracing::info!(?victim, slot, "player died with amulet of loss");
                break;
            }
        }

        // Always create the corpse; items only move when lose mode is not NONE.
        let corpse_id = self
            .items
            .insert(crate::item::Item::new(DEAD_HUMAN_CORPSE, 1));
        if let Some(item) = self.items.get_mut(corpse_id) {
            item.set_description(player_corpse_special_description(
                &victim_name,
                sex,
                killer_name.as_deref(),
            ));
        }
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

        // M7 — LOSE_INVENTORY_ALL drops everything; LOSE_INVENTORY_SOME uses 10% chance
        // + containers always (`crmain.cc:276-281`).
        for slot in 1u8..=10u8 {
            let Some(iid) = self.get_player_inventory_item(victim, slot) else {
                continue;
            };
            let drop = if lose_all {
                true
            } else {
                let is_container = self
                    .items
                    .get(iid)
                    .is_some_and(|i| self.items_db.is_container(i.item_type));
                is_container || self.parity_rand_mod(10) == 0
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim_harness::{
        ensure_walkable_tile, insert_player, insert_spectator_player, minimal_world, test_player,
    };
    use tfs_rust_common::Position;

    /// Place a non-container item in a player's inventory slot.
    fn place_item(world: &mut GameWorld, cid: CreatureId, slot: u8, item_type: u16) -> ItemId {
        let iid = world.items.insert(crate::item::Item::new(item_type, 1));
        world
            .internal_add_item_to_inventory_slot(cid, slot, iid)
            .expect("place item");
        iid
    }

    /// M7 — red skull (PlayerkillerEnd != 0) → LOSE_INVENTORY_ALL: every slot drops.
    #[test]
    fn m7_red_skull_drops_all_inventory() {
        let mut world = minimal_world();
        let cid = insert_player(&mut world, {
            let mut p = test_player("RedSkull", Position::new(100, 100, 7));
            p.playerkiller_end = 1_000_000; // non-zero → red skull
            p.exact_lethal_blow = false; // no AoL scan
            p
        });
        let iid1 = place_item(&mut world, cid, 1, 2148); // gold
        let iid2 = place_item(&mut world, cid, 2, 2148); // gold
        let iid3 = place_item(&mut world, cid, 5, 2148); // gold

        world.player_death_drop_inventory(cid);

        // All three items should have been removed from inventory (parent changed to corpse).
        let still_in_inv = [iid1, iid2, iid3]
            .iter()
            .filter(|iid| {
                world.items.get(**iid).is_some_and(|i| {
                    matches!(i.parent, Some(crate::cylinder::Cylinder::Inventory { .. }))
                })
            })
            .count();
        assert_eq!(still_in_inv, 0, "red skull drops all inventory items");
    }

    /// M7 — KEEP_INVENTORY flag → LOSE_INVENTORY_NONE: no items drop.
    #[test]
    fn m7_keep_inventory_flag_drops_nothing() {
        let mut world = minimal_world();
        // Register a group with the keepinventory flag.
        let mut groups = std::collections::HashMap::new();
        let mut flags = std::collections::HashMap::new();
        flags.insert("keepinventory".to_string(), true);
        groups.insert(
            1u16,
            tfs_rust_content::groups::Group {
                id: 1,
                name: "test".into(),
                access: true,
                max_depot_items: 0,
                max_vip_entries: 0,
                flags,
            },
        );
        world.groups = std::sync::Arc::new(tfs_rust_content::groups::GroupDatabase { groups });

        let cid = insert_player(&mut world, {
            let mut p = test_player("Keeper", Position::new(100, 100, 7));
            p.group_id = 1;
            p.exact_lethal_blow = false;
            p
        });
        let iid1 = place_item(&mut world, cid, 1, 2148);
        let iid2 = place_item(&mut world, cid, 5, 2148);

        world.player_death_drop_inventory(cid);

        // Both items should still be in inventory.
        for iid in [iid1, iid2] {
            let in_inv = world.items.get(iid).is_some_and(|i| {
                matches!(i.parent, Some(crate::cylinder::Cylinder::Inventory { .. }))
            });
            assert!(in_inv, "KEEP_INVENTORY flag preserves item in inventory");
        }
    }

    /// M7 — default (no red skull, no KEEP_INVENTORY) → LOSE_INVENTORY_SOME:
    /// containers always drop, other slots 1/10 chance.
    #[test]
    fn m7_default_mode_is_some_creates_corpse() {
        let mut world = minimal_world();
        let cid = insert_player(&mut world, {
            let mut p = test_player("Normal", Position::new(100, 100, 7));
            p.playerkiller_end = 0;
            p.exact_lethal_blow = false;
            p
        });
        let _iid = place_item(&mut world, cid, 1, 2148);

        world.player_death_drop_inventory(cid);

        // Corpse item 3128 should exist on the tile.
        let corpse = world
            .items
            .iter()
            .find(|(_, i)| i.item_type == 3128)
            .map(|(_, i)| i);
        assert!(corpse.is_some(), "corpse 3128 always created on death");
        assert_eq!(
            corpse.map(|i| i.description()),
            Some("You recognize Normal."),
            "no murderer → recognize-only look text (`crmain.cc:255`)"
        );
    }

    #[test]
    fn pvp_corpse_sets_killed_by_description() {
        // 772 `crmain.cc:253-264` — `Murderer` from last-hit attacker name.
        let mut world = minimal_world();
        let killer = insert_player(&mut world, test_player("Bob", Position::new(101, 100, 7)));
        let victim = insert_player(&mut world, {
            let mut p = test_player("Alice", Position::new(100, 100, 7));
            p.base.last_hit_by = Some(killer);
            p
        });
        world.player_death_drop_inventory(victim);
        let desc = world
            .items
            .iter()
            .find_map(|(_, i)| (i.item_type == 3128).then(|| i.description().to_string()));
        assert_eq!(
            desc.as_deref(),
            Some("You recognize Alice. He was killed by Bob.")
        );
    }

    fn pkt_is_poff_at(pkt: &[u8], pos: Position) -> bool {
        pkt.len() >= 7
            && pkt[0] == 0x83
            && u16::from_le_bytes([pkt[1], pkt[2]]) == pos.x
            && u16::from_le_bytes([pkt[3], pkt[4]]) == pos.y
            && pkt[5] == pos.z
            && pkt[6] == 3
    }

    #[test]
    fn logout_broadcasts_poff_not_blockhit_to_spectators() {
        let mut world = minimal_world();
        let spectator_pos = Position::new(100, 100, 7);
        let leaver_pos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, spectator_pos, 100);
        ensure_walkable_tile(&mut world.map, leaver_pos, 100);

        let spectator_conn = ConnId(1);
        let _spectator = insert_spectator_player(
            &mut world,
            spectator_conn,
            test_player("Watcher", spectator_pos),
        );
        let leaver =
            insert_spectator_player(&mut world, ConnId(2), test_player("Leaver", leaver_pos));

        world.pending_outgoing.clear();
        world.broadcast_player_logout_poff(leaver);

        let pkts = world
            .pending_outgoing
            .get(&spectator_conn)
            .expect("spectator must receive logout packets");
        assert!(
            pkts.iter().any(|p| pkt_is_poff_at(p, leaver_pos)),
            "logout must broadcast CONST_ME_POFF (3), not BLOCKHIT (4); packets={pkts:?}"
        );
        assert!(
            !pkts
                .iter()
                .any(|p| p.len() >= 7 && p[0] == 0x83 && p[6] == 4),
            "logout must not send CONST_ME_BLOCKHIT"
        );
    }

    #[test]
    fn login_place_sends_add_creature_to_spectators() {
        let mut world = minimal_world();
        let spectator_pos = Position::new(100, 100, 7);
        let joiner_pos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, spectator_pos, 100);
        ensure_walkable_tile(&mut world.map, joiner_pos, 100);

        let spectator_conn = ConnId(1);
        let _spectator = insert_spectator_player(
            &mut world,
            spectator_conn,
            test_player("Watcher", spectator_pos),
        );
        let joiner = insert_player(&mut world, test_player("Joiner", joiner_pos));

        world.pending_outgoing.clear();
        let placed = world
            .place_player_on_login(joiner, joiner_pos, 1)
            .expect("login place");
        assert_eq!(placed, joiner_pos);

        let pkts = world
            .pending_outgoing
            .get(&spectator_conn)
            .expect("spectator must receive login appear");
        assert!(
            pkts.iter().any(|p| !p.is_empty() && p[0] == 0x6A),
            "login placeCreature must send AddCreature (0x6A) to nearby clients; packets={pkts:?}"
        );
        assert!(
            pkts.iter().any(|p| {
                p.len() >= 7
                    && p[0] == 0x83
                    && u16::from_le_bytes([p[1], p[2]]) == joiner_pos.x
                    && u16::from_le_bytes([p[3], p[4]]) == joiner_pos.y
                    && p[5] == joiner_pos.z
                    && p[6] == 11
            }),
            "login placeCreature must broadcast CONST_ME_TELEPORT (11); packets={pkts:?}"
        );
    }
}
