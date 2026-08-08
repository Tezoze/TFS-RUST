//! PC-4 — Fight/chase/secure mode setters + PVP gating.
//!
//! C++ reference (mechanics, `tibia-game-master/src/`):
//! - `TCombat::SetAttackMode` — `crcombat.cc:325-337` (change → `DelayAttack(2000)`).
//! - `TCombat::SetChaseMode` — `crcombat.cc:339-346` (NONE/CLOSE only).
//! - `TCombat::SetSecureMode` — `crcombat.cc:348-355` (DISABLED/ENABLED only).
//! - `TCreature::BlockLogout` — `crmain.cc:433-453` (sets `EarliestLogoutRound` +
//!   `EarliestProtectionZoneRound`).
//! - `TPlayer::AttackStimulus` — `crplayer.cc:407-410` (`BlockLogout(60, false)` on being targeted).
//! - `TCombat::SetAttackDest` !Follow arm — `crcombat.cc:432-437` (AttackStimulus + Master BlockLogout).
//! - Secure-mode gate — `crcombat.cc:374-381` (`SetAttackDest` `!Follow`) + `:563-568` (`Attack`).
//!
//! Skull marks (`IsAttackJustified`, `RecordAttack`, …) live in [`super::skulls`].
//! `RecordMurder` / assigning `PlayerkillerEnd` remain P2.

use tfs_rust_common::WorldType;

use crate::combat::math::FightMode;
use crate::creature::{ChaseMode, CreatureKind};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::player_flags::{
    has_player_flag, PLAYER_FLAG_CANNOT_USE_COMBAT, PLAYER_FLAG_CANNOT_BE_ATTACKED,
};

impl GameWorld {
    /// 772 `0xA7` `FIGHT_MODES` packet body — `SetAttackMode` + `SetChaseMode` + `SetSecureMode`
    /// (`crcombat.cc:325-355`, `receiving.cc` `FIGHT_MODES`).
    ///
    /// - `SetAttackMode`: validates `OFFENSIVE`/`BALANCED`/`DEFENSIVE`; on change, calls
    ///   `DelayAttack(2000)` before writing `AttackMode` (`crcombat.cc:333-336`).
    /// - `SetChaseMode`: validates `NONE`/`CLOSE`; writes `ChaseMode` (`crcombat.cc:339-345`).
    ///   Does **not** override `Close` forced by an active follow (`Following ⇒ CLOSE`).
    /// - `SetSecureMode`: validates `DISABLED`/`ENABLED`; writes `SecureMode`
    ///   (`crcombat.cc:348-354`).
    pub(crate) fn player_set_fight_modes(
        &mut self,
        cid: CreatureId,
        raw_fight_mode: u8,
        raw_chase_mode: u8,
        raw_secure_mode: u8,
    ) {
        let server_ms = self.server_ms;

        // `SetAttackMode` — `crcombat.cc:325-337`.
        let new_attack_mode = FightMode::from_wire(raw_fight_mode);
        let attack_mode_changed = self.creatures.get(cid).is_some_and(|k| match k {
            CreatureKind::Player(p) => p.attack_mode != new_attack_mode,
            _ => false,
        });
        if attack_mode_changed {
            // `DelayAttack(2000)` before writing the new mode (`crcombat.cc:334`).
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().delay_attack_ms(server_ms, 2000);
            }
        }

        // `SetChaseMode` — `crcombat.cc:339-346` (only NONE/CLOSE accepted).
        // L2 — 772 logs and returns without writing on invalid values; Rust previously
        // clamped to NONE, which reset a valid chase mode on a malformed `0xA7` byte.
        let chase = match raw_chase_mode {
            0 => ChaseMode::None,
            1 => ChaseMode::Close,
            other => {
                tracing::warn!(
                    conn_id = ?cid,
                    raw_chase_mode = other,
                    "FightModes: 772 SetChaseMode only accepts NONE(0)/CLOSE(1); ignoring"
                );
                if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                    p.attack_mode = new_attack_mode;
                }
                return;
            }
        };

        // `SetSecureMode` — `crcombat.cc:348-355` (only DISABLED/ENABLED accepted).
        // L2 — same: log and return without writing on invalid values.
        let secure = match raw_secure_mode {
            0 => false,
            1 => true,
            other => {
                tracing::warn!(
                    conn_id = ?cid,
                    raw_secure_mode = other,
                    "FightModes: 772 SetSecureMode only accepts DISABLED(0)/ENABLED(1); ignoring"
                );
                if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                    p.attack_mode = new_attack_mode;
                    if p.base.follow_target.is_none() {
                        p.base.chase_mode = chase;
                    }
                }
                return;
            }
        };

        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.attack_mode = new_attack_mode;
            // Do not override `Close` forced by an active follow (`Following ⇒ CLOSE`).
            if p.base.follow_target.is_none() {
                p.base.chase_mode = chase;
            }
            p.secure_mode = secure;
        }
    }

    /// Ordinary combat / aggressive-spell logout block — delay from `config.lua` `pzLocked`.
    ///
    /// 772 callers hardcode `BlockLogout(60, …)`; we map `pzLocked` ms → rounds
    /// ([`PvpConfig::pz_locked_rounds`](crate::config::PvpConfig::pz_locked_rounds)).
    pub(crate) fn player_block_logout_infight(&mut self, cid: CreatureId, block_pz: bool) {
        let delay = self.pvp_config.pz_locked_rounds();
        self.player_block_logout(cid, delay, block_pz);
    }

    /// 772 `TPlayer::AttackStimulus` — `crplayer.cc:407-410`.
    ///
    /// Fired by `SetAttackDest(!Follow)` on the **target** (`crcombat.cc:433`) when
    /// `AttackDest` changes — for monsters, that is the idle walk prelude
    /// (`crnonpl.cc:2784`), not strategy `Target = …`. No-op for non-players / dead.
    pub(crate) fn player_attack_stimulus(&mut self, cid: CreatureId) {
        let alive_player = self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Player(_)) && k.base().health > 0
        });
        if alive_player {
            self.player_block_logout_infight(cid, false);
        }
    }

    /// 772 `TCombat::SetAttackDest` success arm for `!Follow` — `crcombat.cc:432-437`.
    ///
    /// Call only when `AttackDest` actually changes (C++ early-outs on same dest+follow).
    /// `Target->AttackStimulus` then `Master->BlockLogout(60, Target->Type == PLAYER)`.
    pub(crate) fn combat_on_attack_dest_changed(&mut self, master: CreatureId, target: CreatureId) {
        self.player_attack_stimulus(target);
        let target_is_player =
            matches!(self.creatures.get(target), Some(CreatureKind::Player(_)));
        self.player_block_logout_infight(master, target_is_player);
    }

    /// Unjustified PvP / white-skull logout block — delay from `config.lua` `whiteSkullTime`.
    ///
    /// 772 `BlockLogout(900, true)` on player kill (`crmain.cc:823`); TFS extends
    /// `CONDITION_INFIGHT` by `WHITE_SKULL_TIME * 1000` (`player.cpp:3671-3673`).
    pub(crate) fn player_block_logout_white_skull(&mut self, cid: CreatureId) {
        let delay = self.pvp_config.white_skull_rounds();
        self.player_block_logout(cid, delay, true);
    }

    /// 772 `TCreature::BlockLogout` — `crmain.cc:433-453` + `CheckState` (`crplayer.cc:1246`).
    ///
    /// Sets `EarliestLogoutRound = max(., RoundNr + Delay)` and, when `block_pz` is true (or
    /// the player already has a pending PZ block), `EarliestProtectionZoneRound = max(.,
    /// RoundNr + Delay)`. In `NON_PVP` worlds, `block_pz` is cleared (`crmain.cc:434-436`).
    /// Also refreshes TFS-domain `CONDITION_INFIGHT` (scripts / `ICON_SWORDS`) and sends icons
    /// (`CheckState` / `Player::sendIcons`).
    /// Skipped for non-players and for `PlayerFlag_NotGainInFight` / 772 `NO_LOGOUT_BLOCK`
    /// (`crmain.cc:438`, TFS `Player::addInFightTicks` — `player.cpp:2246`).
    pub(crate) fn player_block_logout(&mut self, cid: CreatureId, delay_rounds: u32, block_pz: bool) {
        if self.player_has_flag(cid, crate::player_flags::PLAYER_FLAG_NOT_GAIN_IN_FIGHT) {
            return;
        }
        let world_type = self.pvp_config.world_type;
        let round_nr = self.round_nr;
        // `NON_PVP` clears `BlockProtectionZone` (`crmain.cc:434-436`).
        let block_pz = block_pz && world_type != WorldType::NoPvp;
        // Capture before `get_mut` — failsafe uses Connection==NULL (`crmain.cc:444-448`).
        let has_conn = self.conn_for_creature(cid).is_some();

        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) else {
            return;
        };

        // `EarliestProtectionZoneRound` — only extended when `block_pz` or already pending
        // (`crmain.cc:439-443`).
        let in_pz_branch = block_pz || p.earliest_protection_zone_round > round_nr;
        if in_pz_branch {
            let earliest = round_nr.saturating_add(delay_rounds);
            if p.earliest_protection_zone_round < earliest {
                p.earliest_protection_zone_round = earliest;
            }
        } else if !has_conn {
            // Disconnected failsafe — do not extend `EarliestLogoutRound`.
            return;
        }

        // `EarliestLogoutRound` — always extended when not caught by the failsafe above.
        let earliest = round_nr.saturating_add(delay_rounds);
        if p.earliest_logout_round < earliest {
            p.earliest_logout_round = earliest;
        }

        let remaining = p.earliest_logout_round.saturating_sub(round_nr);
        if remaining == 0 {
            return;
        }

        // TFS `addInFightTicks` domain — `CONDITION_INFIGHT` for scripts + swords icon.
        // Duration tracks the pending logout round (same clock as 772 `EarliestLogoutRound`).
        let remaining_i32 = remaining.min(i32::MAX as u32) as i32;
        let ticks_ms = remaining_i32.saturating_mul(1000);
        let cond = crate::condition::ActiveCondition {
            id: 0,
            sub_id: 0,
            ctype: tfs_rust_common::enums::ConditionType::Infight,
            data: crate::condition::ConditionData::Generic { ticks: ticks_ms },
            timer_rounds_left: Some(remaining_i32),
        skill_count: 0,
        skill_max_count: 0,
        };
        crate::combat::apply_condition(&mut self.creatures, cid, cond);
        self.on_condition_started(cid, tfs_rust_common::enums::ConditionType::Infight);
    }

    /// Secure-mode PVP gate — `crcombat.cc:374-381` (`SetAttackDest` `!Follow`) + `:563-568`
    /// (`Attack`). Returns `true` when the attack is blocked by secure mode.
    ///
    /// Fires when: attacker is a player, target is a player, `SecureMode == ENABLED`,
    /// `WorldType == Pvp`, and `!IsAttackJustified(target)`. In `PvpEnforced` / `NoPvp` worlds,
    /// secure mode does not gate (the `WorldType == NORMAL` check in C++ maps to `== Pvp`).
    pub(crate) fn player_secure_mode_blocks_attack(
        &self,
        attacker: CreatureId,
        target: CreatureId,
    ) -> bool {
        if self.pvp_config.world_type != WorldType::Pvp {
            return false;
        }
        let (attacker_secure, both_players) = match (
            self.creatures.get(attacker),
            self.creatures.get(target),
        ) {
            (Some(CreatureKind::Player(a)), Some(CreatureKind::Player(_))) => (a.secure_mode, true),
            _ => return false,
        };
        both_players && attacker_secure && !self.player_is_attack_justified(attacker, target)
    }

    /// `CheckRight(NO_ATTACK)` equivalent — `crcombat.cc:391-394,589-593`. Returns `true` when
    /// the player has the `CannotUseCombat` group flag (772 `NO_ATTACK` right), blocking all
    /// attack actions. Non-players are never blocked.
    pub(crate) fn player_attack_blocked_by_right(&self, cid: CreatureId) -> bool {
        let flags = self.player_group_flags(cid);
        has_player_flag(flags, PLAYER_FLAG_CANNOT_USE_COMBAT)
    }

    /// M1 — `CheckRight(target, INVULNERABLE)` equivalent — `crmain.cc:536-538`. Returns `true`
    /// when the target player has the `CannotBeAttacked` group flag (772 `INVULNERABLE` right),
    /// which zeroes incoming damage. Non-players are never invulnerable via this check
    /// (monsters use race-data immunities, handled separately in `combat_execute_with_stimulus`).
    pub(crate) fn player_is_invulnerable(&self, target: CreatureId) -> bool {
        let flags = self.player_group_flags(target);
        has_player_flag(flags, PLAYER_FLAG_CANNOT_BE_ATTACKED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PvpConfig;
    use crate::creature::{CreatureKind, MonsterState};
    use crate::ids::CreatureId;
    use tfs_rust_common::WorldType;

    /// Helper: build a minimal `GameWorld` with the given `WorldType` for PVP-gate tests.
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
        player.guid = name.len() as u32; // unique guid per player
        world.creatures.insert(CreatureKind::Player(player))
    }

    #[test]
    fn secure_mode_blocks_in_pvp_world() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let t = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(world.player_secure_mode_blocks_attack(a, t));
    }

    #[test]
    fn secure_mode_does_not_block_in_no_pvp_world() {
        let mut world = make_pvp_world(WorldType::NoPvp);
        let a = insert_player(&mut world, "alice");
        let t = insert_player(&mut world, "bob");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        // WorldType != Pvp → secure mode gate does not fire.
        assert!(!world.player_secure_mode_blocks_attack(a, t));
    }

    #[test]
    fn secure_mode_off_does_not_block() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let t = insert_player(&mut world, "bob");
        // secure_mode defaults to false.
        assert!(!world.player_secure_mode_blocks_attack(a, t));
    }

    #[test]
    fn secure_mode_does_not_block_monster_target() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        let m = world.creatures.insert(CreatureKind::Monster(
            crate::creature::Monster::new(
                crate::sim_harness::minimal_creature_base(),
                tfs_rust_common::Position::new(1, 1, 7),
            ),
        ));
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.secure_mode = true;
        }
        assert!(!world.player_secure_mode_blocks_attack(a, m));
    }

    #[test]
    fn block_logout_extends_earliest_logout_round() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        world.register_conn_mapping(tfs_rust_common::ConnId(1), a);
        world.player_block_logout(a, 60, false);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 160);
            assert_eq!(p.earliest_protection_zone_round, 0);
        }
    }

    #[test]
    fn block_logout_with_pz_block_extends_protection_zone_round() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        world.player_block_logout(a, 60, true);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 160);
            assert_eq!(p.earliest_protection_zone_round, 160);
        }
    }

    #[test]
    fn block_logout_skipped_for_not_gain_in_fight() {
        use std::collections::HashMap;
        use tfs_rust_content::groups::{Group, GroupDatabase};

        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let mut flag_map = HashMap::new();
        flag_map.insert("notgaininfight".to_string(), true);
        world.groups = std::sync::Arc::new(GroupDatabase {
            groups: HashMap::from([(
                6u16,
                Group {
                    id: 6,
                    name: "god".to_string(),
                    access: true,
                    max_depot_items: 0,
                    max_vip_entries: 0,
                    flags: flag_map,
                },
            )]),
        });
        let a = insert_player(&mut world, "gm");
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.group_id = 6;
        }
        world.player_block_logout(a, 60, true);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 0);
            assert_eq!(p.earliest_protection_zone_round, 0);
        }
    }

    #[test]
    fn block_logout_pz_block_cleared_in_no_pvp_world() {
        let mut world = make_pvp_world(WorldType::NoPvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        world.register_conn_mapping(tfs_rust_common::ConnId(1), a);
        // block_pz=true but WorldType == NoPvp → PZ block cleared.
        world.player_block_logout(a, 60, true);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 160);
            assert_eq!(p.earliest_protection_zone_round, 0);
        }
    }

    #[test]
    fn block_logout_skips_logout_round_when_disconnected() {
        // `Connection == NULL` failsafe (`crmain.cc:444-448`) — do not extend
        // EarliestLogoutRound for a dead-connection body (unless PZ-block branch).
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        // No conn mapping → Connection == NULL.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.earliest_logout_round = 130;
        }
        world.player_block_logout(a, 60, false);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(
                p.earliest_logout_round, 130,
                "disconnected failsafe must not extend EarliestLogoutRound"
            );
        }
    }

    #[test]
    fn block_logout_takes_max_of_existing_and_new() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, a);
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.earliest_logout_round = 200;
        }
        world.player_block_logout(a, 60, false);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            // max(200, 100+60=160) = 200.
            assert_eq!(p.earliest_logout_round, 200);
        }
    }

    #[test]
    fn block_logout_infight_uses_pz_locked_from_config() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        world.pvp_config.pz_locked_ms = 30_000; // 30s → 30 rounds
        let a = insert_player(&mut world, "alice");
        world.register_conn_mapping(tfs_rust_common::ConnId(1), a);
        world.player_block_logout_infight(a, false);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.earliest_logout_round, 130);
            assert!(
                p.base
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == tfs_rust_common::enums::ConditionType::Infight),
                "Infight condition applied for TFS domain / swords icon"
            );
        }
    }

    #[test]
    fn logout_denied_while_earliest_logout_round_pending() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let a = insert_player(&mut world, "alice");
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, a);
        // Ensure a tile exists so the zone check path runs.
        let pos = tfs_rust_common::Position::new(0, 0, 7);
        crate::sim_harness::ensure_walkable_tile(
            &mut world.map,
            pos,
            crate::sim_harness::TEST_SYNTHETIC_GROUND_WP,
        );
        world.player_block_logout(a, 60, false);
        assert!(
            !world.player_logout_allowed(conn, a, false),
            "772 LogoutPossible must deny while EarliestLogoutRound > RoundNr"
        );
        // Icons packet must be queued for the client (`0xA2` + swords bit).
        let queued = world
            .pending_outgoing
            .get(&conn)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        assert!(
            queued.iter().any(|pkt| pkt.len() >= 2 && pkt[0] == 0xA2 && (pkt[1] & 0x80) != 0),
            "expected 0xA2 icons update with ICON_SWORDS (0x80), got {queued:?}"
        );
        world.round_nr = 160;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.earliest_logout_round = 0;
            p.base
                .active_conditions
                .retain(|c| c.ctype != tfs_rust_common::enums::ConditionType::Infight);
        }
        assert!(world.player_logout_allowed(conn, a, false));
    }

    #[test]
    fn monster_damage_locks_player_victim_logout_and_icons() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let player = insert_player(&mut world, "alice");
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);
        let monster = world.creatures.insert(CreatureKind::Monster(
            crate::creature::Monster::new(
                crate::sim_harness::minimal_creature_base(),
                tfs_rust_common::Position::new(1, 1, 7),
            ),
        ));
        let applied = world.combat_execute_with_stimulus(
            Some(monster),
            player,
            &crate::combat::CombatDamage {
                primary: (tfs_rust_common::enums::CombatType::Physical, -10),
                secondary: (tfs_rust_common::enums::CombatType::Physical, 0),
            },
            &crate::combat::CombatParams::default(),
        );
        assert!(applied > 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(player) {
            assert!(
                p.earliest_logout_round > 100,
                "772 Attack Target->BlockLogout must lock the player victim"
            );
            assert!(
                p.base
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == tfs_rust_common::enums::ConditionType::Infight),
                "Infight must apply for swords icon"
            );
        }
        let queued = world
            .pending_outgoing
            .get(&conn)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        assert!(
            queued
                .iter()
                .any(|pkt| pkt.len() >= 2 && pkt[0] == 0xA2 && (pkt[1] & 0x80) != 0),
            "expected 0xA2 ICON_SWORDS when player is hit, got {queued:?}"
        );
    }

    #[test]
    fn monster_idle_set_attack_dest_locks_player_before_hit() {
        // 772 idle walk: fist>0 → ATTACKING → SetAttackDest → AttackStimulus
        // (`crnonpl.cc:2778-2784`, `crcombat.cc:433`) — not on Target assign / selectTarget.
        let mut world = make_pvp_world(WorldType::Pvp);
        world.round_nr = 100;
        let player_pos = tfs_rust_common::Position::new(100, 100, 7);
        let monster_pos = tfs_rust_common::Position::new(102, 100, 7);
        crate::sim_harness::ensure_walkable_tile(
            &mut world.map,
            player_pos,
            crate::sim_harness::TEST_SYNTHETIC_GROUND_WP,
        );
        crate::sim_harness::ensure_walkable_tile(
            &mut world.map,
            monster_pos,
            crate::sim_harness::TEST_SYNTHETIC_GROUND_WP,
        );
        let mut player = crate::sim_harness::test_player("alice", player_pos);
        player.guid = 1;
        let player = world.creatures.insert(CreatureKind::Player(player));
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);
        world.map.register_creature_at(player_pos, player);

        let monster = crate::sim_harness::insert_monster(&mut world, "Rat", monster_pos, 100);
        world.map.register_creature_at(monster_pos, monster);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(monster) {
            m.melee_skill = 10;
            m.is_hostile = true;
            m.base.follow_target = Some(player);
            m.base.attack_target = None;
            m.state = MonsterState::Sleeping;
        }

        world.monster_idle_maybe_enter_attacking(monster);

        assert_eq!(
            world.creatures.get(monster).and_then(|k| k.base().attack_target),
            Some(player),
            "SetAttackDest must copy Target → AttackDest"
        );
        if let Some(CreatureKind::Player(p)) = world.creatures.get(player) {
            assert!(
                p.earliest_logout_round > 100,
                "AttackStimulus must lock player on idle SetAttackDest"
            );
            assert!(
                p.base
                    .active_conditions
                    .iter()
                    .any(|c| c.ctype == tfs_rust_common::enums::ConditionType::Infight),
                "Infight must apply for swords icon on SetAttackDest"
            );
        }
        let queued = world
            .pending_outgoing
            .get(&conn)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        assert!(
            queued
                .iter()
                .any(|pkt| pkt.len() >= 2 && pkt[0] == 0xA2 && (pkt[1] & 0x80) != 0),
            "expected 0xA2 ICON_SWORDS when monster SetAttackDest, got {queued:?}"
        );

        // Same dest again → early-out, no error / still locked.
        let logout_round = world
            .creatures
            .get(player)
            .and_then(|k| match k {
                CreatureKind::Player(p) => Some(p.earliest_logout_round),
                _ => None,
            })
            .unwrap_or(0);
        world.monster_idle_maybe_enter_attacking(monster);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(player) {
            assert_eq!(p.earliest_logout_round, logout_round);
        }
    }

    #[test]
    fn set_fight_modes_writes_attack_mode_with_delay_on_change() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.server_ms = 5000;
        let a = insert_player(&mut world, "alice");
        // Default attack_mode is Balanced (wire byte 2). Change to Offensive (1).
        world.player_set_fight_modes(a, 1, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.attack_mode, FightMode::Offensive);
            // DelayAttack(2000) on change — `crcombat.cc:334`.
            assert_eq!(p.base.earliest_attack_ms, 7000);
        }
    }

    #[test]
    fn set_fight_modes_no_delay_when_attack_mode_unchanged() {
        let mut world = make_pvp_world(WorldType::Pvp);
        world.server_ms = 5000;
        let a = insert_player(&mut world, "alice");
        // Default is Balanced (2). Setting Balanced again → no DelayAttack.
        world.player_set_fight_modes(a, 2, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.attack_mode, FightMode::Balanced);
            assert_eq!(p.base.earliest_attack_ms, 0);
        }
    }

    #[test]
    fn set_fight_modes_writes_secure_mode() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        world.player_set_fight_modes(a, 2, 0, 1);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert!(p.secure_mode);
        }
        // Toggle back off.
        world.player_set_fight_modes(a, 2, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert!(!p.secure_mode);
        }
    }

    #[test]
    fn set_fight_modes_does_not_override_chase_when_following() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        // Simulate active follow → chase_mode forced to Close.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(a) {
            p.base.follow_target = Some(a); // dummy target
            p.base.chase_mode = ChaseMode::Close;
        }
        // Player sends chase_mode = NONE (0); should not override Close while following.
        world.player_set_fight_modes(a, 2, 0, 0);
        if let Some(CreatureKind::Player(p)) = world.creatures.get(a) {
            assert_eq!(p.base.chase_mode, ChaseMode::Close);
        }
    }

    #[test]
    fn invulnerable_check_false_for_empty_groups() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        // Empty GroupDatabase → no flags → not invulnerable.
        assert!(!world.player_is_invulnerable(a));
    }

    #[test]
    fn attack_blocked_by_right_false_for_empty_groups() {
        let mut world = make_pvp_world(WorldType::Pvp);
        let a = insert_player(&mut world, "alice");
        assert!(!world.player_attack_blocked_by_right(a));
    }

    /// NoLogout is flag-only — `zone` stays Normal; logout must still deny
    /// (`crmain.cc` `IsNoLogoutField`; OTBM sets `TILESTATE_NOLOGOUT` only).
    #[test]
    fn logout_denied_on_nologout_flag_with_normal_zone() {
        use crate::game_world_lifecycle::LogoutPossible;
        use crate::tile::{flags as tilestate, Tile, TileBody};
        use tfs_rust_common::enums::ZoneType;

        let mut world = make_pvp_world(WorldType::Pvp);
        let pos = tfs_rust_common::Position::new(0, 0, 7);
        world.map.insert_tile(
            pos,
            Tile::Normal(TileBody {
                ground: Some(crate::sim_harness::TEST_SYNTHETIC_GROUND_WP),

                ground_item: None,
                down_items: Vec::new(),
                top_items: Vec::new(),
                creatures: Vec::new(),
                flags: tilestate::NOLOGOUT,
                zone: ZoneType::Normal,
            }),
        );
        let a = insert_player(&mut world, "alice");
        assert_eq!(
            world.player_logout_possible(a),
            LogoutPossible::NoLogoutField
        );
    }
}
